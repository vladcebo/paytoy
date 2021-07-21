/// Reads transactions from a CSV file
/// Make it a separate file in case we want to add new methods
/// such as reading from a non-CSV file and so on
use std::{error::Error, path::Path};

use csv::{ReaderBuilder, Trim};

use crate::records::TransactionRecord;

/// Read transactions from a CSV file
/// Returns a vector with all the transactions nicely packet into structs
pub fn read_from_csv<P: AsRef<Path>>(path: P) -> Result<Vec<TransactionRecord>, Box<dyn Error>> {
    let mut csv_reader = ReaderBuilder::new()
        .trim(Trim::All)
        .flexible(true)
        .from_path(path)?;

    let mut transactions = Vec::new();
    for record in csv_reader.deserialize::<TransactionRecord>() {
        transactions.push(record?);
    }

    Ok(transactions)
}

#[cfg(test)]
mod tests {
    use rust_decimal_macros::dec;

    use super::*;
    use crate::records::TransactionType;

    #[test]
    fn test_no_file_exists() {
        let transactions = read_from_csv("tests/data/non_existent.csv");
        assert!(transactions.is_err());
    }

    /// Tests that we can read and parse all transactions
    #[test]
    fn test_transaction_reader() {
        let transactions =
            read_from_csv("tests/data/test_serde.csv").expect("Test file is not found");

        // Validate a few fields to give us enough confidence that parsing is successful
        assert_eq!(transactions[0].tr_type, TransactionType::Deposit);

        assert_eq!(transactions[1].tr_type, TransactionType::Withdrawal);
        assert_eq!(transactions[1].client, 6);
        assert_eq!(transactions[1].tx, 5);
        assert_eq!(transactions[1].amount, Some(dec!(9.0)));

        assert_eq!(transactions[4].tr_type, TransactionType::ChargeBack);
        assert_eq!(transactions[4].amount, None);
    }
}
