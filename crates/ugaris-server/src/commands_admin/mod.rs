//! Admin/god text commands (`src/system/command.c` god sections).
//!
//! `character.rs` holds the giant `/command` multiplexer
//! (`apply_admin_character_command`); grants, tuning, and skill/exp helper
//! math live in sibling modules.

mod character;
mod exp;
mod grants;
mod skills;
mod status;
mod tuning;

pub(crate) use character::*;
pub(crate) use exp::*;
pub(crate) use grants::*;
pub(crate) use skills::*;
pub(crate) use status::*;
pub(crate) use tuning::*;

use super::*;
