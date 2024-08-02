use std::collections::HashMap;
use std::fmt;
use serde::de::{MapAccess, Visitor};
use std::collections::BTreeMap;

use crate::mtable::LocationType;
use crate::mtable::VarType;
use serde::Deserialize;
use serde::ser::SerializeMap;
use serde::Serialize;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InitMemoryTableEntry {
    pub ltype: LocationType,
    pub is_mutable: bool,
    pub offset: u32,
    pub vtype: VarType,
    /// convert from [u8; 8] via u64::from_le_bytes
    pub value: u64,
    pub eid: u32,
}

#[derive(Default, Debug)]
pub struct InitMemoryTable(pub BTreeMap<(LocationType, u32), InitMemoryTableEntry>);

impl InitMemoryTable {
    pub fn new(entries: Vec<InitMemoryTableEntry>) -> Self {
        let mut map = BTreeMap::new();

        entries.into_iter().for_each(|entry| {
            map.insert((entry.ltype, entry.offset), entry);
        });

        Self(map)
    }

    pub fn try_find(&self, ltype: LocationType, offset: u32) -> Option<&InitMemoryTableEntry> {
        self.0.get(&(ltype, offset))
    }
}



impl Serialize for InitMemoryTable {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (k, v) in &self.0 {
            map.serialize_entry(&k, v)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for InitMemoryTable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct InitMemoryTableVisitor;

        impl<'de> Visitor<'de> for InitMemoryTableVisitor {
            type Value = InitMemoryTable;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a map of (LocationType, u32) to InitMemoryTableEntry")
            }

            fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut map = HashMap::with_capacity(access.size_hint().unwrap_or(0));

                while let Some((key, value)) = access.next_entry()? {
                    map.insert(key, value);
                }

                Ok(InitMemoryTable(map))
            }
        }

        deserializer.deserialize_map(InitMemoryTableVisitor)
    }
}

