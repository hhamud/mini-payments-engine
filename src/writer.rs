use crate::{account::Account, ledger::Ledger};
use anyhow::Result;
use csv::Writer;
use std::io::stdout;

pub fn output_report(ledger: &Ledger) -> Result<()> {
    let mut wtr = Writer::from_writer(stdout());

    let accounts: Vec<&Account> = ledger.accounts.values().collect();

    for account in accounts {
        wtr.serialize(account)?;
    }

    wtr.flush()?;

    Ok(())
}
