/// TBD: either held or total can be removed
pub struct ClientAccount {
    id: u16,
    available: f32,
    /// TBD: Not needed since can be computed from available
    held: f32,
    /// TBD: Not needed, computed from available + held
    total: f32,
    locked: bool,
}


impl ClientAccount {
    /// Constructs a new client account with an id
    pub fn new(id: u16) -> Self {
        Self {
            id,
            available: 0.0,
            held: 0.0,
            total: 0.0,
            locked: false,
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
        self.total
    }

    /// Check if the account is frozen
    pub fn is_locked(&self) -> bool {
        self.locked
    }

    /// Deposits `amount` to the account with a specific transaction id
    pub fn deposit(&mut self, transaction_id: u32, amount: f32) {
        self.available += amount;
        self.total += amount;
    }

    /// Withdraws `amount` from the account with a specific transaction id
    /// Returns an `Err` if no there are no sufficient funds
    pub fn withdraw(&mut self, transaction_id: u32, amount: f32) -> anyhow::Result<()> {
        if self.available > amount {
            self.available -= amount;
            self.total -= amount;
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Insufficient funds. Available {}",
                self.available
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

        client.deposit(1, 20.00);
        client.deposit(2, 35.00);

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
}
