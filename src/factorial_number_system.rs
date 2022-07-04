use num_bigint::BigUint;
use num_traits::{identities::Zero, One, ToPrimitive};

#[derive(Debug)]
pub struct FNS {
    pub digits: Vec<usize>,
}

impl FNS {
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        let value = BigUint::from_bytes_be(&bytes);
        Self::from(value)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut digits = self.digits.clone();
        digits.reverse();

        let mut result = BigUint::zero();
        for (index, digit) in digits.into_iter().enumerate() {
            result += BigUint::from(digit) * factorial(index + 1);
        }

        result.to_bytes_be()
    }

    pub fn to_permutation(&self, size: usize) -> Vec<usize> {
        let mut available = (0..size).collect::<Vec<_>>();

        let mut digits = vec![0; size - self.digits.len() - 1];
        digits.extend(&self.digits);

        let mut permutation = Vec::new();
        for digit in digits {
            permutation.push(available.remove(digit));
        }

        permutation.extend(&available);
        permutation
    }

    pub fn from_permutation(permutation: Vec<usize>) -> Self {
        let mut available = (0..permutation.len()).collect::<Vec<_>>();

        let mut digits = Vec::new();
        for perm_digit in &permutation[..permutation.len() - 1] {
            let index = available.iter().position(|v| v == perm_digit).unwrap();
            available.remove(index);
            digits.push(index);
        }

        while digits.len() > 1 && digits[0] == 0 {
            digits.remove(0);
        }

        Self { digits }
    }
}

impl<T> From<T> for FNS
where
    BigUint: From<T>,
{
    fn from(value: T) -> Self {
        let value = BigUint::from(value);

        // Find extent
        let mut num_digits = 1;
        loop {
            let fac = factorial(num_digits + 1);
            if fac > value {
                break;
            }
            num_digits += 1;
        }

        let mut digits = Vec::new();
        let mut value = value;
        for index in (1..=num_digits).rev() {
            let fac = factorial(index);
            let digit = value.clone() / fac.clone();
            value -= digit.clone() * fac;
            digits.push(digit.to_usize().unwrap());
        }

        Self { digits }
    }
}

fn factorial(mut n: usize) -> BigUint {
    let mut result = BigUint::one();

    while n > 1 {
        result *= BigUint::from(n);
        n -= 1;
    }

    result
}

////////////////////////////////////////////////////
/// Methods for permuting huffman tables

impl FNS {
    pub fn permute_huffman_table<T: AsRef<[usize]>, U: AsMut<[u8]>>(
        &self,
        sizes: T,
        mut values: U,
    ) -> bool {
        let sizes = sizes.as_ref();
        let values = values.as_mut();

        let mut offset = 0;
        for &size in sizes {
            if size <= self.digits.len() {
                offset += size;
                continue;
            }

            // Found valid span!
            let permutation = self.to_permutation(size);
            let values = &mut values[offset..offset + size];
            let mut old_values = values.to_vec();
            old_values.sort();

            for (index, perm) in permutation.into_iter().enumerate() {
                values[index] = old_values[perm];
            }

            return true;
        }

        false
    }

    pub fn read_huffman_table<T: AsRef<[usize]>, U: AsRef<[u8]>>(
        sizes: T,
        values: U,
    ) -> Vec<Vec<u8>> {
        let sizes = sizes.as_ref();
        let values = values.as_ref();

        let mut outputs = Vec::new();
        let mut offset = 0;
        for &size in sizes {
            if size <= 1 {
                offset += size;
                continue;
            }

            let orig_values = &values[offset..offset + size];
            let mut sorted_values = orig_values.to_vec();
            sorted_values.sort();

            let mut permutation = Vec::new();
            for value in orig_values {
                permutation.push(sorted_values.iter().position(|v| value == v).unwrap());
            }

            // Found valid span!
            let bytes = Self::from_permutation(permutation.clone()).to_bytes();

            if bytes == vec![0] {
                offset += size;
                continue;
            }

            outputs.push(bytes);
        }

        outputs
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_biguint_to_fns() {
        assert_eq!(FNS::from(0u32).digits, vec![0]);
        assert_eq!(FNS::from(1u32).digits, vec![1]);
        assert_eq!(FNS::from(2u32).digits, vec![1, 0]);
        assert_eq!(FNS::from(6u32).digits, vec![1, 0, 0]);
        assert_eq!(FNS::from(24u32).digits, vec![1, 0, 0, 0]);

        assert_eq!(FNS::from(5u32).digits, vec![2, 1]);
        assert_eq!(FNS::from(10u32).digits, vec![1, 2, 0]);
        assert_eq!(FNS::from(15u32).digits, vec![2, 1, 1]);
        assert_eq!(FNS::from(20u32).digits, vec![3, 1, 0]);
    }

    #[test]
    fn test_to_permutation() {
        assert_eq!(FNS::from(0u32).to_permutation(3), vec![0, 1, 2]);
        assert_eq!(FNS::from(1u32).to_permutation(3), vec![0, 2, 1]);
        assert_eq!(FNS::from(2u32).to_permutation(3), vec![1, 0, 2]);
        assert_eq!(FNS::from(3u32).to_permutation(3), vec![1, 2, 0]);
        assert_eq!(FNS::from(4u32).to_permutation(3), vec![2, 0, 1]);
        assert_eq!(FNS::from(5u32).to_permutation(3), vec![2, 1, 0]);
    }

    #[test]
    fn test_from_permutation() {
        for v in 0u32..120 {
            let fns = FNS::from(v);
            let perm = fns.to_permutation(5);
            let fns_new = FNS::from_permutation(perm.clone());
            // println!("V: {v}, {:?}, P: {perm:?}, D: {:?}", fns.digits, fns_new.digits);
            assert_eq!(fns.digits, fns_new.digits);
        }
    }

    #[test]
    fn test_to_bytes() {
        for value in 1u16..1000 {
            let mut bytes = value.to_be_bytes().to_vec();
            while bytes.len() > 1 && bytes[0] == 0 {
                bytes.remove(0);
            }
            let fns = FNS::from_bytes(bytes.clone());
            let out_bytes = fns.to_bytes();

            // println!("{value}: {bytes:?} {fns:?} {out_bytes:?}");
            assert_eq!(bytes, out_bytes);
        }
    }
}
