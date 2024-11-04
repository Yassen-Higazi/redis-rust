use super::cmd_options::CmdOptions;

#[derive(Debug)]
pub struct Configuration {
    pub port: String,

    pub host: String,

    pub dir: String,

    pub filename: String,
}

impl Configuration {
    pub fn get_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub fn get(&self, attr: &str) -> Option<String> {
        match attr {
            "port" => Some(self.port.clone()),

            "host" => Some(self.host.clone()),

            "dir" => Some(self.dir.clone()),

            "filename" => Some(self.filename.clone()),

            _ => None,
        }
    }
}

impl From<CmdOptions> for Configuration {
    fn from(value: CmdOptions) -> Self {
        Self {
            port: value.port,
            host: value.host,
            dir: value.dir,
            filename: value.filename,
        }
    }
}
