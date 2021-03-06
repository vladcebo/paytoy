use log::*;
use std::{self, env};

use crate::{
    account_manager::{MTAccountManager, STAccountManager},
    bench::create_large_test_file,
    paytoy::PayToyApp,
    transactions_reader::MTReader,
};

mod account_manager;
mod bench;
mod client_account;
mod paytoy;
mod records;
mod transactions_reader;

static LARGE_TEST_FILE_NAME: &'static str = "tests/data/test_large.csv";
static NUM_RECORDS: usize = 10000000;

// A simple test function to run qualitative benchmarks
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
    // env_logger::builder()
    //     .target(env_logger::Target::Stdout)
    //     .filter_level(LevelFilter::Info)
    //     .init();

    let args: Vec<String> = env::args().collect();

    // Make sure there is one and only one argument to the program
    // TODO: maybe add some arguments with help or something
    if args.len() != 2 {
        error!("A file name argument must be provided as a single input argument");
        std::process::exit(0);
    }

    let input_file = &args[1];
    info!("Starting application on the file: {}", input_file);

    // For the final application, use both multithreader CSV reader
    // and multithreaded account manager for processing multiple clients in parallel
    let num_cores = num_cpus::get();
    if num_cores >= 4 {
        let reader = MTReader::new().with_threads(num_cores / 2);
        let manager = MTAccountManager::new(num_cores / 2);

        if let Err(err) = PayToyApp::run(input_file, reader, manager, true) {
            error!("Failed to run the application: {:?}", err);
            std::process::exit(0);
        };
    } else {
        let reader = MTReader::new().with_threads(2);
        let manager = STAccountManager::new();

        if let Err(err) = PayToyApp::run(input_file, reader, manager, true) {
            error!("Failed to run the application: {:?}", err);
            std::process::exit(0);
        };
    }

    // For this benchmark, I get around 6-7 millions of records/second on my machine
    // run_benchmarks(true);
}
