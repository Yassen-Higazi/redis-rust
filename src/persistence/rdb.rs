use std::fs::{File, OpenOptions};
use std::io::{BufReader, Read};
use std::path::PathBuf;

use anyhow::{ensure, Context};

use super::persistence_interface::Persistent;

#[allow(dead_code)]
pub struct RDB {
    reader: BufReader<File>,
}

impl RDB {
    pub fn new(file_path: &PathBuf) -> anyhow::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(file_path)?;

        Ok(Self {
            reader: BufReader::new(file),
        })
    }
}

impl Persistent for RDB {
    fn save(&self) -> anyhow::Result<()> {
        todo!()
    }

    fn load(&mut self) -> anyhow::Result<()> {
        let mut header_buf: [u8; 5] = [0; 5];

        self.reader
            .read_exact(&mut header_buf)
            .with_context(|| "Could not read rdb file")?;

        let magic_string = String::from_utf8(header_buf.to_vec())?;

        println!("{magic_string}");

        ensure!(magic_string == "REDIS", "Invalid rdb file");

        let mut metadata_buf: [u8; 50] = [0; 50];

        self.reader.read_exact(&mut metadata_buf)?;

        println!("{}", String::from_utf8(metadata_buf.to_vec())?);

        Ok(())
    }
}
