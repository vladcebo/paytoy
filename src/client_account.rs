use std::{any, collections::HashMap};

use anyhow::Context;

use crate::records::TransactionId;

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
    amount: f32,
}

impl TransactionHist {
    fn new(amount: f32) -> Self {
        Self {
            state: DisputeProgress::Idle,
            amount,
        }
    }
}

/// Represents a client account where transactions can be performed
pub struct ClientAccount {
    /// Unique identifier for the client account
    id: u16,
    /// Total available funds (for trading etc.)
    available: f32,
    /// Total held funds
    held: f32,
    /// Frozen account
    locked: bool,

    /// Stores all the historical transactions since we should be able to dispute them
    /// In reality that would be some kind of a database, but a hashmap should work for the moment
    transaction_history: HashMap<TransactionId, TransactionHist>,
}

impl ClientAccount {
    /// Constructs a new client account with an id
    pub fn new(id: u16) -> Self {
        Self {
            id,
            available: 0.0,
            held: 0.0,
            locked: false,

            transaction_history: HashMap::new(),
        }
    }

    /// Get the account id
    pub fn id(&self) -> u16 {
        self.id
    }

    /// Get the available funds
    pub fn available(&self) -> f32 {
        self.available
    }

    /// Get the held funds
    pub fn held(&self) -> f32 {
        self.held
    }

    /// Get the total funds
    pub fn total(&self) -> f32 {
        self.available + self.held
    }

    /// Check if the account is frozen
    pub fn is_locked(&self) -> bool {
        self.locked
    }

    /// Deposits `amount` to the account with a specific transaction id
    /// Returns an `Error` in case the transaction already exists
    pub fn deposit(&mut self, transaction_id: TransactionId, amount: f32) -> anyhow::Result<()> {
        if self.transaction_history.get(&transaction_id).is_none() {
            self.available += amount;
            self.transaction_history
                .insert(transaction_id, TransactionHist::new(amount));
            Ok(())
        } else {
            return Err(anyhow::anyhow!("Transaction already exists",));
        }
    }

    /// Withdraws `amount` from the account with a specific transaction id
    /// Returns an `Error` if no there are no sufficient funds or the transaction already exists
    pub fn withdraw(&mut self, transaction_id: TransactionId, amount: f32) -> anyhow::Result<()> {
        if self.transaction_history.get(&transaction_id).is_none() {
            if amount < self.available {
                self.available -= amount;
                self.transaction_history
                    .insert(transaction_id, TransactionHist::new(amount));
                Ok(())
            } else {
                return Err(anyhow::anyhow!(
                    "Insufficient funds. Available {}",
                    self.available
                ));
            }
        } else {
            return Err(anyhow::anyhow!("Transaction already exists",));
        }
    }

    /// Represents a client claim to reverse a transaction
    /// Makes available funds decrease by the disputed amount and held funds increase
    /// Returns an `Error` in case there is no such transaction with the specified id
    /// or if the transaction is already disputed
    pub fn dispute(&mut self, transaction_id: TransactionId) -> anyhow::Result<()> {
        let transaction = self
            .transaction_history
            .get_mut(&transaction_id)
            .with_context(|| "Transaction does not exist")?;

        if transaction.state == DisputeProgress::Idle {
            self.available -= transaction.amount;
            self.held += transaction.amount;
            transaction.state = DisputeProgress::InProgress;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Dispute already in progress or done"))
        }
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

        if transaction.state == DisputeProgress::InProgress {
            self.available += transaction.amount;
            self.held -= transaction.amount;
            transaction.state = DisputeProgress::Done;

            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Cannot resolve a transaction that is not disputed"
            ))
        }
    }
}

#[cfg(test)]
mod tests {

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

        assert!(client.deposit(1, 20.00).is_ok());
        assert!(client.deposit(2, 35.00).is_ok());

        assert_eq!(client.available(), 55.00);
        assert_eq!(client.total(), 55.00);
        assert_eq!(client.held(), 0.00);
        assert_eq!(client.is_locked(), false);

        assert!(client.withdraw(3, 24.00).is_ok());

        assert_eq!(client.available(), 31.00);
        assert_eq!(client.total(), 31.00);
        assert_eq!(client.held(), 0.00);
        assert_eq!(client.is_locked(), false);

        assert!(client.withdraw(4, 44.00).is_err());

        // Transaction fails since we try to withdraw more than we have
        // The same amount remains
        assert_eq!(client.available(), 31.00);
        assert_eq!(client.total(), 31.00);
        assert_eq!(client.held(), 0.00);
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

        assert!(client.deposit(1, 20.00).is_ok());
        assert!(client.deposit(2, 35.00).is_ok());

        // Non-existent transaction, nothing to dispute
        assert!(client.dispute(3).is_err());
        // Can be disputed
        assert!(client.dispute(1).is_ok());

        assert_eq!(client.available(), 35.00);
        assert_eq!(client.total(), 55.00);
        assert_eq!(client.held(), 20.00);
        assert_eq!(client.is_locked(), false);

        // Resolve step
        assert!(client.resolve(1).is_ok());

        assert_eq!(client.available(), 55.00);
        assert_eq!(client.total(), 55.00);
        assert_eq!(client.held(), 0.00);
        assert_eq!(client.is_locked(), false);
    }
}
