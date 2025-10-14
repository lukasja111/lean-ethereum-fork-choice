# Fork Choice Implementation

This module implements the LMD-GHOST fork choice algorithm for the Lean Ethereum consensus client, based on the specification provided.

## Overview

The fork choice algorithm determines the canonical chain when multiple valid chains exist. This implementation uses LMD-GHOST (Latest Message Driven Greedy Heaviest Observed SubTree) with modifications for the simplified protocol design.

## Key Components

### Core Modules

- **`helpers.rs`** - Core fork choice algorithm implementation and utility functions
- **`handlers.rs`** - Event handlers for blocks, attestations, and time progression
- **`store.rs`** - Fork choice store that maintains all necessary state

### Key Features

1. **LMD-GHOST Algorithm**: Chooses the heaviest subtree based on validator votes
2. **Justification Integration**: Only justified blocks serve as fork choice starting points
3. **Vote Timing**: Careful timing of vote processing to prevent attacks
4. **Safe Targets**: Mechanism to find blocks safe for validators to vote on
5. **Reorganization Support**: Handles chain reorganizations when new votes change the head

## Test Structure

Tests are organized into multiple files similar to the original Python structure:

- **`basic_algorithm_tests.rs`** - Core LMD-GHOST algorithm tests
- **`fork_scenario_tests.rs`** - Complex fork scenarios and edge cases
- **`store_integration_tests.rs`** - Integration tests for the Store functionality
- **`common/mod.rs`** - Shared test utilities and helper functions

## Usage

```rust
use fork_choice::*;
use containers::*;

// Create a fork choice store
let store = Store::new(genesis_state, genesis_block, config);

// Process new blocks
handlers::on_block(&mut store, new_block);

// Process attestations/votes
handlers::on_attestation(&mut store, vote, false);

// Get the current head
let head = store.head;

// Get proposal head for a specific slot
let proposal_head = store.get_proposal_head(slot);
```

## Implementation Details

The fork choice algorithm:

1. Starts from the most recently justified block
2. Follows validator votes forward through the tree of blocks  
3. At each branch, picks the child with the most votes
4. Votes for descendants count toward ancestors
5. Continues until reaching a leaf (the chain head)

The implementation respects finalization - fork choice can never revert finalized blocks and always starts from the finalized chain.

## Testing

Run all tests with:
```bash
cargo test --package fork-choice
```

Run specific test groups:
```bash
cargo test --package fork-choice basic_algorithm_tests
cargo test --package fork-choice fork_scenario_tests  
cargo test --package fork-choice store_integration_tests
```

`get_fork_choice_head`

`get_latest_justified`

`Store`

`get_forkchoice_store`

`update_head`

`update_safe_target`

`get_vote_target`

`accept_new_votes`

`tick_interval`

`get_proposal_head`

## Handlers
`on_tick`

`on_attestation`

`on_block`

