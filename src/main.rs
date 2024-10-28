use anyhow::Result;
use clap::Parser;
use mini_payments_engine::command::Command;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Command::parse();
    cli.run().await
}
