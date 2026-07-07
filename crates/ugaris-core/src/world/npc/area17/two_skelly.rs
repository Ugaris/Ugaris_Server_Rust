//! Two-City raised skeleton (`CDR_TWOSKELLY`) driver data.

#[allow(unused_imports)]
use crate::world::*;

// ---- legacy driver registry surface (moved from character_driver.rs) ----

#[allow(unused_imports)]
use crate::character_driver::*;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TwoSkellyDriverData {
    pub last_talk_tick: i32,
    pub current_victim: Option<CharacterId>,
    pub alive_tick: i32,
}
