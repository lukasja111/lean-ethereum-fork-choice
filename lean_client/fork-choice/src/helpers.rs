use std::collections::HashMap;
use crate::types::*;
use crate::store::Store;
use containers::{Bytes32, Block, ValidatorIndex, Checkpoint, State, Slot};
use containers::ssz::H256;

const ZERO_HASH: Bytes32 = Bytes32(H256::zero());

/// LMD-GHOST algorithm implementation
/// Finds the chain with the most validator support by following the heaviest subtree
pub fn get_fork_choice_head(
    blocks: &HashMap<Root, Block>,
    root: &Root,
    latest_votes: &HashMap<ValidatorIndex, Checkpoint>,
    min_score: Option<usize>,
) -> ForkChoiceResult<Root> {
    let min_score = min_score.unwrap_or(0);
    
    // Start from provided root or find genesis if zero hash
    let mut current_root = if *root == ZERO_HASH {
        find_genesis_block(blocks)?
    } else {
        *root
    };

    // Count votes for each block by propagating votes up the ancestry
    let vote_weights = calculate_vote_weights(blocks, latest_votes, &current_root)?;

    // Build children map for efficient traversal
    let children_map = build_children_map(blocks, &vote_weights, min_score);

    // Follow the heaviest path using LMD-GHOST
    current_root = follow_heaviest_path(blocks, &children_map, &vote_weights, current_root)?;
    
    Ok(current_root)
}

/// Find the genesis block (earliest slot)
fn find_genesis_block(blocks: &HashMap<Root, Block>) -> ForkChoiceResult<Root> {
    blocks.iter()
        .min_by_key(|(_, block)| block.slot.0)
        .map(|(root, _)| *root)
        .ok_or(ForkChoiceError::StoreNotInitialized)
}

/// Calculate vote weights by propagating votes up the ancestry
fn calculate_vote_weights(
    blocks: &HashMap<Root, Block>,
    latest_votes: &HashMap<ValidatorIndex, Checkpoint>,
    start_root: &Root,
) -> ForkChoiceResult<HashMap<Root, usize>> {
    let mut vote_weights: HashMap<Root, usize> = HashMap::new();
    let start_slot = blocks.get(start_root)
        .ok_or(ForkChoiceError::StoreNotInitialized)?
        .slot;

    for vote in latest_votes.values() {
        if let Some(_) = blocks.get(&vote.root) {
            // Propagate vote up the ancestry chain
            let mut current_hash = vote.root;
            while let Some(block) = blocks.get(&current_hash) {
                if block.slot >= start_slot {
                    *vote_weights.entry(current_hash).or_insert(0) += 1;
                    current_hash = block.parent_root;
                } else {
                    break;
                }
            }
        }
    }

    Ok(vote_weights)
}

/// Build a map of parent -> children relationships
fn build_children_map(
    blocks: &HashMap<Root, Block>,
    vote_weights: &HashMap<Root, usize>,
    min_score: usize,
) -> HashMap<Root, Vec<Root>> {
    let mut children_map: HashMap<Root, Vec<Root>> = HashMap::new();
    
    for (block_hash, block) in blocks {
        let has_sufficient_votes = vote_weights.get(block_hash).unwrap_or(&0) >= &min_score;
        let is_not_genesis = block.parent_root != ZERO_HASH;
        
        if has_sufficient_votes && is_not_genesis {
            children_map
                .entry(block.parent_root)
                .or_insert_with(Vec::new)
                .push(*block_hash);
        }
    }
    
    children_map
}

/// Follow the heaviest path using LMD-GHOST tie-breaking rules
fn follow_heaviest_path(
    blocks: &HashMap<Root, Block>,
    children_map: &HashMap<Root, Vec<Root>>,
    vote_weights: &HashMap<Root, usize>,
    start_root: Root,
) -> ForkChoiceResult<Root> {
    let mut current = start_root;
    
    loop {
        if let Some(children) = children_map.get(&current) {
            if children.is_empty() {
                return Ok(current);
            }
            
            // Find child with most votes, use slot and hash as tie-breakers
            current = children.iter()
                .max_by_key(|child| {
                    let vote_weight = vote_weights.get(*child).unwrap_or(&0);
                    let block_slot = blocks[*child].slot.0;
                    let block_hash_bytes = child.0.as_bytes();
                    (vote_weight, block_slot, block_hash_bytes)
                })
                .copied()
                .ok_or(ForkChoiceError::StoreNotInitialized)?;
        } else {
            return Ok(current);
        }
    }
}

/// Find the latest justified checkpoint across all states
pub fn get_latest_justified(states: &HashMap<Root, State>) -> Option<Checkpoint> {
    states.values()
        .max_by_key(|state| state.latest_justified.slot.0)
        .map(|state| state.latest_justified.clone())
}

/// Create an initial fork-choice store with proper initialization
pub fn get_forkchoice_store(
    anchor_state: State, 
    anchor_block: Block, 
    anchor_root: Root,
    config: ForkChoiceConfig
) -> ForkChoiceResult<Store> {
    Store::new(anchor_state, anchor_block, anchor_root, config)
}

/// Calculate and update the store's head using the fork choice algorithm
pub fn update_head(store: &mut Store) -> ForkChoiceResult<()> {
    // Update latest justified checkpoint
    if let Some(latest_justified) = get_latest_justified(&store.states) {
        store.latest_justified = latest_justified;
    }
    
    // Calculate new head using LMD-GHOST
    store.head = get_fork_choice_head(
        &store.blocks,
        &store.latest_justified.root,
        &store.latest_known_votes,
        None,
    )?;

    // Update finalized checkpoint from head state
    if let Some(head_state) = store.states.get(&store.head) {
        store.latest_finalized = head_state.latest_finalized.clone();
    }
    
    Ok(())
}

/// Find and update the safe voting target (requires >2/3 validator support)
pub fn update_safe_target(store: &mut Store) -> ForkChoiceResult<()> {
    let min_target_score = ((store.config.num_validators.0 * 2) / 3) + 1; 
    
    store.safe_target = get_fork_choice_head(
        &store.blocks,
        &store.latest_justified.root,
        &store.latest_new_votes,
        Some(min_target_score as usize),
    )?;
    
    Ok(())
}

/// Check if a slot is within the justification window
fn is_justifiable_slot(finalized_slot: Slot, target_slot: Slot) -> bool {
    const JUSTIFICATION_WINDOW: u64 = 64; // ~2 epochs
    target_slot.0 > finalized_slot.0 && 
    target_slot.0 <= finalized_slot.0 + JUSTIFICATION_WINDOW
}

/// Calculate the optimal voting target for validators
pub fn get_vote_target(store: &Store) -> ForkChoiceResult<Checkpoint> {
    let mut target_block_root = store.head;

    // Ensure we don't vote too far ahead of the safe target
    let max_iterations = 10; // Prevent infinite loops
    for _ in 0..max_iterations {
        if let (Some(current_block), Some(safe_block)) = (
            store.blocks.get(&target_block_root),
            store.blocks.get(&store.safe_target)
        ) {
            if current_block.slot > safe_block.slot {
                target_block_root = current_block.parent_root;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    // Walk back to find a justifiable target
    let mut iterations = 0;
    while iterations < max_iterations {
        if let Some(target_block) = store.blocks.get(&target_block_root) {
            if is_justifiable_slot(store.latest_finalized.slot, target_block.slot) {
                return Ok(Checkpoint {
                    root: target_block_root,
                    slot: target_block.slot,
                });
            }
            target_block_root = target_block.parent_root;
        } else {
            break;
        }
        iterations += 1;
    }

    // Fallback to latest justified if no suitable target found
    Ok(store.latest_justified.clone())
}

/// Move votes from new_votes to known_votes and update head
pub fn accept_new_votes(store: &mut Store) -> ForkChoiceResult<()> {
    // Move all new votes to known votes
    for (validator_id, vote) in store.latest_new_votes.drain() {
        store.latest_known_votes.insert(validator_id, vote);
    }
    
    // Recalculate head with the new votes
    update_head(store)?;
    Ok(())
}

/// Handle time advancement and interval-based logic
pub fn tick_interval(store: &mut Store, has_proposal: bool) -> ForkChoiceResult<()> {
    store.time += 1;
    let current_interval = store.time % store.config.intervals_per_slot.0;
    
    match current_interval {
        0 => {
            // First interval of slot - process votes if there's a proposal
            if has_proposal {
                accept_new_votes(store)?;
            }
        }
        2 => {
            // Third interval of slot - update safe target
            update_safe_target(store)?;
        }
        _ => {
            // No special action for other intervals
        }
    }
    
    Ok(())
}

/// Get the head for block proposal at a specific slot
pub fn get_proposal_head(store: &mut Store, slot: Slot) -> ForkChoiceResult<Root> {
    let slot_time = store.config.genesis_time.0 + slot.0 * store.config.seconds_per_slot.0;
    
    // Advance time to the proposal slot
    crate::handlers::on_tick(store, slot_time, true)?;
    
    // Process any pending votes
    accept_new_votes(store)?;
    
    Ok(store.head)
}