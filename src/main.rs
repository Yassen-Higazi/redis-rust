#![allow(unused_imports)]
use anyhow::Ok;
use redis_server::listen;

mod redis_server;
mod redis_service;
mod resp;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    listen().await?;

    Ok(())
}
