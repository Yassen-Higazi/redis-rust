use std::net::SocketAddr;

use anyhow::{ensure, Context};
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
        master_id: String,
        master_offset: i64,
        address: SocketAddr,
        master_address: String,
    },
}

impl Replica {
    pub fn new(port: u16, role: Role, master_address: Option<String>) -> Self {
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

    pub fn new_slave(port: u16, master_address: Option<String>) -> Self {
        Self::Slave {
            id: gen_id(),
            master_offset: -1,
            master_id: String::from("?"),
            master_address: master_address.expect("Master address is required for slave"),
            address: SocketAddr::from(([127, 0, 0, 1], port)),
        }
    }

    pub async fn init(&mut self) -> anyhow::Result<()> {
        match self {
            Self::Master { .. } => Ok(()),

            Self::Slave { .. } => {
                println!("Initializing slave replica...");
                println!(
                    "Connecting to master at {}",
                    self.get_master_address().unwrap()
                );

                self.master_handshake()
                    .await
                    .map_err(|e| {
                        println!("Error In master handshake: {e:?}");
                    })
                    .unwrap_or(());

                Ok(())
            }
        }
    }

    fn get_master_address(&self) -> Option<String> {
        match self {
            Self::Master { .. } => None,
            Self::Slave { master_address, .. } => Some(master_address.clone()),
        }
    }

    fn get_address(&self) -> SocketAddr {
        match self {
            Self::Master { address, .. } => *address,
            Self::Slave { address, .. } => *address,
        }
    }

    fn get_master_id(&self) -> String {
        match self {
            Self::Master { id, .. } => id.clone(),
            Self::Slave { master_id, .. } => master_id.clone(),
        }
    }

    fn get_replication_offset(&self) -> i64 {
        match self {
            Self::Master {
                replication_offset, ..
            } => *replication_offset,
            Self::Slave { master_offset, .. } => *master_offset,
        }
    }

    fn set_replication_offset(&mut self, offset: i64) {
        match self {
            Self::Master {
                replication_offset, ..
            } => *replication_offset = offset,
            Self::Slave { master_offset, .. } => *master_offset = offset,
        }
    }

    fn set_master_id(&mut self, id: String) {
        match self {
            Self::Master { .. } => {}
            Self::Slave { master_id, .. } => *master_id = id,
        }
    }

    async fn master_handshake(&mut self) -> anyhow::Result<()> {
        let mut stream = TcpStream::connect(self.get_master_address().unwrap())
            .await
            .with_context(|| "Could not connect to master")?;

        self.ping_master(&mut stream)
            .await
            .with_context(|| "Could not ping master")?;

        self.replconf(&mut stream)
            .await
            .with_context(|| "Could not send REPLCONF command to master")?;

        self.psync(Some(&mut stream))
            .await
            .with_context(|| "Could not send PSYNC command to master")?;

        Ok(())
    }

    async fn ping_master(&self, stream: &mut TcpStream) -> anyhow::Result<()> {
        stream
            .write_all(
                RespDataTypes::Array(vec![RespDataTypes::BulkString("PING".to_string())])
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

        Ok(())
    }

    async fn replconf(&self, stream: &mut TcpStream) -> anyhow::Result<()> {
        let buffer = &mut [0u8; 512];

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
                RespDataTypes::Array(vec![
                    RespDataTypes::BulkString("REPLCONF".to_string()),
                    RespDataTypes::BulkString("capa".to_string()),
                    RespDataTypes::BulkString("psync2".to_string()),
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

        Ok(())
    }

    pub async fn psync(
        &mut self,
        _stream: Option<&mut TcpStream>,
    ) -> anyhow::Result<RespDataTypes> {
        match self {
            Self::Master {
                id,
                replication_offset,
                ..
            } => Ok(RespDataTypes::SimpleString(format!(
                "FULLRESYNC {id} {replication_offset}"
            ))),

            Self::Slave { .. } => {
                let buffer = &mut [0u8; 512];

                let stream = _stream.ok_or_else(|| anyhow::anyhow!("Stream is not available"))?;

                stream
                    .write_all(
                        RespDataTypes::Array(vec![
                            RespDataTypes::BulkString("PSYNC".to_string()),
                            RespDataTypes::BulkString(self.get_master_id()),
                            RespDataTypes::BulkString(self.get_replication_offset().to_string()),
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

                let res = String::from_utf8_lossy(buffer);

                println!("Handshake (3/3) Master response: {}", res);

                let mut splits = res.split_whitespace();

                let command = splits
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("Could not parse master response"))?;

                ensure!(
                    command == "+FULLRESYNC",
                    "Master did not respond with FULLRESYNC to PSYNC command"
                );

                let master_id = splits
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("Could not parse master ID from response"))?
                    .to_string();

                let replication_offset = splits
                    .next()
                    .ok_or_else(|| {
                        anyhow::anyhow!("Could not parse replication offset from response")
                    })?
                    .parse::<i64>()
                    .with_context(|| "Could not parse replication offset")?;

                println!("Master ID: {master_id}");
                println!("Replication offset: {replication_offset}");

                self.set_master_id(master_id);
                self.set_replication_offset(replication_offset);

                stream
                    .read(buffer)
                    .await
                    .with_context(|| "Could not read master response")?;

                let res = String::from_utf8_lossy(buffer);

                println!("Master Responseded with RDB file: {}", res);

                Ok(RespDataTypes::SimpleString("Ok".to_string()))
            }
        }
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
