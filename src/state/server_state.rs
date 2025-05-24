use std::{path::PathBuf, sync::Arc};

use tokio::{net::TcpStream, sync::Mutex};

use crate::{
    configs::{cmd_options::CmdOptions, configurations::Configuration},
    resp::RespDataTypes,
};

use super::replication_state::Replica;

#[allow(dead_code)]
#[derive(Debug)]
pub struct ServerState {
    id: String,
    replication: Replica,
    config: Configuration,
}

impl ServerState {
    pub fn new(config: Configuration) -> Self {
        Self {
            id: String::new(),
            replication: Replica::new(
                config.port.parse::<u16>().unwrap(),
                config.replication_role.clone(),
                config.get_master_address().clone(),
            ),
            config,
        }
    }

    pub fn get_address(&self) -> String {
        self.config.get_address()
    }

    pub fn get_rdb_path(&self) -> PathBuf {
        self.config.get_rdb_path()
    }

    pub fn get_from_config(&self, key: &str) -> Option<String> {
        self.config.get(key)
    }

    pub fn get_replication_status(&self) -> String {
        self.replication.get_replication_status()
    }

    pub async fn init_replica(&mut self) -> anyhow::Result<Arc<Mutex<TcpStream>>> {
        self.replication.init().await
    }

    pub async fn psync(&mut self) -> anyhow::Result<RespDataTypes> {
        self.replication.psync(None).await
    }

    pub async fn register_replica(
        &mut self,
        connection: Arc<Mutex<TcpStream>>,
    ) -> anyhow::Result<()> {
        self.replication.register_replica(connection).await
    }

    pub async fn replicate_command(&mut self, command: &RespDataTypes) -> anyhow::Result<()> {
        self.replication.replicate_command(command).await
    }
}

impl From<CmdOptions> for ServerState {
    fn from(cmd_options: CmdOptions) -> Self {
        let config = Configuration::from(cmd_options);
        Self::new(config)
    }
}
