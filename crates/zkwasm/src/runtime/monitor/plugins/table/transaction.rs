use std::collections::HashMap;
use std::usize;

use specs::etable::EventTableEntry;
use specs::slice_backend::SliceBackendBuilder;
use specs::step::StepInfo;

use crate::runtime::monitor::plugins::table::Event;

use super::slice_builder::SliceBuilder;
use super::Command;
use super::FlushStrategy;

pub(crate) type TransactionId = usize;

struct Checkpoint {
    // transaction start index
    start: usize,
}

struct SafelyAbortPosition {
    capacity: u32,
    cursor: Option<usize>,
}

impl SafelyAbortPosition {
    fn new(capacity: u32) -> Self {
        Self {
            capacity,
            cursor: None,
        }
    }

    fn update(&mut self, len: usize) {
        if let Some(cursor) = self.cursor.as_ref() {
            assert!(len >= *cursor);
        }

        self.cursor = Some(len);
    }

    fn reset(&mut self) {
        self.cursor = None;
    }

    fn finalize(&self) -> usize {
        self.cursor.unwrap_or(self.capacity as usize)
    }
}

pub(super) struct HostTransaction<B: SliceBackendBuilder> {
    slice_backend_builder: B,
    slices: Vec<B::Output>,
    capacity: u32,

    safely_abort_position: SafelyAbortPosition,
    lazy_committed: HashMap<TransactionId, usize>,
    logs: Vec<EventTableEntry>,
    started: HashMap<TransactionId, Checkpoint>,
    controller: Box<dyn FlushStrategy>,
    host_is_full: bool,

    pub(crate) slice_builder: SliceBuilder,
}

impl<B: SliceBackendBuilder> HostTransaction<B> {
    pub(super) fn new(
        capacity: u32,
        slice_backend_builder: B,
        controller: Box<dyn FlushStrategy>,
    ) -> Self {
        Self {
            slice_backend_builder,
            slices: Vec::new(),
            slice_builder: SliceBuilder::new(),
            capacity,

            safely_abort_position: SafelyAbortPosition::new(capacity),
            lazy_committed: HashMap::new(),
            logs: Vec::new(),
            started: HashMap::new(),
            controller,
            host_is_full: false,
        }
    }

    fn now(&self) -> usize {
        self.logs.len()
    }

    pub(super) fn len(&self) -> usize {
        self.logs.len()
    }

    fn in_transaction(&self) -> bool {
        !self.started.is_empty() || !self.lazy_committed.is_empty()
    }

    // begin the transaction
    fn start(&mut self, idx: TransactionId) {
        if self.started.contains_key(&idx) {
            panic!("transaction id exists")
        }

        let checkpoint = Checkpoint { start: self.now() };

        if !self.in_transaction() {
            self.safely_abort_position.update(checkpoint.start);
        }

        self.started.insert(idx, checkpoint);
    }

    fn commit(&mut self, idx: TransactionId, lazy: bool) {
        let checkpoint = self.started.remove(&idx).unwrap();

        let now = self.now();

        // If there exists a same plugin transaciton lazied,
        // we can safely commit the previous transaction
        self.lazy_committed.remove(&idx);

        if !self.in_transaction() {
            self.safely_abort_position
                .update(if lazy { checkpoint.start } else { now });
        }

        if lazy {
            self.lazy_committed.insert(idx, now);
        }
    }

    fn abort(&mut self) {
        if self.len() == 0 {
            return;
        }

        if !self.in_transaction() {
            let now = self.now();
            self.safely_abort_position.update(now);
        }

        let rollback = self.safely_abort_position.finalize();
        let mut logs = std::mem::take(&mut self.logs);

        {
            let committed_logs = logs.drain(0..rollback);

            let slice = self.slice_builder.build(committed_logs.collect());
            self.slices.push(self.slice_backend_builder.build(slice));
        }

        {
            self.host_is_full = false;
            self.safely_abort_position.reset();
            self.lazy_committed.clear();
            self.started.clear();
        }

        // controller should be reset and we will replay the remaining logs
        {
            let command = self.controller.notify(Event::Reset);
            assert!(command == Command::Noop);
            self.replay(logs);
        }
    }

    pub(super) fn finalized(mut self) -> Vec<B::Output> {
        self.abort();

        self.slices
    }
}

impl<B: SliceBackendBuilder> HostTransaction<B> {
    fn replay(&mut self, logs: Vec<EventTableEntry>) {
        for log in logs {
            self.insert(log);
        }
    }

    pub(crate) fn insert(&mut self, log: EventTableEntry) {
        if self.logs.len() == self.capacity as usize {
            self.abort();
        }

        let command = match log.step_info {
            StepInfo::ExternalHostCall { op, .. } => {
                if self.host_is_full {
                    self.abort();
                }

                self.controller.notify(Event::HostCall(op))
            }
            _ => Command::Noop,
        };

        match command {
            Command::Noop => {
                self.logs.push(log);
            }
            Command::Start(id) => {
                self.start(id);
                self.logs.push(log);
            }
            Command::Commit(id, lazy) => {
                self.logs.push(log);
                self.commit(id, lazy);
            }
            Command::Abort => {
                self.insert(log);
                self.host_is_full = true;
            }
            Command::CommitAndAbort(id, lazy) => {
                self.logs.push(log);
                self.commit(id, lazy);
                self.host_is_full = true;
            }
        }
    }
}
