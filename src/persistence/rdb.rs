use std::collections::HashMap;
use std::fmt::Display;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, Read};
use std::path::PathBuf;
use std::sync::Arc;

use crate::database::Database;

use anyhow::{bail, ensure, Context};

use chrono::{DateTime, Utc};

use super::persistence_interface::Persistent;

#[derive(Debug)]
enum KeyType {
    String,
    List,
    Set,
    SortedSet,
    Hash,
    Zipmap,
    Ziplist,
    Intset,
    ZHashMap,
    ZSortedSet,
    ListQuickList,
}

impl Display for KeyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyType::String => write!(f, "String"),
            KeyType::List => write!(f, "List"),
            KeyType::Set => write!(f, "Set"),
            KeyType::SortedSet => write!(f, "SortedSet"),
            KeyType::Hash => write!(f, "Hash"),
            KeyType::Zipmap => write!(f, "Zipmap"),
            KeyType::Ziplist => write!(f, "Ziplist"),
            KeyType::Intset => write!(f, "Intset"),
            KeyType::ZHashMap => write!(f, "ZHashMap"),
            KeyType::ZSortedSet => write!(f, "ZSortedSet"),
            KeyType::ListQuickList => write!(f, "ListQuickList"),
        }
    }
}

impl From<u8> for KeyType {
    fn from(value: u8) -> Self {
        match value {
            0x00 => KeyType::String,
            0x01 => KeyType::List,
            0x02 => KeyType::Set,
            0x03 => KeyType::SortedSet,
            0x04 => KeyType::Hash,
            0x09 => KeyType::Zipmap,
            0x0A => KeyType::Ziplist,
            0x0B => KeyType::Intset,
            0x0C => KeyType::ZSortedSet,
            0x0D => KeyType::ZHashMap,
            0x0E => KeyType::ListQuickList,

            _ => panic!("Invalid key type"),
        }
    }
}

#[allow(dead_code)]
#[derive(PartialEq, Eq, Debug)]
enum OperationCode {
    Eof,

    SelectDb,

    // Expiretime,
    //
    // ExpiretimeMs,
    ResizeDb,

    Aux,
}

impl Display for OperationCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OperationCode::Eof => write!(f, "EOF"),
            OperationCode::Aux => write!(f, "AUX"),
            OperationCode::SelectDb => write!(f, "SELECTDB"),
            OperationCode::ResizeDb => write!(f, "RESIZE_DB"),
            // OperationCode::Expiretime => write!(f, "EXPIRETIME"),
            // OperationCode::ExpiretimeMs => write!(f, "EXPIRETIME_MS"),
        }
    }
}

impl TryFrom<&u8> for OperationCode {
    type Error = &'static str;

    fn try_from(value: &u8) -> Result<Self, Self::Error> {
        match value {
            0xFF => Ok(OperationCode::Eof),

            0xFE => Ok(OperationCode::SelectDb),

            // 0xFD => Ok(OperationCode::Expiretime),
            //
            // 0xFC => Ok(OperationCode::ExpiretimeMs),
            0xFB => Ok(OperationCode::ResizeDb),

            0xFA => Ok(OperationCode::Aux),

            _ => Err("Invalid operation code"),
        }
    }
}

#[allow(dead_code, clippy::upper_case_acronyms)]
pub struct RDB {
    reader: Option<BufReader<File>>,
}

impl RDB {
    pub fn new(file_path: &PathBuf) -> anyhow::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(file_path);

        let buff_option = match file {
            Ok(file) => Some(BufReader::new(file)),

            Err(e) => match e.kind() {
                std::io::ErrorKind::NotFound => None,

                _ => bail!("Could not open file: {}", e),
            },
        };

        Ok(Self {
            reader: buff_option,
        })
    }

    fn decode_length(&self, data: &[u8]) -> anyhow::Result<(usize, usize)> {
        let first_byte = data[0];

        let mut next_byte_idx = 1usize;

        let bit1 = (first_byte & 0b10000000) >> 7;

        let bit2 = (first_byte & 0b01000000) >> 6;

        let len = match (bit1, bit2) {
            (0, 1) => {
                let byte1 = (first_byte & 0x3F) as u16;
                let byte2 = data[next_byte_idx] as u16;
                next_byte_idx += 1;

                ((byte1 << 8) | byte2) as usize
            }

            (1, 0) => {
                next_byte_idx += 4;

                u32::from_be_bytes([
                    data[next_byte_idx],
                    data[next_byte_idx + 1],
                    data[next_byte_idx + 2],
                    data[next_byte_idx + 3],
                ]) as usize
            }

            // handle (0, 0) and (1, 1)
            _ => (first_byte & 0b00111111) as usize,
        };

        Ok((len, next_byte_idx))
    }

    fn decode_string(&self, data: &[u8]) -> anyhow::Result<(String, usize)> {
        let (len, next_byte_idx) = self.decode_length(data)?;

        let mut last_idx = next_byte_idx;

        match len {
            // 8 bit unsigned integer as string
            0 => {
                let num = u8::from_le_bytes([data[next_byte_idx]]);

                last_idx += 1;

                Ok((num.to_string(), last_idx))
            }

            // 16 bit unsigned integer as string
            1 => {
                let num = u16::from_le_bytes([data[next_byte_idx], data[next_byte_idx + 1]]);

                last_idx += 2;

                Ok((num.to_string(), last_idx))
            }

            // 32 bit unsigned integer as string
            2 => {
                let num = u32::from_le_bytes([
                    data[next_byte_idx],
                    data[next_byte_idx + 1],
                    data[next_byte_idx + 2],
                    data[next_byte_idx + 3],
                ]);

                last_idx += 4;

                Ok((num.to_string(), last_idx))
            }

            // Compressed String
            3 => todo!(),

            // Length Prefixed String
            _ => {
                last_idx += len;

                ensure!(last_idx <= data.len(), "Invalid length");

                Ok((
                    String::from_utf8(data[next_byte_idx..last_idx].to_vec())?,
                    last_idx,
                ))
            }
        }
    }

    fn decode_expiration_time(
        &self,
        data: &[u8],
    ) -> anyhow::Result<Option<(DateTime<Utc>, usize)>> {
        let first_byte = data[0];

        let mut next_byte_idx = 1;

        let experation_duration = match first_byte {
            // Timestampe in secends
            0xFD => {
                let unix_time = u32::from_be_bytes([
                    data[next_byte_idx + 3],
                    data[next_byte_idx + 2],
                    data[next_byte_idx + 1],
                    data[next_byte_idx],
                ]);

                next_byte_idx += 4;

                let expiration_time =
                    DateTime::from_timestamp(unix_time as i64, 0).expect("Invalid timestamp");

                Some((expiration_time, next_byte_idx))
            }

            // Timestampe in miliseconds
            0xFC => {
                let unix_time = u64::from_be_bytes([
                    data[next_byte_idx + 7],
                    data[next_byte_idx + 6],
                    data[next_byte_idx + 5],
                    data[next_byte_idx + 4],
                    data[next_byte_idx + 3],
                    data[next_byte_idx + 2],
                    data[next_byte_idx + 1],
                    data[next_byte_idx],
                ]);

                next_byte_idx += 8;

                let expiration_time =
                    DateTime::from_timestamp_millis(unix_time as i64).expect("Invalid timestamp");

                Some((expiration_time, next_byte_idx))
            }

            _ => None,
        };

        Ok(experation_duration)
    }

    fn decode_key_value(
        &self,
        data: &[u8],
        key_type: &KeyType,
    ) -> anyhow::Result<(Vec<u8>, usize)> {
        match key_type {
            KeyType::String => {
                let (key, next_idx) = self
                    .decode_string(data)
                    .with_context(|| "Could not parse key value for Type String")?;

                Ok((key.as_bytes().to_vec(), next_idx))
            }

            KeyType::List => todo!(),

            KeyType::Set => todo!(),

            KeyType::SortedSet => todo!(),

            KeyType::Hash => todo!(),

            KeyType::Zipmap => todo!(),

            KeyType::Ziplist => todo!(),

            KeyType::Intset => todo!(),

            KeyType::ZHashMap => todo!(),

            KeyType::ZSortedSet => todo!(),

            KeyType::ListQuickList => todo!(),
        }
    }

    fn decode_key(
        &self,
        data: &[u8],
    ) -> anyhow::Result<(String, Vec<u8>, KeyType, Option<DateTime<Utc>>, usize)> {
        let mut current_idx = 0usize;

        let expiration_time = self
            .decode_expiration_time(data)
            .with_context(|| "Could not parse expiration time")?;

        let expiration_time = match expiration_time {
            Some((expiration_time, next_idx)) => {
                current_idx += next_idx;

                Some(expiration_time)
            }

            None => None,
        };

        let key_type = KeyType::from(data[current_idx]);

        current_idx += 1;

        let (key_name, next_idx) = self
            .decode_string(&data[current_idx..])
            .with_context(|| "Could not parse key name")?;

        current_idx += next_idx;

        let (key_value, next_idx) = self.decode_key_value(&data[current_idx..], &key_type)?;

        current_idx += next_idx;

        Ok((key_name, key_value, key_type, expiration_time, current_idx))
    }

    async fn parse_file(&self, data: &[u8]) -> anyhow::Result<HashMap<u32, Arc<Database>>> {
        let mut current_idx = 0;

        let mut headers = HashMap::<String, String>::new();

        let mut databases = HashMap::<u32, Arc<Database>>::new();

        let mut selected_db: u32 = 0;
        let mut db_hashmap_size: u32;
        let mut expiration_hashmap_size: u32;

        loop {
            let op_code = OperationCode::try_from(&data[current_idx]);

            current_idx += 1;

            match &op_code {
                Ok(code) => match code {
                    OperationCode::Aux => {
                        let (key_string, key_next_idx) =
                            self.decode_string(&data[current_idx..]).with_context(|| {
                                format!("Could not parse header string in {code} section")
                            })?;

                        current_idx += key_next_idx;

                        let (value_string, value_next_idx) =
                            self.decode_string(&data[current_idx..]).with_context(|| {
                                format!("Could not parse header string in {code} section")
                            })?;

                        headers.insert(key_string, value_string);

                        current_idx += value_next_idx;
                    }

                    OperationCode::SelectDb => {
                        let (value, next_idx) =
                            self.decode_length(&data[current_idx..]).with_context(|| {
                                format!("Could not parse header string in {code} section")
                            })?;

                        current_idx += next_idx;

                        selected_db = value as u32;

                        databases.insert(selected_db, Arc::new(Database::new(selected_db)));
                    }

                    OperationCode::ResizeDb => {
                        let (db_size, next_idx) = self
                            .decode_length(&data[current_idx..])
                            .with_context(|| format!("Could not parse value in {code} section"))?;

                        db_hashmap_size = db_size as u32;

                        current_idx += next_idx;

                        let (expiration_size, next_idx) = self
                            .decode_length(&data[current_idx..])
                            .with_context(|| format!("Could not parse value in {code} section"))?;

                        current_idx += next_idx;

                        expiration_hashmap_size = expiration_size as u32;

                        println!("db_size: {db_hashmap_size}, expiration_size: {expiration_hashmap_size}");

                        loop {
                            let (name, value, _, expiration, next_idx) =
                                self.decode_key(&data[current_idx..]).with_context(|| {
                                    format!("Could not parse key in {code} section")
                                })?;

                            current_idx += next_idx;

                            let mut should_insert = true;

                            if let Some(exp) = expiration {
                                if exp < Utc::now() {
                                    should_insert = false;
                                }
                            }

                            if should_insert {
                                databases
                                    .get_mut(&selected_db)
                                    .with_context(|| {
                                        format!("Could not find database {selected_db}")
                                    })?
                                    .insert(name, String::from_utf8(value)?, expiration)
                                    .await;
                            }

                            if OperationCode::try_from(&data[current_idx]).is_ok() {
                                break;
                            } else {
                                continue;
                            }
                        }
                    }

                    OperationCode::Eof => {
                        break;
                    }
                },

                Err(err) => bail!("Invalid operation code: {err}"),
            }
        }

        Ok(databases)
    }
}

impl Persistent for RDB {
    fn save(&self) -> anyhow::Result<()> {
        todo!()
    }

    fn load(&mut self) -> anyhow::Result<HashMap<u32, Arc<Database>>> {
        let mut data = Vec::new();

        let bytes_read = {
            match self.reader {
                Some(ref mut reader) => reader
                    .read_to_end(&mut data)
                    .with_context(|| "Could not read rdb file")?,

                None => 0,
            }
        };

        if bytes_read == 0 {
            let mut hashmap = HashMap::new();

            hashmap.insert(0, Arc::new(Database::new(0)));

            return Ok(hashmap);
        }

        let magic_string = String::from_utf8(data[0..9].to_vec())?;

        ensure!(&magic_string[0..5] == "REDIS", "Invalid rdb file");

        let future = self.parse_file(&data[9..]);

        let databases = futures::executor::block_on(future)?;

        Ok(databases)
    }
}
