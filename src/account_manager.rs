use hashbrown::HashMap;

use log::*;

use crate::{
    client_account::ClientAccount,
    records::{ClientId, TransactionRecord},
    transactions_reader::TransactionsStream,
};

pub trait AccountManager {
    fn execute_transactions(&mut self, transactions: TransactionsStream);

    fn report(&self);
}

/// Manages client accounts by processing transactions
pub struct STAccountManager {
    /// A "database" of client accounts
    accounts: HashMap<ClientId, ClientAccount>,
}

fn report_headers() {
    // since row ordering doens't matter, just loop the hashmap
    // formatting should be nice if the values are not extremly large
    println!("client,     available,          held,         total,   locked");
}

impl AccountManager for STAccountManager {
    fn execute_transactions(&mut self, transactions: TransactionsStream) {
        for record in transactions {
            debug!("Processing transaction record: {:?}", record);
            let client = self.get_or_create_account(record.client);

            // TODO: remove boilerplate error handling duplicates
            // Just match the proper transaction and log if there's an error
            let processing_result = match record.tr_type {
                crate::records::TransactionType::Deposit => match record.amount {
                    Some(amount) => client.deposit(record.tx, amount),
                    None => Err(anyhow::anyhow!("Transaction failed due to missing amount")),
                },
                crate::records::TransactionType::Withdrawal => match record.amount {
                    Some(amount) => client.withdraw(record.tx, amount),
                    None => Err(anyhow::anyhow!("Transaction failed due to missing amount")),
                },
                crate::records::TransactionType::Dispute => client.dispute(record.tx),
                crate::records::TransactionType::Resolve => client.resolve(record.tx),
                crate::records::TransactionType::ChargeBack => client.chargeback(record.tx),
            };

            if let Err(err) = processing_result {
                error!("Transaction failed. {} | {:?}", err, record);
            }
        }
    }

    /// Reports the status of all accounts to the stdout
    fn report(&self) {
        report_headers();
        self.report_accounts();

    }
}

impl STAccountManager {
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

    fn report_accounts(&self) {
        for (_, account) in &self.accounts {
            println!("{}", account);
        }
    }
}

/// Account manager, but multithreaded
/// Schedules work to other managers
pub struct MTAccountManager {
    num_threads: usize,
    managers: Vec<STAccountManager>,
}

impl AccountManager for MTAccountManager {
    fn execute_transactions(&mut self, transactions: TransactionsStream) {
        let mut handles = Vec::new();
        let mut tx_queues = Vec::new();
        for _ in 0..self.num_threads {
            let (queue_tx, queue_rx) = crossbeam_channel::unbounded::<TransactionRecord>();
            tx_queues.push(queue_tx);
            let handle = std::thread::spawn(move || {
                // use the single threaded manager here
                let mut manager = STAccountManager::new();
                manager.execute_transactions(Box::new(queue_rx.into_iter()));

                // return the accounts managed the single threaded managers
                manager
            });

            handles.push(handle);
        }

        // use a simple round robin strategy, but make sure the same client is always managed by the same thread
        for record in transactions {
            // println!("{}", record.tx);
            if tx_queues[(record.client % self.num_threads as u16) as usize]
                .send(record)
                .is_err()
            {
                break;
            };
        }
        // tell the workers that there's no more work
        drop(tx_queues);

        for handle in handles {
            if let Ok(manager) = handle.join() {
                self.managers.push(manager);
            } else {
                error!("A manager panicked. Information lost");
            }
        }
    }

    fn report(&self) {
        report_headers();
        for manager in &self.managers {
            manager.report_accounts();
        }
    }
}

impl MTAccountManager {
    pub fn new(num_threads: usize) -> Self {
        Self { num_threads,
        managers: Vec::new(), }
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
        let mut manager = STAccountManager::new();
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
