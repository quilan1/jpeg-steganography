use std::cell::RefCell;
use std::io::{Read, Write};

use anyhow::Result;
use num_bigint::BigUint;

use crate::jpeg::{segments::HuffmanTableData, Jpeg};
use crate::{
    fns::{MaxBaseValue, TryFromInput, NS2},
    processors::{DhtReader, DhtWriter},
};

pub struct WriteData {
    pub approx_max_size: usize,
    pub secret_size: usize,
}

pub fn write_secret<R: Read, W: Write, T: AsRef<[u8]>>(
    reader: &mut R,
    writer: &mut W,
    secret: T,
) -> Result<WriteData> {
    let secret = secret.as_ref();
    let mut jpeg = Jpeg::read_segments(reader)?;

    let table_sizes = RefCell::new(Vec::new());
    let table_values = RefCell::new(Vec::new());
    jpeg.process_segments(DhtReader::new(|table: &HuffmanTableData| {
        table_sizes.borrow_mut().push(table.sizes.clone());
        table_values.borrow_mut().push(table.values.clone());
    }))?;

    let table_sizes = table_sizes.into_inner();
    let mut table_values = table_values.into_inner();

    let ns = {
        let value = BigUint::from_bytes_be(&encode_secret(secret));
        match NS2::try_from_input(value, &table_sizes) {
            None => anyhow::bail!("Couldn't fit secret into image"),
            Some(ns) => ns,
        }
    };

    ns.permute_values(&mut table_values);

    let table_index = RefCell::new(0usize);
    jpeg.process_segments_mut(DhtWriter::new(writer, |table: &mut HuffmanTableData| {
        let mut table_index = table_index.borrow_mut();
        table.values = table_values[*table_index].clone();
        *table_index += 1;
    }))?;

    let approx_max_size = table_sizes.max_base_value().to_bytes_be().len();
    let secret_size = BigUint::from(ns).to_bytes_be().len();

    Ok(WriteData {
        approx_max_size,
        secret_size,
    })
}

fn encode_secret(secret: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    output.push(0xBE); // A minimal safety header
    output.push(0xEF);
    output.extend(secret);
    output
}

pub fn read_secret<R: Read>(reader: &mut R) -> Result<Option<Vec<u8>>> {
    let jpeg = Jpeg::read_segments(reader)?;

    let table_sizes = RefCell::new(Vec::new());
    let table_values = RefCell::new(Vec::new());
    jpeg.process_segments(DhtReader::new(|table: &HuffmanTableData| {
        table_sizes.borrow_mut().push(table.sizes.clone());
        table_values.borrow_mut().push(table.values.clone());
    }))?;

    let table_sizes = table_sizes.into_inner();
    let table_values = table_values.into_inner();

    let ns = NS2::read_values(&table_sizes, &table_values);
    let data = num_bigint::BigUint::from(ns).to_bytes_be();

    if data.len() <= 2 || data[0] != 0xBE || data[1] != 0xEF {
        return Ok(None);
    }

    Ok(Some(data[2..].to_vec()))
}
