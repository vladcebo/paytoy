use std::fmt::Display;

use hashbrown::HashMap;

use anyhow::Context;
use rust_decimal::Decimal;

use crate::records::{ClientId, TransactionId};

/// Represents a state of a transaction dispute
#[derive(PartialEq, Debug)]
enum DisputeProgress {
    /// Transaction is not disputed
    Idle,
    /// Transaction dispute in progress
    InProgress,
    /// Transaction is either resolved or is chargedback
    Done,
}

/// A historical transaction stored in a database
struct TransactionHist {
    /// State of the transaction
    state: DisputeProgress,
    /// Amount of money involved
    amount: Decimal,
}

impl TransactionHist {
    fn new(amount: Decimal) -> Self {
        Self {
            state: DisputeProgress::Idle,
            amount,
        }
    }
}

/// Represents a client account where transactions can be performed
pub struct ClientAccount {
    /// Unique identifier for the client account
    /// Not really needed since the manager knows everything about ids
    id: ClientId,
    /// Total available funds (for trading etc.)
    available: Decimal,
    /// Total held funds
    held: Decimal,
    /// Frozen account
    locked: bool,

    /// Stores all the historical transactions since we should be able to dispute them
    /// In reality that would be some kind of a database, but a hashmap should work for the moment
    transaction_history: HashMap<TransactionId, TransactionHist>,
}

impl ClientAccount {
    /// Constructs a new client account with an id
    pub fn new(id: ClientId) -> Self {
        Self {
            id,
            available: Decimal::ZERO,
            held: Decimal::ZERO,
            locked: false,

            transaction_history: HashMap::new(),
        }
    }

    /// Get the account id
    #[allow(dead_code)]
    pub fn id(&self) -> ClientId {
        self.id
    }

    /// Get the available funds
    pub fn available(&self) -> Decimal {
        self.available
    }

    /// Get the held funds
    pub fn held(&self) -> Decimal {
        self.held
    }

    /// Get the total funds
    pub fn total(&self) -> Decimal {
        self.available + self.held
    }

    /// Check if the account is frozen
    pub fn is_locked(&self) -> bool {
        self.locked
    }

    /// Deposits `amount` to the account with a specific transaction id
    /// Returns an `Error` in case the transaction already exists
    pub fn deposit(
        &mut self,
        transaction_id: TransactionId,
        amount: Decimal,
    ) -> anyhow::Result<()> {
        if self.transaction_history.contains_key(&transaction_id) {
            return Err(anyhow::anyhow!("Transaction already exists",));
        }

        self.available += amount;
        self.transaction_history
            .insert(transaction_id, TransactionHist::new(amount));

        Ok(())
    }

    /// Withdraws `amount` from the account with a specific transaction id
    /// Returns an `Error` if no there are no sufficient funds or the transaction already exists
    pub fn withdraw(
        &mut self,
        transaction_id: TransactionId,
        amount: Decimal,
    ) -> anyhow::Result<()> {
        if self.transaction_history.contains_key(&transaction_id) {
            return Err(anyhow::anyhow!("Transaction already exists",));
        }

        if amount > self.available {
            return Err(anyhow::anyhow!(
                "Insufficient funds. Requested {} but available {}",
                amount,
                self.available
            ));
        }

        self.available -= amount;
        // No need to save history for withdrawals since they're not disputed
        // self.transaction_history
        //     .insert(transaction_id, TransactionHist::new(amount));

        Ok(())
    }

    /// Represents a client claim to reverse a transaction
    /// Makes available funds decrease by the disputed amount and held funds increase
    /// Returns an `Error` in case there is no such transaction with the specified id
    /// or if the transaction is already disputed
    pub fn dispute(&mut self, transaction_id: TransactionId) -> anyhow::Result<()> {
        let transaction = self
            .transaction_history
            .get_mut(&transaction_id)
            .with_context(|| "A deposit transaction with such id does not exist")?;

        if transaction.state != DisputeProgress::Idle {
            return Err(anyhow::anyhow!("Dispute already in progress or done"));
        }

        if transaction.amount > self.available {
            return Err(anyhow::anyhow!("Not enough funds to open a dispute"));
        }

        self.available -= transaction.amount;
        self.held += transaction.amount;
        transaction.state = DisputeProgress::InProgress;

        Ok(())
    }

    /// Represents a resolved dispute
    /// Makes available funds increase by the disputed amount and held funds decrease
    /// Returns an `Error` in case there is no such transaction with the specified id
    /// or the transaction was not disputed in the first place
    pub fn resolve(&mut self, transaction_id: TransactionId) -> anyhow::Result<()> {
        let transaction = self
            .transaction_history
            .get_mut(&transaction_id)
            .with_context(|| "Transaction does not exist")?;

        if transaction.state != DisputeProgress::InProgress {
            return Err(anyhow::anyhow!(
                "Cannot resolve a transaction that is not disputed"
            ));
        }

        if transaction.amount > self.held {
            return Err(anyhow::anyhow!(
                "Not enough held funds to resolve a dispute"
            ));
        }

        self.available += transaction.amount;
        self.held -= transaction.amount;
        transaction.state = DisputeProgress::Done;

        Ok(())
    }

    /// Represents a chargeback for a dispute
    /// Final state of a dispute, funds that were held are being withdrawn
    /// Client's held funds and total funds shall decrease by the disputed amount
    /// Returns an `Error` in case there is no such transaction with the specified id
    /// or the transaction was not disputed in the first place
    pub fn chargeback(&mut self, transaction_id: TransactionId) -> anyhow::Result<()> {
        let transaction = self
            .transaction_history
            .get_mut(&transaction_id)
            .with_context(|| "Transaction does not exist")?;

        if transaction.state != DisputeProgress::InProgress {
            return Err(anyhow::anyhow!(
                "Cannot resolve a transaction that is not disputed"
            ));
        }

        if transaction.amount > self.held {
            return Err(anyhow::anyhow!(
                "Not enough held funds to chargeback a dispute"
            ));
        }

        self.held -= transaction.amount;
        self.locked = true;
        transaction.state = DisputeProgress::Done;

        Ok(())
    }
}

impl Display for ClientAccount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{:6}, {:14.4}, {:14.4}, {:14.4},     {}",
            self.id(),
            self.available(),
            self.held(),
            self.total(),
            self.is_locked()
        ))
    }
}

#[cfg(test)]
mod tests {

    use rust_decimal_macros::dec;

    use super::ClientAccount;

    /*  Basic test case for deposits and withdrawal to the account
        User scenario:
            1) Deposit 20$ and then 35$
            2) Withdraw 14$
            3) Try to withdraw 44$, cannot, since no available funds
    */
    #[test]
    fn test_deposit_and_withdrawal() {
        let mut client = ClientAccount::new(1);

        assert!(client.deposit(1, dec!(20.00)).is_ok());
        assert!(client.deposit(2, dec!(35.00)).is_ok());

        assert_eq!(client.available(), dec!(55.00));
        assert_eq!(client.total(), dec!(55.00));
        assert_eq!(client.held(), dec!(0.00));
        assert_eq!(client.is_locked(), false);

        assert!(client.withdraw(3, dec!(24.00)).is_ok());

        assert_eq!(client.available(), dec!(31.00));
        assert_eq!(client.total(), dec!(31.00));
        assert_eq!(client.held(), dec!(0.00));
        assert_eq!(client.is_locked(), false);

        assert!(client.withdraw(4, dec!(44.00)).is_err());

        // Transaction fails since we try to withdraw more than we have
        // The same amount remains
        assert_eq!(client.available(), dec!(31.00));
        assert_eq!(client.total(), dec!(31.00));
        assert_eq!(client.held(), dec!(0.00));
        assert_eq!(client.is_locked(), false);
    }

    /* User scenario:
        1) Make two deposits of 20$ and then 35$, total of 55$
        2) Realize that the deposit for 20$ was erroneous, open a dispute
        3) Now we have 35$ available and a held amount of 20$
        4) Transaction is resolved, the held money is going to available and everything good
    */
    #[test]
    fn test_dispute_resolved() {
        let mut client = ClientAccount::new(1);

        assert!(client.deposit(1, dec!(20.00)).is_ok());
        assert!(client.deposit(2, dec!(35.00)).is_ok());

        // Non-existent transaction, nothing to dispute
        assert!(client.dispute(3).is_err());
        // Can be disputed
        assert!(client.dispute(1).is_ok());

        assert_eq!(client.available(), dec!(35.00));
        assert_eq!(client.total(), dec!(55.00));
        assert_eq!(client.held(), dec!(20.00));
        assert_eq!(client.is_locked(), false);

        // Resolve step
        assert!(client.resolve(1).is_ok());

        assert_eq!(client.available(), dec!(55.00));
        assert_eq!(client.total(), dec!(55.00));
        assert_eq!(client.held(), dec!(0.00));
        assert_eq!(client.is_locked(), false);
    }

    /* User scenario:
        1) Make a deposit on 10$, available and total of 10$
        2) Open a dispute on it
        3) Now we have 0$ available, and 10$ held
        3) Chargeback occurs, account is frozen, 0$ available and total (reversed transaction)
    */
    #[test]
    fn test_dispute_chargeback() {
        let mut client = ClientAccount::new(1);

        assert!(client.deposit(1, dec!(10.00)).is_ok());
        assert!(client.dispute(1).is_ok());

        assert_eq!(client.available(), dec!(0.00));
        assert_eq!(client.total(), dec!(10.00));
        assert_eq!(client.held(), dec!(10.00));
        assert_eq!(client.is_locked(), false);

        assert!(client.chargeback(1).is_ok());

        assert_eq!(client.available(), dec!(0.00));
        assert_eq!(client.total(), dec!(0.00));
        assert_eq!(client.held(), dec!(0.00));
        assert_eq!(client.is_locked(), true);
    }

    /* User scenario:
        1) Make a deposit on 10$, available and total of 10$
        2) Withdraw 5$
        3) Make a dispute on 1
        3) Cannot dispute it, since only 5$ available
    */
    #[test]
    fn test_invalid_dispute() {
        let mut client = ClientAccount::new(1);

        assert!(client.deposit(1, dec!(10.00)).is_ok());
        assert!(client.withdraw(2, dec!(5.00)).is_ok());

        assert!(client.dispute(1).is_err());

        assert_eq!(client.available(), dec!(5.00));
        assert_eq!(client.total(), dec!(5.00));
        assert_eq!(client.held(), dec!(0.00));
        assert_eq!(client.is_locked(), false);
    }
}
