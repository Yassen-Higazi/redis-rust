use std::{collections::HashMap, sync::Arc};

use crate::database::Database;

#[allow(dead_code)]
pub trait Persistent: Sync + Send {
    fn save(&self) -> anyhow::Result<()>;

    fn load(&mut self) -> anyhow::Result<HashMap<u32, Arc<Database>>>;
}
