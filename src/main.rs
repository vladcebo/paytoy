use env_logger::Target;
use log::*;
use std::{self, env};

use crate::bench::create_large_test_file;

mod client_account;
mod records;
mod transactions_reader;
mod bench;
mod account_manager;

static LARGE_TEST_FILE_NAME: &'static str = "tests/data/test_large.csv";
static NUM_RECORDS: usize = 1000000;

fn main() {
    // TODO: disable logging in the test environment
    env_logger::builder().target(Target::Stdout).filter_level(LevelFilter::Debug).init();

    let args: Vec<String> = env::args().collect();

    // Make sure there is one and only one argument to the program
    // TODO: maybe add some arguments with help or something
    assert_eq!(args.len(), 2);

    let input_file = &args[1];
    info!("Reading CSV file: {}", input_file);

    create_large_test_file(LARGE_TEST_FILE_NAME, NUM_RECORDS,true);

    bench::read_raw_file(LARGE_TEST_FILE_NAME);
    bench::st_bulk_transaction_reader(LARGE_TEST_FILE_NAME);

    /* TODO:
        1) Highly likely the csv/serde parsing is the bottleneck, need to benchmark
        2) Get a stream instead of a vector from the csv? or better an iterator (since many channel implementations support into_iter())
        3) Can we parse csv in parallel?
        4) Other edge cases for the account:
            - can a withdrawal be disputed?
            - locked account?
            - not enough held for dispute?
        5) Document stuff
    */
}
