use std::{io::Cursor, path::Path};

use anyhow::Result;
use binary_rw::FileStream;
use bitstream_io::{
    huffman::{compile_read_tree, compile_write_tree, ReadHuffmanTree, WriteHuffmanTree},
    BigEndian, BitRead, BitReader, BitWrite, BitWriter, HuffmanRead, HuffmanWrite, Numeric,
};

use crate::{
    factorial_number_system::FNS,
    huffman::construct_huffman_table,
    jpeg::{Component, DhtData, DriData, SofData, SosData, ToVec},
    processors::write_section,
    ProcessSection, Section,
};

type HuffmanTreeRead = Box<[ReadHuffmanTree<BigEndian, u8>]>;
type HuffmanTreeWrite = Box<[WriteHuffmanTree<BigEndian, u8>]>;

#[derive(Default)]
struct Jpeg {
    frame: SofData,
    dc_tables_read: [HuffmanTreeRead; 2],
    ac_tables_read: [HuffmanTreeRead; 2],
    dc_tables_write: [HuffmanTreeWrite; 2],
    ac_tables_write: [HuffmanTreeWrite; 2],
    restart_interval: u32,
    scan: SosData,
}

impl Jpeg {
    fn huffman_table_read(&self, table_class: u32, table_index: usize) -> &HuffmanTreeRead {
        if table_class == 0 {
            &self.dc_tables_read[table_index]
        } else {
            &self.ac_tables_read[table_index]
        }
    }

    fn huffman_table_write(&self, table_class: u32, table_index: usize) -> &HuffmanTreeWrite {
        if table_class == 0 {
            &self.dc_tables_write[table_index]
        } else {
            &self.ac_tables_write[table_index]
        }
    }

    fn huffman_table_read_mut(
        &mut self,
        table_class: u32,
        table_index: usize,
    ) -> &mut HuffmanTreeRead {
        if table_class == 0 {
            &mut self.dc_tables_read[table_index]
        } else {
            &mut self.ac_tables_read[table_index]
        }
    }

    fn huffman_table_write_mut(
        &mut self,
        table_class: u32,
        table_index: usize,
    ) -> &mut HuffmanTreeWrite {
        if table_class == 0 {
            &mut self.dc_tables_write[table_index]
        } else {
            &mut self.ac_tables_write[table_index]
        }
    }
}

pub struct DhtProcessorWriter {
    writer: FileStream,
    secret: FNS,
    is_written: bool,
    jpeg: Jpeg,
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
            jpeg: Jpeg::default(),
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

    fn process_dht_section(&mut self, section: &Section) -> Result<bool> {
        let mut saved_table = false;

        let mut dht_data = DhtData::try_from(&section.data[..])?;
        for table in &mut dht_data.tables {
            // {
            //     println!(
            //         "Huffman table #{}: Type {}",
            //         table.table_index,
            //         ["DC", "AC"][table.table_class as usize]
            //     );
            //     for (value, bits) in construct_huffman_table(&table.sizes, &table.values) {
            //         println!(
            //             "\t{value}\t{}",
            //             bits.into_iter()
            //                 .map(|v| v.to_string())
            //                 .collect::<Vec<_>>()
            //                 .join("")
            //         );
            //     }
            //     println!();
            // }

            *self
                .jpeg
                .huffman_table_read_mut(table.table_class, table.table_index) =
                compile_read_tree::<BigEndian, _>(construct_huffman_table(
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

            // {
            //     println!("Huffman table #{}: Type {}", table.table_index, ["DC", "AC"][table.table_class as usize]);
            //     for (value, bits) in construct_huffman_table(&table.sizes, &table.values) {
            //         println!("\t{value}\t{}", bits.into_iter().map(|v| v.to_string()).collect::<Vec<_>>().join(""));
            //     }
            //     println!();
            // }

            *self
                .jpeg
                .huffman_table_write_mut(table.table_class, table.table_index) =
                Box::new([compile_write_tree::<BigEndian, _>(
                    construct_huffman_table(&table.sizes, &table.values),
                )?]);
        }

        let section = Section {
            data: dht_data.to_vec(),
            ..*section
        };

        write_section(&mut self.writer, &section)?;
        Ok(saved_table)
    }
}

impl ProcessSection for DhtProcessorWriter {
    type Output = bool;

    fn process_section(&mut self, section: &Section) -> Result<Self::Output> {
        use crate::Marker::*;
        let Section { marker, data, .. } = section;
        let marker = *marker;

        match marker {
            SOF0 | SOF1 | SOF2 => {
                self.jpeg.frame = SofData::try_from(&data[..])?;
                write_section(&mut self.writer, section)?;
                Ok(false)
            }

            DHT => Ok(self.process_dht_section(section)?),

            SOS => {
                self.jpeg.scan = SosData::try_from(&data[..])?;

                let mut fixed_data = Vec::new();
                let mut data_iter = self.jpeg.scan.image_data.iter().cloned();
                while let Some(value) = data_iter.next() {
                    fixed_data.push(value);
                    if value == 0xFF {
                        let value = data_iter.next().unwrap();
                        if value != 0x00 {
                            fixed_data.push(value);
                        }
                    }
                }

                self.jpeg.scan.image_data = process_entropy_stream(&self.jpeg, &fixed_data)?;
                let section = Section {
                    data: self.jpeg.scan.to_vec(),
                    ..*section
                };
                write_section(&mut self.writer, &section)?;
                Ok(false)
            }

            DRI => {
                let DriData { count } = DriData::try_from(&data[..])?;
                self.jpeg.restart_interval = count;
                write_section(&mut self.writer, section)?;
                Ok(false)
            }

            _ => {
                write_section(&mut self.writer, section)?;
                Ok(false)
            }
        }
    }
}

fn process_entropy_stream(jpeg: &Jpeg, data: &Vec<u8>) -> Result<Vec<u8>> {
    struct ComponentInfo<'a> {
        component: &'a Component,
        dc_table: ReadWriteTable<'a>,
        ac_table: ReadWriteTable<'a>,
    }

    let components_info = {
        let mut components = Vec::new();
        for scan_component in &jpeg.scan.components {
            let component_index = jpeg
                .frame
                .components
                .iter()
                .position(|c| c.component_id == scan_component.component_id)
                .unwrap();

            let component = &jpeg.frame.components[component_index];
            let dc_table = ReadWriteTable {
                read: jpeg.huffman_table_read(0, scan_component.dc_table_index),
                write: jpeg.huffman_table_write(0, scan_component.dc_table_index),
            };
            let ac_table = ReadWriteTable {
                read: jpeg.huffman_table_read(1, scan_component.ac_table_index),
                write: jpeg.huffman_table_write(1, scan_component.ac_table_index),
            };

            components.push(ComponentInfo {
                component,
                dc_table,
                ac_table,
            });
        }
        components
    };

    let mut mcus_left_until_restart = jpeg.restart_interval;
    let mut eob_run = 0;

    let (mcu_horizontal_samples, mcu_vertical_samples) = {
        let horizontal = components_info
            .iter()
            .map(|component_info| component_info.component.h_factor as u16)
            .collect::<Vec<_>>();
        let vertical = components_info
            .iter()
            .map(|component_info| component_info.component.v_factor as u16)
            .collect::<Vec<_>>();
        (horizontal, vertical)
    };

    let (max_mcu_x, max_mcu_y) = {
        let h_max = components_info
            .iter()
            .map(|c| c.component.h_factor)
            .max()
            .unwrap();
        let v_max = components_info
            .iter()
            .map(|c| c.component.v_factor)
            .max()
            .unwrap();

        (
            (jpeg.frame.width + h_max * 8 - 1) / (h_max * 8),
            (jpeg.frame.height + v_max * 8 - 1) / (v_max * 8),
        )
    };

    let mut out_data = Vec::with_capacity(data.len());
    let mut marker_positions = Vec::new();
    {
        let reader = Cursor::new(data);
        let writer = Cursor::new(&mut out_data);
        let mut reader = BitReader::endian(reader, BigEndian);
        let mut writer = BitWriter::endian(writer, BigEndian);
        let mut read_writer = ReadWriter::new(&mut reader, &mut writer);

        for mcu_y in 0..max_mcu_y {
            if mcu_y * 8 >= jpeg.frame.height {
                break;
            }

            for mcu_x in 0..max_mcu_x {
                if mcu_x * 8 >= jpeg.frame.width {
                    break;
                }

                if jpeg.restart_interval > 0 {
                    if mcus_left_until_restart == 0 {
                        read_writer.byte_align()?;
                        marker_positions
                            .push(read_writer.writer.writer().unwrap().position() as usize);
                        let marker_header = read_writer.read::<u8>(8)?;
                        assert_eq!(marker_header, 0xFF);

                        read_writer.read::<u8>(8)?;

                        eob_run = 0;
                        mcus_left_until_restart = jpeg.restart_interval;
                    }

                    mcus_left_until_restart -= 1;
                }

                for (i, component_info) in components_info.iter().enumerate() {
                    let dc_table = &component_info.dc_table;
                    let ac_table = &component_info.ac_table;
                    read_writer.set_tables(dc_table, ac_table);

                    for _v_pos in 0..mcu_vertical_samples[i] {
                        for _h_pos in 0..mcu_horizontal_samples[i] {
                            decode_block(&mut read_writer, jpeg, &mut eob_run)?;
                        }
                    }
                }
            }
        }
    }

    let mut data = out_data;
    let mut out_data = Vec::new();
    for (index, value) in data.drain(..).enumerate() {
        out_data.push(value);
        if value == 0xFF {
            if !marker_positions.contains(&index) {
                out_data.push(0x00);
            }
        }
    }

    Ok(out_data)
}

fn decode_block<'a>(
    read_writer: &mut ReadWriter<'a>,
    jpeg: &Jpeg,
    eob_run: &mut u16,
) -> Result<()> {
    if jpeg.scan.spectral_start == 0 {
        // Section F.2.2.1
        // Figure F.12

        let value = read_writer.read_huffman(ReadType::DC)?;
        match value {
            0 => {}
            1..=11 => {
                read_writer.read::<u16>(value.into())?;
            }
            _ => panic!(),
        }
    }

    let mut index = jpeg.scan.spectral_start.max(1);
    if index < jpeg.scan.spectral_end && *eob_run > 0 {
        *eob_run -= 1;
        return Ok(());
    }

    // Section F.1.2.2.1
    while index < jpeg.scan.spectral_end {
        let byte = read_writer.read_huffman(ReadType::AC)?;
        let r = byte >> 4;
        let s = byte & 0x0f;

        if s == 0 {
            match r {
                15 => index += 16, // Run length of 16 zero coefficients.
                _ => {
                    *eob_run = (1 << r) - 1;

                    if r > 0 {
                        *eob_run += read_writer.read::<u16>(r.into())?;
                    }

                    break;
                }
            }
        } else {
            index += r as u32;

            if index >= jpeg.scan.spectral_end {
                break;
            }

            read_writer.read::<u16>(s.into())?;
            index += 1;
        }
    }

    Ok(())
}

///////////////////////////////////////////////

pub struct DhtProcessorReader;

impl ProcessSection for DhtProcessorReader {
    type Output = Option<String>;

    fn process_section(&mut self, section: &Section) -> Result<Self::Output> {
        use crate::Marker::*;
        let Section { marker, data, .. } = section;
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

#[derive(Debug)]
enum ReadType {
    DC,
    AC,
}

struct ReadWriteTable<'a> {
    read: &'a HuffmanTreeRead,
    write: &'a HuffmanTreeWrite,
}

struct ReadWriter<'a> {
    reader: &'a mut BitReader<Cursor<&'a Vec<u8>>, BigEndian>,
    writer: &'a mut BitWriter<Cursor<&'a mut Vec<u8>>, BigEndian>,

    dc_table: Option<&'a ReadWriteTable<'a>>,
    ac_table: Option<&'a ReadWriteTable<'a>>,
}

impl<'a> ReadWriter<'a> {
    fn new(
        reader: &'a mut BitReader<Cursor<&'a Vec<u8>>, BigEndian>,
        writer: &'a mut BitWriter<Cursor<&'a mut Vec<u8>>, BigEndian>,
    ) -> Self {
        Self {
            reader,
            writer,
            dc_table: None,
            ac_table: None,
        }
    }

    fn set_tables(&mut self, dc_table: &'a ReadWriteTable, ac_table: &'a ReadWriteTable) {
        self.dc_table = Some(dc_table);
        self.ac_table = Some(ac_table);
    }

    fn byte_align(&mut self) -> Result<()> {
        self.reader.byte_align();
        self.writer.byte_align()?;
        Ok(())
    }

    fn read<T: Numeric + std::fmt::Display + std::fmt::Binary>(&mut self, bits: u32) -> Result<T> {
        let value = self.reader.read(bits)?;
        self.writer.write::<T>(bits, value)?;
        Ok(value)
    }

    fn read_huffman(&mut self, read_type: ReadType) -> Result<u8> {
        let value = match read_type {
            ReadType::DC => self
                .reader
                .read_huffman(self.dc_table.unwrap().read.as_ref())?,
            ReadType::AC => self
                .reader
                .read_huffman(self.ac_table.unwrap().read.as_ref())?,
        };

        match read_type {
            ReadType::DC => self
                .writer
                .write_huffman(&self.dc_table.unwrap().write[0], value)?,
            ReadType::AC => self
                .writer
                .write_huffman(&self.ac_table.unwrap().write[0], value)?,
        }

        Ok(value)
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
