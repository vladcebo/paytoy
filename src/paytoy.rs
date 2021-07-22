use std::path::Path;

use crate::{account_manager::AccountManager, transactions_reader::TransactionCSVReader};

// The main application

pub struct PayToyApp {}

impl PayToyApp {
    pub fn run<P: AsRef<Path>>(
        path: P,
        reader: impl TransactionCSVReader,
        mut manager: impl AccountManager,
        report_results: bool,
    ) -> anyhow::Result<()> {
        let transactions = reader.read_csv(path).unwrap();

        manager.execute_transactions(transactions);

        if report_results {
            manager.report();
        }

        Ok(())
    }
}
