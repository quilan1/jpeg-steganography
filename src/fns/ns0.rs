use num_bigint::BigUint;
use num_traits::{ToPrimitive, Zero};

use super::traits::{Bases, DigitsBases, InnerDigits, MaxBaseValue, TryFromInput};

type Digit = usize;

// Input: usize
// Digits: usize
impl_ns!(BASE: NS0, Digit);

impl InnerDigits<usize> for NS0 {
    fn inner_digits(&self) -> Vec<usize> {
        self.digits.clone()
    }
}

impl Bases for NS0 {
    fn bases<U>(&self) -> Vec<BigUint> {
        (self.digits.len() + 1).bases()
    }
}

impl MaxBaseValue for NS0 {
    fn max_base_value(&self) -> BigUint {
        (self.digits.len() + 1).max_base_value()
    }
}

impl TryFromInput<usize> for NS0 {
    fn try_from_input(value: BigUint, input: &usize) -> Option<Self>
    where
        Self: Sized,
    {
        if value >= input.max_base_value() {
            return None;
        }

        let mut value = value;
        let mut digits = Vec::new();
        let bases = input.bases();
        for base in bases {
            let digit = &value / &base;
            value -= &digit * &base;
            digits.push(digit.to_usize().unwrap());
        }

        Some(Self { digits })
    }
}

impl NS0 {
    pub fn to_permutation(&self) -> Vec<usize> {
        let size = self.digits.len() + 1;
        let mut available = (0..size).collect::<Vec<_>>();

        let digits = self.digits.clone();
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

        Self { digits }
    }

    pub fn permute_values(&self, values: &mut [u8]) {
        let permutation = self.to_permutation();
        let mut old_values = values.to_vec();
        old_values.sort();

        for (index, perm) in permutation.into_iter().enumerate() {
            values[index] = old_values[perm];
        }
    }

    pub fn read_values(values: &[u8]) -> Self {
        let mut sorted_values = values.to_vec();
        sorted_values.sort();

        let mut permutation = Vec::new();
        for value in values {
            permutation.push(sorted_values.iter().position(|v| value == v).unwrap());
        }

        Self::from_permutation(permutation.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(v: u32, input: usize) -> Option<NS0> {
        NS0::try_from_input(BigUint::from(v), &input)
    }

    fn digits(v: u32, input: usize) -> Option<Vec<usize>> {
        n(v, input).map(|ns| ns.inner_digits())
    }

    fn to_perm(v: u32, input: usize) -> Option<Vec<usize>> {
        n(v, input).map(|ns| ns.to_permutation())
    }

    fn big(ns: NS0) -> u32 {
        u32::try_from(BigUint::from(ns)).unwrap()
    }

    fn from_perm(permutation: Vec<usize>) -> u32 {
        big(NS0::from_permutation(permutation))
    }

    #[test]
    fn test_from_biguint() {
        assert_eq!(digits(1, 2), Some(vec![1]));
        assert_eq!(digits(2, 3), Some(vec![1, 0]));
        assert_eq!(digits(6, 4), Some(vec![1, 0, 0]));
        assert_eq!(digits(24, 5), Some(vec![1, 0, 0, 0]));

        assert_eq!(digits(5, 3), Some(vec![2, 1]));
        assert_eq!(digits(10, 4), Some(vec![1, 2, 0]));
        assert_eq!(digits(15, 4), Some(vec![2, 1, 1]));
        assert_eq!(digits(20, 4), Some(vec![3, 1, 0]));
    }

    #[test]
    fn test_from_biguint_input_too_small() {
        assert_eq!(digits(1, 1), None);
        assert_eq!(digits(2, 2), None);
        assert_eq!(digits(6, 3), None);
        assert_eq!(digits(24, 4), None);

        assert_eq!(digits(5, 2), None);
        assert_eq!(digits(10, 3), None);
        assert_eq!(digits(15, 3), None);
        assert_eq!(digits(20, 3), None);
    }

    #[test]
    fn test_to_from_biguint() {
        for i in 1..120u32 {
            let n = BigUint::from(i);
            let ns = NS0::try_from_input(n.clone(), &5).expect(&format!("Expected value for {n:}"));
            let v = BigUint::from(ns);
            assert_eq!(n, v);
        }
    }

    #[test]
    fn test_to_permutation() {
        assert_eq!(to_perm(0, 3), Some(vec![0, 1, 2]));
        assert_eq!(to_perm(1, 3), Some(vec![0, 2, 1]));
        assert_eq!(to_perm(2, 3), Some(vec![1, 0, 2]));
        assert_eq!(to_perm(3, 3), Some(vec![1, 2, 0]));
        assert_eq!(to_perm(4, 3), Some(vec![2, 0, 1]));
        assert_eq!(to_perm(5, 3), Some(vec![2, 1, 0]));
    }

    #[test]
    fn test_from_permutation() {
        assert_eq!(from_perm(vec![0, 1, 2]), 0);
        assert_eq!(from_perm(vec![0, 2, 1]), 1);
        assert_eq!(from_perm(vec![1, 0, 2]), 2);
        assert_eq!(from_perm(vec![1, 2, 0]), 3);
        assert_eq!(from_perm(vec![2, 0, 1]), 4);
        assert_eq!(from_perm(vec![2, 1, 0]), 5);
    }

    #[test]
    fn test_to_from_permutation() {
        for i in 0..120 {
            let p = to_perm(i, 5).expect(&format!("Expected value for {i}"));
            let n = from_perm(p);
            assert_eq!(i, n);
        }
    }

    #[test]
    fn test_permute_values() {
        let ns = n(3, 3).unwrap();

        let mut buf = vec![3, 5, 10];
        ns.permute_values(&mut buf);
        assert_eq!(buf, vec![5, 10, 3]);
        ns.permute_values(&mut buf);
        assert_eq!(buf, vec![5, 10, 3]);
    }

    #[test]
    fn test_read_values() {
        assert_eq!(big(NS0::read_values(&vec![3, 5, 10])), 0);
        assert_eq!(big(NS0::read_values(&vec![3, 10, 5])), 1);
        assert_eq!(big(NS0::read_values(&vec![5, 3, 10])), 2);
        assert_eq!(big(NS0::read_values(&vec![5, 10, 3])), 3);
        assert_eq!(big(NS0::read_values(&vec![10, 3, 5])), 4);
        assert_eq!(big(NS0::read_values(&vec![10, 5, 3])), 5);
    }
}
