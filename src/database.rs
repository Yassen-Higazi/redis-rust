use chrono::{DateTime, Utc};
use regex::Regex;
use std::collections::HashMap;
use tokio::sync::Mutex;

pub type Record = (String, Option<DateTime<Utc>>);

#[allow(dead_code)]
#[derive(Debug)]
pub struct Database {
    id: u32,

    data_hashmap: Mutex<HashMap<String, Record>>,
}

impl Database {
    pub fn new(id: u32) -> Self {
        Database {
            id,
            data_hashmap: Mutex::new(HashMap::new()),
        }
    }

    pub async fn get(&self, key: &str) -> Option<(String, Option<DateTime<Utc>>)> {
        let hashmap = self.data_hashmap.lock().await;

        hashmap.get(key).cloned()
    }

    pub async fn remove(&self, key: &str) {
        let mut hashmap = self.data_hashmap.lock().await;

        hashmap.remove(key);
    }

    pub async fn insert(&self, key: String, value: String, expire_time: Option<DateTime<Utc>>) {
        let mut hashmap = self.data_hashmap.lock().await;

        hashmap.insert(key, (value, expire_time));
    }

    pub async fn keys(&self) -> Vec<String> {
        let hashmap = self.data_hashmap.lock().await;

        hashmap.keys().cloned().collect()
    }

    pub async fn keys_from_pattren(&self, pattern: &str) -> Vec<String> {
        let hashmap = self.data_hashmap.lock().await;

        let re = Regex::new(pattern).expect("Invalid regex pattern");

        hashmap
            .keys()
            .filter(|key| re.is_match(key))
            .cloned()
            .collect()
    }
}
