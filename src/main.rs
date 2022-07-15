mod fns;
mod huffman;
mod jpeg;
mod processors;
mod rw_stream;

use clap::{arg, command, Command};

fn main() -> anyhow::Result<()> {
    let matches = command!()
        .arg(arg!(path: <PATH> "Image path"))
        .subcommand(
            Command::new("write")
                .arg(arg!(output: <OUTPUT> "Output path"))
                .arg(arg!(secret: <SECRET> "Secret phrase")),
        )
        .subcommand(Command::new("read"))
        .get_matches();

    let path = matches.get_one::<String>("path").unwrap();
    let mut jpeg = jpeg::Jpeg::read_file_segments(path)?;

    if let Some(matches) = matches.subcommand_matches("write") {
        let output_path = matches.get_one::<String>("output").unwrap();
        let secret = matches.get_one::<String>("secret").unwrap();
        write_secret_to_file(&mut jpeg, output_path, secret)?;
    } else if let Some(_) = matches.subcommand_matches("read") {
        read_secret_from_jpeg(&jpeg)?;
    } else {
        let mut processor = processors::DebugReader::new(|msg| println!("{}", msg));
        jpeg.process_segments(&mut processor)?;
    }

    Ok(())
}

fn encode_secret(secret: &str) -> Vec<u8> {
    let mut output = Vec::new();
    output.push(0xBE); // A minimal safety header
    output.push(0xEF);
    output.extend(secret.as_bytes());
    output
}

fn write_secret_to_file<P: AsRef<std::path::Path>>(
    jpeg: &mut jpeg::Jpeg,
    path: P,
    secret: &str,
) -> anyhow::Result<()> {
    use fns::{MaxBaseValue, TryFromInput};
    use jpeg::segments::HuffmanTableData;
    use std::cell::RefCell;

    let table_sizes = RefCell::<Vec<Vec<usize>>>::new(Vec::new());
    let table_values = RefCell::<Vec<Vec<u8>>>::new(Vec::new());
    let read_processor = processors::DhtReader::new(|table: &HuffmanTableData| {
        table_sizes.borrow_mut().push(table.sizes.clone());
        table_values.borrow_mut().push(table.values.clone());
    });
    jpeg.process_segments(&read_processor)?;

    let table_sizes = table_sizes.into_inner();
    let mut table_values = table_values.into_inner();
    let max_len = table_sizes.max_base_value().to_bytes_be().len();
    println!("Maximum message length: ~{max_len} bytes");

    let ns = {
        let value = num_bigint::BigUint::from_bytes_be(&encode_secret(secret));
        match fns::NS2::try_from_input(value, &table_sizes) {
            None => anyhow::bail!("Couldn't fit message into ~{max_len} bytes"),
            Some(ns) => ns,
        }
    };
    ns.permute_values(&mut table_values);

    let table_index = RefCell::new(0usize);
    let writer = std::io::BufWriter::new(std::fs::File::create(path)?);
    let mut processor = processors::DhtWriter::new(writer, |table: &mut HuffmanTableData| {
        let mut table_index = table_index.borrow_mut();
        table.values = table_values[*table_index].clone();
        *table_index += 1;
    })?;

    jpeg.process_segments_mut(&mut processor)?;

    println!("Message successfully written!");
    Ok(())
}

fn read_secret_from_jpeg(jpeg: &jpeg::Jpeg) -> anyhow::Result<()> {
    use jpeg::segments::HuffmanTableData;
    use std::cell::RefCell;

    let table_sizes = RefCell::<Vec<Vec<usize>>>::new(Vec::new());
    let table_values = RefCell::<Vec<Vec<u8>>>::new(Vec::new());
    let read_processor = processors::DhtReader::new(|table: &HuffmanTableData| {
        table_sizes.borrow_mut().push(table.sizes.clone());
        table_values.borrow_mut().push(table.values.clone());
    });
    jpeg.process_segments(&read_processor)?;

    let table_sizes = table_sizes.into_inner();
    let table_values = table_values.into_inner();

    let ns = fns::NS2::read_values(&table_sizes, &table_values);
    let data = num_bigint::BigUint::from(ns).to_bytes_be();

    if data.len() <= 2 || data[0] != 0xBE || data[1] != 0xEF {
        println!("No message found within file");
        return Ok(());
    }

    println!(
        "Encoded message: {}",
        String::from_utf8(data[2..].to_vec())?
    );

    Ok(())
}
