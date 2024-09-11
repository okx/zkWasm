#![feature(trait_alias)]
#![deny(warnings)]
#![allow(
    clippy::assertions_on_constants,
    clippy::too_many_arguments,
    clippy::type_complexity
)]

use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;

use brtable::BrTable;
use brtable::ElemTable;
use configure_table::ConfigureTable;
use imtable::InitMemoryTable;
use itable::InstructionTable;
use jtable::InheritedFrameTable;
use slice_backend::SliceBackend;
use state::InitializationState;

#[macro_use]
extern crate lazy_static;

pub mod args;
pub mod brtable;
pub mod configure_table;
pub mod encode;
pub mod etable;
pub mod external_host_call_table;
pub mod host_function;
pub mod imtable;
pub mod itable;
pub mod jtable;
pub mod mtable;
pub mod slice;
pub mod slice_backend;
pub mod state;
pub mod step;
pub mod types;

#[derive(Debug)]
pub struct CompilationTable {
    pub itable: Arc<InstructionTable>,
    pub imtable: Arc<InitMemoryTable>,
    pub br_table: Arc<BrTable>,
    pub elem_table: Arc<ElemTable>,
    pub configure_table: Arc<ConfigureTable>,
    pub initial_frame_table: Arc<InheritedFrameTable>,
    pub initialization_state: Arc<InitializationState<u32>>,
}

pub struct ExecutionTable<B: SliceBackend> {
    pub slice_backend: B,
    pub context_input_table: Vec<u64>,
    pub context_output_table: Vec<u64>,
}

pub struct Tables<B: SliceBackend> {
    pub compilation_tables: CompilationTable,
    pub execution_tables: ExecutionTable<B>,
}

impl<B: SliceBackend> Tables<B> {
    pub fn write(
        &self,
        dir: &Path,
        name_of_frame_table_slice: impl Fn(usize) -> String,
        name_of_event_table_slice: impl Fn(usize) -> String,
        name_of_external_host_call_table_slice: impl Fn(usize) -> String,
    ) {
        const DEBUG: bool = false;

        fn write_file(folder: &Path, filename: String, buf: &String) {
            let folder = folder.join(filename);
            let mut fd = File::create(folder.as_path()).unwrap();

            fd.write_all(buf.as_bytes()).unwrap();
        }

        write_file(
            dir,
            "itable.json".to_string(),
            &serde_json::to_string_pretty(&self.compilation_tables.itable).unwrap(),
        );

        self.execution_tables
            .slice_backend
            .iter()
            .enumerate()
            .for_each(|(index, slice)| {
                if DEBUG {
                    let path = dir.join(name_of_event_table_slice(index));
                    slice.etable.write(&path).unwrap();

                    let path = dir.join(name_of_frame_table_slice(index));
                    slice.frame_table.write(&path).unwrap();
                }

                {
                    let path = dir.join(name_of_external_host_call_table_slice(index));
                    slice.external_host_call_table.write(&path).unwrap();
                }
            })
    }
}
