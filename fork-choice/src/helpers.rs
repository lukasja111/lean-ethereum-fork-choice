use std::collections::HashMap;

pub type Root = [u8; 32];
pub type Slot = u64;
pub type ValidatorI = u64;
pub type Interval = u64;

pub const ZERO_HASH: Root = [0u8; 32];
pub const INTERVALS_PER_SLOT: Interval = 8;
pub const SECONDS_PER_SLOT: u64 = 12;


/*
 !!! WARNING !!!
 We will later import the agreed upon versions of the structs and
 implementations below, this is the super-glue solution
 !!! WARNING !!!
*/ 
#[derive(Debug, Clone)]
pub struct Config {
    pub genesis_time: u64,
    pub num_validators: usize,
    pub seconds_per_slot: u64,
    pub intervals_per_slot: Interval,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checkpoint {
    pub slot: Slot,
    pub root: Root,
}


#[derive(Debug, Clone)]
pub struct Block {
    pub slot: Slot,
    pub parent_root: Root,
    pub state_root: Root,
    pub attestations: Vec<SignedVote>,
}

#[derive(Debug, Clone)]
pub struct State {
    pub latest_justified: Checkpoint,
    pub latest_finalized: Checkpoint,
}

#[derive(Debug, Clone)]
pub struct SignedVote {
    pub validator_id: ValidatorI,
    pub message: Checkpoint,
}

pub fn hash_tree_root(block: &Block) -> Root {
    block.state_root
}

pub fn process_block(parent_state: State, _block: &Block) -> State {
    parent_state
}

fn is_justified_slot(_finalized_slot: Slot, _target_slot: Slot) -> bool {
    true
}
/*
!!! END OF CONFIGURABLES !!!
*/

#[derive(Debug, Clone)]
pub struct Store {
    pub time: Interval,
    pub config: Config,
    pub head: Root,
    pub safe_target: Root,
    pub latest_justified: Checkpoint,
    pub latest_finalized: Checkpoint,
    pub blocks: HashMap<Root, Block>,
    pub states: HashMap<Root, State>,
    pub latest_known_votes: HashMap<ValidatorI, Checkpoint>,
    pub latest_new_votes: HashMap<ValidatorI, Checkpoint>,

}

pub fn get_forkchoice_store(anchor_state: State, anchor_block: Block, config: Config) -> Store {
    let block_root = hash_tree_root(&anchor_block);
    let time = anchor_block.slot*config.intervals_per_slot;

    Store {
        time, 
        config, 
        head: block_root,
        safe_target: block_root,
        latest_justified: anchor_state.latest_justified.clone(),
        latest_finalized: anchor_state.latest_finalized.clone(),
        blocks: [(block_root, anchor_block)].into(),
        states: [(block_root, anchor_state)].into(),
        latest_known_votes: HashMap::new(),
        latest_new_votes: HashMap::new(),
    }
}


pub fn get_fork_choice_head(blocks: &HashMap<Root, Block>, mut root: Root, latest_votes: &HashMap<ValidatorI, Checkpoint>, min_score: usize,) -> Root {
    if root == ZERO_HASH {
        root = blocks
            .iter()
            .min_by_key(|(_, block)| block.slot)
            .map(|(r, _)| *r)
            .expect("Err: blocks can't be empty.");
    }

    let mut vote_weights: HashMap<Root, usize> = HashMap::new();
    for v in latest_votes.values() {
        if let Some(mut curr) = blocks.get(&v.root).map(|_| v.root) {
            while blocks[&curr].slot > blocks[&root].slot {
                *vote_weights.entry(curr).or_insert(0) += 1;
                curr = blocks[&curr].parent_root;
            }
        }
    }


    let mut children_map: HashMap<Root, Vec<Root>> = HashMap::new();
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
                let wa = vote_weights.get(*a).copied().unwrap_or(0);
                let wb = vote_weights.get(*b).copied().unwrap_or(0);
                wa.cmp(&wb)
                    .then_with(|| blocks[*a].slot.cmp(&blocks[*b].slot))
                    .then_with(|| (*a).cmp(*b))
            })
            .unwrap();
    }
}

pub fn get_latest_justified(states: &HashMap<Root, State>) -> Option<Checkpoint> {
    states.values().max_by_key(|state| state.latest_justified.slot).map(|s| s.latest_justified.clone())
}

pub fn update_head(store: &mut Store) {

    if let Some(latest_justified) = get_latest_justified(&store.states) {
        store.latest_justified = latest_justified;
    }

    store.head = get_fork_choice_head(&store.blocks, store.latest_justified.root, &store.latest_known_votes, 0, );

    if let Some(state) = store.states.get(&store.head) {
        store.latest_finalized = state.latest_finalized.clone();
    }
}

pub fn update_safe_target(store: &mut Store) {
    let num_validators = store.config.num_validators.max(1);
    let min_target_score = (num_validators*2+2)/3;
    store.safe_target = get_fork_choice_head(&store.blocks, store.latest_justified.root, &store.latest_new_votes, min_target_score, );
}

pub fn accept_new_votes(store: &mut Store) {
    store.latest_known_votes.extend(store.latest_new_votes.drain());
    update_head(store);
}

pub fn tick_interval(store: &mut Store, has_proposal: bool) {
    store.time += 1;
    let curr_interval = store.time%store.config.intervals_per_slot;

    match curr_interval {
        0 if has_proposal => accept_new_votes(store),
        2 => update_safe_target(store),
        _ if curr_interval != 1 => accept_new_votes(store),
        _ => {}
    }
}

pub fn get_vote_target(store: &Store) -> Checkpoint {
    let mut target_root = store.head;

    for _ in 0..3 {
        if store.blocks[&target_root].slot > store.blocks[&store.safe_target].slot {
            target_root = store.blocks[&target_root].parent_root;
        } else {
            break;
        }
    }

    while !is_justified_slot(store.latest_finalized.slot, store.blocks[&target_root].slot) {
        target_root = store.blocks[&target_root].parent_root;
    }

    let target_block = &store.blocks[&target_root];
    Checkpoint {
        root: target_root,
        slot: target_block.slot,
    }
}

pub fn get_proposal_head(store: &mut Store, slot: Slot) -> Root {
    let slot_time = store.config.genesis_time+(slot*store.config.seconds_per_slot);

    crate::handlers::on_tick(store, slot_time, true);
    accept_new_votes(store);
    store.head
}