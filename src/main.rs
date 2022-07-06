mod factorial_number_system;
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
    print_table_secret_sizes(jpeg)?;

    let has_written = std::rc::Rc::new(std::sync::Mutex::new(false));
    let writer = std::io::BufWriter::new(std::fs::File::create(path)?);
    let secret = num_bigint::BigUint::from_bytes_be(&encode_secret(secret));
    let mut processor = processors::DhtWriter::new(
        writer,
        |table_class, table_index, max_secret: num_bigint::BigUint| {
            let mut has_written = has_written.lock().unwrap();

            if *has_written || max_secret <= secret {
                return None;
            }

            let secret_bytes = secret.to_bytes_be();
            let max_secret_len = max_secret.to_bytes_be().len();
            println!(
                "{} DHT #{table_index}: Encoding secret in {} bytes out of {max_secret_len} total",
                ["DC", "AC"][table_class],
                secret_bytes.len(),
            );

            *has_written = true;
            Some(secret_bytes)
        },
    )?;

    jpeg.process_segments_mut(&mut processor)?;

    Ok(())
}

fn print_table_secret_sizes(jpeg: &jpeg::Jpeg) -> anyhow::Result<()> {
    let mut processor = processors::DhtReader::new(
        |table_class, table_index, max_secret: num_bigint::BigUint, _| {
            let max_secret_len = max_secret.to_bytes_be().len();
            println!(
                "{} DHT #{table_index} supports ~{max_secret_len} bytes.",
                ["DC", "AC"][table_class]
            );
        },
    );

    jpeg.process_segments(&mut processor)?;

    Ok(())
}

fn read_secret_from_jpeg(jpeg: &jpeg::Jpeg) -> anyhow::Result<()> {
    let mut processor = processors::DhtReader::new(|_, _, _, maybe_secret: Vec<u8>| {
        if maybe_secret.len() <= 2 || maybe_secret[0] != 0xBE || maybe_secret[1] != 0xEF {
            return;
        }

        let secret = String::from_utf8(maybe_secret[2..].to_vec()).unwrap();
        println!("Found secret: {secret}");
    });

    jpeg.process_segments(&mut processor)?;

    Ok(())
}
