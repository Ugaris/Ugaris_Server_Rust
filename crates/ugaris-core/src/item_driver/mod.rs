//! Legacy item-driver registry.
//!
//! Ports the C `item_driver` dispatch from `src/system/libload.c` plus the
//! per-driver behavior from `src/module/*.c` and `src/area/*/*.c`. Submodules
//! mirror the C source layout: shared families (doors, chests, potions, ...)
//! come from `src/module/base.c`, and `area*`/named modules come from the
//! matching legacy area or module file.

mod alchemy;
mod area11_palace;
mod area12_mine;
mod area13_dungeon;
mod area14_random;
mod area15_swamp;
mod area16_forest;
mod area17_two;
mod area18_bones;
mod area19_nomad;
mod area2;
mod area20_lq;
mod area22_lab;
mod area25_warped;
mod area26_staffer;
mod area28_forest;
mod area29_brannington;
mod area30_clan;
mod area31_warrmines;
mod area34_teufel;
mod area36_caligar;
mod area37_arkhata;
mod area4_pents;
mod area6_edemon;
mod area8_fdemon;
mod arena;
mod assemble;
mod books;
mod chests;
mod dispatch;
mod doors;
mod food;
mod helpers;
mod ice;
mod ids;
mod lights;
mod orbs;
mod potions;
mod saltmine;
mod scrolls;
mod sewers;
mod shrines;
mod teleports;
mod traps;
mod types;
mod xmas;

pub use alchemy::*;
pub(crate) use area11_palace::*;
pub(crate) use area12_mine::*;
pub(crate) use area13_dungeon::*;
pub use area14_random::*;
pub(crate) use area15_swamp::*;
pub use area16_forest::*;
pub use area17_two::*;
pub(crate) use area18_bones::*;
pub use area19_nomad::*;
pub(crate) use area2::*;
pub(crate) use area20_lq::*;
pub(crate) use area22_lab::*;
pub use area25_warped::*;
pub(crate) use area26_staffer::*;
pub(crate) use area28_forest::*;
pub use area29_brannington::*;
pub use area30_clan::*;
pub(crate) use area31_warrmines::*;
pub(crate) use area34_teufel::*;
pub(crate) use area36_caligar::*;
pub(crate) use area37_arkhata::*;
pub(crate) use area4_pents::*;
pub use area6_edemon::*;
pub use area8_fdemon::*;
pub use arena::*;
pub use assemble::*;
pub use books::*;
pub use chests::*;
pub use dispatch::*;
pub(crate) use doors::*;
pub(crate) use food::*;
pub(crate) use helpers::*;
pub(crate) use ice::*;
pub use ids::*;
pub(crate) use lights::*;
pub(crate) use orbs::*;
pub(crate) use potions::*;
pub(crate) use saltmine::*;
pub(crate) use scrolls::*;
// Explicit `pub` re-export of the 3 `scrolls` helpers `shrine_indecisiveness`
// (`random.c:1780-1802`) needs cross-crate: the rest of `scrolls` stays
// `pub(crate)` (item-driver-internal), see `lower_value`'s doc comment.
pub use scrolls::{bare_value, lower_value, skillmax};
pub(crate) use sewers::*;
pub(crate) use shrines::*;
pub use teleports::*;
pub(crate) use traps::*;
pub use types::*;
pub(crate) use xmas::*;

#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};

use crate::{
    direction::Direction,
    do_action::ItemUseRequest,
    entity::{
        Character, CharacterFlags, CharacterValue, Item, ItemFlags, CHARACTER_VALUE_COUNT,
        MAX_MODIFIERS, POWERSCALE,
    },
    ids::{CharacterId, ItemId},
    item_ops::consume_item,
    legacy::{action, profession, MAX_MAP},
    text::{COL_DARK_GRAY, COL_LIGHT_GREEN, COL_RESET},
    tick::TICKS_PER_SECOND,
};
