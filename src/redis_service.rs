use std::collections::HashMap;
use std::sync::RwLock;

use crate::resp::{Commands, RespDataTypes};

use anyhow::bail;

pub struct RedisService {
    storage: RwLock<HashMap<String, String>>,
}

impl RedisService {
    pub fn new() -> Self {
        Self {
            storage: RwLock::new(HashMap::new()),
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

                        Commands::Set(key, value) => {
                            let lock = self.storage.write();

                            match lock {
                                Ok(mut storage) => {
                                    storage.insert(key, value.clone());
                                }

                                Err(err) => {
                                    eprintln!("Error: {err:?}");

                                    bail!("Internal Error")
                                }
                            }

                            "+OK\r\n".as_bytes().to_vec()
                        }

                        Commands::Get(key) => {
                            let lock = self.storage.read();

                            match lock {
                                Ok(storage) => {
                                    let value_opt = storage.get(&key);

                                    if let Some(value) = value_opt {
                                        format!("+{value}\r\n").as_bytes().to_vec()
                                    } else {
                                        bail!("Internal Error")
                                    }
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
