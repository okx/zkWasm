use super::Slice;
use super::SliceBackend;
use std::collections::VecDeque;

#[derive(Default)]
pub struct InMemoryBackend {
    slices: VecDeque<Slice>,
}

// impl Iterator for InMemoryBackend {
//     type Item = Slice;
//
//     fn next(&mut self) -> Option<Self::Item> {
//         self.slices.pop_front()
//     }
// }

impl IntoIterator for InMemoryBackend {
    type Item = Slice;
    type IntoIter = Box<dyn Iterator<Item = Slice>>;

    fn into_iter(self) -> Self::IntoIter {
        let i = self.slices.into_iter().map(|slice| slice.clone());

        Box::new(i)
    }
}

impl SliceBackend for InMemoryBackend {
    fn push(&mut self, slice: Slice) {
        self.slices.push_back(slice)
    }

    fn len(&self) -> usize {
        self.slices.len()
    }

    fn is_empty(&self) -> bool {
        self.slices.is_empty()
    }

    fn for_each1<'a>(&'a self, f: Box<dyn Fn((usize, &Slice)) + 'a>) {
        self.slices.iter().enumerate().for_each(f)
    }

    fn iter(&self) -> Box<dyn Iterator<Item = Slice> + '_> {
        let i = self.slices.iter().map(|slice| slice.clone());

        Box::new(i)
    }
}
