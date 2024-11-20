pub trait Persistent {
    fn save(&self) -> anyhow::Result<()>;

    fn load(&mut self) -> anyhow::Result<()>;
}
