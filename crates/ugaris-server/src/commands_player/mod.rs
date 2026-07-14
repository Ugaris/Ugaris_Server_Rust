//! Player-level legacy text commands (`src/system/command.c` non-god
//! sections), split by concern: `types` (KeyringCommandResult), `text`
//! (legacy string/number/message helpers), `settings` (client + player
//! preference toggles), `social` (lastseen/complain/pk/steal), `help`
//! (help/time text), `achievements`, and `progress`
//! (demonlords/orbs/treasures/tunnel trackers).

mod achievements;
mod help;
mod progress;
mod settings;
mod social;
mod text;
mod types;

pub(crate) use achievements::*;
pub(crate) use help::*;
pub(crate) use progress::*;
pub(crate) use settings::*;
pub(crate) use social::*;
pub(crate) use text::*;
pub(crate) use types::*;

use super::*;
