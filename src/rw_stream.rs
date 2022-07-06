use std::io::Cursor;

use anyhow::Result;
use bitstream_io::{
    huffman::{ReadHuffmanTree, WriteHuffmanTree},
    BigEndian, BitRead, BitReader, BitWrite, BitWriter, HuffmanRead, HuffmanWrite, Numeric,
};

type ReadCursor<'a> = Cursor<&'a Vec<u8>>;
type WriteCursor<'a> = Cursor<&'a mut Vec<u8>>;

type HuffmanTreeReadInner = ReadHuffmanTree<BigEndian, u8>;
type HuffmanTreeWriteInner = WriteHuffmanTree<BigEndian, u8>;
type HuffmanTreeRead = Box<[HuffmanTreeReadInner]>;
type HuffmanTreeWrite = Box<[HuffmanTreeWriteInner]>;

pub struct RWStream<'a> {
    reader: BitReader<ReadCursor<'a>, BigEndian>,
    writer: BitWriter<WriteCursor<'a>, BigEndian>,
    dc_tree: Option<&'a HuffmanRWTree>,
    ac_tree: Option<&'a HuffmanRWTree>,
}

#[derive(Default)]
pub struct HuffmanRWTree {
    reader: HuffmanTreeRead,
    writer: HuffmanTreeWrite,
}

impl<'a> RWStream<'a> {
    pub fn new(read: &'a Vec<u8>, write: &'a mut Vec<u8>) -> Self {
        let read_cursor = Cursor::new(read);
        let write_cursor = Cursor::new(write);
        let reader = BitReader::endian(read_cursor, BigEndian);
        let writer = BitWriter::endian(write_cursor, BigEndian);
        Self {
            reader,
            writer,
            dc_tree: None,
            ac_tree: None,
        }
    }

    pub fn writer_position(&mut self) -> usize {
        self.writer.writer().unwrap().position() as usize
    }

    pub fn set_tables(&mut self, dc_tree: &'a HuffmanRWTree, ac_tree: &'a HuffmanRWTree) {
        self.dc_tree = Some(dc_tree);
        self.ac_tree = Some(ac_tree);
    }

    pub fn byte_align(&mut self) -> Result<()> {
        self.reader.byte_align();
        self.writer.byte_align()?;
        Ok(())
    }

    pub fn read<T: Numeric + std::fmt::Display + std::fmt::Binary>(
        &mut self,
        bits: u32,
    ) -> Result<T> {
        let value = self.reader.read(bits)?;
        self.writer.write::<T>(bits, value)?;
        Ok(value)
    }

    pub fn read_huffman_dc(&mut self) -> Result<u8> {
        let value = self.reader.read_huffman(self.dc_tree.unwrap().reader())?;

        self.writer
            .write_huffman(&self.dc_tree.unwrap().writer(), value)?;

        Ok(value)
    }

    pub fn read_huffman_ac(&mut self) -> Result<u8> {
        let value = self.reader.read_huffman(self.ac_tree.unwrap().reader())?;

        self.writer
            .write_huffman(&self.ac_tree.unwrap().writer(), value)?;

        Ok(value)
    }
}

impl HuffmanRWTree {
    pub fn new(reader: HuffmanTreeRead, writer: HuffmanTreeWrite) -> Self {
        Self { reader, writer }
    }

    pub fn reader(&self) -> &[HuffmanTreeReadInner] {
        self.reader.as_ref()
    }

    pub fn writer(&self) -> &HuffmanTreeWriteInner {
        &self.writer[0]
    }
}
