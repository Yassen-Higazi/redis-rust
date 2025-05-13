use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{Mutex, RwLock};

use crate::configs::configurations::Configuration;
use crate::database::Database;
use crate::persistence::persistence_interface::Persistent;
use crate::resp::{Commands, RespDataTypes};

use anyhow::bail;

pub struct RedisService {
    selected_db: u32,
    configs: Mutex<Configuration>,
    databases: RwLock<HashMap<u32, Arc<Database>>>,
}

impl RedisService {
    pub async fn new(configs: Configuration, mut persistent_layer: Box<dyn Persistent>) -> Self {
        let databases = persistent_layer
            .load()
            .expect("Could not load data from persistent layer");

        Self {
            selected_db: 0,
            configs: Mutex::new(configs),
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

    pub async fn execute_command(&self, command: &str) -> anyhow::Result<Vec<u8>> {
        let command_vec = command.split("\r\n").collect::<Vec<&str>>();

        let data_result = RespDataTypes::try_from(command_vec[..command_vec.len() - 1].to_vec());

        let data = data_result.expect("Invalid command");

        let cmd = Commands::try_from(data);

        match cmd {
            Ok(cmd) => {
                let response = match cmd {
                    Commands::Ping => b"+PONG\r\n".to_vec(),

                    Commands::Echo(message) => format!("+{message}\r\n").as_bytes().to_vec(),

                    Commands::Set(key, value, expiration) => {
                        let db = self.get_selected_db().await;

                        db.insert(key, value.clone(), expiration).await;

                        "+OK\r\n".as_bytes().to_vec()
                    }

                    Commands::Get(key) => {
                        let db = self.get_selected_db().await;

                        let value_opt = db.get(&key).await;

                        let mut result = "$-1\r\n".as_bytes().to_vec();

                        if let Some((value, expiration)) = value_opt {
                            let success = format!("${}\r\n{value}\r\n", value.len())
                                .as_bytes()
                                .to_vec();

                            if let Some(instant) = expiration {
                                if instant > Instant::now() {
                                    result = success;
                                } else {
                                    db.remove(&key).await;
                                }
                            } else {
                                result = success
                            }
                        }
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
                            .to_string()
                            .as_bytes()
                            .to_vec()
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
                                                let configs = self.configs.lock().await;

                                                if let Some(value) = configs.get(attr) {
                                                    res.push(attr.to_owned());
                                                    res.push(value);
                                                } else {
                                                    bail!("No Config with name {attr}");
                                                };
                                            }

                                            None => bail!("Invalid Config command"),
                                        }
                                    }

                                    RespDataTypes::from(res).to_string().as_bytes().to_vec()
                                }

                                _ => {
                                    bail!("Invalid Config command")
                                }
                            }
                        } else {
                            bail!("Invalid Config command")
                        }
                    }
                };

                Ok(response)
            }

            Err(message) => bail!(message),
        }
    }
}
