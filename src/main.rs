use anyhow::Ok;
use clap::Parser;

mod configs;
mod database;
mod persistence;
mod redis_server;
mod redis_service;
mod resp;
mod state;
mod utils;

use configs::cmd_options::CmdOptions;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = CmdOptions::parse();

    let redis_server = redis_server::RedisServer::new(args);

    redis_server.listen().await?;

    Ok(())
}
