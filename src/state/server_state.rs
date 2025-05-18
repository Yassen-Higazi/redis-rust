use std::path::PathBuf;

use tokio::sync::RwLock;

use crate::configs::{cmd_options::CmdOptions, configurations::Configuration};

use super::replication_state::Replication;

#[allow(dead_code)]
#[derive(Debug)]
pub struct ServerState {
    id: String,
    config: RwLock<Configuration>,
    replication: Replication,
}

impl ServerState {
    pub fn new(config: Configuration) -> Self {
        Self {
            id: String::new(),
            config: RwLock::new(config),
            replication: Replication::new(),
        }
    }

    pub async fn get_address(&self) -> String {
        self.config.read().await.get_address()
    }

    pub async fn get_rdb_path(&self) -> PathBuf {
        self.config.read().await.get_rdb_path()
    }

    pub async fn get_from_config(&self, key: &str) -> Option<String> {
        self.config.read().await.get(key)
    }
}

impl From<CmdOptions> for ServerState {
    fn from(cmd_options: CmdOptions) -> Self {
        let config = Configuration::from(cmd_options);
        Self::new(config)
    }
}
