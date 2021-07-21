use std::{self, env};
use log::*;

mod records;
mod transactions_reader;
mod client_account;

fn main() {

    // TODO: disable logging in the test environment
    env_logger::init();

    let args: Vec<String> = env::args().collect();

    // Make sure there is one and only one argument to the program
    assert_eq!(args.len(), 2);

    let input_file = &args[1];
    info!("Reading CSV file: {}", input_file);

}
