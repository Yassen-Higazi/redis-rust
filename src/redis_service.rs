use chrono::Utc;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::RwLock;

use crate::database::Database;
use crate::persistence::persistence_interface::Persistent;
use crate::resp::{Commands, RespDataTypes};
use crate::state::server_state::ServerState;

use anyhow::{bail, Context};

#[derive(Debug)]
pub struct RedisService {
    selected_db: u32,
    state: Arc<RwLock<ServerState>>,
    databases: RwLock<HashMap<u32, Arc<Database>>>,
}

impl RedisService {
    pub async fn new(
        configs: Arc<RwLock<ServerState>>,
        mut persistent_layer: Box<dyn Persistent>,
    ) -> Self {
        let databases = persistent_layer
            .load()
            .expect("Could not load data from persistent layer");

        Self {
            selected_db: 0,
            state: configs,
            databases: RwLock::new(databases),
        }
    }

    async fn get_selected_db(&self) -> Arc<Database> {
        return self
            .databases
            .read()
            .await
            .get(&self.selected_db)
            .expect("No DB found")
            .clone();
    }

    async fn read_rdb_file(&self, path: PathBuf) -> anyhow::Result<Vec<u8>> {
        println!("RDB Path: {path:?}");

        let buffer = if let Ok(file) = OpenOptions::new()
            .read(true)
            .open(path)
            .await
            .with_context(|| "could not open rdb file")
        {
            let mut buffer: Vec<u8> = Vec::new();

            let mut reader = BufReader::new(file);

            reader.read_to_end(&mut buffer).await?;

            buffer
        } else {
            hex::decode("524544495330303131fa0972656469732d76657205372e322e30fa0a72656469732d62697473c040fa056374696d65c26d08bc65fa08757365642d6d656dc2b0c41000fa08616f662d62617365c000fff06e3bfec0ff5aa2").unwrap()
        };

        Ok(buffer)
    }

    pub async fn execute_command(
        &self,
        command: &str,
        stream: &mut TcpStream,
    ) -> anyhow::Result<()> {
        let command_vec = command.split("\r\n").collect::<Vec<&str>>();

        let data_result = RespDataTypes::try_from(command_vec[..command_vec.len() - 1].to_vec());

        let data = data_result.expect("Invalid command");

        let cmd = Commands::try_from(data);

        match cmd {
            Ok(cmd) => {
                let response = match cmd {
                    Commands::Ping => Some(RespDataTypes::SimpleString("PONG".to_string())),

                    Commands::Echo(message) => Some(RespDataTypes::SimpleString(message)),

                    Commands::Set(key, value, expiration) => {
                        let db = self.get_selected_db().await;

                        db.insert(key, value.clone(), expiration).await;

                        Some(RespDataTypes::SimpleString("OK".to_string()))
                    }

                    Commands::Get(key) => {
                        let db = self.get_selected_db().await;

                        let value_opt = db.get(&key).await;

                        let mut result = RespDataTypes::SimpleError(None);

                        if let Some((value, expiration)) = value_opt {
                            let success = RespDataTypes::BulkString(value);

                            if let Some(instant) = expiration {
                                if instant > Utc::now() {
                                    result = success;
                                } else {
                                    db.remove(&key).await;
                                }
                            } else {
                                result = success
                            }
                        }

                        println!("Get result: {}", result);

                        Some(result)
                    }

                    Commands::Keys(key) => {
                        let db = self.get_selected_db().await;

                        let result_vec = if key == "*" {
                            db.keys().await
                        } else {
                            db.keys_from_pattren(&key).await
                        };

                        Some(RespDataTypes::from(result_vec))
                    }

                    Commands::Config(options) => {
                        if let Some(subcommand) = options.first() {
                            match subcommand.to_uppercase().as_str() {
                                "GET" => {
                                    let mut res = Vec::new();

                                    for i in 1..options.len() {
                                        let attribute = options.get(i);

                                        match attribute {
                                            Some(attr) => {
                                                let value = self
                                                    .state
                                                    .read()
                                                    .await
                                                    .get_from_config(attr)
                                                    .await;

                                                if let Some(value) = value {
                                                    res.push(attr.to_owned());
                                                    res.push(value);
                                                } else {
                                                    bail!("No Config with name {attr}");
                                                };
                                            }

                                            None => bail!("Invalid Config command"),
                                        }
                                    }

                                    Some(RespDataTypes::from(res))
                                }

                                _ => {
                                    bail!("Invalid Config command")
                                }
                            }
                        } else {
                            bail!("Invalid Config command")
                        }
                    }

                    Commands::Info(key) => {
                        let result = match key.unwrap_or("*".to_string()).to_uppercase().as_str() {
                            "REPLICATION" => self.state.read().await.get_replication_status(),

                            _ => bail!("Invalid Info Sub command"),
                        };

                        Some(RespDataTypes::BulkString(result))
                    }

                    Commands::REPLCONF(op1, op2) => {
                        println!("REPLCONF {op1} {op2}");

                        Some(RespDataTypes::SimpleString("OK".to_string()))
                    }

                    Commands::PSYNC(op1, op2) => {
                        println!("PSYNC {op1} {op2}");

                        let mut server_state = self.state.write().await;

                        let res = server_state.psync().await?;

                        stream.write_all(res.to_string().as_bytes()).await?;

                        let path = server_state.get_rdb_path().await;

                        let buffer = self.read_rdb_file(path).await?;

                        stream
                            .write_all(
                                [
                                    format!("${}\r\n", buffer.len()).as_bytes(),
                                    buffer.as_slice(),
                                ]
                                .concat()
                                .as_slice(),
                            )
                            .await
                            .with_context(|| "could not write to stream")?;

                        None
                    }
                };

                match response {
                    Some(resp) => {
                        println!("Response: {resp:?}");

                        stream
                            .write_all(resp.to_string().as_bytes())
                            .await
                            .with_context(|| "could not write to stream")
                            .map_err(|e| {
                                println!("{e:?}");
                            })
                            .unwrap_or(());
                    }

                    None => {
                        println!("No response");
                    }
                }

                Ok(())
            }

            Err(message) => bail!(message),
        }
    }
}
