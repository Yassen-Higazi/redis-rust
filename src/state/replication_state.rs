use std::net::SocketAddr;

use anyhow::Context;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use crate::{resp::RespDataTypes, utils::gen_id};

#[allow(unused)]
#[derive(Debug, Clone)]
pub enum Role {
    Master,
    Slave,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Replica {
    Master {
        id: String,
        address: SocketAddr,
        slaves: Vec<Replica>,
        replication_offset: i64,
    },

    Slave {
        id: String,
        address: SocketAddr,
        master_address: SocketAddr,
    },
}

impl Replica {
    pub fn new(port: u16, role: Role, master_address: Option<SocketAddr>) -> Self {
        match role {
            Role::Master => Self::new_master(),

            Role::Slave => Self::new_slave(port, master_address),
        }
    }

    pub fn new_master() -> Self {
        Self::Master {
            id: gen_id(),
            replication_offset: 0,
            address: SocketAddr::from(([127, 0, 0, 1], 6379)),
            slaves: Vec::new(),
        }
    }

    pub fn new_slave(port: u16, master_address: Option<SocketAddr>) -> Self {
        Self::Slave {
            id: gen_id(),
            master_address: master_address.expect("Master address is required for slave"),
            address: SocketAddr::from(([127, 0, 0, 1], port)),
        }
    }

    pub async fn init(&self) -> anyhow::Result<()> {
        match self {
            Self::Master { .. } => Ok(()),

            Self::Slave { master_address, .. } => {
                println!("Initializing slave replica...");
                println!("Connecting to master at {}", master_address);

                self.master_handshake()
                    .await
                    .map_err(|e| {
                        println!("Error pinging master: {e:?}");
                    })
                    .unwrap_or(());

                Ok(())
            }
        }
    }

    fn get_master_address(&self) -> Option<SocketAddr> {
        match self {
            Self::Master { .. } => None,
            Self::Slave { master_address, .. } => Some(*master_address),
        }
    }

    fn get_address(&self) -> SocketAddr {
        match self {
            Self::Master { address, .. } => *address,
            Self::Slave { address, .. } => *address,
        }
    }

    async fn master_handshake(&self) -> anyhow::Result<()> {
        let mut stream = TcpStream::connect(self.get_master_address().unwrap())
            .await
            .with_context(|| "Could not connect to master")?;

        stream
            .write_all(
                RespDataTypes::from(vec![String::from("PING")])
                    .to_string()
                    .as_bytes(),
            )
            .await
            .with_context(|| "Could not send PING command to master")?;

        let buffer = &mut [0u8; 512];

        stream
            .read(buffer)
            .await
            .with_context(|| "Could not read master response")?;

        println!(
            "Handshake (1/3) Master response: {}",
            String::from_utf8_lossy(buffer)
        );

        stream
            .write_all(
                RespDataTypes::Array(vec![
                    RespDataTypes::BulkString("REPLCONF".to_string()),
                    RespDataTypes::BulkString("listening-port".to_string()),
                    RespDataTypes::BulkString(self.get_address().port().to_string()),
                ])
                .to_string()
                .as_bytes(),
            )
            .await
            .with_context(|| "Could not send REPLCONF listening-port command to master")?;

        stream
            .read(buffer)
            .await
            .with_context(|| "Could not read master response")?;

        println!(
            "Handshake (2.1/3) Master response: {}",
            String::from_utf8_lossy(buffer)
        );

        stream
            .write_all(
                RespDataTypes::from(vec![
                    String::from("REPLCONF"),
                    String::from("capa"),
                    String::from("psync2"),
                ])
                .to_string()
                .as_bytes(),
            )
            .await
            .with_context(|| "Could not send REPLCONF capa command to master")?;

        stream
            .read(buffer)
            .await
            .with_context(|| "Could not read master response")?;

        println!(
            "Handshake (2.2/3) Master response: {}",
            String::from_utf8_lossy(buffer)
        );

        // let mut buffer = [0u8; 512];
        //
        // stream
        //     .read_exact(&mut buffer)
        //     .await
        //     .with_context(|| "Could not read master response")?;
        //
        // println!("Master response: {}", String::from_utf8_lossy(&buffer));

        Ok(())
    }

    pub fn get_replication_status(&self) -> String {
        match self {
            Self::Master {
                id,
                slaves,
                replication_offset,
                ..
            } => format!(
                "# Replication
role:master
connected_slaves:{}
master_replid:{id}
master_repl_offset:{replication_offset}
second_repl_offset:-1
repl_backlog_active:0
repl_backlog_size:1048576
repl_backlog_first_byte_offset:0
repl_backlog_histlen:
                            ",
                slaves.len()
            ),
            Self::Slave { id, .. } => format!(
                "# Replication
role:slave
slave_replid:{id} "
            ),
        }
    }
}
