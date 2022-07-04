use anyhow::Result;
use binary_rw::{BinaryWriter, Endian, WriteStream};

use crate::Section;

pub fn write_section<W: WriteStream>(writer: &mut W, section: &Section) -> Result<()> {
    let Section { marker, data, .. } = section;

    let mut writer = BinaryWriter::new(writer, Endian::Big);

    writer.write_u8(0xFF)?;
    writer.write_u8(Into::<u8>::into(*marker))?;

    use crate::Marker::*;
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
