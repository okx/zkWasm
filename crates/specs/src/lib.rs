#![feature(trait_alias)]
#![deny(warnings)]
#![allow(
    clippy::assertions_on_constants,
    clippy::too_many_arguments,
    clippy::type_complexity
)]

use std::path::Path;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use brtable::BrTable;
use brtable::ElemTable;
use configure_table::ConfigureTable;
use etable::EventTable;
use imtable::InitMemoryTable;
use itable::InstructionTable;
use jtable::FrameTable;
use jtable::InheritedFrameTable;
use state::InitializationState;

use crate::external_host_call_table::ExternalHostCallTable;

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
pub mod state;
pub mod step;
pub mod types;

#[derive(Clone,Debug,Serialize, Deserialize)]
pub struct CompilationTable {
    pub itable: Arc<InstructionTable>,
    pub imtable: Arc<InitMemoryTable>,
    pub br_table: Arc<BrTable>,
    pub elem_table: Arc<ElemTable>,
    pub configure_table: Arc<ConfigureTable>,
    pub initial_frame_table: Arc<InheritedFrameTable>,
    pub initialization_state: Arc<InitializationState<u32>>,
}

#[derive(Clone,Default,Serialize, Deserialize)]
pub struct ExecutionTable {
    pub etable: Vec<EventTable>,
    pub frame_table: Vec<FrameTable>,
    pub external_host_call_table: ExternalHostCallTable,
    pub context_input_table: Vec<u64>,
    pub context_output_table: Vec<u64>,
}

#[derive(Clone,Serialize, Deserialize)]
pub struct Tables {
    pub compilation_tables: CompilationTable,
    pub execution_tables: ExecutionTable,
}

impl Tables {
    pub fn write(&self, _dir: &Path, _name_of_frame_table_slice: impl Fn(usize) -> String) {
        panic!("modify by scf")
    }
}
