mod entropy_stream;
#[allow(clippy::module_inception)]
mod jpeg;
mod marker;
pub mod segments;

pub use entropy_stream::process_entropy_stream;
pub use jpeg::{Jpeg, ProcessSegment, ProcessSegmentMut, Segment};
pub use marker::Marker;
