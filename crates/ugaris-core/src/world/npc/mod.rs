//! NPC drivers: one file per NPC (data, parser, dialogue
//! table, and world logic together). Area NPCs live in
//! `areaN/` subdirectories; multi-area/system NPCs at the
//! top level.

pub mod aclerk;
pub mod area1;
pub mod area11;
pub mod area12;
pub mod area13;
pub mod area16;
pub mod area17;
pub mod area19;
pub mod area2;
pub mod area20;
pub mod area22;
pub mod area3;
pub mod area30;
pub mod area32;
pub mod area4;
pub mod area8;
pub mod arena;
pub mod bank;
pub mod clubmaster;
pub mod gate_fight;
pub mod gate_welcome;
pub mod janitor;
pub mod lostcon;
pub mod macro_npc;
pub mod merchant;
pub mod trader;

#[allow(unused_imports)]
pub use aclerk::*;
#[allow(unused_imports)]
pub use area1::*;
#[allow(unused_imports)]
pub use area11::*;
#[allow(unused_imports)]
pub use area12::*;
#[allow(unused_imports)]
pub use area13::*;
#[allow(unused_imports)]
pub use area16::*;
#[allow(unused_imports)]
pub use area17::*;
#[allow(unused_imports)]
pub use area19::*;
#[allow(unused_imports)]
pub use area2::*;
#[allow(unused_imports)]
pub use area20::*;
#[allow(unused_imports)]
pub use area22::*;
#[allow(unused_imports)]
pub use area3::*;
#[allow(unused_imports)]
pub use area30::*;
#[allow(unused_imports)]
pub use area32::*;
#[allow(unused_imports)]
pub use area4::*;
#[allow(unused_imports)]
pub use area8::*;
#[allow(unused_imports)]
pub use arena::*;
#[allow(unused_imports)]
pub use bank::*;
#[allow(unused_imports)]
pub use clubmaster::*;
#[allow(unused_imports)]
pub use gate_fight::*;
#[allow(unused_imports)]
pub use gate_welcome::*;
#[allow(unused_imports)]
pub use janitor::*;
#[allow(unused_imports)]
pub use lostcon::*;
#[allow(unused_imports)]
pub use macro_npc::*;
#[allow(unused_imports)]
pub use merchant::*;
#[allow(unused_imports)]
pub use trader::*;
