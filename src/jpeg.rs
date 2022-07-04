use std::path::Path;

use anyhow::Result;

use crate::marker::Marker;

#[derive(Clone)]
pub struct Section {
    pub index: usize,
    pub marker: Marker,
    pub data: Vec<u8>,
}

pub struct JpegFile {
    pub sections: Vec<Section>,
}

impl JpegFile {
    pub fn read_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let bytes = std::fs::read(path.as_ref())?;
        let sections = Self::scan_sections(bytes);
        Ok(Self { sections })
    }

    fn scan_sections(bytes: Vec<u8>) -> Vec<Section> {
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
        let mut section: Option<Section> = None;
        let mut prev_index = 2;
        for (index, marker) in markers {
            if let Some(ref section) = section {
                let offset = match section.marker {
                    SOI | EOI | RST(_) => 0,
                    _ => 2,
                };
                sections.push(Section {
                    data: bytes[prev_index + offset..index].to_vec(),
                    ..*section
                });
                prev_index = index + 2;
            }

            section = Some(Section {
                index,
                marker,
                data: Vec::new(),
            })
        }

        if let Some(ref section) = section {
            sections.push(Section {
                data: bytes[prev_index..index].to_vec(),
                ..*section
            });
        }

        sections
    }

    pub fn process_sections<P: ProcessSection>(&self, processor: &mut P) -> Result<()> {
        for section in &self.sections {
            processor.process_section(section)?;
        }

        Ok(())
    }

    pub fn process_sections_with_callback<T, P, F>(
        &self,
        processor: &mut P,
        callback: F,
    ) -> Result<()>
    where
        P: ProcessSection<Output = T>,
        F: Fn(T),
    {
        for section in &self.sections {
            callback(processor.process_section(section)?);
        }

        Ok(())
    }
}

pub trait ProcessSection {
    type Output;
    fn process_section(&mut self, section: &Section) -> Result<Self::Output>;
}

////////////////////////////////////

pub trait ToVec {
    fn to_vec(&self) -> Vec<u8>;
}

#[derive(Default)]
pub struct Component {
    pub component_id: u32,
    pub h_factor: u32,
    pub v_factor: u32,
    pub table_index: usize,
}

impl TryFrom<&[u8]> for Component {
    type Error = anyhow::Error;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        let component_id = data[0];
        let sample_factors = data[1];
        let h_factor = sample_factors >> 4;
        let v_factor = sample_factors & 0xF;
        let table_index = data[2];

        Ok(Self {
            component_id: component_id as u32,
            h_factor: h_factor as u32,
            v_factor: v_factor as u32,
            table_index: table_index as usize,
        })
    }
}

#[derive(Default)]
pub struct SofData {
    pub precision: u32,
    pub width: u32,
    pub height: u32,
    pub components: Vec<Component>,
}

impl TryFrom<&[u8]> for SofData {
    type Error = anyhow::Error;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        let precision = data[0];
        let height = u16::from_be_bytes(data[1..3].try_into().unwrap());
        let width = u16::from_be_bytes(data[3..5].try_into().unwrap());
        let num_components = data[5];

        let data = &data[6..];
        let mut components = Vec::new();
        for component in 0..num_components as usize {
            let data = &data[3 * component as usize..];
            components.push(data.try_into()?);
        }

        Ok(Self {
            precision: precision as u32,
            width: width as u32,
            height: height as u32,
            components,
        })
    }
}

#[derive(Default)]
pub struct QuantizationTable {
    pub precision: u32,
    pub table_index: usize,
    pub values: Vec<u8>,
}

impl ToVec for QuantizationTable {
    fn to_vec(&self) -> Vec<u8> {
        let mut output = Vec::new();
        output.push(((self.precision as u8) << 4) | self.table_index as u8);
        output.extend(&self.values);
        output
    }
}

impl TryFrom<&[u8]> for QuantizationTable {
    type Error = anyhow::Error;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        let pq_byte = data[0];
        let precision = pq_byte >> 4;
        let table_index = pq_byte & 0xF;
        let values = data[1..65].to_vec();

        Ok(QuantizationTable {
            precision: precision as u32,
            table_index: table_index as usize,
            values,
        })
    }
}

#[derive(Default)]
pub struct DqtData {
    pub tables: Vec<QuantizationTable>,
}

impl ToVec for DqtData {
    fn to_vec(&self) -> Vec<u8> {
        let mut output = Vec::new();
        for table in &self.tables {
            output.extend(table.to_vec());
        }
        output
    }
}

impl TryFrom<&[u8]> for DqtData {
    type Error = anyhow::Error;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        let mut tables = Vec::new();

        let mut data = &data[..];
        while !data.is_empty() {
            tables.push(QuantizationTable::try_from(data)?);
            data = &data[65..];
        }

        Ok(Self { tables })
    }
}

#[derive(Default)]
pub struct HuffmanTableData {
    pub table_class: u32,
    pub table_index: usize,
    pub sizes: Vec<usize>,
    pub values: Vec<u8>,
}

impl ToVec for HuffmanTableData {
    fn to_vec(&self) -> Vec<u8> {
        let mut output = Vec::new();
        output.push(((self.table_class as u8) << 4) | self.table_index as u8);
        output.extend(self.sizes.iter().map(|v| *v as u8));
        output.extend(&self.values);
        output
    }
}

impl TryFrom<&[u8]> for HuffmanTableData {
    type Error = anyhow::Error;

    fn try_from(mut data: &[u8]) -> Result<Self, Self::Error> {
        let table_info = data[0];
        let table_class = table_info >> 4;
        let table_index = table_info & 0xF;

        data = &data[1..];
        let sizes = data[0..16].to_vec();
        let num_values = sizes.iter().map(|&v| v as usize).sum::<usize>();

        data = &data[16..];
        let values = data[0..num_values].to_vec();

        Ok(Self {
            table_class: table_class as u32,
            table_index: table_index as usize,
            sizes: sizes.into_iter().map(|v| v as usize).collect(),
            values,
        })
    }
}

#[derive(Default)]
pub struct DhtData {
    pub tables: Vec<HuffmanTableData>,
}

impl ToVec for DhtData {
    fn to_vec(&self) -> Vec<u8> {
        let mut output = Vec::new();
        for table in &self.tables {
            output.extend(table.to_vec());
        }
        output
    }
}

impl TryFrom<&[u8]> for DhtData {
    type Error = anyhow::Error;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        let mut tables = Vec::new();

        let mut data = &data[..];
        while data.len() > 0 {
            let table = HuffmanTableData::try_from(data)?;
            data = &data[17 + table.values.len()..];
            tables.push(table);
        }

        Ok(Self { tables })
    }
}

#[derive(Default)]
pub struct ScanComponentData {
    pub component_id: u32,
    pub dc_table_index: usize,
    pub ac_table_index: usize,
}

impl ToVec for ScanComponentData {
    fn to_vec(&self) -> Vec<u8> {
        let mut output = Vec::new();
        output.push(self.component_id as u8);
        output.push(((self.dc_table_index as u8) << 4) | self.ac_table_index as u8);
        output
    }
}

impl TryFrom<&[u8]> for ScanComponentData {
    type Error = anyhow::Error;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        let component_id = data[0];
        let table = data[1];
        let dc_table_index = table >> 4;
        let ac_table_index = table & 0xF;

        Ok(Self {
            component_id: component_id as u32,
            dc_table_index: dc_table_index as usize,
            ac_table_index: ac_table_index as usize,
        })
    }
}

#[derive(Default)]
pub struct SosData {
    pub spectral_start: u32,
    pub spectral_end: u32,
    pub approx_high: u32,
    pub approx_low: u32,
    pub components: Vec<ScanComponentData>,
    pub image_data: Vec<u8>,
}

impl ToVec for SosData {
    fn to_vec(&self) -> Vec<u8> {
        let mut output = Vec::new();
        output.push(self.components.len() as u8);
        for table in &self.components {
            output.extend(table.to_vec());
        }
        output.push(self.spectral_start as u8);
        output.push(self.spectral_end as u8);
        output.push(((self.approx_high as u8) << 4) | self.approx_low as u8);
        output.extend(&self.image_data);
        output
    }
}

impl TryFrom<&[u8]> for SosData {
    type Error = anyhow::Error;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        let num_components = data[0] as usize;

        let data = &data[1..];
        let mut components = Vec::new();
        for component_index in 0..num_components {
            components.push(ScanComponentData::try_from(&data[2 * component_index..])?);
        }

        let data = &data[2 * num_components as usize..];
        let spectral_start = data[0];
        let spectral_end = data[1];
        let a = data[2];
        let approx_high = a >> 4;
        let approx_low = a & 0xF;

        Ok(Self {
            spectral_start: spectral_start as u32,
            spectral_end: spectral_end as u32 + 1,
            approx_high: approx_high as u32,
            approx_low: approx_low as u32,
            components,
            image_data: data[3..].to_vec(),
        })
    }
}

#[derive(Default)]
pub struct RestartData {
    pub image_data: Vec<u8>,
}

impl TryFrom<&[u8]> for RestartData {
    type Error = anyhow::Error;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        Ok(Self {
            image_data: data.to_vec(),
        })
    }
}

#[derive(Default)]
pub struct DriData {
    pub count: u32,
}

impl TryFrom<&[u8]> for DriData {
    type Error = anyhow::Error;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        let count = u16::from_be_bytes(data[0..2].try_into().unwrap());
        Ok(Self {
            count: count as u32,
        })
    }
}
