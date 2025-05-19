use std::{net::SocketAddr, path::PathBuf};

use crate::state::replication_state::Role;

use super::cmd_options::CmdOptions;

#[derive(Debug, Clone)]
pub struct Configuration {
    pub port: String,

    pub host: String,

    pub dir: String,

    pub filename: String,

    pub master_address: Option<SocketAddr>,

    pub replication_role: Role,
}

impl Configuration {
    pub fn get_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub fn get_rdb_path(&self) -> PathBuf {
        PathBuf::from(format!("{}/{}", self.dir, self.filename))
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

    pub fn get_master_address(&self) -> Option<SocketAddr> {
        self.master_address
    }
}

impl From<CmdOptions> for Configuration {
    fn from(value: CmdOptions) -> Configuration {
        Self {
            dir: value.dir,
            port: value.port,
            host: value.host,
            filename: value.filename,
            master_address: value.replicatof,
            replication_role: value.replicatof.map_or(Role::Master, |_| Role::Slave),
        }
    }
}
