use std::path::Path;

use anyhow::Result;
use binary_rw::FileStream;
use bitstream_io::{
    huffman::{compile_read_tree, compile_write_tree},
    BigEndian,
};

use crate::{
    factorial_number_system::FNS,
    huffman::construct_huffman_table,
    jpeg::{process_entropy_stream, segments::*, Jpeg, Marker::*, ProcessSegment, Segment},
    rw_stream::HuffmanRWTree,
};

pub struct DhtProcessorWriter {
    writer: FileStream,
    secret: FNS,
    is_written: bool,
}

impl DhtProcessorWriter {
    pub fn new<P: AsRef<Path>>(path: P, secret: String) -> Result<Self> {
        let writer = FileStream::new(path.as_ref(), binary_rw::OpenType::OpenAndCreate)?;
        let secret = Self::encode_secret(secret);
        let secret = FNS::from_bytes(secret);
        println!("Encoded secret: {} bytes", secret.digits.len());

        Ok(Self {
            writer,
            secret,
            is_written: false,
        })
    }

    fn encode_secret(secret: String) -> Vec<u8> {
        let mut output = Vec::new();
        output.push(0xBE);
        output.push(0xEF);
        output.extend((secret.len() as u16).to_be_bytes());
        output.extend(secret.as_bytes());
        output
    }

    fn process_dht_section(&mut self, jpeg: &mut Jpeg, section: &Segment) -> Result<bool> {
        let mut saved_table = false;

        let mut dht_data = DhtData::try_from(&section.data[..])?;
        for table in &mut dht_data.tables {
            let read_tree = compile_read_tree::<BigEndian, _>(construct_huffman_table(
                &table.sizes,
                &table.values,
            ))?;

            match self.is_written {
                true => {}
                false => {
                    if self
                        .secret
                        .permute_huffman_table(&table.sizes, &mut table.values)
                    {
                        self.is_written = true;
                        saved_table = true;
                    }
                }
            }

            let write_tree = Box::new([compile_write_tree::<BigEndian, _>(
                construct_huffman_table(&table.sizes, &table.values),
            )?]);

            *jpeg.huffman_table_mut(table.table_class, table.table_index) =
                HuffmanRWTree::new(read_tree, write_tree);
        }

        let section = Segment {
            data: dht_data.to_vec(),
            ..*section
        };

        Jpeg::write_segment(&mut self.writer, &section)?;
        Ok(saved_table)
    }
}

impl ProcessSegment for DhtProcessorWriter {
    type Output = bool;

    fn process_segment(&mut self, jpeg: &mut Jpeg, section: &Segment) -> Result<Self::Output> {
        let Segment { marker, .. } = section;
        let marker = *marker;

        match marker {
            DHT => return Ok(self.process_dht_section(jpeg, section)?),
            SOS => {
                jpeg.scan.image_data = process_entropy_stream(&jpeg, &jpeg.scan.image_data)?;

                let section = Segment {
                    data: jpeg.scan.to_vec(),
                    ..*section
                };
                Jpeg::write_segment(&mut self.writer, &section)?;
                Ok(false)
            }
            _ => {
                Jpeg::write_segment(&mut self.writer, section)?;
                Ok(false)
            }
        }
    }
}

///////////////////////////////////////////////

pub struct DhtProcessorReader;

impl ProcessSegment for DhtProcessorReader {
    type Output = Option<String>;

    fn process_segment(&mut self, _: &mut Jpeg, section: &Segment) -> Result<Self::Output> {
        let Segment { marker, data, .. } = section;
        let marker = *marker;

        if marker != DHT {
            return Ok(None);
        }

        let dht_data = DhtData::try_from(&data[..])?;
        for table in dht_data.tables {
            for bytes in FNS::read_huffman_table(&table.sizes, &table.values) {
                if bytes.len() >= 4 && bytes[0] == 0xBE && bytes[1] == 0xEF {
                    return Ok(Some(String::from_utf8(bytes[4..].to_vec()).unwrap()));
                }
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bnf() {
        let values = vec![0, 1, 2, 3, 4, 5];
        let sizes = vec![values.len()];

        for input_value in 3..4 {
            let input = vec![input_value];

            let secret = FNS::from_bytes(input.clone());
            let mut new_values = &mut values.clone()[..];
            secret.permute_huffman_table(&sizes, &mut new_values);

            if let Some(result) = FNS::read_huffman_table(&sizes, &new_values).first() {
                println!("Result: {result:?}");
                assert_eq!(input, *result);
                println!();
            }
        }
    }
}
