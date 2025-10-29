use std::collections::{HashMap, HashSet};
use containers::{Bytes32, Block, State, Checkpoint, Slot};
use crate::store::ValidatorId;

pub const ZERO_HASH: Bytes32 = Bytes32(ssz::H256::zero());

/// Process a block and update the provided parent state.
///
/// - Validates attestations in the block (basic checks) and counts unique validator
///   votes for the block.
/// - If the counted votes reach the 2/3 justification threshold, update the
///   state's `latest_justified` and (when applicable) `latest_finalized` fields.
pub fn process_block(mut parent_state: State, block: &Block) -> State {
    parent_state.slot = block.slot;

    let mut vote_count: usize = 0;
    let mut seen_validators: HashSet<u64> = HashSet::new();
    let mut i: u64 = 0;

    loop {
        match block.body.attestations.get(i) {
            Ok(attestation) => {
                let vid = attestation.data.validator_id.0;
                if seen_validators.contains(&vid) {
                    i += 1;
                    continue;
                }

                let within_slot = attestation.data.slot.0 <= block.slot.0;
                let validator_in_range = (attestation.data.validator_id.0 as u64) < parent_state.config.num_validators;
                let source_not_after_justified = attestation.data.source.slot.0 <= parent_state.latest_justified.slot.0;

                if within_slot && validator_in_range && source_not_after_justified {
                    vote_count += 1;
                    seen_validators.insert(vid);
                }

                i += 1;
            }
            Err(_) => break,
        }
    }

    let total_validators = parent_state.config.num_validators as usize;
    if total_validators > 0 {
        let threshold = (2 * total_validators + 2) / 3;
        if vote_count >= threshold {
            let block_hash = hash_tree_root(block);
            parent_state.latest_justified = Checkpoint { slot: block.slot, root: Bytes32(block_hash.0) };
            if block.slot.0 > parent_state.latest_finalized.slot.0 + 1 {
                parent_state.latest_finalized = parent_state.latest_justified.clone();
            }
        }
    }

    parent_state
}

/// Compute the hash tree root of a block and return it as `Bytes32`.
pub fn hash_tree_root(block: &Block) -> Bytes32 {
    use containers::block::hash_tree_root as container_hash;
    container_hash(block)
}

/// Return true if `target_slot` is a slot that may be considered justified
/// relative to a `finalized_slot` under simple bounds used by the tests.
fn is_justified_slot(finalized_slot: Slot, target_slot: Slot) -> bool {
    target_slot.0 >= finalized_slot.0 && target_slot.0 <= finalized_slot.0 + 64
}

/// Run the LMD-GHOST fork choice rule to find the head starting at `root`.
///
/// - `blocks` contains the known block tree.
/// - `latest_votes` maps validator ids to their latest voted checkpoint.
/// - `min_score` filters children with fewer than `min_score` supporting votes.
pub fn get_fork_choice_head(
    blocks: &HashMap<Bytes32, Block>,
    mut root: Bytes32,
    latest_votes: &HashMap<ValidatorId, Checkpoint>,
    min_score: usize,
) -> Bytes32 {
    if root == ZERO_HASH {
        root = blocks.iter().min_by_key(|(_, block)| block.slot.0).map(|(r, _)| *r).expect("Blocks cannot be empty");
    }

    let mut vote_weights: HashMap<Bytes32, usize> = HashMap::new();
    for vote in latest_votes.values() {
        if let Some(mut curr) = blocks.get(&vote.root).map(|_| vote.root) {
            while blocks.get(&curr).map_or(false, |block| block.slot.0 > blocks[&root].slot.0) {
                *vote_weights.entry(curr).or_insert(0) += 1;
                curr = blocks[&curr].parent_root;
            }
        }
    }

    let mut children_map: HashMap<Bytes32, Vec<Bytes32>> = HashMap::new();
    for (block_hash, block) in blocks {
        if block.parent_root != ZERO_HASH {
            if vote_weights.get(block_hash).copied().unwrap_or(0) >= min_score {
                children_map.entry(block.parent_root).or_default().push(*block_hash);
            }
        }
    }

    let mut curr = root;
    loop {
        let children = match children_map.get(&curr) {
            Some(list) if !list.is_empty() => list,
            _ => return curr,
        };

        curr = *children
            .iter()
            .max_by(|a, b| {
                let weight_a = vote_weights.get(a).copied().unwrap_or(0);
                let weight_b = vote_weights.get(b).copied().unwrap_or(0);
                weight_a.cmp(&weight_b)
                    .then_with(|| blocks[a].slot.0.cmp(&blocks[b].slot.0))
                    .then_with(|| a.cmp(b))
            })
            .unwrap();
    }
}

/// Find the latest justified checkpoint among a map of block states.
pub fn get_latest_justified(states: &HashMap<Bytes32, State>) -> Option<Checkpoint> {
    states.values().max_by_key(|state| state.latest_justified.slot.0).map(|s| s.latest_justified.clone())
}

use crate::store::Store;

/// Update the store's `head` based on current known votes and justified info.
pub fn update_head(store: &mut Store) {
    if let Some(latest_justified) = get_latest_justified(&store.states) {
        store.latest_justified = latest_justified;
    }

    store.head = get_fork_choice_head(&store.blocks, store.latest_justified.root, &store.latest_known_votes, 0);

    if let Some(state) = store.states.get(&store.head) {
        store.latest_finalized = state.latest_finalized.clone();
    }
}

/// Compute and set the store's `safe_target` using a 2/3 minimum target score.
pub fn update_safe_target(store: &mut Store) {
    let num_validators = store.config.num_validators as usize;
    let min_target_score = (num_validators * 2 + 2) / 3;
    store.safe_target = get_fork_choice_head(&store.blocks, store.latest_justified.root, &store.latest_new_votes, min_target_score);
}

/// Move new votes into the known votes set and refresh the head.
pub fn accept_new_votes(store: &mut Store) {
    for (validator_id, vote) in store.latest_new_votes.drain() {
        store.latest_known_votes.insert(validator_id, vote);
    }
    update_head(store);
}

/// Advance the store by one protocol interval and run the corresponding phase
/// handlers (proposal, vote collection, safety update, final processing).
pub fn tick_interval(store: &mut Store, has_proposal: bool) {
    store.time += 1;
    let curr_interval = store.time % store.config.intervals_per_slot;
    let curr_slot = store.time / store.config.intervals_per_slot;

    match curr_interval {
        0 => {
            if has_proposal {
                accept_new_votes(store);
                cleanup_old_votes(store, curr_slot);
            }
        }
        1 => {}
        2 => update_safe_target(store),
        3 => accept_new_votes(store),
        _ => accept_new_votes(store),
    }
}

/// Remove votes older than the configured maximum age from the store.
fn cleanup_old_votes(store: &mut Store, current_slot: u64) {
    let max_vote_age = 32;

    store.latest_known_votes.retain(|_, checkpoint| current_slot.saturating_sub(checkpoint.slot.0) <= max_vote_age);

    store.latest_new_votes.retain(|_, checkpoint| current_slot.saturating_sub(checkpoint.slot.0) <= max_vote_age);
}

/// Determine an appropriate checkpoint target that validators should vote for.
pub fn get_vote_target(store: &Store) -> Checkpoint {
    let mut target_root = store.head;

    let current_slot = store.time / store.config.intervals_per_slot;
    let safety_margin = 2;

    while let Some(block) = store.blocks.get(&target_root) {
        if block.slot.0 + safety_margin <= current_slot {
            break;
        }
        if block.parent_root == ZERO_HASH {
            break;
        }
        target_root = block.parent_root;
    }

    let mut candidate = target_root;
    while let Some(block) = store.blocks.get(&candidate) {
        if is_ancestor_of(&store.blocks, candidate, store.safe_target) {
            target_root = candidate;
            break;
        }
        if block.parent_root == ZERO_HASH {
            break;
        }
        candidate = block.parent_root;
    }

    while let Some(block) = store.blocks.get(&target_root) {
        if is_justified_slot(store.latest_finalized.slot, block.slot) {
            break;
        }
        if block.parent_root == ZERO_HASH {
            break;
        }
        target_root = block.parent_root;
    }

    let target_block = &store.blocks[&target_root];
    Checkpoint { root: target_root, slot: target_block.slot }
}

/// Return true if `ancestor` is an ancestor (or equal) of `descendant` in
/// the provided block tree. Limits the walk to prevent infinite loops.
fn is_ancestor_of(blocks: &HashMap<Bytes32, Block>, ancestor: Bytes32, descendant: Bytes32) -> bool {
    if ancestor == descendant {
        return true;
    }

    let mut current = descendant;
    for _ in 0..256 {
        if let Some(block) = blocks.get(&current) {
            if block.parent_root == ancestor {
                return true;
            }
            if block.parent_root == ZERO_HASH {
                return false;
            }
            current = block.parent_root;
        } else {
            return false;
        }
    }
    false
}