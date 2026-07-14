//! Admin command DB round-trips (`src/system/command.c` async
//! `lookup_name`/notes/punishment workers), split by concern:
//! `info` (lastseen/querystats/look/klog/values/allow), `transfers`
//! (jail + cross-area/eviction moves), `discipline` (flag/punish/
//! exterminate), `identity` (rename/lockname/rmdeath/complain).

mod discipline;
mod identity;
mod info;
mod transfers;

pub(crate) use discipline::*;
pub(crate) use identity::*;
pub(crate) use info::*;
pub(crate) use transfers::*;

use super::*;
