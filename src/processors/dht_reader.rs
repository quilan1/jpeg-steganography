use anyhow::Result;
use num_bigint::BigUint;

use crate::{
    factorial_number_system::SFNS,
    jpeg::{segments::DhtData, Jpeg, Marker, ProcessSegment, Segment},
};

pub struct DhtReader<F> {
    callback: F,
}

impl<F> DhtReader<F> {
    pub fn new(callback: F) -> Self {
        Self { callback }
    }
}

impl<F: Fn(usize, usize, BigUint, Vec<u8>)> ProcessSegment for DhtReader<F> {
    fn process_segment(&self, _: &Jpeg, segment: &Segment) -> Result<()> {
        match segment.marker {
            Marker::DHT => {
                let dht_data = DhtData::try_from(&segment.data[..])?;
                for table in dht_data.tables {
                    let max_message = SFNS::max_message(&table.sizes);
                    let bytes = SFNS::from_size_values(&table.sizes, &table.values);
                    (self.callback)(table.table_class, table.table_index, max_message, bytes);
                }
            }
            _ => {}
        }

        Ok(())
    }
}
