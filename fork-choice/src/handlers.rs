use crate::helpers::*;

pub fn on_tick(store: &mut Store, time: u64, has_proposal: bool) {
    let elapsed_intervals = time.saturating_sub(store.config.genesis_time)*store.config.intervals_per_slot/store.config.seconds_per_slot;
    while store.time<elapsed_intervals {
        let next_has_proposal = has_proposal && (store.time+1 == elapsed_intervals);
        tick_interval(store, next_has_proposal);
    }
}

pub fn on_attestation(store: &mut Store, attestation: SignedVote, is_from_block: bool) {
    let validator_id = attestation.validator_id;
    let vote = attestation.message;

    if is_from_block {
        
        if store
            .latest_known_votes
            .get(&validator_id)
            .map_or(true, |v| v.slot < vote.slot) {
                store.latest_known_votes.insert(validator_id, vote.clone());
            }
        if let Some(existing) = store.latest_new_votes.get(&validator_id) {
            if existing.slot < vote.slot {
                store.latest_new_votes.remove(&validator_id);
            }
        }
    } else {
        let curr_slot = store.time/store.config.intervals_per_slot;
        if vote.slot > curr_slot {
            return;
        }
        if store
            .latest_new_votes
            .get(&validator_id)
            .map_or(true, |v| v.slot < vote.slot) {
                store.latest_new_votes.insert(validator_id, vote);
            }
    }
}


pub fn on_block(store: &mut Store, block: Block) {
    let block_root = hash_tree_root(&block); 
    if store.blocks.contains_key(&block_root) {
        return;
    }
    assert!(store.states.contains_key(&block.parent_root), "Err: missing parent state");
    let parent_state = store.states.get(&block.parent_root).unwrap().clone();

    let new_state = process_block(parent_state, &block);
    for attestation in &block.attestations {
        on_attestation(store, attestation.clone(), true);
    }

    store.blocks.insert(block_root, block);
    store.states.insert(block_root, new_state);

    update_head(store);
}