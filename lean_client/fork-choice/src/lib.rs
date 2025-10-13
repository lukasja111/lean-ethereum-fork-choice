//! Fork Choice Algorithm Implementation
//! 
//! This crate implements the LMD-GHOST fork choice algorithm for Ethereum consensus.
//! It provides the core functionality for tracking blocks, votes, and determining
//! the canonical chain head in a proof-of-stake blockchain.
//! 
//! # Example
//! 
//! ```rust
//! use fork_choice::*;
//! 
//! // Initialize a fork choice store
//! let config = ForkChoiceConfig::default();
//! let anchor_state = State::default();
//! let anchor_block = Block::default();
//! let anchor_root = Bytes32::default();
//! 
//! let mut store = Store::new(anchor_state, anchor_block, anchor_root, config)?;
//! 
//! // Process blocks and votes
//! // store.on_block(block, block_root)?;
//! // store.on_attestation(vote, false)?;
//! ```

pub mod types;
pub mod store;  
pub mod helpers;
pub mod handlers;
pub mod utils;

// Re-export main types and functions for easy use
pub use types::{
    ForkChoiceConfig, ForkChoiceError, ForkChoiceResult,
    Root, Interval
};

// Re-export container types directly  
pub use containers::{
    Block, BlockBody, Checkpoint, SignedVote, Vote, State,
    Bytes32, Uint64, ValidatorIndex, Slot
};

pub use store::Store;

pub use handlers::{
    on_tick, on_attestation, on_block
};

pub use helpers::{
    get_fork_choice_head, get_latest_justified, 
    get_forkchoice_store, update_head, update_safe_target,
    get_vote_target, accept_new_votes, tick_interval,
    get_proposal_head
};

pub use utils::{
    SECONDS_PER_SLOT, INTERVALS_PER_SLOT, SECONDS_PER_INTERVAL
};

/// High-level fork choice interface
impl Store {
    /// Process a new block 
    pub fn on_block(&mut self, block: Block, block_root: Root) -> ForkChoiceResult<()> {
        handlers::on_block(self, block, block_root)
    }
    
    /// Process an attestation/vote
    pub fn on_attestation(&mut self, signed_vote: SignedVote, is_from_block: bool) -> ForkChoiceResult<()> {
        handlers::on_attestation(self, signed_vote, is_from_block)
    }
    
    /// Advance time and trigger any interval-based processing
    pub fn on_tick(&mut self, time: u64, has_proposal: bool) -> ForkChoiceResult<()> {
        handlers::on_tick(self, time, has_proposal)
    }
    
    /// Get the current canonical head
    pub fn get_head(&self) -> Root {
        self.head
    }
    
    /// Get the voting target for validators
    pub fn get_vote_target(&self) -> ForkChoiceResult<Checkpoint> {
        helpers::get_vote_target(self)
    }
    
    /// Get the head for block proposal
    pub fn get_proposal_head(&mut self, slot: Slot) -> ForkChoiceResult<Root> {
        helpers::get_proposal_head(self, slot)
    }
}