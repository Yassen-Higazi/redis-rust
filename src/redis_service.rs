use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::database::Database;
use crate::persistence::persistence_interface::Persistent;
use crate::resp::{Commands, RespDataTypes};
use crate::state::server_state::ServerState;

use anyhow::bail;

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

    pub async fn execute_command(&self, command: &str) -> anyhow::Result<RespDataTypes> {
        let command_vec = command.split("\r\n").collect::<Vec<&str>>();

        let data_result = RespDataTypes::try_from(command_vec[..command_vec.len() - 1].to_vec());

        let data = data_result.expect("Invalid command");

        let cmd = Commands::try_from(data);

        match cmd {
            Ok(cmd) => {
                let response = match cmd {
                    Commands::Ping => RespDataTypes::SimpleString("PONG".to_string()),

                    Commands::Echo(message) => RespDataTypes::SimpleString(message),

                    Commands::Set(key, value, expiration) => {
                        let db = self.get_selected_db().await;

                        db.insert(key, value.clone(), expiration).await;

                        RespDataTypes::SimpleString("OK".to_string())
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

                        result
                    }

                    Commands::Keys(key) => {
                        let db = self.get_selected_db().await;

                        let result_vec = if key == "*" {
                            db.keys().await
                        } else {
                            db.keys_from_pattren(&key).await
                        };

                        RespDataTypes::from(result_vec)
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

                                    RespDataTypes::from(res)
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

                        RespDataTypes::BulkString(result)
                    }
                };

                Ok(response)
            }

            Err(message) => bail!(message),
        }
    }
}
