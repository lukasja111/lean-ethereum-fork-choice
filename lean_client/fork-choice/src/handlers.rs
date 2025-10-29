use std::collections::HashMap;
use containers::{Block, SignedVote, ValidatorIndex, Checkpoint};
use containers::block::hash_tree_root;
use crate::helpers::*;
use crate::store::{Store, ValidatorId};

/// Advance store time to `time` (absolute seconds since genesis) and process
/// any protocol intervals that have elapsed. If `has_proposal` is true for the
/// final interval processed, the proposal phase will accept new votes.
pub fn on_tick(store: &mut Store, time: u64, has_proposal: bool) {
    let elapsed_intervals = time.saturating_sub(store.config.genesis_time) * store.config.intervals_per_slot / store.config.seconds_per_slot;

    while store.time < elapsed_intervals {
        let next_has_proposal = has_proposal && (store.time + 1 == elapsed_intervals);
        tick_interval(store, next_has_proposal);
    }
}

/// Process a signed attestation (vote) from gossip or included in a block.
///
/// - Validates the attestation with `is_valid_attestation`.
/// - If `is_from_block` is true the vote updates known votes immediately,
///   otherwise it is placed into `latest_new_votes` for later acceptance.
pub fn on_attestation(store: &mut Store, attestation: SignedVote, is_from_block: bool) {
    let validator_id = ValidatorId(ValidatorIndex(attestation.data.validator_id.0));
    let vote = attestation.data.target.clone();

    if !is_valid_attestation(store, &attestation) {
        return;
    }

    if is_from_block {
        if should_update_vote(&store.latest_known_votes, &validator_id, &vote) {
            store.latest_known_votes.insert(validator_id, vote.clone());
        }

        if let Some(existing) = store.latest_new_votes.get(&validator_id) {
            if existing.slot.0 < vote.slot.0 {
                store.latest_new_votes.remove(&validator_id);
            }
        }
    } else {
        let curr_slot = store.time / store.config.intervals_per_slot;

        if vote.slot.0 > curr_slot || curr_slot.saturating_sub(vote.slot.0) > 32 {
            return;
        }

        if !store.blocks.contains_key(&vote.root) {
            return;
        }

        if should_update_vote(&store.latest_new_votes, &validator_id, &vote) {
            store.latest_new_votes.insert(validator_id, vote);
        }
    }
}

/// Perform  validation of an attestation relative to the store.
fn is_valid_attestation(store: &Store, attestation: &SignedVote) -> bool {
    let vote_data = &attestation.data;

    if vote_data.validator_id.0 >= store.config.num_validators {
        return false;
    }

    if !store.blocks.contains_key(&vote_data.target.root) {
        return false;
    }

    if vote_data.source.slot.0 > store.latest_justified.slot.0 {
        return false;
    }

    if let Some(target_block) = store.blocks.get(&vote_data.target.root) {
        if target_block.slot != vote_data.target.slot {
            return false;
        }
    }

    if vote_data.target.slot.0 <= vote_data.source.slot.0 {
        return false;
    }

    true
}

/// Return true if `new_vote` should replace the existing vote for
/// `validator_id` (i.e. it's newer or there is no existing vote).
fn should_update_vote(votes: &HashMap<ValidatorId, Checkpoint>, validator_id: &ValidatorId, new_vote: &Checkpoint) -> bool {
    votes.get(validator_id).map_or(true, |existing| existing.slot.0 < new_vote.slot.0)
}

/// Process a newly received block: compute its state, process included
/// attestations, add the block and state to the store, and update the chain head.
pub fn on_block(store: &mut Store, block: Block) {
    let block_root = hash_tree_root(&block);

    if store.blocks.contains_key(&block_root) {
        return;
    }

    assert!(store.states.contains_key(&block.parent_root), "Missing parent state for block");

    let parent_state = store.states.get(&block.parent_root).unwrap().clone();
    let new_state = process_block(parent_state, &block);

    let mut i: u64 = 0;
    loop {
        match block.body.attestations.get(i) {
            Ok(attestation) => {
                on_attestation(store, attestation.clone(), true);
                i += 1;
            }
            Err(_) => break,
        }
    }

    store.blocks.insert(block_root, block);
    store.states.insert(block_root, new_state);

    update_head(store);
}