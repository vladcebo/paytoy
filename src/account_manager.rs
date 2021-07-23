use hashbrown::HashMap;

use log::*;

use crate::{
    client_account::ClientAccount,
    records::{ClientId, TransactionRecord},
    transactions_reader::TransactionsStream,
};

/// The final report after executing all the transactions
pub struct Report {
    accounts: HashMap<ClientId, ClientAccount>,
}

impl Report {
    pub fn report(&self) {
        // formatting should be nice if the values are not extremly large
        println!("client,     available,          held,         total,   locked");
        // since row ordering doens't matter, just report from individual accounts
        for (_, account) in &self.accounts {
            println!("{}", account);
        }
    }
}

pub trait AccountManager {
    /// Executes the transactions on the stream and return the report of all accounts
    fn execute_transactions(self, transactions: TransactionsStream) -> Report;
}

/// Manages client accounts by processing transactions
pub struct STAccountManager {
    /// A "database" of client accounts
    accounts: HashMap<ClientId, ClientAccount>,
}

/// A single threaded account manager
/// One single threaded (the thread where this function is called)
/// will execute all the transactions
impl AccountManager for STAccountManager {
    fn execute_transactions(mut self, transactions: TransactionsStream) -> Report {
        for record in transactions {
            debug!("Processing transaction record: {:?}", record);
            let client = self.get_or_create_account(record.client);

            if client.is_locked() {
                warn!(
                    "Account {} is locked and cannot accept more transactions | {:?}",
                    client, record
                );
                continue;
            }

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

        Report {
            accounts: self.accounts,
        }
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
}

/// Account manager, but multithreaded
/// Assigns to each thread a subset of clients, so the work can be distributed more evenly
pub struct MTAccountManager {
    num_threads: usize,
}

impl AccountManager for MTAccountManager {
    fn execute_transactions(self, transactions: TransactionsStream) -> Report {
        let mut handles = Vec::new();
        let mut tx_queues = Vec::new();
        for _ in 0..self.num_threads {
            let (queue_tx, queue_rx) = crossbeam_channel::bounded::<TransactionRecord>(10000);
            tx_queues.push(queue_tx);
            let handle = std::thread::spawn(move || {
                // use the single threaded manager here
                let manager = STAccountManager::new();
                let report = manager.execute_transactions(Box::new(queue_rx.into_iter()));

                // return the accounts managed the single threaded managers
                report
            });

            handles.push(handle);
        }

        // use a simple round robin strategy, but make sure the same client is always managed by the same thread
        for record in transactions {
            let worker_id = (record.client % self.num_threads as u16) as usize;
            trace!("Dispatching record {:?} to worker {}", record, worker_id);
            if tx_queues[worker_id].send(record).is_err() {
                break;
            };
        }
        // tell the workers that there's no more work
        drop(tx_queues);

        let mut full_report = Report {
            accounts: HashMap::with_capacity(1000),
        };

        for handle in handles {
            if let Ok(report) = handle.join() {
                for (client_id, account) in report.accounts {
                    full_report.accounts.insert(client_id, account);
                }
            } else {
                error!("A manager panicked. Information lost");
            }
        }

        full_report
    }
}

impl MTAccountManager {
    pub fn new(num_threads: usize) -> Self {
        Self { num_threads }
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;

    use crate::transactions_reader::{self, TransactionCSVReader, TransactionsStream};

    use super::*;

    /* Tests the provided test case returns:
        client, available, held, total, locked
        1, 1.5, 0.0, 1.5, false
        2, 2.0, 0.0, 2.0, false
    */

    fn test_basic_transactions(manager: impl AccountManager, transactions: TransactionsStream) {
        let report = manager.execute_transactions(transactions);

        let account1 = report.accounts.get(&1).unwrap();
        let account2 = report.accounts.get(&2).unwrap();

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

    #[test]
    fn test_basic_transactions_st() {
        let transactions = transactions_reader::STBulkReader::new()
            .read_csv("tests/data/test_basic.csv")
            .unwrap();
        let manager = STAccountManager::new();

        test_basic_transactions(manager, transactions);
    }

    #[test]
    fn test_basic_transactions_mt() {
        let transactions = transactions_reader::MTReader::new()
            .read_csv("tests/data/test_basic.csv")
            .unwrap();
        let manager = MTAccountManager::new(2);

        test_basic_transactions(manager, transactions);
    }

    // Test with a locked client
    fn test_locked_client(manager: impl AccountManager, transactions: TransactionsStream) {
        let report = manager.execute_transactions(transactions);

        let account = report.accounts.get(&1).unwrap();

        assert_eq!(account.id(), 1);
        assert_eq!(account.available(), dec!(2.5));
        assert_eq!(account.held(), dec!(0.0));
        assert_eq!(account.total(), dec!(2.5));
        assert_eq!(account.is_locked(), true);
    }

    #[test]
    fn test_basic_locked_st() {
        let transactions = transactions_reader::STBulkReader::new()
            .read_csv("tests/data/test_locked.csv")
            .unwrap();
        let manager = STAccountManager::new();

        test_locked_client(manager, transactions);
    }

    #[test]
    fn test_basic_locked_mt() {
        let transactions = transactions_reader::MTReader::new()
            .read_csv("tests/data/test_locked.csv")
            .unwrap();
        let manager = MTAccountManager::new(2);

        test_locked_client(manager, transactions);
    }

    #[test]
    fn test_correctness() {
        let transactions = transactions_reader::STBulkReader::new()
            .read_csv("tests/data/test_correctnes.csv")
            .unwrap();
        let manager = STAccountManager::new();

        let st_report = manager.execute_transactions(transactions);

        let transactions = transactions_reader::MTReader::new()
            .read_csv("tests/data/test_correctnes.csv")
            .unwrap();
        let manager = MTAccountManager::new(2);

        let mt_report = manager.execute_transactions(transactions);

        for client_id in 1..u16::max_value() {
            let expected = Decimal::from(client_id);
            assert_eq!(
                st_report.accounts.get(&client_id).unwrap().total(),
                expected
            );
            assert_eq!(
                mt_report.accounts.get(&client_id).unwrap().total(),
                expected
            );
        }
    }
}
