use std::path::Path;

use crate::{account_manager::AccountManager, transactions_reader::TransactionCSVReader};

/// The main application
pub struct PayToyApp {}

impl PayToyApp {
    /// Runs the application for a specific file in `path`
    /// an abstract implementation of a CSV reader and account manager is used
    /// those can be single-threaded, multi-threaded or other
    pub fn run<P: AsRef<Path>>(
        path: P,
        reader: impl TransactionCSVReader,
        manager: impl AccountManager,
        report_results: bool,
    ) -> anyhow::Result<()> {
        let transactions = reader.read_csv(path)?;

        let report = manager.execute_transactions(transactions);

        if report_results {
            report.report();
        }

        Ok(())
    }
}
