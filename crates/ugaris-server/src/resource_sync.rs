use super::*;

/// C: `player_stats()` in `src/system/player.c:2944` pushes value/HP/
/// endurance/mana/exp/gold/item packets to the owning session whenever
/// `CF_UPDATE`/`CF_ITEMS` are set on the character, then clears the flags.
/// C keeps a per-session shadow of every field it last sent and diffs
/// field-by-field; Rust has no such shadow cache yet, so instead of diffing
/// we send a full snapshot of the flagged category (same shape as the
/// login payload built by `login_payload`/`inventory_snapshot_payload`)
/// whenever the flag is set. This still preserves C's flag-gating
/// semantics: nothing is sent for a session unless `UPDATE` or `ITEMS` is
/// actually set, and the flag is cleared immediately after sending.
pub(crate) fn queue_resource_sync_frames(runtime: &mut ServerRuntime, world: &mut World) -> usize {
    let sessions: Vec<_> = runtime
        .players
        .iter()
        .filter_map(|(&session_id, player)| {
            if player.state != PlayerConnectionState::Normal {
                return None;
            }
            Some((session_id, player.character_id?))
        })
        .collect();

    let mut sent_sessions = 0;
    for (session_id, character_id) in sessions {
        let Some(character) = world.characters.get(&character_id) else {
            continue;
        };
        let send_values = character.flags.contains(CharacterFlags::UPDATE);
        let send_items = character.flags.contains(CharacterFlags::ITEMS);
        if !send_values && !send_items {
            continue;
        }

        let mut builder = PacketBuilder::new();
        if send_values {
            for value in 0..ugaris_core::entity::CHARACTER_VALUE_COUNT {
                builder.set_value0(value as u8, character.values[0][value]);
                builder.set_value1(value as u8, character.values[1][value]);
            }
            builder
                .set_hp((character.hp / POWERSCALE) as u16)
                .set_endurance((character.endurance / POWERSCALE) as u16)
                .set_mana((character.mana / POWERSCALE) as u16)
                .set_lifeshield((character.lifeshield / POWERSCALE) as u16)
                .exp(character.exp)
                .exp_used(character.exp_used);
        }
        if send_items {
            let (cursor_sprite, cursor_flags) = character
                .cursor_item
                .and_then(|item_id| item_packet_fields(world, item_id))
                .unwrap_or((0, 0));
            builder.set_cursor_item(cursor_sprite, cursor_flags);

            for slot in 0..character.inventory.len().min(u8::MAX as usize + 1) {
                let (sprite, flags) = character.inventory[slot]
                    .and_then(|item_id| item_packet_fields(world, item_id))
                    .unwrap_or((0, 0));
                builder.set_item(slot as u8, sprite, flags);
            }
            builder.gold(character.gold);
        }

        if runtime.send_to_session(session_id, builder.into_payload()) {
            sent_sessions += 1;
        }

        if let Some(character) = world.characters.get_mut(&character_id) {
            let mut clear = CharacterFlags::empty();
            if send_values {
                clear |= CharacterFlags::UPDATE;
            }
            if send_items {
                clear |= CharacterFlags::ITEMS;
            }
            character.flags.remove(clear);
        }
    }

    sent_sessions
}
