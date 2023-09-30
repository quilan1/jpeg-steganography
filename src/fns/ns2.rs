use num_bigint::BigUint;
use num_traits::Zero;

use super::{
    traits::{DigitsBases, TryFromInput, ValidInputs},
    NS1,
};

type Digit = NS1;
type Input = Vec<Vec<u8>>;
type InnerDigit = Vec<Vec<usize>>;
impl_base_ns!(NS2, Digit);
impl_sub_ns!(NS2, Input, InnerDigit);

impl NS2 {
    pub fn permute_values(&self, values: &mut Input) {
        for (digit, values) in self.digits.iter().zip(values) {
            digit.permute_values(values);
        }
    }

    pub fn read_values(input: &Input, values: &Input) -> Self {
        let mut result = BigUint::zero();
        for (input, (base, values)) in input.iter().zip(
            super::traits::get_bases(&input.valid())
                .into_iter()
                .zip(values),
        ) {
            let value = BigUint::from(NS1::read_values(input, values));
            result += base * value;
        }
        NS2::try_from_input(result, input).unwrap()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_fun_call)]

    use super::super::traits::InnerDigits;
    use super::*;

    fn n(v: u32, input: &Input) -> Option<NS2> {
        NS2::try_from_input(BigUint::from(v), input)
    }

    fn digits(v: u32, input: &Input) -> Option<Vec<InnerDigit>> {
        n(v, input).map(|ns| ns.inner_digits())
    }

    fn big(ns: NS2) -> u32 {
        u32::try_from(BigUint::from(ns)).unwrap()
    }

    #[test]
    fn test_invalid() {
        assert_eq!(
            digits(1, &vec![vec![1, 1], vec![2]]),
            Some(vec![vec![vec![], vec![]], vec![vec![1]]])
        );
        assert_eq!(
            digits(1, &vec![vec![1], vec![2, 1, 1]]),
            Some(vec![vec![vec![]], vec![vec![1], vec![], vec![]]])
        );
    }

    #[test]
    fn test_from_biguint_binary() {
        let input = vec![vec![2, 2], vec![2, 2]];
        let bin = |v: Vec<usize>| vec![vec![vec![v[0]], vec![v[1]]], vec![vec![v[2]], vec![v[3]]]];

        assert_eq!(digits(0, &input), Some(bin(vec![0, 0, 0, 0])));
        assert_eq!(digits(1, &input), Some(bin(vec![0, 0, 0, 1])));
        assert_eq!(digits(2, &input), Some(bin(vec![0, 0, 1, 0])));
        assert_eq!(digits(3, &input), Some(bin(vec![0, 0, 1, 1])));
        assert_eq!(digits(4, &input), Some(bin(vec![0, 1, 0, 0])));
        assert_eq!(digits(5, &input), Some(bin(vec![0, 1, 0, 1])));
        assert_eq!(digits(6, &input), Some(bin(vec![0, 1, 1, 0])));
        assert_eq!(digits(7, &input), Some(bin(vec![0, 1, 1, 1])));
        assert_eq!(digits(15, &input), Some(bin(vec![1, 1, 1, 1])));
    }

    #[test]
    fn test_from_biguint() {
        let input = vec![vec![3, 3], vec![3, 3], vec![3, 3]];

        assert_eq!(
            digits(35, &input),
            Some(vec![
                vec![vec![0, 0], vec![0, 0]],
                vec![vec![0, 0], vec![0, 0]],
                vec![vec![2, 1], vec![2, 1]]
            ])
        );

        assert_eq!(
            digits(36, &input),
            Some(vec![
                vec![vec![0, 0], vec![0, 0]],
                vec![vec![0, 0], vec![0, 1]],
                vec![vec![0, 0], vec![0, 0]]
            ])
        );

        assert_eq!(
            digits(1295, &input),
            Some(vec![
                vec![vec![0, 0], vec![0, 0]],
                vec![vec![2, 1], vec![2, 1]],
                vec![vec![2, 1], vec![2, 1]]
            ])
        );

        assert_eq!(
            digits(1296, &input),
            Some(vec![
                vec![vec![0, 0], vec![0, 1]],
                vec![vec![0, 0], vec![0, 0]],
                vec![vec![0, 0], vec![0, 0]]
            ])
        );

        assert_eq!(
            digits(46655, &input),
            Some(vec![
                vec![vec![2, 1], vec![2, 1]],
                vec![vec![2, 1], vec![2, 1]],
                vec![vec![2, 1], vec![2, 1]]
            ])
        );

        assert_eq!(digits(46656, &input), None);
    }

    #[test]
    fn test_to_from_biguint() {
        let input = vec![vec![3, 3], vec![3, 3]];
        for i in 1..1296u32 {
            let n = BigUint::from(i);
            let ns =
                NS2::try_from_input(n.clone(), &input).expect(&format!("Expected value for {n:}"));
            let v = BigUint::from(ns);
            assert_eq!(n, v);
        }
    }

    #[test]
    fn test_permute_values() {
        let mut buf = vec![vec![0, 5, 10, 15, 20, 25], vec![0, 2, 4, 6]];
        let input = vec![vec![3, 3], vec![2, 2]];
        let ns = n(103, &input).unwrap();

        ns.permute_values(&mut buf);
        assert_eq!(buf, vec![vec![10, 0, 5, 15, 25, 20], vec![2, 0, 6, 4]]);
        ns.permute_values(&mut buf);
        assert_eq!(buf, vec![vec![10, 0, 5, 15, 25, 20], vec![2, 0, 6, 4]]);
    }

    #[test]
    fn test_read_values() {
        let buf = vec![vec![10, 0, 5, 15, 25, 20], vec![2, 0, 6, 4]];
        let input = vec![vec![3, 3], vec![2, 2]];
        assert_eq!(big(NS2::read_values(&input, &buf)), 103);
    }
}
