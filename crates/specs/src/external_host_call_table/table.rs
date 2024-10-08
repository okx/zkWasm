use super::ExternalHostCallEntry;
use crate::etable::EventTableEntry;
use crate::step::StepInfo;

impl TryFrom<&EventTableEntry> for ExternalHostCallEntry {
    type Error = ();

    fn try_from(entry: &EventTableEntry) -> Result<Self, Self::Error> {
        match &entry.step_info {
            StepInfo::ExternalHostCall { op, value, sig, .. } => Ok(ExternalHostCallEntry {
                eid: entry.eid as usize,
                op: *op,
                value: value.unwrap(),
                is_ret: sig.is_ret(),
            }),
            _ => Err(()),
        }
    }
}
