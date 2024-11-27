use std::collections::HashMap;
use std::time::Instant;
use tokio::sync::Mutex;

use crate::configs::configurations::Configuration;
use crate::persistence::persistence_interface::Persistent;
use crate::resp::{Commands, RespDataTypes};

use anyhow::bail;

pub struct RedisService {
    configs: Mutex<Configuration>,
    persistent_layer: Mutex<Box<dyn Persistent>>,
    storage: Mutex<HashMap<String, (String, Option<Instant>)>>,
}

impl RedisService {
    pub async fn new(configs: Configuration, persistent_layer: Box<dyn Persistent>) -> Self {
        let instance = Self {
            configs: Mutex::new(configs),
            storage: Mutex::new(HashMap::new()),
            persistent_layer: Mutex::new(persistent_layer),
        };

        let mut layer = instance.persistent_layer.lock().await;

        layer
            .load()
            .expect("Could not load data from persistent layer");

        drop(layer);

        instance
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
                        let mut storage_guard = self.storage.lock().await;

                        storage_guard.insert(key, (value.clone(), expiration));

                        "+OK\r\n".as_bytes().to_vec()
                    }

                    Commands::Get(key) => {
                        let mut storage_guard = self.storage.lock().await;

                        let value_opt = storage_guard.get(&key);

                        let mut result = "$-1\r\n".as_bytes().to_vec();

                        if let Some((value, expiration)) = value_opt {
                            let success = format!("${}\r\n{value}\r\n", value.len())
                                .as_bytes()
                                .to_vec();

                            if let Some(instant) = expiration {
                                if instant > &Instant::now() {
                                    result = success;
                                } else {
                                    storage_guard.remove(&key);
                                }
                            } else {
                                result = success
                            }
                        }

                        result
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
                                                    res.push(format!(
                                                        "${}\r\n{}\r\n${}\r\n{}\r\n",
                                                        attr.len(),
                                                        attr,
                                                        value.len(),
                                                        value
                                                    ));
                                                } else {
                                                    bail!("No Config with name {attr}");
                                                };
                                            }

                                            None => bail!("Invalid Config command"),
                                        }
                                    }

                                    let result =
                                        format!("*{}\r\n{}", (options.len() - 1) * 2, res.join(""));

                                    result.as_bytes().to_vec()
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
