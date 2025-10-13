use crate::{Bytes32, Slot};
use ssz_derive::Ssz;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Ssz, Default, Serialize, Deserialize)]
pub struct Checkpoint {
    pub root: Bytes32,
    pub slot: Slot,
}