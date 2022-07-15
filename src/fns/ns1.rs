use num_bigint::BigUint;
use num_traits::Zero;

use super::{
    traits::{Bases, Digits, DigitsBases, InnerDigits, MaxBaseValue, TryFromInput, ValidInputs},
    NS0,
};

type Digit = NS0;

// Input: Vec<usize>
// Digits: NS0
impl_ns!(SUB: NS1, Digit, Vec<usize>);

impl NS1 {
    pub fn permute_values(&self, values: &mut [u8]) {
        let values = self.split_values_mut(values);
        for (digit, values) in self.digits.iter().zip(values) {
            digit.permute_values(values)
        }
    }

    fn split_values_mut<'a>(&self, mut values: &'a mut [u8]) -> Vec<&'a mut [u8]> {
        let mut results = Vec::new();
        for digit in &self.digits {
            let size = digit.digits.len() + 1;
            let (local_values, next_values) = values.split_at_mut(size);
            values = next_values;
            results.push(local_values);
        }
        results
    }

    pub fn read_values<'a>(input: &Vec<usize>, values: &'a [u8]) -> Self {
        let mut result = BigUint::zero();
        let values = Self::split_values(input, values);
        for (base, values) in super::traits::get_bases(&input.valid())
            .into_iter()
            .zip(values)
        {
            let value = BigUint::from(NS0::read_values(values));
            result += base * value;
        }
        NS1::try_from_input(result, input).unwrap()
    }

    fn split_values<'a>(sizes: &Vec<usize>, mut values: &'a [u8]) -> Vec<&'a [u8]> {
        let mut results = Vec::new();
        for size in sizes.valid() {
            let (local_values, next_values) = values.split_at(size);
            values = next_values;
            results.push(local_values);
        }
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(v: u32, input: &Vec<usize>) -> Option<NS1> {
        NS1::try_from_input(BigUint::from(v), input)
    }

    fn digits(v: u32, input: &Vec<usize>) -> Option<Vec<Vec<usize>>> {
        n(v, input).map(|ns| ns.inner_digits())
    }

    fn big(ns: NS1) -> u32 {
        u32::try_from(BigUint::from(ns)).unwrap()
    }

    #[test]
    fn test_invalid_0() {
        assert_eq!(digits(1, &vec![2, 0, 0, 2]), Some(vec![vec![0], vec![1]]));
        assert_eq!(digits(2, &vec![2, 0, 0, 2]), Some(vec![vec![1], vec![0]]));
        assert_eq!(digits(3, &vec![2, 0, 0, 2]), Some(vec![vec![1], vec![1]]));
        assert_eq!(digits(4, &vec![2, 0, 0, 2]), None);
    }

    #[test]
    fn test_invalid_1() {
        assert_eq!(digits(1, &vec![2, 1, 1, 2]), Some(vec![vec![0], vec![1]]));
        assert_eq!(digits(2, &vec![2, 1, 1, 2]), Some(vec![vec![1], vec![0]]));
        assert_eq!(digits(3, &vec![2, 1, 1, 2]), Some(vec![vec![1], vec![1]]));
        assert_eq!(digits(4, &vec![2, 1, 1, 2]), None);
    }

    #[test]
    fn test_from_biguint() {
        assert_eq!(digits(1, &vec![2, 2]), Some(vec![vec![0], vec![1]]));
        assert_eq!(digits(2, &vec![2, 2]), Some(vec![vec![1], vec![0]]));
        assert_eq!(digits(3, &vec![2, 2]), Some(vec![vec![1], vec![1]]));
        assert_eq!(digits(4, &vec![2, 2]), None);

        assert_eq!(digits(5, &vec![2, 3]), Some(vec![vec![0], vec![2, 1]]));
        assert_eq!(digits(10, &vec![2, 3]), Some(vec![vec![1], vec![2, 0]]));
        assert_eq!(digits(12, &vec![2, 3]), None);
        assert_eq!(digits(15, &vec![3, 3]), Some(vec![vec![1, 0], vec![1, 1]]));
        assert_eq!(digits(20, &vec![3, 3]), Some(vec![vec![1, 1], vec![1, 0]]));
    }

    #[test]
    fn test_to_from_biguint() {
        for i in 1..864u32 {
            let n = BigUint::from(i);
            let ns = NS1::try_from_input(n.clone(), &vec![3, 3, 4])
                .expect(&format!("Expected value for {n:}"));
            let v = BigUint::from(ns);
            assert_eq!(n, v);
        }
    }

    #[test]
    fn test_permute_values() {
        let input = vec![3, 3];
        let ns = n(10, &input).unwrap();

        let mut buf = vec![3, 5, 10, 15, 20, 25];
        ns.permute_values(&mut buf);
        assert_eq!(buf, vec![3, 10, 5, 25, 15, 20]);
        ns.permute_values(&mut buf);
        assert_eq!(buf, vec![3, 10, 5, 25, 15, 20]);
    }

    #[test]
    fn test_read_values() {
        let buf = vec![25, 15, 5, 0, 20, 10];
        assert_eq!(big(NS1::read_values(&vec![0, 6], &buf)), 679);
        assert_eq!(big(NS1::read_values(&vec![1, 5], &buf)), 110);
        assert_eq!(big(NS1::read_values(&vec![2, 4], &buf)), 31);
        assert_eq!(big(NS1::read_values(&vec![3, 3], &buf)), 31);
        assert_eq!(big(NS1::read_values(&vec![4, 2], &buf)), 47);
        assert_eq!(big(NS1::read_values(&vec![5, 1], &buf)), 110);
        assert_eq!(big(NS1::read_values(&vec![6, 0], &buf)), 679);
    }
}
