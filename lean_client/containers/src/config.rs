use ssz_derive::Ssz;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Ssz, Default, Serialize, Deserialize)]
pub struct Config {
    /// total validators in the network
    pub num_validators: u64,
    /// genesis timestamp (seconds since UNIX epoch)
    pub genesis_time: u64,
}