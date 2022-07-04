use std::path::Path;

use anyhow::Result;
use binary_rw::FileStream;

use crate::{
    jpeg::{DqtData, QuantizationTable, ToVec},
    processors::write_section,
    ProcessSection, Section,
};

pub struct DqtProcessorWriter {
    writer: FileStream,
    secret: Vec<u8>,
    has_written: bool,
}

impl DqtProcessorWriter {
    pub fn new<P: AsRef<Path>>(path: P, secret: String) -> Result<Self> {
        let writer = FileStream::new(path.as_ref(), binary_rw::OpenType::OpenAndCreate)?;
        let secret = Self::encode_secret(secret);
        Ok(Self {
            writer,
            secret,
            has_written: false,
        })
    }

    fn encode_secret(secret: String) -> Vec<u8> {
        let mut output = Vec::new();
        output.push(0xBE);
        output.push(0xEF);

        // Can't allow values in DQT
        let len0 = (secret.len() / 254) as u8 + 1;
        let len1 = (secret.len() % 254) as u8 + 1;
        output.push(len0);
        output.push(len1);
        // output.extend((secret.len() as u16).to_be_bytes());

        output.extend(secret.as_bytes());
        output
    }
}

impl ProcessSection for DqtProcessorWriter {
    type Output = bool;

    fn process_section(&mut self, section: &Section) -> Result<Self::Output> {
        use crate::Marker::*;
        let Section { marker, data, .. } = section;
        let marker = *marker;

        if self.has_written || marker != DQT {
            write_section(&mut self.writer, section)?;
            return Ok(false);
        }

        let mut padded_chunks = self
            .secret
            .as_slice()
            .chunks(64)
            .map(|chunk| match chunk.len() {
                64 => chunk.to_vec(),
                _ => {
                    let mut vec = chunk.to_vec();
                    vec.resize(64, 1);
                    vec
                }
            })
            .collect::<Vec<_>>();

        let mut dqt_data = DqtData::try_from(&data[..])?;

        let mut new_tables = Vec::new();
        for chunk in padded_chunks.drain(..) {
            new_tables.push(QuantizationTable {
                precision: 0,
                table_index: 0,
                values: chunk,
            });
        }

        new_tables.extend(dqt_data.tables);
        dqt_data.tables = new_tables;

        write_section(
            &mut self.writer,
            &Section {
                data: dqt_data.to_vec(),
                ..*section
            },
        )?;

        self.has_written = true;

        Ok(true)
    }
}

///////////////////////////////////////////////

pub struct DqtProcessorReader;

impl ProcessSection for DqtProcessorReader {
    type Output = Option<String>;

    fn process_section(&mut self, section: &Section) -> Result<Self::Output> {
        use crate::Marker::*;
        let Section { marker, data, .. } = section;
        let marker = *marker;

        if marker != DQT {
            return Ok(None);
        }

        let dqt_data = DqtData::try_from(&data[..])?;
        for table in &dqt_data.tables {
            let data = &table.values[..];

            // Check for custom data header
            if data[0] != 0xBE || data[1] != 0xEF {
                continue;
            }

            let data = &data[2..];
            let len0 = (data[0] - 1) as usize;
            let len1 = (data[1] - 1) as usize;
            let length = len0 * 254 + len1;
            // let length = u16::from_be_bytes(data[..2].try_into().unwrap()) as usize;

            let data = &data[2..];
            return Ok(Some(String::from_utf8(data[..length].to_vec()).unwrap()));
        }

        Ok(None)
    }
}
