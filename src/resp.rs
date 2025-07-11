use anyhow::bail;
use chrono::{DateTime, Utc};
use regex::Regex;
use std::{fmt::Display, time::Duration};

#[derive(Debug, Clone)]
pub enum RespDataTypes {
    #[allow(dead_code)]
    SimpleString(String),

    Integer(i64),

    BulkString(String),

    Array(Vec<RespDataTypes>),

    SimpleError(Option<String>),
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

impl Display for RespDataTypes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SimpleString(value) => write!(f, "+{value}\r\n"),

            Self::Integer(value) => write!(f, ":{}\r\n", value),

            Self::BulkString(value) => write!(f, "${}\r\n{}\r\n", value.len(), value),

            Self::Array(arr) => {
                let mut result = String::new();

                for item in arr {
                    result.push_str(&item.to_string());
                }

                write!(f, "*{}\r\n{}", arr.len(), result)
            }

            Self::SimpleError(str) => {
                write!(f, "$-{}\r\n", str.to_owned().unwrap_or("1".to_string()))
            }
        }
    }
}

impl From<Vec<String>> for RespDataTypes {
    fn from(value: Vec<String>) -> Self {
        let mut columns: Vec<RespDataTypes> = Vec::with_capacity(value.len());

        for item in value {
            columns.push(RespDataTypes::from(item));
        }

        Self::Array(columns)
    }
}

impl From<String> for RespDataTypes {
    fn from(value: String) -> Self {
        let result = value.parse::<f64>();

        match result {
            Ok(num) => Self::Integer(num as i64),

            Err(_) => Self::BulkString(value),
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

    Set(String, String, Option<DateTime<Utc>>),

    Get(String),

    Config(Vec<String>),

    Keys(String),

    Info(Option<String>),

    REPLCONF(String, String),

    PSYNC(String, String),
}

impl Commands {
    fn decode_command_options(
        arr: &[RespDataTypes],
        name: &str,
        must_have_options: bool,
    ) -> anyhow::Result<Vec<String>> {
        let mut options = Vec::new();

        if arr.len() < 2 {
            if must_have_options {
                bail!("{name} command must be followed by a key");
            } else {
                return Ok(options);
            }
        }

        for i in 1..arr.len() {
            let record_option = arr.get(i);

            match record_option {
                None => {
                    if must_have_options {
                        bail!("{name} command must be followed by a key");
                    } else {
                        break;
                    }
                }

                Some(record) => match record {
                    RespDataTypes::BulkString(string) => {
                        options.push(string.to_owned());
                    }

                    RespDataTypes::Integer(int) => {
                        options.push(int.to_string());
                    }

                    _ => bail!("{name} command must be followed by a key"),
                },
            }
        }

        Ok(options)
    }
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
                                let options =
                                    Self::decode_command_options(&arr, "ECHO", true).unwrap();

                                Ok(Self::Echo(options[0].clone()))
                            }

                            "PING" => Ok(Commands::Ping),

                            "SET" => {
                                let options =
                                    Self::decode_command_options(&arr, "SET", true).unwrap();

                                let mut expires_at = None;

                                if let Some(exp_unit_str) = options.get(2) {
                                    let now = Utc::now();

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
                                let options =
                                    Self::decode_command_options(&arr, "GET", true).unwrap();

                                Ok(Self::Get(options[0].clone()))
                            }

                            "KEYS" => {
                                let options =
                                    Self::decode_command_options(&arr, "KEYS", true).unwrap();

                                Ok(Self::Keys(options[0].clone()))
                            }

                            "CONFIG" => {
                                let options =
                                    Self::decode_command_options(&arr, "CONFIG", true).unwrap();

                                Ok(Self::Config(options))
                            }

                            "INFO" => {
                                let options =
                                    Self::decode_command_options(&arr, "INFO", false).unwrap();

                                Ok(Self::Info(options.first().cloned()))
                            }

                            "REPLCONF" => {
                                let options =
                                    Self::decode_command_options(&arr, "REPLCONF", true).unwrap();

                                if options.len() == 2 {
                                    Ok(Self::REPLCONF(options[0].clone(), options[1].clone()))
                                } else {
                                    Err("Invalid REPLCONF command")
                                }
                            }

                            "PSYNC" => {
                                let options =
                                    Self::decode_command_options(&arr, "PSYNC", true).unwrap();

                                if options.len() == 2 {
                                    Ok(Self::PSYNC(options[0].clone(), options[1].clone()))
                                } else {
                                    Err("Invalid PSYNC command")
                                }
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
