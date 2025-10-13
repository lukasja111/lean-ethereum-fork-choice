// tests/state_basic.rs
use containers::{block::{BlockBody, hash_tree_root}, state::State, types::Uint64, ValidatorIndex};
use pretty_assertions::assert_eq;

#[path = "common.rs"]
mod common;
use common::sample_config;

#[test]
fn test_generate_genesis() {
    let config = sample_config();
    let state = State::generate_genesis(Uint64(config.genesis_time), Uint64(config.num_validators));

    assert_eq!(state.config, config);
    assert_eq!(state.slot.0, 0);

    let empty_body = BlockBody { attestations: ssz::PersistentList::default() };
    assert_eq!(state.latest_block_header.body_root, hash_tree_root(&empty_body));

    
    assert!(state.historical_block_hashes.is_empty());
    assert!(state.justified_slots.is_empty());
    assert!(state.justifications_roots.is_empty());
    assert!(state.justifications_validators.is_empty());
}

#[test]
fn test_proposer_round_robin() {
    let state = State::generate_genesis(Uint64(0), Uint64(4));
    assert!(state.is_proposer(containers::types::ValidatorIndex(0)));
}

#[test]
fn test_slot_justifiability_rules() {
    use containers::slot::Slot;

    assert!(Slot(1).is_justifiable_after(Slot(0)));
    assert!(Slot(9).is_justifiable_after(Slot(0))); // perfect square
    assert!(Slot(6).is_justifiable_after(Slot(0))); // pronic (2*3)
}

#[test]
fn test_hash_tree_root() {
    let body = BlockBody { attestations: ssz::PersistentList::default() };
    let block = containers::block::Block {
        slot: containers::slot::Slot(1),
        proposer_index: ValidatorIndex(0),
    parent_root: containers::types::Bytes32(ssz::H256::zero()),
    state_root: containers::types::Bytes32(ssz::H256::zero()),
        body,
    };

    let root = hash_tree_root(&block);
    assert_ne!(root, containers::types::Bytes32(ssz::H256::zero()));
}