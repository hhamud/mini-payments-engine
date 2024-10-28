use crate::{
    account::Account,
    transaction::{Transaction, TransactionState, TransactionType},
};
use anyhow::Result;
use indexmap::IndexMap;
use rust_decimal::Decimal;
use std::collections::{HashMap, VecDeque};
use thiserror::Error;

pub type Client = u16;
pub type TransactionId = u32;

#[derive(Debug)]
pub struct Ledger {
    pub accounts: HashMap<Client, Account>,
    pub history: IndexMap<TransactionId, TransactionState>,
    pub unprocessed: VecDeque<TransactionState>,
}

#[derive(Debug, Error)]
pub enum LedgerError {
    #[error("Transaction Not found: {0}")]
    TransactionNotFound(TransactionId),

    #[error("Transaction amount missing: {0}")]
    TransactionAmountMissing(TransactionId),

    #[error("Client Account is missing: {0}")]
    AccountMissing(Client),

    #[error("Transaction is not disputed: {0}")]
    TransactionIsNotDisputed(TransactionId),
}

impl Ledger {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            history: IndexMap::new(),
            unprocessed: VecDeque::new(),
        }
    }

    fn add_history(&mut self, tx: TransactionState) {
        self.history.insert(tx.tx, tx);
    }

    fn add_unprocessed_transaction(&mut self, tx: TransactionState) {
        self.unprocessed.push_back(tx);
        self.unprocessed
            .make_contiguous()
            .sort_by_key(|transaction| transaction.tx);
    }

    fn get_account(&mut self, tx: &TransactionState) -> Result<&mut Account, LedgerError> {
        //assumption: No missing accounts
        self.accounts
            .get_mut(&tx.client)
            .ok_or_else(|| LedgerError::AccountMissing(tx.client))
    }

    fn get_historical_transaction_amount(
        &self,
        tx: &TransactionState,
        check_dispute: bool,
    ) -> Result<Decimal, LedgerError> {
        match self.history.get(&tx.tx) {
            Some(transaction) => {
                if check_dispute && !transaction.disputed {
                    return Err(LedgerError::TransactionIsNotDisputed(transaction.tx));
                }

                transaction
                    .amount
                    .ok_or_else(|| LedgerError::TransactionAmountMissing(transaction.tx))
            }
            None => Err(LedgerError::TransactionNotFound(tx.tx)),
        }
    }

    fn check_transaction(&mut self, tx: TransactionState) -> Result<()> {
        match tx.tx_type {
            TransactionType::Deposit => {
                self.add_history(tx.clone());
                let amount = tx
                    .amount
                    .ok_or_else(|| LedgerError::TransactionAmountMissing(tx.tx))?;

                match self.get_account(&tx) {
                    Ok(account) => {
                        account.deposit(amount)?;
                        return Ok(());
                    }
                    Err(_) => {
                        let account = Account::new(&mut amount.clone(), tx.client);
                        self.accounts.insert(tx.client, account);
                        Ok(())
                    }
                }
            }

            TransactionType::Withdrawal => {
                self.add_history(tx.clone());
                let amount = tx
                    .amount
                    .ok_or_else(|| LedgerError::TransactionAmountMissing(tx.tx))?;

                match self.get_account(&tx) {
                    Ok(account) => account.withdraw(amount)?,
                    Err(_) => {
                        self.add_unprocessed_transaction(tx.clone());
                        return Ok(());
                    }
                };

                Ok(())
            }
            TransactionType::Dispute => {
                self.history
                    .entry(tx.tx)
                    .and_modify(|transaction| transaction.disputed = true);

                let amount = self.get_historical_transaction_amount(&tx, false)?;

                let account = self.get_account(&tx)?;

                account.dispute(amount)?;

                Ok(())
            }
            TransactionType::Chargeback => {
                let amount = self.get_historical_transaction_amount(&tx, true)?;

                let account = self.get_account(&tx)?;
                account.chargeback(amount)?;

                Ok(())
            }
            TransactionType::Resolve => {
                let amount = self.get_historical_transaction_amount(&tx, true)?;

                let account = self.get_account(&tx)?;
                account.resolve(amount)?;

                self.history
                    .entry(tx.tx)
                    .and_modify(|transaction| transaction.disputed = false);

                Ok(())
            }
        }
    }

    fn process_unprocessed_transactions(&mut self) -> Result<()> {
        while let (Some(last_tx), Some(unpro_tx)) = (self.history.last(), self.unprocessed.front())
        {
            if last_tx.0 + 1 != unpro_tx.tx {
                break;
            }
            let transaction = self.unprocessed.pop_front().unwrap();
            self.check_transaction(transaction)?;
        }
        Ok(())
    }

    pub fn process_transaction(&mut self, tx: TransactionState) -> Result<()> {
        if let Some(last_tx) = self.history.last() {
            if let TransactionType::Withdrawal | TransactionType::Deposit = tx.tx_type {
                if last_tx.0 + 1 != tx.tx {
                    self.add_unprocessed_transaction(tx.clone());
                    return Ok(());
                };

                if let Some(unpro_tx) = self.unprocessed.front() {
                    if last_tx.0 + 1 == unpro_tx.tx {
                        let transaction = self.unprocessed.pop_front().unwrap();
                        self.check_transaction(transaction)?
                    };
                }
            }
        }

        self.check_transaction(tx)?;

        self.process_unprocessed_transactions()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_deposit_new_account() {
        let mut ledger = Ledger::new();
        let tx = TransactionState {
            tx: 1,
            client: 1,
            tx_type: TransactionType::Deposit,
            amount: Some(dec!(100.0)),
            disputed: false,
        };
        assert!(ledger.process_transaction(tx).is_ok());
        assert_eq!(ledger.accounts.len(), 1);
        assert_eq!(ledger.accounts[&1].total_funds, dec!(100.0));
    }

    #[test]
    fn test_withdrawal_insufficient_funds() {
        let mut ledger = Ledger::new();
        let deposit = TransactionState {
            tx: 1,
            client: 1,
            tx_type: TransactionType::Deposit,
            amount: Some(dec!(50.0)),
            disputed: false,
        };
        let withdrawal = TransactionState {
            tx: 2,
            client: 1,
            tx_type: TransactionType::Withdrawal,
            amount: Some(dec!(100.0)),
            disputed: false,
        };
        assert!(ledger.process_transaction(deposit).is_ok());
        assert!(ledger.process_transaction(withdrawal).is_err());
    }

    #[test]
    fn test_dispute_non_existent_transaction() {
        let mut ledger = Ledger::new();
        let tx = TransactionState {
            tx: 1,
            client: 1,
            tx_type: TransactionType::Dispute,
            amount: None,
            disputed: false,
        };
        assert!(ledger.process_transaction(tx).is_err());
    }

    #[test]
    fn test_withdrawal() {
        let mut ledger = Ledger::new();
        let tx = TransactionState {
            tx: 1,
            client: 1,
            tx_type: TransactionType::Deposit,
            amount: Some(dec!(1.0)),
            disputed: false,
        };

        assert!(ledger.process_transaction(tx).is_ok());

        let tx_2 = TransactionState {
            tx: 2,
            client: 1,
            tx_type: TransactionType::Withdrawal,
            amount: Some(dec!(1.0)),
            disputed: false,
        };

        assert!(ledger.process_transaction(tx_2).is_ok());

        assert_eq!(ledger.accounts[&1].total_funds, dec!(0.0))
    }

    #[test]
    fn test_dispute_resolve() {
        let mut ledger = Ledger::new();
        let tx = TransactionState {
            tx: 1,
            client: 1,
            tx_type: TransactionType::Deposit,
            amount: Some(dec!(1.0)),
            disputed: false,
        };

        assert!(ledger.process_transaction(tx).is_ok());

        let tx_2 = TransactionState {
            tx: 1,
            client: 1,
            tx_type: TransactionType::Dispute,
            amount: None,
            disputed: false,
        };

        assert!(ledger.process_transaction(tx_2).is_ok());

        assert_eq!(ledger.accounts[&1].total_funds, dec!(1.0));
        assert_eq!(ledger.accounts[&1].held_funds, dec!(1.0));
        assert_eq!(ledger.accounts[&1].available_funds, dec!(0.0));

        let tx_3 = TransactionState {
            tx: 1,
            client: 1,
            tx_type: TransactionType::Resolve,
            amount: None,
            disputed: false,
        };

        assert!(ledger.process_transaction(tx_3).is_ok());

        assert_eq!(ledger.accounts[&1].total_funds, dec!(1.0));
        assert_eq!(ledger.accounts[&1].held_funds, dec!(0.0));
        assert_eq!(ledger.accounts[&1].available_funds, dec!(1.0));
    }

    #[test]
    fn test_dispute_chargeback() {
        let mut ledger = Ledger::new();
        let tx = TransactionState {
            tx: 1,
            client: 1,
            tx_type: TransactionType::Deposit,
            amount: Some(dec!(1.0)),
            disputed: false,
        };

        assert!(ledger.process_transaction(tx).is_ok());

        let tx_2 = TransactionState {
            tx: 1,
            client: 1,
            tx_type: TransactionType::Dispute,
            amount: None,
            disputed: false,
        };

        assert!(ledger.process_transaction(tx_2).is_ok());

        assert_eq!(ledger.accounts[&1].total_funds, dec!(1.0));
        assert_eq!(ledger.accounts[&1].held_funds, dec!(1.0));
        assert_eq!(ledger.accounts[&1].available_funds, dec!(0.0));

        let tx_3 = TransactionState {
            tx: 1,
            client: 1,
            tx_type: TransactionType::Chargeback,
            amount: None,
            disputed: false,
        };

        assert!(ledger.process_transaction(tx_3).is_ok());

        assert_eq!(ledger.accounts[&1].total_funds, dec!(0.0));
        assert_eq!(ledger.accounts[&1].held_funds, dec!(0.0));
        assert_eq!(ledger.accounts[&1].available_funds, dec!(0.0));
    }

    #[test]
    fn test_withdraw_out_of_place_transaction() {
        let mut ledger = Ledger::new();
        let tx = TransactionState {
            tx: 1,
            client: 1,
            tx_type: TransactionType::Deposit,
            amount: Some(dec!(1.0)),
            disputed: false,
        };

        assert!(ledger.process_transaction(tx).is_ok());

        let tx_2 = TransactionState {
            tx: 3,
            client: 2,
            tx_type: TransactionType::Withdrawal,
            amount: Some(dec!(1.0)),
            disputed: false,
        };

        assert!(ledger.process_transaction(tx_2).is_ok());

        let tx_3 = TransactionState {
            tx: 2,
            client: 2,
            tx_type: TransactionType::Deposit,
            amount: Some(dec!(1.0)),
            disputed: false,
        };

        assert!(ledger.process_transaction(tx_3).is_ok());
        assert_eq!(ledger.accounts[&1].total_funds, dec!(1.0));
        assert_eq!(ledger.accounts[&2].total_funds, dec!(0.0));
    }

    #[test]
    fn test_chargeback_non_disputed_transaction() {
        let mut ledger = Ledger::new();
        let deposit = TransactionState {
            tx: 1,
            client: 1,
            tx_type: TransactionType::Deposit,
            amount: Some(dec!(100.0)),
            disputed: false,
        };
        let chargeback = TransactionState {
            tx: 1,
            client: 1,
            tx_type: TransactionType::Chargeback,
            amount: None,
            disputed: false,
        };
        assert!(ledger.process_transaction(deposit).is_ok());

        assert!(matches!(
            ledger
                .process_transaction(chargeback)
                .unwrap_err()
                .downcast::<LedgerError>(),
            Ok(LedgerError::TransactionIsNotDisputed(1))
        ));
    }

    #[test]
    fn test_resolve_non_disputed_transaction() {
        let mut ledger = Ledger::new();
        let deposit = TransactionState {
            tx: 1,
            client: 1,
            tx_type: TransactionType::Deposit,
            amount: Some(dec!(100.0)),
            disputed: false,
        };
        let resolve = TransactionState {
            tx: 1,
            client: 1,
            tx_type: TransactionType::Resolve,
            amount: None,
            disputed: false,
        };
        assert!(ledger.process_transaction(deposit).is_ok());

        assert!(matches!(
            ledger
                .process_transaction(resolve)
                .unwrap_err()
                .downcast::<LedgerError>(),
            Ok(LedgerError::TransactionIsNotDisputed(1))
        ));
    }

    #[test]
    fn test_transaction_without_amount() {
        let mut ledger = Ledger::new();
        let tx = TransactionState {
            tx: 1,
            client: 1,
            tx_type: TransactionType::Deposit,
            amount: None,
            disputed: false,
        };

        assert!(matches!(
            ledger
                .process_transaction(tx)
                .unwrap_err()
                .downcast::<LedgerError>(),
            Ok(LedgerError::TransactionAmountMissing(1))
        ));
    }
}
