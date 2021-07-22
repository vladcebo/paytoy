use std::path::Path;

use crate::{
    account_manager::AccountManager,
    transactions_reader::{self, TransactionCSVReader},
};

// The main application

pub trait App {
    fn run<P: AsRef<Path>>(path: P, report_results: bool) -> anyhow::Result<()>;
}

pub struct PayToySTApp {}

impl App for PayToySTApp {
    fn run<P: AsRef<Path>>(path: P, report_results: bool) -> anyhow::Result<()> {
        let transactions = transactions_reader::STBulkReader::new()
            .read_csv(path)
            .unwrap();

        let mut manager = AccountManager::new();
        manager.execute_transactions(transactions);

        if report_results {
            manager.report();
        }

        Ok(())
    }
}


pub struct PayToyMTApp {}

impl App for PayToyMTApp {
    fn run<P: AsRef<Path>>(path: P, report_results: bool) -> anyhow::Result<()> {
        let transactions = transactions_reader::MTReader::new()
            .read_csv(path)
            .unwrap();

        let mut manager = AccountManager::new();
        manager.execute_transactions(transactions);

        if report_results {
            manager.report();
        }

        Ok(())
    }
}
