use std::io::Write;

use anyhow::{bail, Result};
use bitstream_io::{
    huffman::{compile_read_tree, compile_write_tree},
    BigEndian,
};

use crate::{
    huffman::construct_huffman_table,
    jpeg::{process_entropy_stream, segments::*, Jpeg, Marker, ProcessSegmentMut, Segment},
    rw_stream::HuffmanRWTree,
};

pub struct DhtWriter<W: Write, F> {
    writer: W,
    callback: F,
}

impl<W: Write, F> DhtWriter<W, F> {
    pub fn new(writer: W, callback: F) -> Result<Self> {
        Ok(Self { writer, callback })
    }
}

impl<W: Write, F: Fn(&mut HuffmanTableData)> ProcessSegmentMut for DhtWriter<W, F> {
    fn process_segment(&mut self, jpeg: &mut Jpeg, segment: &Segment) -> Result<()> {
        let mut segment = segment.clone();
        match segment.marker {
            Marker::DHT => {
                let mut dht_data = DhtData::try_from(&segment.data[..])?;
                for table in &mut dht_data.tables {
                    let read_tree = compile_read_tree::<BigEndian, _>(construct_huffman_table(
                        &table.sizes,
                        &table.values,
                    ))?;

                    (self.callback)(table);

                    let write_tree = Box::new([compile_write_tree::<BigEndian, _>(
                        construct_huffman_table(&table.sizes, &table.values),
                    )?]);

                    let rw_tree = HuffmanRWTree::new(read_tree, write_tree);
                    jpeg.set_huffman_tree(table.table_class, table.table_index, rw_tree);
                }

                segment.data = dht_data.to_vec();
            }

            Marker::SOS => {
                if jpeg.scan.spectral_start != 0 || jpeg.scan.spectral_end != 64 {
                    bail!("Progressive JPEG files not supported")
                }
                jpeg.scan.image_data = process_entropy_stream(&jpeg, &jpeg.scan.image_data)?;
                segment.data = jpeg.scan.to_vec();
            }

            _ => {}
        }

        Jpeg::write_segment(&mut self.writer, &segment)?;
        Ok(())
    }
}
