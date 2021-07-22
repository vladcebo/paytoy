/// Reads transactions from a CSV file
/// Make it a separate file in case we want to add new methods
/// such as reading from a non-CSV file and so on
use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Read},
    path::Path,
};

use anyhow::Context;
use crossbeam_channel::{Receiver, Sender};
use csv::{ByteRecord, ReaderBuilder, Trim};
use threadpool::ThreadPool;

use crate::records::TransactionRecord;

use log::*;

/// A type that represents a stream of transactions arriving into the system
/// Many channels (such as crossbeam) implement iterator interface, so can be used for multithreading
pub type TransactionsStream = Box<dyn Iterator<Item = TransactionRecord>>;

/// Trait to read CSV files into a `TransactionsStream`
pub trait TransactionCSVReader {
    /// Read transactions from a CSV file
    /// Returns a vector with all the transactions nicely packet into structs
    fn read_csv<P: AsRef<Path>>(self, path: P) -> anyhow::Result<TransactionsStream>;
}

/// A single threaded bulk reader
/// Reads and parses everything upfront and returns a stream to the records
pub struct STBulkReader {}

impl STBulkReader {
    pub fn new() -> Self {
        Self {}
    }
}

impl TransactionCSVReader for STBulkReader {
    fn read_csv<P: AsRef<Path>>(self, path: P) -> anyhow::Result<TransactionsStream> {
        let start_time = std::time::Instant::now();
        info!("STBulkReader reading the transactions");
        let mut csv_reader = ReaderBuilder::new()
            .trim(Trim::All)
            .flexible(true)
            .from_path(path)?;

        // Read as byte records, that should improve the performance without a lot of reallocations
        let mut raw_record = csv::ByteRecord::new();
        let headers = csv_reader.byte_headers()?.clone();

        let mut transactions = Vec::new();
        while csv_reader.read_byte_record(&mut raw_record)? {
            let record = raw_record.deserialize::<TransactionRecord>(Some(&headers));
            // for simplicity, ignore transactions that cannot be parsed
            if let Ok(record) = record {
                transactions.push(record);
            }
        }

        info!(
            "Read {} records in {:?}. Throughput: {} millions/second",
            transactions.len(),
            start_time.elapsed(),
            transactions.len() as f32 / (1000000.0 * start_time.elapsed().as_secs_f32())
        );

        Ok(Box::new(transactions.into_iter()))
    }
}

// A multithreaded reader
pub struct MTReader {
    num_threads: usize,
    block_size: usize,
}

impl MTReader {
    pub fn new() -> Self {
        Self {
            num_threads: num_cpus::get(),
            block_size: 32 * 1024,
        }
    }

    pub fn with_threads(mut self, num_threads: usize) -> Self {
        self.num_threads = num_threads;
        self
    }

    pub fn block_size(mut self, block_size: usize) -> Self {
        self.block_size = block_size;
        self
    }
}

impl TransactionCSVReader for MTReader {
    fn read_csv<P: AsRef<Path>>(mut self, path: P) -> anyhow::Result<TransactionsStream> {
        let mut file_reader =
            BufReader::with_capacity(2 * self.block_size, std::fs::File::open(path)?);
        let mut headers = vec![];

        // read first row
        file_reader
            .read_until(b'\n', &mut headers)
            .with_context(|| "Failed to read the headers")?;

        let pool = ThreadPool::new(self.num_threads);

        let (parsed_tx, parsed_rx) =
            crossbeam_channel::unbounded::<(u32, Vec<TransactionRecord>)>();

        let (reorder_tx, reorder_rx) = crossbeam_channel::unbounded::<TransactionRecord>();

        Self::start_reorder(parsed_rx, reorder_tx);

        // Read blocks of transactions
        let mut block_id = 0;
        while let Some(block) = self.read_block(&mut file_reader) {
            block_id += 1;
            // the parsed blocks may arrive out of order, so we need to perform a reordering
            Self::dispatch_csv_block(&pool, block_id, block, parsed_tx.clone());
        }

        Ok(Box::new(reorder_rx.into_iter()))
    }
}

impl MTReader {
    /// Dispatch a CSV raw block for parsing
    fn dispatch_csv_block(
        pool: &ThreadPool,
        block_id: u32,
        block: Vec<u8>,
        parsed_tx: Sender<(u32, Vec<TransactionRecord>)>,
    ) {
        // For now consider that the headers if read then they're OK and equal to below
        let headers = ByteRecord::from(vec!["type", "client", "tx", "amount"]);
        pool.execute(move || {
            let mut csv_reader = ReaderBuilder::new()
                .trim(Trim::All)
                .has_headers(true)
                .flexible(true)
                .from_reader(block.as_slice());

            let mut raw_record = csv::ByteRecord::new();
            // Looks like I have found a bug in CSV library
            // It doesn't trim the first row if has_headers = false and the headers are supplied to deserialize
            csv_reader.set_byte_headers(headers.clone());

            let mut transactions = Vec::new();
            while let Ok(true) = csv_reader.read_byte_record(&mut raw_record) {
                let record = raw_record.deserialize::<TransactionRecord>(Some(&headers));
                if let Ok(record) = record {
                    transactions.push(record);
                }
            }

            // Will ignore the channel closed for now
            let _ = parsed_tx.send((block_id, transactions));
        });
    }

    fn start_reorder(
        parsed_rx: Receiver<(u32, Vec<TransactionRecord>)>,
        reorder_tx: Sender<TransactionRecord>,
    ) {
        // Ignore the join handle, since the lifetime of the thread is tied to the lifetime of the input and output channels
        let _ = std::thread::spawn(move || {
            let mut waiting_for = 1;
            let mut queue = HashMap::new();
            while let Ok(block) = parsed_rx.recv() {
                if block.0 == waiting_for {
                    for record in block.1 {
                        if reorder_tx.send(record).is_err() {
                            return;
                        };
                    }
                    waiting_for += 1;
                    // Clear backlog
                    while let Some(transactions) = queue.remove(&waiting_for) {
                        for record in transactions {
                            if reorder_tx.send(record).is_err() {
                                return;
                            };
                        }
                        waiting_for += 1;
                    }
                } else if block.0 > waiting_for {
                    queue.insert(block.0, block.1);
                }
            }
        });
    }

    // Reads a big block until new line
    fn read_block(&mut self, reader: &mut impl BufRead) -> Option<Vec<u8>> {
        let mut block = vec![0; self.block_size];
        // put additional for adjustments
        block.reserve(1000);

        match reader.read(&mut block) {
            Ok(0) => None,
            Ok(n) => {
                block.truncate(n);
                // do not care if we reach EOF for now
                let _ = reader.read_until(b'\n', &mut block);
                Some(block)
            }
            Err(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal_macros::dec;

    use super::*;
    use crate::records::TransactionType;

    #[test]
    fn test_no_file_exists() {
        let transactions = STBulkReader::new().read_csv("tests/data/non_existent.csv");
        assert!(transactions.is_err());
    }

    fn test_transaction_reader(reader: impl TransactionCSVReader, path: &str) {
        let mut transactions = reader.read_csv(&path).expect("Test file is not found");

        // Validate a few fields to give us enough confidence that parsing is successful
        let trans = transactions.next().unwrap();
        assert_eq!(trans.tr_type, TransactionType::Deposit);

        let trans = transactions.next().unwrap();
        assert_eq!(trans.tr_type, TransactionType::Withdrawal);
        assert_eq!(trans.client, 6);
        assert_eq!(trans.tx, 5);
        assert_eq!(trans.amount, Some(dec!(9.0)));

        let trans = transactions.skip(2).next().unwrap();
        assert_eq!(trans.tr_type, TransactionType::ChargeBack);
        assert_eq!(trans.amount, None);
    }

    /// Tests that we can read and parse all transactions
    #[test]
    fn test_st_bulk_transaction_reader_serde() {
        let reader = STBulkReader::new();
        test_transaction_reader(reader, "tests/data/test_serde.csv");
    }

    #[test]
    fn test_mt_reader_transaction_reader_serde() {
        let reader = MTReader::new();
        test_transaction_reader(reader, "tests/data/test_serde.csv");
    }

    #[test]
    fn test_mt_reader_transaction_reader_big() {
        let reader = MTReader::new();
        let mut transactions = reader
            .read_csv("tests/data/test_mt_reader.csv")
            .expect("Test file is not found");
        for i in 1..20001 {
            assert_eq!(transactions.next().unwrap().tx, i);
        }
        assert!(transactions.next().is_none());
    }
}
