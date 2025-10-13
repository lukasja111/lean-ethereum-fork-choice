# Lean-Ethereum Fork-Choice

Lean-Ethereum client-side fork-choice implementation.

This is a basic fork-choice implementation rewritten from Lean-Ethereum Python.

## TO-DO
- [ ] Implement a function to check whether the block is within a justifiable window  
- [ ] Add more complex logic in `fn on_attestation` (currently assumes that votes are always valid)
- [ ] Add testing  
- [ ] Merge with the first team’s code and try to integrate our fork-choice implementation with their modules  
