use std::collections::HashMap;
use containers::{
    Bytes32, Uint64, ValidatorIndex, Slot, Block, BlockBody, 
    SignedVote, Vote, Checkpoint, State
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

// Fork choice specific types
pub type Interval = u64;
pub type Root = Bytes32;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForkChoiceConfig {
    pub num_validators: Uint64,
    pub genesis_time: Uint64,
    pub seconds_per_slot: Uint64,
    pub intervals_per_slot: Uint64,
}

impl Default for ForkChoiceConfig {
    fn default() -> Self {
        Self {
            num_validators: Uint64(0),
            genesis_time: Uint64(0),
            seconds_per_slot: Uint64(12),
            intervals_per_slot: Uint64(3),
        }
    }
}

// Error types for fork choice operations
#[derive(Error, Debug)]
pub enum ForkChoiceError {
    #[error("Parent state not found for block")]
    ParentStateNotFound,
    #[error("Vote slot {0} is in the future")]
    VoteInFuture(u64),
    #[error("Block already processed")]
    BlockAlreadyProcessed,
    #[error("Invalid vote: {0}")]
    InvalidVote(String),
    #[error("Store not initialized properly")]
    StoreNotInitialized,
}

pub type ForkChoiceResult<T> = Result<T, ForkChoiceError>;

// Re-export container types for convenience (avoid duplicates)
pub use containers::{
    BlockHeader, SignedBlock
};
