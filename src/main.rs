use log::*;
use std::{self, env};

mod client_account;
mod records;
mod transactions_reader;

fn main() {
    // TODO: disable logging in the test environment
    env_logger::init();

    let args: Vec<String> = env::args().collect();

    // Make sure there is one and only one argument to the program
    assert_eq!(args.len(), 2);

    let input_file = &args[1];
    info!("Reading CSV file: {}", input_file);

    /* TODO:
        1) Highly likely the csv/serde parsing is the bottleneck, need to benchmark
        2) Get a stream instead of a vector from the csv? or better an iterator (since many channel implementations support into_iter())
        3) Can we parse csv in parallel?
        4) Other edge cases for the account:
            - can a withdrawal be disputed?
            - others?
        5) Document stuff
    */
}
