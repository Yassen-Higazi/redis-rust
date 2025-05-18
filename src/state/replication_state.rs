#[derive(Debug, Clone)]
pub struct Replication {
    pub master: Option<String>,
    pub slaves: Vec<String>,
}

impl Replication {
    pub fn new() -> Self {
        Self {
            master: None,
            slaves: Vec::new(),
        }
    }
}
