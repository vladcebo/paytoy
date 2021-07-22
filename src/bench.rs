use std::{io::{BufWriter, Read, Write}, time::Duration};

use log::debug;

use crate::transactions_reader::{STBulkReader, TransactionCSVReader};



// Benchmarking functions

/// Generates a large file with records, suitable for benchmarking
pub fn create_large_test_file(path: &str, num_records: usize, use_all_clients: bool) {
    let mut writer = BufWriter::new(std::fs::File::create(path).unwrap());

    writer
        .write_fmt(format_args!("type,  client,  tx,  amount\n"))
        .unwrap();

    for trans_id in 1..num_records + 1 {
        let mut client_id = 1u16;
        if use_all_clients {
            client_id = (trans_id % 65536) as u16;
        }

        let mut tr_type = "deposit";
        if trans_id % 2 == 0 {
            tr_type = "withdrawal"
        }

        let _ = writer.write_fmt(format_args!(
            "{},  {},  {},  {}\n",
            tr_type, client_id, trans_id, "243.2312"
        ));
    }
}

pub fn st_bulk_transaction_reader(path: &str) {
    let reader = STBulkReader::new();
    let _ = reader.read_csv(path).unwrap();
}

pub fn read_raw_file(path: &str) {
    let t = std::time::Instant::now();
    let mut file = std::fs::File::open(path).unwrap();
    let mut buf = Vec::new();
    let _ = file.read_to_end(&mut buf);
    debug!("Time to read the raw file: {:?}", t.elapsed());
}