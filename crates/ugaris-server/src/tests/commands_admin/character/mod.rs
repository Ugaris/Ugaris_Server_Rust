//! Admin `character.rs` command tests, split by command family.

mod anticheat;
mod clan_club;
mod debug_reset;
mod flags;
mod items_world;
mod military;
mod misc_commands;
mod moderation;
mod pent_macro;
mod questlog_fix;
mod session;
mod shrine_tunnel;
mod teleport;

#[allow(unused_imports)]
use anticheat::*;
#[allow(unused_imports)]
use clan_club::*;
#[allow(unused_imports)]
use debug_reset::*;
#[allow(unused_imports)]
use flags::*;
#[allow(unused_imports)]
use items_world::*;
#[allow(unused_imports)]
use military::*;
#[allow(unused_imports)]
use misc_commands::*;
#[allow(unused_imports)]
use moderation::*;
#[allow(unused_imports)]
use pent_macro::*;
#[allow(unused_imports)]
use questlog_fix::*;
#[allow(unused_imports)]
use session::*;
#[allow(unused_imports)]
use shrine_tunnel::*;
#[allow(unused_imports)]
use teleport::*;

use super::*;

pub(crate) fn goto_test_world() -> World {
    let mut world = World::default();
    world.map = ugaris_core::map::MapGrid::new(300, 300);
    // Past the `/jump` busy window (`ticker - ch[cn].regen_ticker < TICKS *
    // 3`) so freshly logged-in test characters (`regen_ticker: 0`) aren't
    // considered "still catching their breath".
    world.tick.0 = TICKS_PER_SECOND * 10;
    world
}

pub(crate) fn setup_god_and_online_target(
    world: &mut World,
    runtime: &mut ServerRuntime,
) -> (CharacterId, CharacterId) {
    let god_id = CharacterId(7);
    let target_id = CharacterId(8);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    world.add_character(login_character(
        target_id,
        &login_block("Target"),
        1,
        11,
        10,
    ));
    let mut target_player = PlayerRuntime::connected(80, 0);
    target_player.character_id = Some(target_id);
    runtime.players.insert(80, target_player);
    (god_id, target_id)
}
