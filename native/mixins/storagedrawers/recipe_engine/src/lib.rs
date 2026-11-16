//! Storage Drawers 1.21 inventory rules, independent of rendering and the game.
//! Quantities are integral; compacting views always share one stock of base units.
pub mod storage;
pub mod network;
pub mod codec;
pub use storage::*;
pub use network::*;

