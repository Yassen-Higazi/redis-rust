use std::fs::{File, OpenOptions};
use std::io::{BufReader, Read};
use std::path::PathBuf;

use anyhow::{bail, ensure, Context, Ok};

use super::persistence_interface::Persistent;

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

    fn parse_length(&self, data: &[u8]) -> anyhow::Result<usize> {
        let first_byte = data[0];

        let digit1 = (first_byte & 0b10000000) >> 7;

        let digit2 = (first_byte & 0b01000000) >> 6;

        let len = match (digit1, digit2) {
            (0, 0) => {
                dbg!("next 6 bits");

                (first_byte & 0b00111111) as usize
            }

            (0, 1) => {
                dbg!("next 14 bits");

                let byte1 = first_byte & 0b00111111;
                let byte2 = data[1];

                (((byte1 as u16) << 8) | byte2 as u16) as usize
            }

            (1, 0) => {
                dbg!("next 4 bytes");

                u32::from_be_bytes([data[1], data[2], data[3], data[4]]) as usize
            }

            (1, 1) => {
                dbg!("next 6 bits with special format");

                let len = first_byte & 0b00111111;

                dbg!("len: {len}, {len:b}");

                len as usize
            }

            _ => bail!("Invalid length Encoding"),
        };

        Ok(len)
    }

    fn parse_string(&self, data: &[u8]) -> anyhow::Result<(String, usize)> {
        let len = self.parse_length(&data[0..5])? + 1;

        let string = match len {
            0 => todo!(),

            1 => todo!(),

            2 => todo!(),

            3 => todo!(),

            _ => String::from_utf8(data[1..len].to_vec())?,
        };

        Ok((string, len))
    }

    fn parse_file_header(&self, data: &[u8]) -> anyhow::Result<()> {
        let op_code = format!("{:X}", data[0]);

        dbg!(&op_code);

        ensure!(op_code == "FA", "Invalid RDB file header");

        let (ver_key_string, ver_key_len) = self
            .parse_string(&data[1..])
            .with_context(|| format!("Could not parse header string in {op_code} section"))?;

        dbg!(ver_key_string, &ver_key_len, data.len());

        let ver_value_string = self
            .parse_string(&data[1 + ver_key_len..])
            .with_context(|| format!("Could not parse header string in {op_code} section"))?;

        dbg!(ver_value_string);

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
