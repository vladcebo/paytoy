/// Reads transactions from a CSV file
/// Make it a separate file in case we want to add new methods
/// such as reading from a non-CSV file and so on
use std::path::Path;

use csv::{ReaderBuilder, Trim};

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
        debug!("STBulkReader reading the transactions");
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

        debug!(
            "Read {} records in {:?}. Throughput: {} millions/second",
            transactions.len(),
            start_time.elapsed(),
            transactions.len() as f32 / (1000000.0 * start_time.elapsed().as_secs_f32())
        );

        Ok(Box::new(transactions.into_iter()))
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
}
