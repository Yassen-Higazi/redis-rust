#![allow(unused_imports)]
use anyhow::{Context, Ok};
use redis_server::listen;

use std::io::{Read, Write};

mod redis_server;
mod resp;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    listen().await?;

    Ok(())
}
