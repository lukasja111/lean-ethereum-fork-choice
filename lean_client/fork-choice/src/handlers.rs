use crate::store::Store;
use crate::types::*;
use crate::utils::SECONDS_PER_INTERVAL;
use containers::{SignedVote, ValidatorIndex, Checkpoint, Block, State, Slot};

/// Time advancement handler called periodically by the node
/// - Before block proposal
/// - After network sync
/// - At regular intervals
pub fn on_tick(store: &mut Store, time: u64, has_proposal: bool) -> ForkChoiceResult<()> {
    if !store.is_initialized() {
        return Err(ForkChoiceError::StoreNotInitialized);
    }
    
    let tick_interval_time = (time.saturating_sub(store.config.genesis_time.0)) / SECONDS_PER_INTERVAL;

    while store.time < tick_interval_time {
        let should_signal_proposal = has_proposal && (store.time + 1) == tick_interval_time;
        crate::helpers::tick_interval(store, should_signal_proposal)?;
    }
    
    Ok(())
}


/// Process an attestation/vote from either a block or gossip
/// Validates and updates the store's vote tracking
pub fn on_attestation(store: &mut Store, signed_vote: SignedVote, is_from_block: bool) -> ForkChoiceResult<()> {
    if !store.is_initialized() {
        return Err(ForkChoiceError::StoreNotInitialized);
    }
    
    // Validate the vote structure
    validate_vote(&signed_vote)?;
    
    let validator_id = ValidatorIndex(signed_vote.data.validator_id.0);
    let target_checkpoint = signed_vote.data.target.clone();

    if is_from_block {
        // Process vote from a block (on-chain vote)
        update_known_votes(store, validator_id, target_checkpoint.clone())?;
        
        // Remove from new votes if this supersedes it
        if let Some(latest_new_vote) = store.latest_new_votes.get(&validator_id) {
            if latest_new_vote.slot <= target_checkpoint.slot {
                store.latest_new_votes.remove(&validator_id);
            }
        }
    } else {
        // Process gossip vote (off-chain vote)
        let current_slot = store.current_slot();
        
        // Prevent voting for future slots
        if target_checkpoint.slot > current_slot {
            return Err(ForkChoiceError::VoteInFuture(target_checkpoint.slot.0));
        }
        
        update_new_votes(store, validator_id, target_checkpoint)?;
    }
    
    Ok(())
}

/// Validate vote structure and basic properties
fn validate_vote(signed_vote: &SignedVote) -> ForkChoiceResult<()> {
    // Basic validation - in a real implementation, this would include:
    // - Signature verification
    // - Validator eligibility checks
    // - Vote structure validation
    
    if signed_vote.data.source.slot >= signed_vote.data.target.slot {
        return Err(ForkChoiceError::InvalidVote(
            "Source slot must be less than target slot".to_string()
        ));
    }
    
    Ok(())
}

/// Update known votes (canonical on-chain vote history)
fn update_known_votes(store: &mut Store, validator_id: ValidatorIndex, vote: Checkpoint) -> ForkChoiceResult<()> {
    if let Some(latest_vote) = store.latest_known_votes.get(&validator_id) {
        if latest_vote.slot < vote.slot {
            store.latest_known_votes.insert(validator_id, vote);
        }
    } else {
        store.latest_known_votes.insert(validator_id, vote);
    }
    Ok(())
}

/// Update new votes (pending votes not yet confirmed on-chain)
fn update_new_votes(store: &mut Store, validator_id: ValidatorIndex, vote: Checkpoint) -> ForkChoiceResult<()> {
    if let Some(latest_vote) = store.latest_new_votes.get(&validator_id) {
        if latest_vote.slot < vote.slot {
            store.latest_new_votes.insert(validator_id, vote);
        }
    } else {
        store.latest_new_votes.insert(validator_id, vote);
    }
    Ok(())
}

/// Process a new block and update the fork choice store
pub fn on_block(store: &mut Store, block: Block, block_root: Root) -> ForkChoiceResult<()> {
    if !store.is_initialized() {
        return Err(ForkChoiceError::StoreNotInitialized);
    }
    
    // Check if block was already processed
    if store.blocks.contains_key(&block_root) {
        return Err(ForkChoiceError::BlockAlreadyProcessed);
    }

    // Verify parent state exists to prevent orphan blocks
    let parent_state = store.get_state(&block.parent_root)
        .ok_or(ForkChoiceError::ParentStateNotFound)?;

    // Simplified state transition - in production, this would call the state transition function
    let new_state = apply_state_transition(parent_state, &block)?;

    // Add block and state to store
    store.add_block(block_root, block.clone())?;
    store.add_state(block_root, new_state);

    // Process attestations in the block
    for signed_vote in &block.body.attestations {
        on_attestation(store, signed_vote.clone(), true)?;
    }

    // Update the head after processing the block
    crate::helpers::update_head(store)?;
    
    Ok(())
}

/// Apply state transition for a block (simplified version)
/// In production, this would be provided by the state transition module
fn apply_state_transition(parent_state: &State, block: &Block) -> ForkChoiceResult<State> {
    let mut new_state = parent_state.clone();
    
    // Simplified state updates - in reality this would be much more complex
    // Update justified checkpoint if conditions are met
    if should_update_justified(&new_state, block) {
        new_state.latest_justified = Checkpoint {
            root: block.state_root,
            slot: block.slot,
        };
    }
    
    // Update finalized checkpoint if conditions are met
    if should_update_finalized(&new_state, block) {
        new_state.latest_finalized = Checkpoint {
            root: block.parent_root,
            slot: Slot(block.slot.0.saturating_sub(1)),
        };
    }
    
    Ok(new_state)
}

/// Determine if justified checkpoint should be updated (simplified)
fn should_update_justified(_state: &State, _block: &Block) -> bool {
    // Simplified logic - in reality this would check:
    // - Supermajority attestations
    // - Epoch boundaries
    // - Casper FFG rules
    true
}

/// Determine if finalized checkpoint should be updated (simplified) 
fn should_update_finalized(state: &State, block: &Block) -> bool {
    // Simplified logic - in reality this would check:
    // - Two consecutive justified epochs
    // - Supermajority link between them
    block.slot.0 > state.latest_finalized.slot.0 + 64 // Roughly 2 epochs
}