use std::{net::SocketAddr, sync::Arc};

use anyhow::{bail, ensure, Context};
use socket2::{SockRef, TcpKeepalive};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::Mutex,
};

use crate::{resp::RespDataTypes, utils::gen_id};

#[allow(unused)]
#[derive(Debug, Clone)]
pub enum Role {
    Master,
    Slave,
}

#[allow(dead_code)]
#[derive(Debug)]
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
        master_connection: Option<Arc<Mutex<TcpStream>>>,
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
            master_connection: None,
            master_id: String::from("?"),
            address: SocketAddr::from(([127, 0, 0, 1], port)),
            master_address: master_address.expect("Master address is required for slave"),
        }
    }

    pub async fn init(&mut self) -> anyhow::Result<Arc<Mutex<TcpStream>>> {
        match self {
            Self::Master { .. } => bail!("Master replica cannot be initialized this way"),

            Self::Slave { .. } => {
                println!("Initializing slave replica...");
                println!(
                    "Connecting to master at {}",
                    self.get_master_address().unwrap()
                );

                self.master_handshake().await.with_context(|| {
                    format!(
                        "Could not initialize slave replica at {}",
                        self.get_address()
                    )
                })
            }
        }
    }

    pub async fn register_replica(
        &mut self,
        connection: Arc<Mutex<TcpStream>>,
    ) -> anyhow::Result<()> {
        match self {
            Self::Master {
                slaves,
                id,
                address,
                ..
            } => {
                println!("Registering replica...");

                let stream = connection.lock().await;

                let replica_addres = stream
                    .peer_addr()
                    .with_context(|| "Could not get peer address")?;

                drop(stream);

                let slave_exist = slaves
                    .iter()
                    .any(|s| s.get_address().port() == replica_addres.port());

                if slave_exist {
                    println!("Slave already exists");
                    return Ok(());
                }

                println!("Registering slave at {replica_addres}");

                let slave = Self::Slave {
                    id: gen_id(),
                    master_offset: -1,
                    master_id: id.clone(),
                    address: replica_addres,
                    master_address: address.to_string(),
                    master_connection: Some(connection),
                };

                slaves.push(slave);

                println!("Slaves: {:?}", slaves);

                Ok(())
            }

            Self::Slave { .. } => Ok(()),
        }
    }

    pub async fn replicate_command(&mut self, command: &RespDataTypes) -> anyhow::Result<()> {
        match self {
            Self::Master { slaves, .. } => {
                println!("Slave count: {}", slaves.len());

                for slave in slaves {
                    match slave {
                        Self::Slave {
                            master_connection,
                            address,
                            ..
                        } => {
                            if let Some(stream) = master_connection {
                                println!("Replicating command to slave at {address}");

                                let mut stream_guard = stream.lock().await;

                                stream_guard
                                    .write_all(command.to_string().as_bytes())
                                    .await
                                    .with_context(|| {
                                        format!(
                                            "Could not send command: {command} to slave {address}"
                                        )
                                    })?;

                                println!("Command replicated to slave at {}", address);
                            }
                        }

                        Self::Master { .. } => {}
                    };
                }
            }

            Self::Slave { .. } => {
                println!("Slave cannot replicate command");
            }
        };

        Ok(())
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

    async fn master_handshake(&mut self) -> anyhow::Result<Arc<Mutex<TcpStream>>> {
        let stream = TcpStream::connect(self.get_master_address().unwrap())
            .await
            .with_context(|| "Could not connect to master")?;

        let ka = TcpKeepalive::new().with_time(std::time::Duration::from_secs(180));
        let sf = SockRef::from(&stream);
        sf.set_tcp_keepalive(&ka)?;

        let connection = Arc::new(Mutex::new(stream));

        let mut connection_guard = connection.lock().await;

        self.ping_master(&mut connection_guard)
            .await
            .with_context(|| "Could not ping master")?;

        self.replconf(&mut connection_guard)
            .await
            .with_context(|| "Could not send REPLCONF command to master")?;

        self.psync(Some(&mut connection_guard))
            .await
            .with_context(|| "Could not send PSYNC command to master")?;

        drop(connection_guard);

        self.set_master_connection(connection.clone());

        Ok(connection)
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

    fn set_master_connection(&mut self, connection: Arc<Mutex<TcpStream>>) {
        match self {
            Self::Master { .. } => {}
            Self::Slave {
                master_connection, ..
            } => *master_connection = Some(connection),
        };
    }
}
