use env_logger::Target;
use log::*;
use std::{self, env};

use crate::{account_manager::{MTAccountManager, STAccountManager}, bench::create_large_test_file, paytoy::PayToyApp, transactions_reader::{MTReader, TransactionCSVReader}};

mod account_manager;
mod bench;
mod client_account;
mod paytoy;
mod records;
mod transactions_reader;

static LARGE_TEST_FILE_NAME: &'static str = "tests/data/test_large.csv";
static NUM_RECORDS: usize = 1000000;

#[allow(dead_code)]
fn run_benchmarks(use_all_accounts: bool) {
    create_large_test_file(LARGE_TEST_FILE_NAME, NUM_RECORDS, use_all_accounts);

    bench::read_raw_file(LARGE_TEST_FILE_NAME);
    bench::st_bulk_transaction_reader(LARGE_TEST_FILE_NAME);
    bench::mt_transaction_reader(LARGE_TEST_FILE_NAME);

    bench::st_bulk_application(LARGE_TEST_FILE_NAME, NUM_RECORDS);
    bench::mt_application(LARGE_TEST_FILE_NAME, NUM_RECORDS);

}

fn main() {
    // TODO: disable logging in the test environment
    env_logger::builder()
        .target(Target::Stdout)
        .filter_level(LevelFilter::Info)
        .init();

    let args: Vec<String> = env::args().collect();

    // Make sure there is one and only one argument to the program
    // TODO: maybe add some arguments with help or something
    if args.len() != 2 {
        error!("A file name argument must be provided as a single input argument");
        std::process::exit(0);
    }

    let input_file = &args[1];
    info!("Starting application on the file: {}", input_file);

    let reader = MTReader::new().with_threads(num_cpus::get());
    let manager = MTAccountManager::new(num_cpus::get());

    if let Err(err) = PayToyApp::run(input_file, reader, manager, true) {
        error!("Failed to run the application: {:?}", err);
        std::process::exit(0);
    };

    /*
        TODO:
        documentation and proper reporting / refactoring
        + edge cases?
    */
}
