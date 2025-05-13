use regex::Regex;
use std::{collections::HashMap, time::Instant};
use tokio::sync::Mutex;

#[allow(dead_code)]
#[derive(Debug)]
pub struct Database {
    id: u32,
    data_hashmap: Mutex<HashMap<String, (String, Option<Instant>)>>,
}

impl Database {
    pub fn new(id: u32) -> Self {
        println!("Creating new database with id: {id}");

        Database {
            id,
            data_hashmap: Mutex::new(HashMap::new()),
        }
    }

    pub async fn get(&self, key: &str) -> Option<(String, Option<Instant>)> {
        let hashmap = self.data_hashmap.lock().await;

        hashmap.get(key).cloned()
    }

    pub async fn remove(&self, key: &str) {
        let mut hashmap = self.data_hashmap.lock().await;

        hashmap.remove(key);
    }

    pub async fn insert(&self, key: String, value: String, expire_time: Option<Instant>) {
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
