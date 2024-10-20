use anyhow::bail;
use regex::Regex;

#[derive(Debug, Clone)]
pub enum RespDataTypes {
    SimpleString(String),

    Integer(i64),

    BulkString(String),

    Array(Vec<RespDataTypes>),

    SimpleError,
}

impl RespDataTypes {
    fn len(value: &RespDataTypes) -> usize {
        match value {
            Self::Array(arr) => arr.len() + 1,

            Self::BulkString(_) => 2,

            _ => 1,
        }
    }
}

impl TryFrom<Vec<&str>> for RespDataTypes {
    type Error = &'static str;

    fn try_from(value: Vec<&str>) -> Result<Self, Self::Error> {
        let array_re = Regex::new(r"^[*]?\d+$").unwrap();
        let bulk_array_re = Regex::new(r"^[$]?\d+$").unwrap();

        let first = value.first().unwrap_or(&"error");

        match *first {
            "+" => {
                let str = value.get(1);

                match str {
                    None => Err("Invalid SimpleString"),

                    Some(string) => Ok(Self::SimpleString(string.to_string())),
                }
            }

            ":" => {
                let num = value.get(1);

                match num {
                    None => Err("Invalid Integer"),

                    Some(number_str) => {
                        let number = number_str.parse::<i64>();

                        if let Ok(value) = number {
                            Ok(Self::Integer(value))
                        } else {
                            Err("Invalid Integer")
                        }
                    }
                }
            }

            _ if array_re.is_match(first) => {
                let n_str = first;

                let n_result = n_str.replace("*", "").parse::<i64>();

                if let Ok(n) = n_result {
                    let mut columns: Vec<RespDataTypes> = Vec::with_capacity(n as usize);

                    let mut i = 1;

                    while i < value.len() {
                        let column_value = RespDataTypes::try_from(value[i..].to_vec());

                        match column_value {
                            Ok(final_value) => {
                                i += RespDataTypes::len(&final_value);

                                columns.push(final_value);
                            }

                            _ => {
                                return column_value;
                            }
                        }
                    }

                    Ok(Self::Array(columns.clone()))
                } else {
                    Err("Invalid Array")
                }
            }

            _ if bulk_array_re.is_match(first) => {
                let string_option = value.get(1);

                match string_option {
                    None => Err("Invalid BulkString"),

                    Some(string) => Ok(Self::BulkString(string.to_string())),
                }
            }

            _ => Err("invalid Type"),
        }
    }
}

#[derive(Debug)]
pub enum Commands {
    Ping,

    Echo(String),
}

impl TryFrom<RespDataTypes> for Commands {
    type Error = &'static str;

    fn try_from(value: RespDataTypes) -> Result<Self, Self::Error> {
        println!("Data: {value:?}");

        match value {
            RespDataTypes::Array(arr) => {
                let command_name = arr.first();

                if let Some(command_name) = command_name {
                    match command_name {
                        RespDataTypes::BulkString(cmd_name) => match cmd_name.as_str() {
                            "ECHO" => {
                                let mut key = String::new();

                                for i in 1..arr.len() {
                                    let record_option = arr.get(i);

                                    match record_option {
                                        None => {
                                            return Err("Echo command must be followed by a key");
                                        }

                                        Some(record) => match record {
                                            RespDataTypes::BulkString(string) => {
                                                key.push_str(string.as_str());
                                            }

                                            RespDataTypes::Integer(int) => {
                                                key.push_str(int.to_string().as_str());
                                            }

                                            _ => {
                                                return Err(
                                                    "Echo command must be fallowed by a key",
                                                )
                                            }
                                        },
                                    }
                                }

                                Ok(Self::Echo(key))
                            }

                            "PING" => Ok(Commands::Ping),

                            _ => Err("Invalid Command"),
                        },

                        _ => Err("Invalid command"),
                    }
                } else {
                    Err("Invalid Command")
                }
            }

            _ => Err("Invalid Command"),
        }
    }
}

pub struct RespService {}

impl RespService {
    pub fn new() -> Self {
        Self {}
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
