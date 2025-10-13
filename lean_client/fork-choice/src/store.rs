use std::collections::HashMap;
use crate::types::*;
use serde::{Deserialize, Serialize};
use containers::{Bytes32, Block, State, ValidatorIndex, Checkpoint, Slot};

/// Store tracks all information needed for the fork-choice algorithm to make decisions.
/// It maintains the current view of the blockchain state and validator votes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Store {
    /// Current time in intervals
    pub time: Interval,
    /// Network configuration  
    pub config: ForkChoiceConfig,
    /// Current canonical chain head (block with most validator support)
    pub head: Root,
    /// Safe target with >2/3 validator support for voting
    pub safe_target: Root,
    /// Latest justified checkpoint (2/3 validators voted, not yet finalized)
    pub latest_justified: Checkpoint,
    /// Latest finalized checkpoint (mathematically irreversible)
    pub latest_finalized: Checkpoint,
    /// All known blocks indexed by their root hash
    pub blocks: HashMap<Root, Block>,
    /// Blockchain states after executing each block
    pub states: HashMap<Root, State>,
    /// Votes already included in the canonical chain
    pub latest_known_votes: HashMap<ValidatorIndex, Checkpoint>,
    /// New votes not yet confirmed on-chain
    pub latest_new_votes: HashMap<ValidatorIndex, Checkpoint>,
}

impl Default for Store {
    fn default() -> Self {
        Self {
            time: 0,
            config: ForkChoiceConfig::default(),
            head: Bytes32::default(),
            safe_target: Bytes32::default(),
            latest_justified: Checkpoint::default(),
            latest_finalized: Checkpoint::default(),
            blocks: HashMap::new(),
            states: HashMap::new(),
            latest_known_votes: HashMap::new(),
            latest_new_votes: HashMap::new(),
        }
    }
}

impl Store {
    /// Create a new Store initialized with an anchor state and block
    pub fn new(
        anchor_state: State, 
        anchor_block: Block, 
        anchor_root: Root, 
        config: ForkChoiceConfig
    ) -> ForkChoiceResult<Self> {
        let anchor_slot = anchor_block.slot;
        
        let store = Store {
            time: anchor_slot.0 * config.intervals_per_slot.0,
            config,
            head: anchor_root,
            safe_target: anchor_root,
            latest_justified: anchor_state.latest_justified.clone(),
            latest_finalized: anchor_state.latest_finalized.clone(),
            blocks: HashMap::from([(anchor_root, anchor_block)]),
            states: HashMap::from([(anchor_root, anchor_state)]),
            latest_known_votes: HashMap::new(),
            latest_new_votes: HashMap::new(),
        };
        
        Ok(store)
    }
    
    /// Check if the store is properly initialized
    pub fn is_initialized(&self) -> bool {
        !self.blocks.is_empty() && !self.states.is_empty()
    }
    
    /// Get the current slot based on time
    pub fn current_slot(&self) -> Slot {
        Slot(self.time / self.config.intervals_per_slot.0)
    }
    
    /// Add a block to the store
    pub fn add_block(&mut self, block_root: Root, block: Block) -> ForkChoiceResult<()> {
        if self.blocks.contains_key(&block_root) {
            return Err(ForkChoiceError::BlockAlreadyProcessed);
        }
        
        self.blocks.insert(block_root, block);
        Ok(())
    }
    
    /// Add a state to the store
    pub fn add_state(&mut self, state_root: Root, state: State) {
        self.states.insert(state_root, state);
    }
    
    /// Get a block by its root
    pub fn get_block(&self, block_root: &Root) -> Option<&Block> {
        self.blocks.get(block_root)
    }
    
    /// Get a state by its root  
    pub fn get_state(&self, state_root: &Root) -> Option<&State> {
        self.states.get(state_root)
    }
}
