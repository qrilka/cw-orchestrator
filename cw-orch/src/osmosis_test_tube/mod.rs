//! Integration testing execution environment backed by a [osmosis-test-tube](osmosis_test_tube) App.
//! It has an associated state that stores deployment information for easy retrieval and contract interactions.

mod core;
mod state;

pub use self::core::*;
pub use state::OsmosisTestTubeState;
