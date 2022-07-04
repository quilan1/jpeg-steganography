use anyhow::Result;

use crate::jpeg::{segments::*, Jpeg, Marker::*, ProcessSegment, Segment};

pub struct DebugProcessor;

impl ProcessSegment for DebugProcessor {
    type Output = ();

    fn process_segment(&mut self, _: &mut Jpeg, section: &Segment) -> Result<Self::Output> {
        let Segment {
            index,
            marker,
            data,
        } = section;
        let marker = *marker;

        match marker {
            RST(_) => return Ok(()),
            _ => {}
        }

        println!(
            "[{:04X}] FF{:02X} {:?}",
            index,
            Into::<u8>::into(marker),
            marker
        );

        match marker {
            // [SPEC] B.2.2 -- Frame header syntax
            SOF0 | SOF1 | SOF2 => {
                println!("\tStart of Frame");
                let SofData {
                    precision,
                    width,
                    height,
                    components,
                } = SofData::try_from(&data[..])?;

                println!(
                    "\tPrecision: {precision}, Width: {width}, Height: {height}, Num Components: {}",
                    components.len()
                );

                for Component {
                    component_id,
                    h_factor,
                    v_factor,
                    table_index,
                } in components
                {
                    println!(
                        "\tComponent: ID={component_id}, HFactor={h_factor}, VFactor={v_factor}, Quant Table={table_index}",
                    );
                }
                println!();
            }

            // [SPEC] B.2.3 -- Scan header syntax
            SOS => {
                println!("\tStart of Scan");
                let SosData {
                    spectral_start,
                    spectral_end,
                    approx_high,
                    approx_low,
                    components,
                    ..
                } = SosData::try_from(&data[..])?;

                for ScanComponentData {
                    component_id,
                    dc_table_index,
                    ac_table_index,
                } in components
                {
                    println!(
                        "\tComponent: ID={component_id}, DC Table={dc_table_index}, AC Table={ac_table_index}"
                    );
                }

                println!(
                    "\tSpectralStart: {spectral_start}, SpectralEnd: {spectral_end}, AH: {approx_high}, AL: {approx_low}");
                println!();
            }

            // [SPEC] B.2.4.1 -- Quantization table-specification syntax
            DQT => {
                println!("\tDefine Quantization Table");
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
                    println!(
                        "\tTable {}: Precision: {}, Table Index: {}",
                        index, precision, table_index
                    );
                    println!("\t\tValues: {:?}", values.to_vec());
                }
                println!();
            }

            // [SPEC] Table B.2.4.2 -- Huffman table-specification syntax
            DHT => {
                println!("\tDefine Huffman Table");
                let DhtData { tables } = DhtData::try_from(&data[..])?;

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
                    println!(
                        "\tTable: {}, Class: {}, Index: {}",
                        index, table_class, table_index
                    );
                    println!("\t\tSizes: {:?}", sizes);
                    println!("\t\tValues: {:?}", values);
                }
                println!();
            }

            // [SPEC] B.2.4.4 -- Restart interval definition syntax
            DRI => {
                let count = u16::from_be_bytes(data[0..2].try_into().unwrap());
                println!("\tDefine Restart Interval: {}", count);
                println!();
            }

            _ => {}
        }
        Ok(())
    }
}
