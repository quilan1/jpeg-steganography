/*
 * Some sections of this code were pulled from the Rust jpeg-decoder library.
 */

use anyhow::Result;

use crate::rw_stream::{HuffmanRWTree, RWStream};

use super::{segments::Component, Jpeg};

struct ComponentInfo<'a> {
    component: &'a Component,
    dc_tree: &'a HuffmanRWTree,
    ac_tree: &'a HuffmanRWTree,
}

pub fn process_entropy_stream(jpeg: &Jpeg, in_data: &Vec<u8>) -> Result<Vec<u8>> {
    let components_info = get_components_info(jpeg);
    let (mcu_horizontal_samples, mcu_vertical_samples) = get_num_samples(&components_info);
    let (max_mcu_x, max_mcu_y) = get_mcu_range(jpeg, &components_info);

    let mut eob_run = 0;
    let mut mcus_left_until_restart = jpeg.restart_interval;

    let in_data = strip_stream_padding(in_data);
    let mut out_data = Vec::with_capacity(in_data.len());
    let mut marker_positions = Vec::new();
    let mut read_writer = RWStream::new(&in_data, &mut out_data);

    for mcu_y in 0..max_mcu_y {
        if mcu_y * 8 >= jpeg.frame.height {
            break;
        }

        for mcu_x in 0..max_mcu_x {
            if mcu_x * 8 >= jpeg.frame.width {
                break;
            }

            if jpeg.restart_interval > 0 {
                if mcus_left_until_restart == 0 {
                    // We should have a byte-aligned RST marker here, let's process it
                    read_writer.byte_align()?;
                    marker_positions.push(read_writer.writer_position());
                    let marker_header = read_writer.read::<u8>(8)?;
                    assert_eq!(marker_header, 0xFF);

                    read_writer.read::<u8>(8)?;

                    eob_run = 0;
                    mcus_left_until_restart = jpeg.restart_interval;
                }

                mcus_left_until_restart -= 1;
            }

            for (i, component_info) in components_info.iter().enumerate() {
                let dc_table = &component_info.dc_tree;
                let ac_table = &component_info.ac_tree;
                read_writer.set_tables(dc_table, ac_table);

                for _v_pos in 0..mcu_vertical_samples[i] {
                    for _h_pos in 0..mcu_horizontal_samples[i] {
                        decode_block(&mut read_writer, jpeg, &mut eob_run)?;
                    }
                }
            }
        }
    }

    let out_data = insert_data_padding(&mut out_data, &marker_positions);
    Ok(out_data)
}

fn decode_block<'a>(read_writer: &mut RWStream<'a>, jpeg: &Jpeg, eob_run: &mut u16) -> Result<()> {
    if jpeg.scan.spectral_start == 0 {
        // Section F.2.2.1
        // Figure F.12

        let value = read_writer.read_huffman_dc()?;
        match value {
            0 => {}
            1..=11 => {
                read_writer.read::<u16>(value.into())?;
            }
            _ => panic!(),
        }
    }

    let mut index = jpeg.scan.spectral_start.max(1);
    if index < jpeg.scan.spectral_end && *eob_run > 0 {
        *eob_run -= 1;
        return Ok(());
    }

    // Section F.1.2.2.1
    while index < jpeg.scan.spectral_end {
        let byte = read_writer.read_huffman_ac()?;
        let r = byte >> 4;
        let s = byte & 0x0f;

        if s == 0 {
            match r {
                15 => index += 16, // Run length of 16 zero coefficients.
                _ => {
                    *eob_run = (1 << r) - 1;

                    if r > 0 {
                        *eob_run += read_writer.read::<u16>(r.into())?;
                    }

                    break;
                }
            }
        } else {
            index += r as u32;

            if index >= jpeg.scan.spectral_end {
                break;
            }

            read_writer.read::<u16>(s.into())?;
            index += 1;
        }
    }

    Ok(())
}

fn strip_stream_padding(in_data: &Vec<u8>) -> Vec<u8> {
    let mut fixed_data = Vec::with_capacity(in_data.len());
    let mut data_iter = in_data.iter().cloned();
    while let Some(value) = data_iter.next() {
        fixed_data.push(value);
        if value == 0xFF {
            let value = data_iter.next().unwrap();
            if value != 0x00 {
                fixed_data.push(value);
            }
        }
    }
    fixed_data
}

fn insert_data_padding(data: &mut Vec<u8>, marker_positions: &Vec<usize>) -> Vec<u8> {
    let mut out_data = Vec::new();
    for (index, value) in data.drain(..).enumerate() {
        out_data.push(value);
        if value == 0xFF {
            if !marker_positions.contains(&index) {
                out_data.push(0x00);
            }
        }
    }
    out_data
}

fn get_components_info(jpeg: &Jpeg) -> Vec<ComponentInfo> {
    let mut components = Vec::new();
    for scan_component in &jpeg.scan.components {
        let component_index = jpeg
            .frame
            .components
            .iter()
            .position(|c| c.component_id == scan_component.component_id)
            .unwrap();

        let component = &jpeg.frame.components[component_index];
        let (dc_table, ac_table) =
            jpeg.get_huffman_trees(scan_component.dc_table_index, scan_component.ac_table_index);

        components.push(ComponentInfo {
            component,
            dc_tree: dc_table,
            ac_tree: ac_table,
        });
    }
    components
}

fn get_num_samples(components_info: &Vec<ComponentInfo>) -> (Vec<u32>, Vec<u32>) {
    let horizontal = components_info
        .iter()
        .map(|component_info| component_info.component.h_factor)
        .collect::<Vec<_>>();
    let vertical = components_info
        .iter()
        .map(|component_info| component_info.component.v_factor)
        .collect::<Vec<_>>();
    (horizontal, vertical)
}

fn get_mcu_range(jpeg: &Jpeg, components_info: &Vec<ComponentInfo>) -> (u32, u32) {
    let h_max = components_info
        .iter()
        .map(|c| c.component.h_factor)
        .max()
        .unwrap();
    let v_max = components_info
        .iter()
        .map(|c| c.component.v_factor)
        .max()
        .unwrap();

    (
        (jpeg.frame.width + h_max * 8 - 1) / (h_max * 8),
        (jpeg.frame.height + v_max * 8 - 1) / (v_max * 8),
    )
}
