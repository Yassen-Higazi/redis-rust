use std::collections::HashMap;
use std::fmt::Display;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, Read};
use std::path::PathBuf;

use anyhow::{bail, ensure, Context};

use super::persistence_interface::Persistent;

#[allow(dead_code)]
#[derive(PartialEq, Eq, Debug)]
pub enum OperationCode {
    Eof,

    SelectDb,

    Expiretime,

    ExpiretimeMs,

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
            OperationCode::Expiretime => write!(f, "EXPIRETIME"),
            OperationCode::ExpiretimeMs => write!(f, "EXPIRETIME_MS"),
        }
    }
}

impl TryFrom<&u8> for OperationCode {
    type Error = &'static str;

    fn try_from(value: &u8) -> Result<Self, Self::Error> {
        match value {
            0xFF => Ok(OperationCode::Eof),

            0xFE => Ok(OperationCode::SelectDb),

            0xFD => Ok(OperationCode::Expiretime),

            0xFB => Ok(OperationCode::ExpiretimeMs),

            0xFA => Ok(OperationCode::Aux),

            _ => Err("Invalid operation code"),
        }
    }
}

#[allow(dead_code, clippy::upper_case_acronyms)]
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
    fn decode_length(&self, data: &[u8]) -> anyhow::Result<(usize, usize)> {
        let first_byte = data[0];

        let mut next_byte_idex = 1usize;

        let bit1 = (first_byte & 0b10000000) >> 7;

        let bit2 = (first_byte & 0b01000000) >> 6;

        let len = match (bit1, bit2) {
            (0, 0) | (1, 1) => (first_byte & 0b00111111) as usize,

            (0, 1) => {
                dbg!("next 14 bits");

                let byte1 = (first_byte & 0b00111111) as u16;
                let byte2 = data[1] as u16;
                next_byte_idex += 1;

                ((byte1 << 8) | byte2) as usize
            }

            (1, 0) => {
                dbg!("next 4 bytes");
                next_byte_idex += 3;

                u32::from_be_bytes([data[1], data[2], data[3], data[4]]) as usize
            }

            _ => todo!(),
        };

        Ok((len, next_byte_idex))
    }

    fn decode_string(&self, data: &[u8]) -> anyhow::Result<(String, usize)> {
        let (len, next_byte_idx) = self.decode_length(data)?;

        let mut last_idx = next_byte_idx;

        ensure!(last_idx <= data.len(), "Invalid length");

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

    fn parse_file(&self, data: &[u8]) -> anyhow::Result<()> {
        let mut current_idx = 0;

        let mut op_code = OperationCode::try_from(&data[current_idx]);

        let mut headers = HashMap::<String, String>::new();

        let mut selected_db = 0u32;

        loop {
            match &op_code {
                Ok(code) => match code {
                    OperationCode::Aux => {
                        current_idx += 1;

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

                        let op_code_res = OperationCode::try_from(&data[current_idx]);

                        if op_code_res.is_ok() {
                            op_code = op_code_res;

                            continue;
                        } else {
                            bail!("Invalid operation code in header");
                        }
                    }

                    OperationCode::SelectDb => {
                        println!("SELECTDB");
                        current_idx += 1;

                        let (value_string, value_next_idx) =
                            self.decode_string(&data[current_idx..]).with_context(|| {
                                format!("Could not parse header string in {code} section")
                            })?;

                        current_idx += value_next_idx;

                        selected_db = value_string.parse::<u32>()?;

                        println!("DB: {selected_db}, current_idx: {current_idx}");

                        break;
                    }

                    OperationCode::Eof => {
                        println!("EOF: {current_idx}");
                        break;
                    }

                    _ => todo!(),
                },

                Err(_) => bail!("Invalid operation code in header"),
            }
        }

        Ok(())
    }
}

impl Persistent for RDB {
    fn save(&self) -> anyhow::Result<()> {
        todo!()
    }

    fn load(&mut self) -> anyhow::Result<()> {
        let mut data = Vec::new();

        let bytes_read = self
            .reader
            .read_to_end(&mut data)
            .with_context(|| "Could not read rdb file")?;

        ensure!(bytes_read > 0, "Empty rdb file");

        let magic_string = String::from_utf8(data[0..9].to_vec())?;

        ensure!(&magic_string[0..5] == "REDIS", "Invalid rdb file");

        self.parse_file(&data[9..])
            .with_context(|| "Could not parse rdb file header")?;

        Ok(())
    }
}
