use num_bigint::BigUint;
use num_traits::One;

pub trait Digits<T> {
    fn digits(&self) -> &Vec<T>;
}

pub trait InnerDigits<T> {
    fn inner_digits(&self) -> Vec<T>;
}

/////////////////////////////////////////////

pub trait DigitsBases<T>
where
    Self: Sized + Bases + Digits<T> + ValidInputs<Vec<T>>,
    T: Clone + MaxBaseValue,
{
    fn digits_bases(&self) -> Vec<(T, BigUint)> {
        let mut results = Vec::new();
        let bases = self.bases();
        let digits = self.digits();

        let rev_bases = bases.into_iter().rev();
        let rev_digits = digits.into_iter().rev();

        for (digit, base) in rev_digits.zip(rev_bases) {
            results.push((digit.clone(), base));
        }

        results.reverse();
        results
    }
}

/////////////////////////////////////////////

macro_rules! biguint_from {
    ($class:tt) => {
        impl From<$class> for BigUint {
            fn from(v: $class) -> BigUint {
                let mut result = BigUint::zero();
                for (digit, base) in v.digits_bases() {
                    result += BigUint::from(digit) * &base;
                }
                result
            }
        }
    };
}

/////////////////////////////////////////////

macro_rules! impl_base_ns {
    ($struct:tt, $digit:tt) => {
        #[derive(Clone, Debug)]
        pub struct $struct {
            pub digits: Vec<$digit>,
        }

        impl From<Vec<$digit>> for $struct {
            fn from(digits: Vec<$digit>) -> Self {
                Self { digits }
            }
        }

        impl super::traits::DigitsBases<$digit> for $struct {}

        impl super::traits::Digits<$digit> for $struct {
            fn digits(&self) -> &Vec<$digit> {
                &self.digits
            }
        }

        impl super::traits::ValidInputs<Vec<$digit>> for $struct {
            fn valid(&self) -> Vec<$digit> {
                self.digits.clone()
            }
        }

        biguint_from!($struct);
    };
}

macro_rules! impl_sub_ns {
    ($struct:tt, $input:ty, $inner_digit:ty) => {
        impl super::traits::Bases for $struct {
            fn bases<U>(&self) -> Vec<BigUint> {
                super::traits::get_bases(self)
            }
        }

        impl super::traits::MaxBaseValue for $struct {
            fn max_base_value(&self) -> BigUint {
                super::traits::base_info(self).0
            }
        }

        impl super::traits::InnerDigits<$inner_digit> for $struct {
            fn inner_digits(&self) -> Vec<$inner_digit> {
                self.digits.iter().map(|d| d.inner_digits()).collect()
            }
        }

        impl super::traits::TryFromInput<$input> for $struct {
            fn try_from_input(value: BigUint, input: &$input) -> Option<Self> {
                super::traits::try_from_input(value, input)
            }
        }
    };
}

/////////////////////////////////////////////

pub trait MaxBaseValue {
    fn max_base_value(&self) -> BigUint;
}

impl MaxBaseValue for usize {
    fn max_base_value(&self) -> BigUint {
        super::factorial(*self)
    }
}

impl MaxBaseValue for u8 {
    fn max_base_value(&self) -> BigUint {
        super::factorial(*self as usize)
    }
}

macro_rules! impl_mbv {
    ($struct:ty) => {
        impl MaxBaseValue for $struct {
            fn max_base_value(&self) -> BigUint {
                base_info(self).0
            }
        }
    };
}

impl_mbv!(Vec<u8>);
impl_mbv!(Vec<usize>);
impl_mbv!(Vec<Vec<u8>>);
impl_mbv!(Vec<Vec<usize>>);

/////////////////////////////////////////////

pub trait Bases {
    fn bases<U>(&self) -> Vec<BigUint>
    where
        Self: Sized + ValidInputs<Vec<U>>,
        U: MaxBaseValue + Clone,
    {
        get_bases(self)
    }
}

impl Bases for usize {
    fn bases<U>(&self) -> Vec<BigUint> {
        (1..*self).rev().map(|v| super::factorial(v)).collect()
    }
}

impl Bases for Vec<usize> {}
impl Bases for Vec<Vec<usize>> {}

/////////////////////////////////////////////

pub trait ValueBases {
    fn value_bases<U>(&self) -> Vec<(U, BigUint)>
    where
        Self: Sized + ValidInputs<Vec<U>>,
        U: MaxBaseValue + Clone,
    {
        base_info(self).1
    }
}

impl ValueBases for Vec<u8> {}
impl ValueBases for Vec<Vec<u8>> {}

/////////////////////////////////////////////

pub fn base_info<T, U>(input: &T) -> (BigUint, Vec<(U, BigUint)>)
where
    T: ValidInputs<Vec<U>>,
    U: MaxBaseValue + Clone,
{
    let mut max_base = BigUint::one();
    let mut bases = Vec::new();
    for input in input.valid().into_iter().rev() {
        bases.push((input.clone(), max_base.clone()));
        max_base *= input.max_base_value();
    }
    bases.reverse();
    (max_base, bases)
}

pub fn get_bases<T, U>(input: &T) -> Vec<BigUint>
where
    T: ValidInputs<Vec<U>>,
    U: MaxBaseValue + Clone,
{
    base_info(input).1.into_iter().map(|v| v.1).collect()
}

/////////////////////////////////////////////

pub trait ValidInputs<O> {
    fn valid(&self) -> O;
}

impl ValidInputs<Vec<u8>> for u8 {
    fn valid(&self) -> Vec<u8> {
        (1..=*self).rev().collect()
    }
}

impl ValidInputs<Vec<usize>> for usize {
    fn valid(&self) -> Vec<usize> {
        (1..=*self).rev().collect()
    }
}

macro_rules! impl_valid_inputs {
    ($struct:ty) => {
        impl ValidInputs<Self> for $struct {
            fn valid(&self) -> Self {
                self.into_iter()
                    .cloned()
                    .filter(|v| !v.valid().is_empty())
                    .collect()
            }
        }
    };
}

impl_valid_inputs!(Vec<u8>);
impl_valid_inputs!(Vec<usize>);
impl_valid_inputs!(Vec<Vec<u8>>);
impl_valid_inputs!(Vec<Vec<usize>>);

/////////////////////////////////////////////

pub trait TryFromInput<T> {
    fn try_from_input(value: BigUint, input: &T) -> Option<Self>
    where
        Self: Sized;
}

pub fn try_from_input<Parent, Child, Input, U>(value: BigUint, input: &Input) -> Option<Parent>
where
    Parent: From<Vec<Child>>,
    Input: ValidInputs<Vec<U>> + ValueBases + MaxBaseValue,
    Child: TryFromInput<U>,
    U: MaxBaseValue + Clone,
{
    if value >= input.max_base_value() {
        return None;
    }

    let mut value = value;
    let mut digits = Vec::new();
    let bases = input.value_bases();
    for (digit_value, base) in bases {
        let digit = &value / &base;
        value -= &digit * &base;

        let digit = Child::try_from_input(digit, &digit_value).unwrap();
        digits.push(digit);
    }

    Some(Parent::from(digits))
}
