//! Area 20 (Live Quest): `src/area/20/lq.c`.
//!
//! Unlike every other `npc/<area>/` module, this area has no fixed
//! `.chr`-template NPC roster - every `CDR_LQNPC` instance is admin-
//! authored at runtime (`world::lq`'s `LqNpcState`/the not-yet-ported
//! `special_driver` admin command table). This module ports `lqnpc`'s
//! per-tick dialogue/movement driver body and `lqnpc_died`'s respawn/
//! mark-setting death hook data (the death hook glue itself lives in
//! `ugaris-server`'s `world_events::death_hooks`, matching the C
//! `ch_died_driver` split precedent).

pub mod lqnpc;

#[allow(unused_imports)]
pub use lqnpc::*;
