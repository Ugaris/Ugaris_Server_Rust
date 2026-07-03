use super::*;

/// C `cl_kill`/`cl_give`/`player_driver_charspell` all capture the target's
/// *current* `ch[co].serial` synchronously while parsing the client packet,
/// before the action is queued or dispatched. Later, `player_driver.c`'s
/// `PAC_KILL` pre-switch guard (and `fireball_driver`/`ball_driver` for
/// character-targeted spells) compare that captured serial against the
/// live character to detect a target slot reused since the click. Look up
/// the live serial the same way here.
fn character_serial(characters: &HashMap<CharacterId, Character>, character: u16) -> u32 {
    characters
        .get(&CharacterId(u32::from(character)))
        .map(|character| character.serial)
        .unwrap_or(0)
}

pub(crate) fn apply_player_action(
    player: &mut PlayerRuntime,
    action: &ClientAction,
    current_tick: u64,
    characters: &HashMap<CharacterId, Character>,
) {
    match action {
        ClientAction::Move { x, y } => player.driver_move(*x as i32, *y as i32),
        ClientAction::Drop { x, y } => player.driver_drop(*x as i32, *y as i32),
        ClientAction::Teleport { teleport, mirror } => {
            player.driver_teleport((*teleport as i32) + (*mirror as i32 * 256));
        }
        ClientAction::WalkDir { direction } if *direction == 0 => {
            player.driver_stop(current_tick, false);
        }
        ClientAction::WalkDir { direction } => player.set_pending_action(QueuedAction {
            action: PlayerActionCode::WalkDir,
            arg1: *direction as i32,
            arg2: 0,
        }),
        ClientAction::MapSpell { spell, x, y } => {
            if *x == 0 {
                let serial = character_serial(characters, *y);
                player.driver_charspell(
                    spell_to_player_action(*spell, true),
                    ugaris_core::ids::CharacterId(*y as u32),
                    serial,
                );
            } else {
                player.driver_mapspell(spell_to_player_action(*spell, false), *x as i32, *y as i32);
            }
        }
        ClientAction::SelfSpell { spell } => {
            player.driver_selfspell(spell_to_player_action(*spell, false));
        }
        ClientAction::CharacterSpell { spell, character } => {
            let serial = character_serial(characters, *character);
            player.driver_charspell(
                spell_to_player_action(*spell, false),
                ugaris_core::ids::CharacterId(*character as u32),
                serial,
            );
        }
        ClientAction::Kill { character } => {
            let serial = character_serial(characters, *character);
            player.driver_kill(ugaris_core::ids::CharacterId(*character as u32), serial);
        }
        ClientAction::Give { character } => {
            let serial = character_serial(characters, *character);
            player.driver_give(ugaris_core::ids::CharacterId(*character as u32), serial);
        }
        ClientAction::Text(bytes) => player.command = bytes.clone(),
        ClientAction::Ticker { tick } => player.client_ticker = *tick,
        ClientAction::Stop => player.driver_stop(current_tick, false),
        _ => {
            if let Some(queued) = action_to_queued(action) {
                player.set_pending_action(queued);
            }
        }
    }
}

pub(crate) fn action_to_queued(action: &ClientAction) -> Option<QueuedAction> {
    let queued = match action {
        ClientAction::Move { x, y } => queued(PlayerActionCode::Move, *x, *y),
        ClientAction::Take { x, y } => queued(PlayerActionCode::Take, *x, *y),
        ClientAction::Drop { x, y } => queued(PlayerActionCode::Drop, *x, *y),
        ClientAction::Kill { character } => queued1(PlayerActionCode::Kill, *character),
        ClientAction::UseMap { x, y } => queued(PlayerActionCode::Use, *x, *y),
        ClientAction::CharacterSpell { spell, character } => {
            queued1(spell_to_player_action(*spell, true), *character)
        }
        ClientAction::MapSpell { spell, x, y } => {
            if *x == 0 {
                queued1(spell_to_player_action(*spell, true), *y)
            } else {
                queued(spell_to_player_action(*spell, false), *x, *y)
            }
        }
        ClientAction::SelfSpell { spell } => queued0(spell_to_player_action(*spell, false)),
        ClientAction::LookMap { x, y } => queued(PlayerActionCode::LookMap, *x, *y),
        ClientAction::Give { character } => queued1(PlayerActionCode::Give, *character),
        ClientAction::Teleport { teleport, mirror } => QueuedAction {
            action: PlayerActionCode::Teleport,
            arg1: (*teleport as i32) + (*mirror as i32 * 256),
            arg2: 0,
        },
        ClientAction::WalkDir { direction } if *direction != 0 => {
            queued1(PlayerActionCode::WalkDir, *direction as u16)
        }
        _ => return None,
    };
    Some(queued)
}

pub(crate) fn clear_completed_use_actions(
    runtime: &mut ServerRuntime,
    completed_actions: &[WorldActionCompletion],
) {
    for completion in completed_actions {
        if completion.action_id != action::USE {
            continue;
        }
        let Some(player) = runtime.player_for_character_mut(completion.character_id) else {
            continue;
        };
        if player.action.action == PlayerActionCode::Use {
            player.driver_halt();
        }
    }
}

pub(crate) fn spell_to_player_action(
    spell: SpellAction,
    character_target: bool,
) -> PlayerActionCode {
    match (spell, character_target) {
        (SpellAction::Bless, _) => PlayerActionCode::Bless,
        (SpellAction::Heal, _) => PlayerActionCode::Heal,
        (SpellAction::Freeze, _) => PlayerActionCode::Freeze,
        (SpellAction::Fireball, true) => PlayerActionCode::FireballCharacter,
        (SpellAction::Fireball, false) => PlayerActionCode::Fireball,
        (SpellAction::Ball, true) => PlayerActionCode::BallCharacter,
        (SpellAction::Ball, false) => PlayerActionCode::Ball,
        (SpellAction::MagicShield, _) => PlayerActionCode::MagicShield,
        (SpellAction::Flash, _) => PlayerActionCode::Flash,
        (SpellAction::Warcry, _) => PlayerActionCode::Warcry,
        (SpellAction::Pulse, _) => PlayerActionCode::Pulse,
    }
}

pub(crate) fn queued(action: PlayerActionCode, x: u16, y: u16) -> QueuedAction {
    QueuedAction {
        action,
        arg1: x as i32,
        arg2: y as i32,
    }
}

pub(crate) fn queued1(action: PlayerActionCode, arg: u16) -> QueuedAction {
    QueuedAction {
        action,
        arg1: arg as i32,
        arg2: 0,
    }
}

pub(crate) fn queued0(action: PlayerActionCode) -> QueuedAction {
    QueuedAction {
        action,
        arg1: 0,
        arg2: 0,
    }
}
