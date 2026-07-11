//! Areas 23/24 NPCs, one file per NPC.
//!
//! See `crate::world::strategy`'s module doc comment for the wider
//! "Areas 23/24 - `strategy.c`" P4 task's C source reference and
//! ported/remaining slice breakdown.

pub mod boss;

#[allow(unused_imports)]
pub use boss::*;
