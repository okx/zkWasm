use serde::Deserialize;
use serde::Serialize;

use crate::host_function::Signature;
use crate::types::ValueType;

pub mod encode;
mod table;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
pub enum ExternalHostCallSignature {
    Argument,
    Return,
}

impl ExternalHostCallSignature {
    pub fn is_ret(&self) -> bool {
        *self == ExternalHostCallSignature::Return
    }
}

impl From<ExternalHostCallSignature> for Signature {
    fn from(sig: ExternalHostCallSignature) -> Signature {
        match sig {
            ExternalHostCallSignature::Argument => Signature {
                params: vec![ValueType::I64],
                return_type: None,
            },
            ExternalHostCallSignature::Return => Signature {
                params: vec![],
                return_type: Some(ValueType::I64),
            },
        }
    }
}

#[derive(Clone,Serialize, Deserialize, Debug)]
pub struct ExternalHostCallEntry {
    pub op: usize,
    pub value: u64,
    #[serde(rename = "is_ret", with = "serde_sig")]
    pub sig: ExternalHostCallSignature,
}

mod serde_sig {
    use super::ExternalHostCallSignature;
    use serde::Deserialize;
    use serde::Deserializer;
    use serde::Serializer;

    pub fn serialize<S>(sig: &ExternalHostCallSignature, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bool(sig.is_ret())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<ExternalHostCallSignature, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = bool::deserialize(deserializer)?;
        if s {
            Ok(ExternalHostCallSignature::Return)
        } else {
            Ok(ExternalHostCallSignature::Argument)
        }
    }
}

#[derive(Clone,Default, Serialize, Deserialize,Debug)]
pub struct ExternalHostCallTable(pub(crate) Vec<ExternalHostCallEntry>);

impl ExternalHostCallTable {
    pub fn new(entries: Vec<ExternalHostCallEntry>) -> Self {
        Self(entries)
    }

    pub fn entries(&self) -> &Vec<ExternalHostCallEntry> {
        &self.0
    }

    pub fn push(&mut self, entry: ExternalHostCallEntry) {
        self.0.push(entry);
    }
}
