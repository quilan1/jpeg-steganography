mod factorial_number_system;
mod huffman;
mod jpeg;
mod marker;
mod processors;

use anyhow::Result;

use clap::{arg, command, ArgGroup, Command};
pub use jpeg::{JpegFile, ProcessSection, Section};
pub use marker::Marker;

enum Mode {
    None,
    DQT,
    DHT,
}

fn main() -> Result<()> {
    let matches = command!()
        .arg(arg!(path: <PATH> "Image path"))
        .arg(arg!(dqt: -q --dqt "Append as DQT"))
        .arg(arg!(dht: -h --dht "Rearrange DHT"))
        .group(ArgGroup::new("mode").args(&["dqt", "dht"]).required(false))
        .subcommand(
            Command::new("write")
                .arg(arg!(output: <OUTPUT> "Output path"))
                .arg(arg!(secret: <SECRET> "Secret phrase")),
        )
        .subcommand(Command::new("read"))
        .get_matches();

    let path = matches.get_one::<String>("path").unwrap();
    let jpeg = JpegFile::read_file(path)?;

    let mode = if matches.is_present("dqt") {
        Mode::DQT
    } else if matches.is_present("dht") {
        Mode::DHT
    } else {
        Mode::None
    };

    if let Some(matches) = matches.subcommand_matches("write") {
        let output_path = matches.get_one::<String>("output").unwrap();
        let secret = matches.get_one::<String>("secret").unwrap();

        match mode {
            Mode::DQT => {
                let mut processor =
                    processors::DqtProcessorWriter::new(output_path, secret.clone())?;
                jpeg.process_sections_with_callback(&mut processor, |wrote_secret| {
                    if wrote_secret {
                        println!("Wrote secret to file!");
                    }
                })?;
            }
            Mode::DHT => {
                let mut processor =
                    processors::DhtProcessorWriter::new(output_path, secret.clone())?;
                jpeg.process_sections_with_callback(&mut processor, |wrote_secret| {
                    if wrote_secret {
                        println!("Wrote secret to file!");
                    }
                })?;
            }
            _ => unimplemented!(),
        }
    } else if let Some(_) = matches.subcommand_matches("read") {
        match mode {
            Mode::DQT => {
                let mut processor = processors::DqtProcessorReader;
                jpeg.process_sections_with_callback(&mut processor, |maybe_secret| {
                    if let Some(secret) = maybe_secret {
                        println!("Found secret: {secret}");
                    }
                })?;
            }
            Mode::DHT => {
                let mut processor = processors::DhtProcessorReader;
                jpeg.process_sections_with_callback(&mut processor, |maybe_secret| {
                    if let Some(secret) = maybe_secret {
                        println!("Found secret: {secret}");
                    }
                })?;
            }
            _ => unimplemented!(),
        }
    } else {
        let mut processor = processors::DebugProcessor;
        jpeg.process_sections(&mut processor)?;
    }

    Ok(())
}
