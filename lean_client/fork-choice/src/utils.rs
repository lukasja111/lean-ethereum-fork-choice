// consts used in fork-choice implementation
pub const SECONDS_PER_SLOT: u64 = 12;
pub const INTERVALS_PER_SLOT: u64 = 3;
pub const SECONDS_PER_INTERVAL: u64 = SECONDS_PER_SLOT / INTERVALS_PER_SLOT;