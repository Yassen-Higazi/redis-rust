#![allow(unused_imports)]
use anyhow::{Context, Ok};
use redis_server::listen;

use std::io::{Read, Write};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

mod redis_server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    listen().await?;

    Ok(())
}
