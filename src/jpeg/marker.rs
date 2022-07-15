// [SPEC] Table B.1
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Marker {
    SOF0,        // Baseline DCT
    SOF1,        // Extended Sequential DCT
    SOF2,        // Progressive DCT
    DHT,         // Huffman Table Specification
    RST(u8),     // Restart markers
    SOI,         // Start of Image
    EOI,         // End of Image
    SOS,         // Start of Scan
    DQT,         // Define Quantization Table(s)
    DNL,         // Define Number of Lines
    DRI,         // Define Restart Interval
    Unknown(u8), // Unknown / misc marker
}

impl From<u8> for Marker {
    fn from(value: u8) -> Self {
        use Marker::*;

        match value {
            0xC0 => SOF0,
            0xC1 => SOF1,
            0xC2 => SOF2,
            0xC4 => DHT,
            0xD0..=0xD7 => RST(value - 0xD0),
            0xD8 => SOI,
            0xD9 => EOI,
            0xDA => SOS,
            0xDB => DQT,
            0xDC => DNL,
            0xDD => DRI,
            _ => Unknown(value),
        }
    }
}

impl From<Marker> for u8 {
    fn from(value: Marker) -> Self {
        use Marker::*;

        match value {
            SOF0 => 0xC0,
            SOF1 => 0xC1,
            SOF2 => 0xC2,
            DHT => 0xC4,
            RST(value) => 0xD0 + value,
            SOI => 0xD8,
            EOI => 0xD9,
            SOS => 0xDA,
            DQT => 0xDB,
            DNL => 0xDC,
            DRI => 0xDD,
            Unknown(value) => value,
        }
    }
}
