use std::collections::VecDeque;
use std::path::PathBuf;

use specs::etable::EventTable;
use specs::external_host_call_table::ExternalHostCallTable;
use specs::jtable::FrameTable;
use specs::slice_backend::Slice;
use specs::slice_backend::SliceBackend;

use crate::names::name_of_etable_slice;
use crate::names::name_of_external_host_call_table_slice;
use crate::names::name_of_frame_table_slice;

struct SlicePath {
    event_table: PathBuf,
    frame_table: PathBuf,
    external_host_call_table: PathBuf,
}

impl Into<Slice> for &SlicePath {
    fn into(self) -> Slice {
        Slice {
            etable: EventTable::read(&self.event_table).unwrap(),
            frame_table: FrameTable::read(&self.frame_table).unwrap(),
            external_host_call_table: ExternalHostCallTable::read(&self.external_host_call_table)
                .unwrap(),
        }
    }
}

pub(crate) struct FileBackend {
    dir_path: PathBuf,
    name: String,
    slices: VecDeque<SlicePath>,
}

impl FileBackend {
    pub(crate) fn new(name: String, dir_path: PathBuf) -> Self {
        FileBackend {
            dir_path,
            name,
            slices: VecDeque::new(),
        }
    }
}

// impl Iterator for FileBackend {
//     type Item = Slice;
//
//     fn next(&mut self) -> Option<Self::Item> {
//         self.slices.pop_front().map(|slice| (&slice).into())
//     }
// }

impl IntoIterator for FileBackend {
    type Item = Slice;
    type IntoIter = Box<dyn Iterator<Item = Slice>>;

    fn into_iter(self) -> Self::IntoIter {
        Box::new(self.slices.into_iter().map(|slice| (&slice).into()))
    }
}

impl SliceBackend for FileBackend {
    fn push(&mut self, slice: Slice) {
        let index = self.slices.len();

        let event_table = {
            let path = self
                .dir_path
                .join(PathBuf::from(name_of_etable_slice(&self.name, index)));
            slice.etable.write(&path).unwrap();
            path
        };

        let frame_table = {
            let path = self
                .dir_path
                .join(PathBuf::from(name_of_frame_table_slice(&self.name, index)));
            slice.frame_table.write(&path).unwrap();
            path
        };

        let external_host_call_table = {
            let path = self
                .dir_path
                .join(PathBuf::from(name_of_external_host_call_table_slice(
                    &self.name, index,
                )));
            slice.external_host_call_table.write(&path).unwrap();
            path
        };

        self.slices.push_back(SlicePath {
            event_table,
            frame_table,
            external_host_call_table,
        });
    }

    fn len(&self) -> usize {
        self.slices.len()
    }

    fn is_empty(&self) -> bool {
        self.slices.is_empty()
    }

    fn iter(&self) -> Box<dyn Iterator<Item = Slice> + '_> {
        Box::new(self.slices.iter().map(|slice| (slice).into()))
    }
}
