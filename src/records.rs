use std::fmt::Display;

use rust_decimal::Decimal;
use serde::Deserialize;

/// Defines a transaction type to the client's asset account
#[derive(Deserialize, PartialEq, Debug)]
pub enum TransactionType {
    /// Deposit will increase the total funds in the client account
    #[serde(rename = "deposit")]
    Deposit,
    /// Withdrawal will decrease the total funds in the client account
    #[serde(rename = "withdrawal")]
    Withdrawal,
    /// Client claims that a previous transaction was erroneous and has to be reversed
    /// Makes available funds decrease by the disputed amount and held funds increase
    #[serde(rename = "dispute")]
    Dispute,
    /// Resolution to a dispute for a given transaction
    /// Makes available funds increase by the disputed amount and held funds decrease
    #[serde(rename = "resolve")]
    Resolve,
    /// Final state of a dispute, funds that were held are being withdrawn
    /// Client's held funds and total funds shall decrease by the disputed amount
    /// If a chargeback occurs, the account is frozen
    #[serde(rename = "chargeback")]
    ChargeBack,
}

pub type TransactionId = u32;
pub type ClientId = u16;

/// Represents a transaction record in our CSV
#[derive(Deserialize, Debug)]
pub struct TransactionRecord {
    /// Transaction type (can't use the type since it's a built-in keyword)
    #[serde(rename = "type")]
    pub tr_type: TransactionType,
    /// The id to uniquely identify the client
    pub client: ClientId,
    /// Transaction id, needed for disputes
    pub tx: TransactionId,
    /// Amount of money. Only available for deposits, withdrawal and chargebacks
    pub amount: Option<Decimal>,
}
