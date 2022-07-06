use anyhow::Result;

use crate::jpeg::{segments::*, Jpeg, Marker::*, ProcessSegment, Segment};

pub struct DebugReader<F> {
    log: F,
}

impl<F> DebugReader<F> {
    pub fn new(log: F) -> Self {
        Self { log }
    }
}

macro_rules! log {
    ($log:expr, $($arg:tt)*) => {
        ($log)(format!($($arg)*));
    };
}

impl<F: Fn(String)> ProcessSegment for DebugReader<F> {
    fn process_segment(&self, _: &Jpeg, segment: &Segment) -> Result<()> {
        let Segment {
            index,
            marker,
            data,
        } = segment;
        let marker = *marker;

        match marker {
            RST(_) => return Ok(()),
            _ => {}
        }

        log!(
            self.log,
            "[{index:04X}] FF{:02X} {marker:?}",
            Into::<u8>::into(marker),
        );

        match marker {
            // [SPEC] B.2.2 -- Frame header syntax
            SOF0 | SOF1 | SOF2 => {
                let SofData {
                    precision,
                    width,
                    height,
                    components,
                } = SofData::try_from(&data[..])?;

                log!(self.log, "\tStart of Frame\n\tPrecision: {precision}, Width: {width}, Height: {height}, Num Components: {}",
                    components.len()
                );

                for Component {
                    component_id,
                    h_factor,
                    v_factor,
                    table_index,
                } in components
                {
                    log!(self.log,
                        "\tComponent: ID={component_id}, HFactor={h_factor}, VFactor={v_factor}, Quant Table={table_index}",
                    );
                }
                log!(self.log, "");
            }

            // [SPEC] B.2.3 -- Scan header syntax
            SOS => {
                let SosData {
                    spectral_start,
                    spectral_end,
                    approx_high,
                    approx_low,
                    components,
                    ..
                } = SosData::try_from(&data[..])?;

                log!(self.log, "\tStart of Scan");
                for ScanComponentData {
                    component_id,
                    dc_table_index,
                    ac_table_index,
                } in components
                {
                    log!(self.log, "\tComponent: ID={component_id}, DC Table={dc_table_index}, AC Table={ac_table_index}");
                }
                log!(self.log, "\tSpectralStart={spectral_start}, SpectralEnd={spectral_end}, AH={approx_high}, AL={approx_low}\n");
            }

            // [SPEC] B.2.4.1 -- Quantization table-specification syntax
            DQT => {
                log!(self.log, "\tDefine Quantization Table");
                let DqtData { tables } = DqtData::try_from(&data[..])?;
                for (
                    index,
                    QuantizationTable {
                        precision,
                        table_index,
                        values,
                    },
                ) in tables.into_iter().enumerate()
                {
                    log!(self.log, "\tTable {index}: Precision={precision}, Table Index={table_index}\n\t\tValues: {:?}", values.to_vec());
                }
                log!(self.log, "");
            }

            // [SPEC] Table B.2.4.2 -- Huffman table-specification syntax
            DHT => {
                let DhtData { tables } = DhtData::try_from(&data[..])?;

                log!(self.log, "\tDefine Huffman Table");
                for (
                    index,
                    HuffmanTableData {
                        table_class,
                        table_index,
                        sizes,
                        values,
                    },
                ) in tables.into_iter().enumerate()
                {
                    log!(self.log, "\tTable: {index}, Class: {table_class}, Index: {table_index}\n\t\tSizes: {sizes:?}\n\t\tValues: {values:?}");
                }
                log!(self.log, "");
            }

            // [SPEC] B.2.4.4 -- Restart interval definition syntax
            DRI => {
                let count = u16::from_be_bytes(data[0..2].try_into().unwrap());
                log!(self.log, "\tDefine Restart Interval: {count}\n");
            }

            _ => {}
        }
        Ok(())
    }
}
