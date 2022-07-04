pub fn construct_huffman_table<U: AsRef<[usize]>, V: AsRef<[u8]>>(
    sizes: U,
    values: V,
) -> Vec<(u8, Vec<u8>)> {
    let sizes = sizes.as_ref();
    let values = values.as_ref();

    let codes = sizes
        .into_iter()
        .enumerate()
        .filter_map(|(size, &count)| match count {
            0 => None,
            _ => Some((size + 1, count)),
        })
        .collect::<Vec<_>>();

    let mut code = 0u16;
    let mut code_table = Vec::new();
    let mut values = values.into_iter();
    let mut last = None;
    let mut last_size = 0;
    for (size, count) in codes {
        code <<= size - last_size;
        last_size = size;
        for _ in 0..count {
            let &value = values.next().unwrap();
            let bits = bin_to_vec(code, size);
            code_table.push((value, bits));
            code += 1;
        }

        last = Some((255, bin_to_vec(code, size)));
    }
    if let Some(last) = last {
        code_table.push(last);
    }

    code_table
}

fn bin_to_vec<T>(mut value: T, size: usize) -> Vec<u8>
where
    T: num_traits::Zero + num_traits::One,
    T: PartialEq + PartialOrd + Copy,
    u8: TryFrom<T>,
    T: std::ops::BitAnd<Output = T>,
    T: std::ops::ShrAssign<T>,
{
    let (zero, one) = (T::zero(), T::one());

    if value == zero {
        let mut output = vec![0];
        output.resize(size, 0);
        return output;
    }

    let mut bits = Vec::new();
    while value > zero {
        let bit = u8::try_from(value & one).unwrap_or_default();
        bits.push(bit);
        value >>= one;
    }

    bits.resize(size, 0);
    bits.reverse();
    bits
}

#[allow(dead_code)]
pub fn print_huffman_table(table: &Vec<(u8, Vec<u8>)>) {
    for (value, bits) in table {
        println!(
            "\t{value}\t{}",
            bits.into_iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join("")
        );
    }
}

/////////////////////////////////////////
