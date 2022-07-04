use std::path::Path;

use anyhow::Result;
use binary_rw::{BinaryWriter, Endian, WriteStream};

use crate::rw_stream::HuffmanRWTree;

use super::{
    segments::*,
    Marker::{self, *},
};

#[derive(Clone)]
pub struct Segment {
    pub index: usize,
    pub marker: Marker,
    pub data: Vec<u8>,
}

#[derive(Default)]
pub struct Jpeg {
    pub frame: SofData,
    pub dc_trees: [HuffmanRWTree; 2],
    pub ac_trees: [HuffmanRWTree; 2],
    pub restart_interval: u32,
    pub scan: SosData,
    pub segments: Vec<Segment>,
}

impl Jpeg {
    pub fn read_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let bytes = std::fs::read(path.as_ref())?;
        let sections = Self::scan_sections(bytes);
        Ok(Self {
            segments: sections,
            ..Default::default()
        })
    }

    fn scan_sections(bytes: Vec<u8>) -> Vec<Segment> {
        use Marker::*;
        let mut markers = Vec::new();

        let mut index = 0;
        while index < bytes.len() - 1 {
            match bytes[index] {
                0xFF => {}
                _ => {
                    index += 1;
                    continue;
                }
            }

            // Markers will never have 0xFF or 0x00 as their second byte
            let marker_byte = bytes[index + 1];
            if marker_byte == 0xFF || marker_byte == 0x00 {
                index += 2;
                continue;
            }

            let marker: Marker = marker_byte.into();
            match marker {
                RST(_) => {}
                _ => {
                    markers.push((index, marker));
                }
            }
            index += 2;
        }

        let mut sections = Vec::new();
        let mut section: Option<Segment> = None;
        let mut prev_index = 2;
        for (index, marker) in markers {
            if let Some(ref section) = section {
                let offset = match section.marker {
                    SOI | EOI | RST(_) => 0,
                    _ => 2,
                };
                sections.push(Segment {
                    data: bytes[prev_index + offset..index].to_vec(),
                    ..*section
                });
                prev_index = index + 2;
            }

            section = Some(Segment {
                index,
                marker,
                data: Vec::new(),
            })
        }

        if let Some(ref section) = section {
            sections.push(Segment {
                data: bytes[prev_index..index].to_vec(),
                ..*section
            });
        }

        sections
    }

    pub fn process_segments<T, P, F>(&mut self, processor: &mut P, callback: F) -> Result<()>
    where
        P: ProcessSegment<Output = T>,
        F: Fn(T) + Copy,
    {
        let segments = self.segments.clone();
        for segment in segments {
            // callback(processor.process_segment(self, &segment)?);
            self.process_segment(&segment, processor, callback)?;
        }

        Ok(())
    }

    fn process_segment<T, P, F>(
        &mut self,
        segment: &Segment,
        processor: &mut P,
        callback: F,
    ) -> Result<()>
    where
        P: ProcessSegment<Output = T>,
        F: Fn(T),
    {
        match segment.marker {
            SOF0 | SOF1 | SOF2 => self.frame = SofData::try_from(&segment.data[..])?,
            SOS => self.scan = SosData::try_from(&segment.data[..])?,
            DRI => {
                let dri_data = DriData::try_from(&segment.data[..])?;
                self.restart_interval = dri_data.count;
            }
            _ => {}
        }

        callback(processor.process_segment(self, &segment)?);
        Ok(())
    }

    pub fn write_segment<W: WriteStream>(writer: &mut W, section: &Segment) -> Result<()> {
        let Segment { marker, data, .. } = section;

        let mut writer = BinaryWriter::new(writer, Endian::Big);

        writer.write_u8(0xFF)?;
        writer.write_u8(Into::<u8>::into(*marker))?;

        match *marker {
            SOI | EOI => {}
            RST(_) => {
                writer.write_bytes(data)?;
            }
            SOS => {
                let num_components = data[0];
                let length = 6 + 2 * num_components;
                writer.write_u16(length as u16)?;
                writer.write_bytes(data)?;
            }
            _ => {
                writer.write_u16(data.len() as u16 + 2)?;
                writer.write_bytes(data)?;
            }
        }

        Ok(())
    }

    pub fn huffman_table(&self, table_class: u32, table_index: usize) -> &HuffmanRWTree {
        if table_class == 0 {
            &self.dc_trees[table_index]
        } else {
            &self.ac_trees[table_index]
        }
    }

    pub fn huffman_table_mut(
        &mut self,
        table_class: u32,
        table_index: usize,
    ) -> &mut HuffmanRWTree {
        if table_class == 0 {
            &mut self.dc_trees[table_index]
        } else {
            &mut self.ac_trees[table_index]
        }
    }
}

pub trait ProcessSegment {
    type Output;
    fn process_segment(&mut self, jpeg: &mut Jpeg, segment: &Segment) -> Result<Self::Output>;
}
