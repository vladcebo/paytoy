use std::collections::HashMap;

use log::*;

use crate::{
    client_account::ClientAccount, records::ClientId, transactions_reader::TransactionsStream,
};

/// Manages client accounts by processing transactions
pub struct AccountManager {
    /// A "database" of client accounts
    accounts: HashMap<ClientId, ClientAccount>,
}

impl AccountManager {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
        }
    }

    fn get_or_create_account(&mut self, client_id: ClientId) -> &mut ClientAccount {
        if !self.accounts.contains_key(&client_id) {
            self.accounts
                .insert(client_id, ClientAccount::new(client_id));
        }

        self.accounts
            .get_mut(&client_id)
            .expect("Invariant: we always have an account since we insert one before that")
    }

    pub fn execute_transactions(&mut self, transactions: TransactionsStream) {
        for record in transactions {
            debug!("Processing transaction record: {:?}", record);
            let client = self.get_or_create_account(record.client);

            // TODO: remove boilerplate error handling duplicates
            // Just match the proper transaction and log if there's an error
            match record.tr_type {
                crate::records::TransactionType::Deposit => match record.amount {
                    Some(amount) => {
                        if let Err(err) = client.deposit(record.tx, amount) {
                            error!("Transaction failed due to: {} | {:?}", err, record);
                        }
                    }
                    None => {
                        error!("Transaction failed due to missing amount | {:?}", record);
                    }
                },
                crate::records::TransactionType::Withdrawal => match record.amount {
                    Some(amount) => {
                        if let Err(err) = client.withdraw(record.tx, amount) {
                            error!("Transaction failed due to: {} | {:?}", err, record);
                        }
                    }
                    None => {
                        error!("Transaction failed due to missing amount {:?}", record);
                    }
                },
                crate::records::TransactionType::Dispute => {
                    if let Err(err) = client.dispute(record.tx) {
                        error!("Transaction failed due to: {} | {:?}", err, record);
                    }
                }
                crate::records::TransactionType::Resolve => {
                    if let Err(err) = client.resolve(record.tx) {
                        error!("Transaction failed due to: {} | {:?}", err, record);
                    }
                }
                crate::records::TransactionType::ChargeBack => {
                    if let Err(err) = client.chargeback(record.tx) {
                        error!("Transaction failed due to: {} | {:?}", err, record);
                    }
                }
            };
        }
    }

    /// Reports the status of all accounts to the stdout
    pub fn report(&self) {
        // since row ordering doens't matter, just loop the hashmap
        // formatting should be nice if the values are not extremly large
        println!("client,     available,          held,         total,   locked");
        for (client_id, account) in &self.accounts {
            println!(
                "{:6} {:14.4} {:14.4} {:14.4}     {}",
                client_id,
                account.available(),
                account.held(),
                account.total(),
                account.is_locked()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal_macros::dec;

    use crate::transactions_reader::{self, TransactionCSVReader};

    use super::*;

    /* Tests the provided test case returns:
        client, available, held, total, locked
        1, 1.5, 0.0, 1.5, false
        2, 2.0, 0.0, 2.0, false
    */
    #[test]
    fn test_basic_transactions() {
        let transactions = transactions_reader::STBulkReader::new()
            .read_csv("tests/data/test_basic.csv")
            .unwrap();
        let mut manager = AccountManager::new();
        manager.execute_transactions(transactions);

        let account1 = manager.accounts.get(&1).unwrap();
        let account2 = manager.accounts.get(&2).unwrap();

        assert_eq!(account1.id(), 1);
        assert_eq!(account1.available(), dec!(1.5));
        assert_eq!(account1.held(), dec!(0.0));
        assert_eq!(account1.total(), dec!(1.5));
        assert_eq!(account1.is_locked(), false);

        assert_eq!(account2.id(), 2);
        assert_eq!(account2.available(), dec!(2.0));
        assert_eq!(account2.held(), dec!(0.0));
        assert_eq!(account2.total(), dec!(2.0));
        assert_eq!(account2.is_locked(), false);
    }
}
