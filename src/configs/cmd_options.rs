use std::{
    io,
    net::{SocketAddr, ToSocketAddrs},
};

use clap::Parser;

#[derive(Debug, Parser)]
pub struct CmdOptions {
    #[arg(short = 'd', long = "dir", default_value = "/tmp/redis-files")]
    pub dir: String,

    #[arg(short = 'f', long = "dbfilename", default_value = "dump.rdb")]
    pub filename: String,

    #[arg(short = 'u', long = "host", default_value = "127.0.0.1")]
    pub host: String,

    #[arg(short = 'p', long = "port", default_value = "6379")]
    pub port: String,

    #[arg(short, long = "replicaof", value_parser = valid_replicaof)]
    pub replicatof: Option<SocketAddr>,
}

fn valid_replicaof(value: &str) -> Result<SocketAddr, String> {
    if value.is_empty() {
        return Err("Replica master address cannot be empty".to_string());
    }

    let str = value.trim().replace(" ", ":");

    let socketaddr = str
        .to_socket_addrs()
        .expect("Invalid replicaof")
        .next()
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::AddrNotAvailable,
                format!("Could not find destination {str}"),
            )
        });

    socketaddr.map_err(|e| format!("Invalid replicaof address: {}", e))
}
