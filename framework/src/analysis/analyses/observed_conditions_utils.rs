use fixedbitset::FixedBitSet;
use serde::ser::SerializeTuple;
use serde::{Deserialize, Serialize, Serializer};
use std::convert::TryFrom;

#[derive(Debug, Deserialize)]
pub struct ConditionRecord {
    condition_id: u64,
    cases: String,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize)]
pub struct Condition {
    id: u64,
    #[serde(serialize_with = "serialize_fixedbitset")]
    observed_states: FixedBitSet,
}

fn serialize_fixedbitset<S>(bitset: &FixedBitSet, ser: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut tup = ser.serialize_tuple(2)?;
    tup.serialize_element(&bitset.len())?;
    tup.serialize_element(bitset.as_slice())?;
    tup.end()
}

impl Condition {
    pub fn get_id(&self) -> u64 {
        self.id
    }

    pub fn update_record(&mut self, update: Condition) {
        assert_eq!(self.id, update.id);
        assert_eq!(
            self.observed_states.len(),
            update.observed_states.len(),
            "Number of branches for condition {} changed: {} != {}",
            self.id,
            self.observed_states.len(),
            update.observed_states.len()
        );

        self.observed_states |= update.observed_states
    }

    pub fn is_unsolved(&self) -> bool {
        self.observed_states.count_ones(..) < self.observed_states.len()
    }
}

#[cfg(test)]
impl Condition {
    pub fn new(id: u64, observed_states: FixedBitSet) -> Self {
        Self {
            id,
            observed_states,
        }
    }
}

impl TryFrom<ConditionRecord> for Condition {
    type Error = &'static str;

    fn try_from(condition_record: ConditionRecord) -> Result<Self, Self::Error> {
        let mut observed_states = FixedBitSet::with_capacity(condition_record.cases.len());
        for (idx, observed_char) in condition_record.cases.chars().enumerate() {
            match observed_char {
                '1' => observed_states.insert(idx),
                '0' => {}
                _ => return Err("Unrecognized input byte!"),
            }
        }

        Ok(Self {
            id: condition_record.condition_id,
            observed_states,
        })
    }
}
