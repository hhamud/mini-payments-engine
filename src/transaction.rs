use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransactionType {
    ///A deposit is a credit to the client's asset account, meaning it should increase the available and
    ///total funds of the client account
    Deposit,

    ///A withdraw is a debit to the client's asset account, meaning it should decrease the available and
    ///total funds of the client account
    Withdrawal,

    ///A dispute represents a client's claim that a transaction was erroneous and should be reversed.
    ///The transaction shouldn't be reversed yet but the associated funds should be held. This means
    ///that the clients' available funds should decrease by the amount disputed, their held funds should
    ///increase by the amount disputed, while their total funds should remain the same.
    Dispute,

    ///A chargeback is the final state of a dispute and represents the client reversing a transaction.
    ///Funds that were held have now been withdrawn. This means that the clients held funds and total
    ///funds should decrease by the amount previously disputed. If a chargeback occurs the client's
    ///account should be immediately frozen.
    Chargeback,

    ///A resolve represents a resolution to a dispute, releasing the associated held funds. Funds that
    ///were previously disputed are no longer disputed. This means that the clients held funds should
    ///decrease by the amount no longer disputed, their available funds should increase by the amount
    ///no longer disputed, and their total funds should remain the same.
    Resolve,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    #[serde(rename = "type")]
    pub tx_type: TransactionType,
    pub client: u16,
    pub tx: u32,
    // precision of up to 4 decimal places, e.g 0.1234
    //#[serde(with = "rust_decimal::serde::arbitrary_precision")]
    #[serde(with = "rust_decimal::serde::float_option")]
    pub amount: Option<Decimal>,
}

impl From<Transaction> for TransactionState {
    fn from(value: Transaction) -> Self {
        Self {
            tx_type: value.tx_type,
            client: value.client,
            tx: value.tx,
            amount: value.amount,
            disputed: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TransactionState {
    pub tx_type: TransactionType,
    pub client: u16,
    pub tx: u32,
    pub amount: Option<Decimal>,
    pub disputed: bool,
}
