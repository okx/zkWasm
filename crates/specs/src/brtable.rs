use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct BrTableEntry {
    pub fid: u32,
    pub iid: u32,
    pub index: u32,
    pub drop: u32,
    pub keep: u32,
    pub dst_pc: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct BrTable(Vec<BrTableEntry>);

impl BrTable {
    pub fn new(entries: Vec<BrTableEntry>) -> Self {
        BrTable(entries)
    }

    pub fn entries(&self) -> &Vec<BrTableEntry> {
        &self.0
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ElemEntry {
    pub table_idx: u32,
    pub type_idx: u32,
    pub offset: u32,
    pub func_idx: u32,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct ElemTable(Vec<ElemEntry>);

impl ElemTable {
    // pub fn insert(&mut self, entry: ElemEntry) {
    //     self.0.insert((entry.table_idx, entry.offset), entry);
    // }
    pub fn new(entries: Vec<ElemEntry>) -> Self {
        ElemTable(entries)
    }

    pub fn entries(&self) -> &Vec<ElemEntry> {
        &self.0
    }
}

pub enum IndirectClass {
    BrTable,
    CallIndirect,
}
