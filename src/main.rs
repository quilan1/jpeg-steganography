mod factorial_number_system;
mod huffman;
mod jpeg;
mod processors;
mod rw_stream;

use anyhow::Result;

use clap::{arg, command, Command};
use jpeg::Jpeg;

fn main() -> Result<()> {
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
    let mut jpeg = Jpeg::read_file(path)?;

    if let Some(matches) = matches.subcommand_matches("write") {
        let output_path = matches.get_one::<String>("output").unwrap();
        let secret = matches.get_one::<String>("secret").unwrap();

        let mut processor = processors::DhtProcessorWriter::new(output_path, secret.clone())?;
        jpeg.process_segments(&mut processor, |wrote_secret| {
            if wrote_secret {
                println!("Wrote secret to file!");
            }
        })?;
    } else if let Some(_) = matches.subcommand_matches("read") {
        let mut processor = processors::DhtProcessorReader;
        jpeg.process_segments(&mut processor, |maybe_secret| {
            if let Some(secret) = maybe_secret {
                println!("Found secret: {secret}");
            }
        })?;
    } else {
        let mut processor = processors::DebugProcessor;
        jpeg.process_segments(&mut processor, |_| {})?;
    }

    Ok(())
}
