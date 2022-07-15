#[macro_use]
mod traits;
mod ns0;
mod ns1;
mod ns2;

use ns0::NS0;
use ns1::NS1;
pub use ns2::NS2;
pub use traits::{MaxBaseValue, TryFromInput};

fn factorial(mut n: usize) -> num_bigint::BigUint {
    use num_traits::One;
    let mut result = num_bigint::BigUint::one();

    while n > 1 {
        result *= num_bigint::BigUint::from(n);
        n -= 1;
    }

    result
}
