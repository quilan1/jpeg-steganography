use num_bigint::BigUint;
use num_traits::{identities::Zero, One, ToPrimitive};

pub struct FNS {
    pub digits: Vec<usize>,
}

pub struct SFNS {
    pub digits: Vec<FNS>,
}

impl FNS {
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        let value = BigUint::from_bytes_be(&bytes);
        Self::from(value)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        BigUint::from(self).to_bytes_be()
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

    pub fn permute_values(&self, values: &mut [u8]) {
        let permutation = self.to_permutation(values.len());
        let mut old_values = values.to_vec();
        old_values.sort();

        for (index, perm) in permutation.into_iter().enumerate() {
            values[index] = old_values[perm];
        }
    }

    pub fn read_values(values: &[u8]) -> FNS {
        let mut sorted_values = values.to_vec();
        sorted_values.sort();

        let mut permutation = Vec::new();
        for value in values {
            permutation.push(sorted_values.iter().position(|v| value == v).unwrap());
        }

        Self::from_permutation(permutation.clone())
    }
}

impl From<&FNS> for BigUint {
    fn from(fns: &FNS) -> Self {
        let mut digits = fns.digits.clone();
        digits.reverse();

        let mut result = BigUint::zero();
        for (index, digit) in digits.into_iter().enumerate() {
            result += BigUint::from(digit) * factorial(index + 1);
        }

        result
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

impl SFNS {
    pub fn new(bytes: &Vec<u8>, sizes: &Vec<usize>) -> Option<Self> {
        let value = BigUint::from_bytes_be(&bytes);

        let (bases, base) = Self::calculate_bases(sizes);
        if value >= base {
            return None;
        }

        let mut value = value;
        let mut digits = Vec::new();
        for base_index in (0..bases.len()).rev() {
            let digit = &value / &bases[base_index];
            digits.push(FNS::from(digit.clone()));
            value -= digit * &bases[base_index];
        }
        digits.reverse();

        Some(Self { digits })
    }

    pub fn permute_values(&self, sizes: &Vec<usize>, values: &mut Vec<u8>) {
        let digit_values = self.get_digit_values(sizes, values);
        for (digit, values) in digit_values {
            digit.permute_values(values);
        }
    }

    pub fn from_size_values(sizes: &Vec<usize>, values: &Vec<u8>) -> Vec<u8> {
        let mut result = BigUint::zero();
        let base_values = Self::get_base_values(sizes, values);
        for (base, values) in base_values {
            result += base * BigUint::from(&FNS::read_values(values));
        }
        result.to_bytes_be()
    }

    pub fn max_message(sizes: &Vec<usize>) -> BigUint {
        Self::calculate_bases(sizes).1
    }

    fn calculate_bases(sizes: &Vec<usize>) -> (Vec<BigUint>, BigUint) {
        let mut bases = Vec::new();
        let mut base = BigUint::from(1u8);
        for size in gt_one(sizes) {
            bases.push(base.clone());
            base *= factorial(size);
        }
        (bases, base)
    }

    fn get_base_values<'a>(sizes: &Vec<usize>, values: &'a Vec<u8>) -> Vec<(BigUint, &'a [u8])> {
        let (bases, _) = Self::calculate_bases(sizes);
        bases.into_iter().zip(split_values(sizes, values)).collect()
    }

    fn get_digit_values<'a>(
        &self,
        sizes: &Vec<usize>,
        values: &'a mut Vec<u8>,
    ) -> Vec<(&FNS, &'a mut [u8])> {
        self.digits
            .iter()
            .zip(split_values_mut(sizes, values))
            .collect()
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

fn gt_one<T: One + Copy + PartialOrd, V: AsRef<[T]>>(sizes: V) -> Vec<T> {
    sizes
        .as_ref()
        .into_iter()
        .cloned()
        .filter(|&v| v > T::one())
        .collect()
}

fn split_values<'a>(sizes: &[usize], mut values: &'a [u8]) -> Vec<&'a [u8]> {
    let mut results = Vec::new();
    for size in gt_one(&sizes) {
        let (local_values, next_values) = values.split_at(size);
        values = next_values;
        results.push(local_values);
    }
    results
}

fn split_values_mut<'a>(sizes: &[usize], mut values: &'a mut [u8]) -> Vec<&'a mut [u8]> {
    let mut results = Vec::new();
    for size in gt_one(&sizes) {
        let (local_values, next_values) = values.split_at_mut(size);
        values = next_values;
        results.push(local_values);
    }
    results
}

////////////////////////////////////////////////////
/// Methods for permuting huffman tables

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
