use std::result;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use std::{collections::HashMap, time::SystemTime};

use crate::resp::{Commands, RespDataTypes};

use anyhow::bail;

pub struct RedisService {
    storage: Mutex<HashMap<String, (String, Option<Instant>)>>,
}

impl RedisService {
    pub fn new() -> Self {
        Self {
            storage: Mutex::new(HashMap::new()),
        }
    }

    pub async fn execute_command(&self, command: &str) -> anyhow::Result<Vec<u8>> {
        let command_vec = command.split("\r\n").collect::<Vec<&str>>();

        let data = RespDataTypes::try_from(command_vec[..command_vec.len() - 1].to_vec());

        if let Ok(data) = data {
            let cmd = Commands::try_from(data);

            println!("Commands: {:?}", cmd);

            match cmd {
                Ok(cmd) => {
                    let response = match cmd {
                        Commands::Ping => b"+PONG\r\n".to_vec(),

                        Commands::Echo(message) => format!("+{message}\r\n").as_bytes().to_vec(),

                        Commands::Set(key, value, expiration) => {
                            let lock = self.storage.lock();

                            match lock {
                                Ok(mut storage) => {
                                    storage.insert(key, (value.clone(), expiration));
                                }

                                Err(err) => {
                                    eprintln!("Error: {err:?}");

                                    bail!("Internal Error")
                                }
                            }

                            "+OK\r\n".as_bytes().to_vec()
                        }

                        Commands::Get(key) => {
                            let lock = self.storage.lock();

                            match lock {
                                Ok(mut storage) => {
                                    let value_opt = storage.get(&key);

                                    let mut result = "$-1\r\n".as_bytes().to_vec();

                                    if let Some((value, expiration)) = value_opt {
                                        let success = format!("${}\r\n{value}\r\n", value.len())
                                            .as_bytes()
                                            .to_vec();

                                        if let Some(instant) = expiration {
                                            if instant > &Instant::now() {
                                                result = success;
                                            } else {
                                                storage.remove(&key);
                                            }
                                        } else {
                                            result = success
                                        }
                                    }

                                    result
                                }

                                Err(err) => {
                                    eprintln!("Error: {err:?}");

                                    bail!("Internal Error")
                                }
                            }
                        }
                    };

                    Ok(response)
                }

                Err(message) => bail!(message),
            }
        } else {
            bail!("Invalid Command");
        }
    }
}
