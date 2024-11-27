#[allow(dead_code)]
pub trait Persistent: Sync + Send {
    fn save(&self) -> anyhow::Result<()>;

    fn load(&mut self) -> anyhow::Result<()>;
}
