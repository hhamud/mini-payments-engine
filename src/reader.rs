use anyhow::Result;
use csv::ReaderBuilder;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use tokio::sync::mpsc::Sender;

use crate::transaction::Transaction;

pub async fn reader(path: &PathBuf, channel: Sender<Transaction>) -> Result<()> {
    let file = File::open(path)?;
    let cap = 4 * 1024 * 1024; // 4MB buffer
    let buf_reader = BufReader::with_capacity(cap, file);
    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        .trim(csv::Trim::All)
        .flexible(true)
        .from_reader(buf_reader);

    for result in rdr.deserialize() {
        let transaction: Transaction = result?;
        if channel.send(transaction).await.is_err() {
            break;
        }
    }

    Ok(())
}
