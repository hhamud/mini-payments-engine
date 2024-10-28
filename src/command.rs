use crate::{ledger::Ledger, reader::reader, writer::output_report};
use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use tokio::{
    spawn,
    sync::{mpsc::channel, oneshot},
};

#[derive(Debug, Parser)]
pub struct Command {
    /// Csv input file
    pub input_file: PathBuf,
}

impl Command {
    pub async fn run(&self) -> Result<()> {
        let (tx, mut rx) = channel(100);
        let (tx_ledger, rx_ledger) = oneshot::channel();
        let file = self.input_file.clone();

        spawn(async move { reader(&file, tx).await });

        spawn(async move {
            let mut ledger = Ledger::new();
            while let Some(transaction) = rx.recv().await {
                ledger
                    .process_transaction(transaction.into())
                    .expect("failed to send transaction");
            }

            tx_ledger.send(ledger).expect("Failed to send ledger");
        });

        let ledger = rx_ledger.await.expect("failed to recieve ledger");
        output_report(&ledger)?;

        Ok(())
    }
}
