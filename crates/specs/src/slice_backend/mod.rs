use crate::etable::EventTable;
use crate::external_host_call_table::ExternalHostCallTable;
use crate::jtable::FrameTable;

pub mod memory;

#[derive(Clone)]
pub struct Slice {
    pub etable: EventTable,
    pub frame_table: FrameTable,
    pub external_host_call_table: ExternalHostCallTable,
}

pub trait SliceBackend:
    IntoIterator<Item = Slice, IntoIter = Box<dyn Iterator<Item = Slice>>>
{
    fn push(&mut self, slice: Slice);
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;

    fn iter(&self) -> Box<dyn Iterator<Item = Slice> + '_>;
}
