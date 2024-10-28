use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize)]
pub struct Account {
    /// Client id
    pub client_id: u16,

    ///The total funds that are available for trading, staking, withdrawal, etc.
    ///This should be equal to the total - held amounts
    pub available_funds: Decimal,

    ///The total funds that are held for dispute. This should be equal to
    ///total - available amounts
    pub held_funds: Decimal,

    /// The total funds that are available or held. This should be equal to available
    /// + held
    pub total_funds: Decimal,

    ///Whether the account is locked. An account is locked if a charge back occurs
    pub locked: bool,
}

#[derive(Debug, Error)]
pub enum AccountError {
    #[error("Account Locked: {0}")]
    AccountLocked(u16),

    #[error("Not Enough Funds in Account {0} to withdraw {1} units")]
    NotEnoughFunds(u16, Decimal),
}

impl Account {
    pub fn new(amount: &mut Decimal, client_id: u16) -> Self {
        amount.rescale(4);
        Self {
            client_id,
            available_funds: *amount,
            held_funds: Decimal::new(0, 4),
            total_funds: *amount,
            locked: false,
        }
    }

    pub fn deposit(&mut self, amount: Decimal) -> Result<(), AccountError> {
        if self.locked {
            return Err(AccountError::AccountLocked(self.client_id));
        }

        self.available_funds += amount;
        self.total_funds += amount;

        assert_eq!(self.total_funds, self.available_funds + self.held_funds);
        Ok(())
    }

    pub fn withdraw(&mut self, amount: Decimal) -> Result<(), AccountError> {
        if self.locked {
            return Err(AccountError::AccountLocked(self.client_id));
        }

        if self.available_funds < amount {
            return Err(AccountError::NotEnoughFunds(self.client_id, amount));
        }

        self.available_funds -= amount;
        self.total_funds -= amount;

        assert_eq!(self.total_funds, self.available_funds + self.held_funds);

        Ok(())
    }

    pub fn dispute(&mut self, amount: Decimal) -> Result<(), AccountError> {
        if self.locked {
            return Err(AccountError::AccountLocked(self.client_id));
        }

        if self.available_funds < amount {
            return Err(AccountError::NotEnoughFunds(self.client_id, amount));
        }

        self.available_funds -= amount;
        self.held_funds += amount;
        assert_eq!(self.total_funds, self.available_funds + self.held_funds);

        Ok(())
    }

    pub fn resolve(&mut self, amount: Decimal) -> Result<(), AccountError> {
        if self.locked {
            return Err(AccountError::AccountLocked(self.client_id));
        }

        if self.held_funds < amount {
            return Err(AccountError::NotEnoughFunds(self.client_id, amount));
        }

        self.held_funds -= amount;
        self.available_funds += amount;

        assert_eq!(self.total_funds, self.available_funds + self.held_funds);

        Ok(())
    }

    pub fn chargeback(&mut self, amount: Decimal) -> Result<(), AccountError> {
        if self.locked {
            return Err(AccountError::AccountLocked(self.client_id));
        };

        if self.held_funds < amount {
            self.locked = true;
            return Err(AccountError::NotEnoughFunds(self.client_id, amount));
        };

        self.held_funds -= amount;
        self.total_funds -= amount;

        self.locked = true;

        assert_eq!(self.total_funds, self.available_funds + self.held_funds);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_new_account_with_zero_balance() {
        let mut amount = Decimal::new(0, 4);
        let account = Account::new(&mut amount, 1);
        assert_eq!(account.available_funds, dec!(0.0000));
        assert_eq!(account.total_funds, dec!(0.0000));
    }

    #[test]
    fn test_deposit_to_locked_account() {
        let mut account = Account::new(&mut dec!(100.0000), 1);
        account.locked = true;
        let result = account.deposit(dec!(50.0000));
        assert!(matches!(result, Err(AccountError::AccountLocked(1))));
    }

    #[test]
    fn test_withdraw_more_than_available() {
        let mut account = Account::new(&mut dec!(100.0000), 1);
        let result = account.withdraw(dec!(150.0000));
        assert!(matches!(result, Err(AccountError::NotEnoughFunds(1, _))));
    }

    #[test]
    fn test_withdraw_exact_available_amount() {
        let mut account = Account::new(&mut dec!(100.0000), 1);
        let result = account.withdraw(dec!(100.0000));
        assert!(result.is_ok());
        assert_eq!(account.available_funds, dec!(0.0000));
        assert_eq!(account.total_funds, dec!(0.0000));
    }

    #[test]
    fn test_dispute_more_than_available() {
        let mut account = Account::new(&mut dec!(100.0000), 1);
        let result = account.dispute(dec!(150.0000));
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_more_than_held() {
        let mut account = Account::new(&mut dec!(100.0000), 1);
        account.dispute(dec!(50.0000)).unwrap();
        let result = account.resolve(dec!(100.0000));
        assert!(result.is_err());
    }

    #[test]
    fn test_chargeback_more_than_held() {
        let mut account = Account::new(&mut dec!(100.0000), 1);
        account.dispute(dec!(50.0000)).unwrap();
        let result = account.chargeback(dec!(100.0000));
        assert!(result.is_err());
        assert!(account.locked);
    }

    #[test]
    fn test_operations_on_locked_account() {
        let mut account = Account::new(&mut dec!(100.0000), 1);
        account.locked = true;
        assert!(account.deposit(dec!(50.0000)).is_err());
        assert!(account.withdraw(dec!(50.0000)).is_err());
        assert!(account.dispute(dec!(50.0000)).is_err());
        assert!(account.resolve(dec!(50.0000)).is_err());
        assert!(account.chargeback(dec!(50.0000)).is_err());
    }
}
