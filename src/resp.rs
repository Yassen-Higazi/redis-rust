use anyhow::bail;

#[derive(Debug)]
pub enum RespDataTypes {
    SimpleString,

    RespError,

    Integer,
}

impl TryFrom<&str> for RespDataTypes {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "+" => Ok(Self::SimpleString),

            "-" => Ok(Self::RespError),

            ":" => Ok(Self::Integer),

            _ => Err("invalid Type"),
        }
    }
}

#[derive(Debug)]
pub enum Commands {
    Ping,

    Echo(String),
}

impl TryFrom<Vec<&str>> for Commands {
    type Error = &'static str;

    fn try_from(value: Vec<&str>) -> Result<Self, Self::Error> {
        let command_str = value.first();

        if let Some(cmd) = command_str {
            match *cmd {
                "PING" => Ok(Self::Ping),

                "ECHO" => {
                    let key = value.get(1);

                    if let Some(key) = key {
                        Ok(Self::Echo(key.to_string()))
                    } else {
                        Err("Echo Command must be followed by a key")
                    }
                }

                _ => Err("Invalid Command"),
            }
        } else {
            Err("Invalid Command")
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

        println!("Command Vec: {command_vec:?}");

        let cmd = Commands::try_from(command_vec[2..].to_vec());

        println!("Command: {cmd:?}");

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
    }
}
