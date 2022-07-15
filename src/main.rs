mod fns;
mod huffman;
mod jpeg;
mod lib_secret;
mod processors;
mod rw_stream;

fn main() -> anyhow::Result<()> {
    use clap::{arg, command, Command};

    let matches = command!()
        .arg(arg!(path: <PATH> "Image path"))
        .subcommand(
            Command::new("write")
                .arg(arg!(output: <OUTPUT> "Output path"))
                .arg(arg!(secret: <SECRET> "Secret phrase")),
        )
        .subcommand(Command::new("read"))
        .get_matches();

    let in_path = matches.get_one::<String>("path").unwrap();

    if let Some(matches) = matches.subcommand_matches("write") {
        let out_path = matches.get_one::<String>("output").unwrap();
        let secret = matches.get_one::<String>("secret").unwrap();
        write_secret_to_file(in_path, out_path, secret)?;
    } else if let Some(_) = matches.subcommand_matches("read") {
        read_secret_from_file(in_path)?;
    } else {
        debug_file(in_path)?;
    }

    Ok(())
}

fn write_secret_to_file<P: AsRef<std::path::Path>, S: AsRef<str>>(
    in_file: P,
    out_file: P,
    secret: S,
) -> anyhow::Result<()> {
    use std::fs::File;
    use std::io::{BufReader, BufWriter, Cursor, Write};

    let start = std::time::Instant::now();
    let mut reader = BufReader::new(File::open(in_file)?);

    let out_data = Vec::<u8>::new();
    let mut writer = Cursor::new(out_data);
    let write_data =
        lib_secret::write_secret(&mut reader, &mut writer, secret.as_ref().as_bytes())?;

    let out_data = writer.into_inner();
    let mut out_file = BufWriter::new(File::create(out_file)?);
    out_file.write(&out_data)?;

    println!(
        "Secret uses ~{} / {} bytes of re-arranged Huffman tables",
        write_data.secret_size, write_data.approx_max_size
    );
    println!("Wrote secret in {} ms", start.elapsed().as_millis());
    Ok(())
}

fn read_secret_from_file<P: AsRef<std::path::Path>>(in_file: P) -> anyhow::Result<()> {
    use std::fs::File;
    use std::io::BufReader;

    let mut reader = BufReader::new(File::open(in_file)?);
    match lib_secret::read_secret(&mut reader)? {
        None => {
            println!("No message found within file");
        }
        Some(secret) => {
            println!("Secret: '{}'", String::from_utf8(secret)?);
        }
    }

    Ok(())
}

fn debug_file<P: AsRef<std::path::Path>>(in_file: P) -> anyhow::Result<()> {
    use std::fs::File;
    use std::io::BufReader;

    let mut reader = BufReader::new(File::open(in_file)?);
    let jpeg = jpeg::Jpeg::read_segments(&mut reader)?;

    let mut processor = processors::DebugReader::new(|msg| println!("{}", msg));
    jpeg.process_segments(&mut processor)?;

    Ok(())
}
