mod entropy_stream;
mod jpeg;
mod marker;
pub mod segments;

pub use entropy_stream::process_entropy_stream;
pub use jpeg::{Jpeg, ProcessSegment, Segment};
pub use marker::Marker;
