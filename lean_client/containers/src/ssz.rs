use ssz::SszHash;
use crate::Bytes32;

// Helper function to compute hash tree root using grandine ssz
pub fn compute_hash_tree_root<T: SszHash>(value: &T) -> Bytes32 {
    let h = value.hash_tree_root(); // returns ssz::H256
    Bytes32(h)
}