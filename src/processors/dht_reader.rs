use anyhow::Result;

use crate::jpeg::{
    segments::{DhtData, HuffmanTableData},
    Jpeg, Marker, ProcessSegment, Segment,
};

pub struct DhtReader<F> {
    callback: F,
}

impl<F> DhtReader<F> {
    pub fn new(callback: F) -> Self {
        Self { callback }
    }
}

impl<F: Fn(&HuffmanTableData)> ProcessSegment for DhtReader<F> {
    fn process_segment(&self, _: &Jpeg, segment: &Segment) -> Result<()> {
        match segment.marker {
            Marker::DHT => {
                let dht_data = DhtData::try_from(&segment.data[..])?;
                dht_data
                    .tables
                    .iter()
                    .for_each(|table| (self.callback)(table));
            }
            _ => {}
        }

        Ok(())
    }
}
