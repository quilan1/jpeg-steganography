use std::io::{Read, Write};

use anyhow::Result;

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
    pub huffman_trees: [HuffmanRWTree; 4],
    pub restart_interval: u32,
    pub scan: SosData,
    pub segments: Vec<Segment>,
}

impl Jpeg {
    pub fn read_segments<R: Read>(reader: &mut R) -> Result<Self> {
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf)?;

        let sections = Self::scan_segments(buf);
        Ok(Self {
            segments: sections,
            ..Default::default()
        })
    }

    fn scan_segments(bytes: Vec<u8>) -> Vec<Segment> {
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

    pub fn process_segments_mut<P>(&mut self, mut processor: P) -> Result<()>
    where
        P: ProcessSegmentMut,
    {
        let segments = self.segments.clone();
        for segment in segments {
            match segment.marker {
                SOF0 | SOF1 | SOF2 => self.frame = SofData::try_from(&segment.data[..])?,
                SOS => self.scan = SosData::try_from(&segment.data[..])?,
                DRI => {
                    let dri_data = DriData::try_from(&segment.data[..])?;
                    self.restart_interval = dri_data.count;
                }
                _ => {}
            }

            processor.process_segment(self, &segment)?;
        }

        Ok(())
    }

    pub fn process_segments<P>(&self, processor: P) -> Result<()>
    where
        P: ProcessSegment,
    {
        for segment in &self.segments {
            processor.process_segment(self, segment)?;
        }

        Ok(())
    }

    pub fn write_segment<W: Write>(writer: &mut W, section: &Segment) -> Result<()> {
        let Segment { marker, data, .. } = section;

        writer.write_all(&[0xFF])?;
        writer.write_all(&[u8::from(*marker)])?;

        match *marker {
            SOI | EOI => {}
            RST(_) => {
                writer.write_all(data)?;
            }
            SOS => {
                let num_components = data[0];
                let length = 6 + 2 * num_components;
                writer.write_all(&(length as u16).to_be_bytes())?;
                writer.write_all(data)?;
            }
            _ => {
                writer.write_all(&(data.len() as u16 + 2).to_be_bytes())?;
                writer.write_all(data)?;
            }
        }

        Ok(())
    }

    pub fn get_huffman_trees(
        &self,
        dc_table_index: usize,
        ac_table_index: usize,
    ) -> (&HuffmanRWTree, &HuffmanRWTree) {
        (
            &self.huffman_trees[dc_table_index],
            &self.huffman_trees[2 + ac_table_index],
        )
    }

    pub fn set_huffman_tree(
        &mut self,
        table_class: usize,
        table_index: usize,
        tree: HuffmanRWTree,
    ) {
        let index = 2 * table_class + table_index;
        self.huffman_trees[index] = tree;
    }
}

pub trait ProcessSegmentMut {
    fn process_segment(&mut self, jpeg: &mut Jpeg, segment: &Segment) -> Result<()>;
}

pub trait ProcessSegment {
    fn process_segment(&self, jpeg: &Jpeg, segment: &Segment) -> Result<()>;
}
