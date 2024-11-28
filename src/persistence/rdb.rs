use std::collections::HashMap;
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

impl TryFrom<&[u8]> for OperationCode {
    type Error = &'static str;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        match value[0] {
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

    fn parse_length(&self, data: &[u8]) -> anyhow::Result<(usize, bool)> {
        let first_byte = data[0];

        let mut is_string = false;

        let digit1 = (first_byte & 0b10000000) >> 7;

        let digit2 = (first_byte & 0b01000000) >> 6;

        let len = match (digit1, digit2) {
            (0, 0) => {
                dbg!("next 6 bits");

                (first_byte & 0b00111111) as usize
            }

            (0, 1) => {
                dbg!("next 14 bits");

                let byte1 = (first_byte & 0b00111111) as u16;
                let byte2 = data[1] as u16;

                ((byte1 << 8) | byte2) as usize
            }

            (1, 0) => {
                dbg!("next 4 bytes");

                u32::from_be_bytes([data[1], data[2], data[3], data[4]]) as usize
            }

            (1, 1) => {
                dbg!("next 6 bits with special format");

                is_string = true;

                let len = first_byte & 0b00111111;

                len as usize
            }

            _ => bail!("Invalid length Encoding"),
        };

        Ok((len, is_string))
    }

    fn parse_string(&self, data: &[u8]) -> anyhow::Result<(String, usize)> {
        let (len, _) = self.parse_length(data)?;

        dbg!(len, [data[len], data[len + 1]]);

        let string = match len {
            0 => String::from_utf8(vec![data[len + 1]])?,

            1 => todo!(),

            2 => todo!(),

            3 => todo!(),

            _ => String::from_utf8(data[1..len + 1].to_vec())?,
        };

        Ok((string, len))
    }

    fn parse_file_header(&self, data: &[u8]) -> anyhow::Result<()> {
        let mut current_idx = 0;

        let op_code = format!("{:X}", data[current_idx]);

        dbg!(&op_code);

        ensure!(op_code == "FA", "Invalid RDB file header");

        current_idx += 1;

        let mut headers = HashMap::<String, String>::new();

        loop {
            let (ver_key_string, ver_key_len) = self
                .parse_string(&data[current_idx..])
                .with_context(|| format!("Could not parse header string in {op_code} section"))?;

            dbg!(&ver_key_string);

            current_idx += ver_key_len;

            let (ver_value_string, ver_value_len) = self
                .parse_string(&data[current_idx..])
                .with_context(|| format!("Could not parse header string in {op_code} section"))?;

            dbg!(&ver_value_string);

            current_idx += ver_value_len;

            headers.insert(ver_key_string, ver_value_string);

            let op_code = OperationCode::try_from(&data[current_idx..]);

            current_idx += 1;

            dbg!(&op_code);

            if !op_code.is_ok_and(|code| code == OperationCode::Aux) {
                break;
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

        dbg!(bytes_read);

        let magic_string = String::from_utf8(data[0..9].to_vec())?;

        ensure!(&magic_string[0..5] == "REDIS", "Invalid rdb file");

        self.parse_file_header(&data[9..])
            .with_context(|| "Could not parse rdb file header")?;

        Ok(())
    }
}
