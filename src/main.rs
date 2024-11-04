use anyhow::Ok;
use clap::Parser;
use redis_server::listen;

mod configs;
mod redis_server;
mod redis_service;
mod resp;

use configs::{cmd_options::CmdOptions, configurations::Configuration};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = CmdOptions::parse();

    let configs = Configuration::from(args);

    println!("Config: {configs:?}");

    listen(configs).await?;

    Ok(())
}
