use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use containers::{Bytes32, Block, State, Checkpoint, ValidatorIndex, Slot};
use containers::config::Config as ContainerConfig;
use containers::block::hash_tree_root;
use crate::helpers::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ValidatorId(pub ValidatorIndex);

impl Hash for ValidatorId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.0.hash(state);
    }
}

impl From<ValidatorIndex> for ValidatorId {
    fn from(vi: ValidatorIndex) -> Self {
        ValidatorId(vi)
    }
}

impl From<ValidatorId> for ValidatorIndex {
    fn from(vid: ValidatorId) -> Self {
        vid.0
    }
}

#[derive(Clone, Debug)]
pub struct ForkChoiceConfig {
    pub genesis_time: u64,
    pub num_validators: u64,
    pub seconds_per_slot: u64,
    pub intervals_per_slot: u64,
}

impl From<&ContainerConfig> for ForkChoiceConfig {
    fn from(config: &ContainerConfig) -> Self {
        ForkChoiceConfig {
            genesis_time: config.genesis_time,
            num_validators: config.num_validators,
            seconds_per_slot: chain::config::SECONDS_PER_SLOT,
            intervals_per_slot: chain::config::INTERVALS_PER_SLOT,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Store {
    pub time: u64,
    pub config: ForkChoiceConfig,
    pub head: Bytes32,
    pub safe_target: Bytes32,
    pub latest_justified: Checkpoint,
    pub latest_finalized: Checkpoint,
    pub blocks: HashMap<Bytes32, Block>,
    pub states: HashMap<Bytes32, State>,
    pub latest_known_votes: HashMap<ValidatorId, Checkpoint>,
    pub latest_new_votes: HashMap<ValidatorId, Checkpoint>,
}

impl Store {
    /// Create a new fork choice store anchored at the provided block and state.
    pub fn new(anchor_state: State, anchor_block: Block, config: ContainerConfig) -> Self {
        let block_root = hash_tree_root(&anchor_block);
        let fork_choice_config = ForkChoiceConfig::from(&config);
        let time = anchor_block.slot.0 * fork_choice_config.intervals_per_slot;

        Store {
            time,
            config: fork_choice_config,
            head: block_root,
            safe_target: block_root,
            latest_justified: anchor_state.latest_justified.clone(),
            latest_finalized: anchor_state.latest_finalized.clone(),
            blocks: [(block_root, anchor_block)].into(),
            states: [(block_root, anchor_state)].into(),
            latest_known_votes: HashMap::new(),
            latest_new_votes: HashMap::new(),
        }
    }

    /// Advance the store to produce a proposal head for the requested slot.
    /// This advances the internal time to the start of `slot`, runs the
    /// appropriate on-tick processing, accepts pending votes and returns the
    /// current head.
    pub fn get_proposal_head(&mut self, slot: Slot) -> Bytes32 {
        let slot_time = self.config.genesis_time + (slot.0 * self.config.seconds_per_slot);
        crate::handlers::on_tick(self, slot_time, true);
        accept_new_votes(self);
        self.head
    }
}