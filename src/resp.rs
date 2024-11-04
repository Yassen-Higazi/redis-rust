use regex::Regex;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub enum RespDataTypes {
    #[allow(dead_code)]
    SimpleString(String),

    Integer(i64),

    BulkString(String),

    Array(Vec<RespDataTypes>),

    #[allow(dead_code)]
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

    Set(String, String, Option<Instant>),

    Get(String),

    Config(Vec<String>),
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
                        RespDataTypes::BulkString(cmd_name) => match cmd_name
                            .to_uppercase()
                            .as_str()
                        {
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

                            "SET" => {
                                let mut options = Vec::new();

                                for i in 1..arr.len() {
                                    let record_option = arr.get(i);

                                    match record_option {
                                        None => {
                                            return Err("SET command must be followed by a key");
                                        }

                                        Some(record) => match record {
                                            RespDataTypes::BulkString(string) => {
                                                options.push(string.to_owned());
                                            }

                                            RespDataTypes::Integer(int) => {
                                                options.push(int.to_string());
                                            }

                                            _ => {
                                                return Err("SET command must be fallowed by a key")
                                            }
                                        },
                                    }
                                }

                                let mut expires_at = None;

                                if let Some(exp_unit_str) = options.get(2) {
                                    let now = Instant::now();

                                    let expiration: u64;

                                    if let Some(exp_duration_str) = options.get(3) {
                                        match exp_unit_str.to_lowercase().as_str() {
                                            "px" => {
                                                expiration =
                                                    exp_duration_str.parse::<u64>().unwrap_or(0);
                                            }

                                            "ex" => {
                                                expiration =
                                                    exp_duration_str.parse::<u64>().unwrap_or(0)
                                                        * 1000;
                                            }

                                            _ => return Err("Invalid Duration"),
                                        }

                                        let duration = Duration::from_millis(expiration);

                                        expires_at = Some(now + duration);
                                    }
                                }

                                Ok(Self::Set(
                                    options[0].clone(),
                                    options[1].clone(),
                                    expires_at,
                                ))
                            }

                            "GET" => {
                                let mut options = Vec::new();

                                for i in 1..arr.len() {
                                    let record_option = arr.get(i);

                                    match record_option {
                                        None => {
                                            return Err("Get command must be followed by a key");
                                        }

                                        Some(record) => match record {
                                            RespDataTypes::BulkString(string) => {
                                                options.push(string.to_owned());
                                            }

                                            RespDataTypes::Integer(int) => {
                                                options.push(int.to_string());
                                            }

                                            _ => {
                                                return Err("Get command must be fallowed by a key")
                                            }
                                        },
                                    }
                                }

                                Ok(Self::Get(options[0].clone()))
                            }

                            "CONFIG" => {
                                let mut options: Vec<String> = Vec::new();

                                for i in 1..arr.len() {
                                    let record_option = arr.get(i);

                                    match record_option {
                                        None => {
                                            return Err(
                                                "Config command must be fallowed by a subcommand",
                                            );
                                        }

                                        Some(record) => match record {
                                            RespDataTypes::BulkString(string) => {
                                                options.push(string.to_owned());
                                            }

                                            RespDataTypes::Integer(int) => {
                                                options.push(int.to_string());
                                            }

                                            _ => return Err("Invalid Config Command"),
                                        },
                                    }
                                }

                                Ok(Self::Config(options))
                            }

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
