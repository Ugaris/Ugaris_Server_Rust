use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClientEffectSlot {
    effect_id: u32,
    serial: i32,
    body: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClientEffectCache {
    pub(crate) slots: Vec<Option<ClientEffectSlot>>,
    used_mask: u64,
}

impl Default for ClientEffectCache {
    fn default() -> Self {
        Self {
            slots: vec![None; MAX_CLIENT_EFFECTS],
            used_mask: 0,
        }
    }
}

pub(crate) fn client_effect_payloads(
    world: &World,
    viewer: &Character,
    view_distance: usize,
    cache: &mut ClientEffectCache,
) -> Vec<bytes::BytesMut> {
    let mut visible_effects: Vec<_> = world
        .effects
        .iter()
        .filter_map(|(&effect_id, effect)| {
            visible_client_effect_body(effect_id, effect, world, viewer, view_distance).map(
                |body| {
                    (
                        effect_id,
                        effect.serial,
                        body.into_iter().collect::<Vec<u8>>(),
                    )
                },
            )
        })
        .collect();
    visible_effects.sort_by_key(|(effect_id, _, _)| *effect_id);
    visible_effects.truncate(MAX_CLIENT_EFFECTS);

    let mut payloads = Vec::new();
    let mut used = vec![false; cache.slots.len()];
    let mut pending = Vec::new();

    for (effect_id, serial, body) in visible_effects {
        if let Some(slot_index) = cache.slots.iter().position(|slot| {
            slot.as_ref()
                .is_some_and(|slot| slot.effect_id == effect_id)
        }) {
            used[slot_index] = true;
            let slot = cache.slots[slot_index].as_mut().expect("slot exists");
            if slot.serial != serial || slot.body != body {
                payloads.push(ugaris_protocol::packet::client_effect(
                    slot_index as u8,
                    &body,
                ));
                slot.serial = serial;
                slot.body = body;
            }
        } else {
            pending.push((effect_id, serial, body));
        }
    }

    for (slot_index, slot) in cache.slots.iter_mut().enumerate() {
        if !used[slot_index] {
            *slot = None;
        }
    }

    for (effect_id, serial, body) in pending {
        let Some(slot_index) = used.iter().position(|used| !*used) else {
            break;
        };
        used[slot_index] = true;
        cache.slots[slot_index] = Some(ClientEffectSlot {
            effect_id,
            serial,
            body: body.clone(),
        });
        payloads.push(ugaris_protocol::packet::client_effect(
            slot_index as u8,
            &body,
        ));
    }

    let used_mask =
        used.iter().enumerate().fold(
            0_u64,
            |mask, (index, used)| {
                if *used {
                    mask | (1_u64 << index)
                } else {
                    mask
                }
            },
        );
    if used_mask != cache.used_mask {
        cache.used_mask = used_mask;
        payloads.push(bytes::BytesMut::from(
            &ugaris_protocol::packet::used_effects(used_mask)[..],
        ));
    }

    payloads
}

pub(crate) fn visible_client_effect_body(
    effect_id: u32,
    effect: &Effect,
    world: &World,
    viewer: &Character,
    view_distance: usize,
) -> Option<bytes::BytesMut> {
    if !effect_visible_to_viewer(effect, world, viewer, view_distance) {
        return None;
    }

    match effect.effect_type {
        EF_MAGICSHIELD => Some(ugaris_protocol::packet::ceffect_shield(
            effect_id as i32,
            effect_character_id(effect)?.0 as i32,
            effect.start_tick,
        )),
        EF_BALL => Some(ugaris_protocol::packet::ceffect_ball(
            effect_id as i32,
            effect.start_tick,
            effect.from_x,
            effect.from_y,
            effect.to_x,
            effect.to_y,
        )),
        EF_FIREBALL => Some(ugaris_protocol::packet::ceffect_fireball(
            effect_id as i32,
            effect.start_tick,
            effect.from_x,
            effect.from_y,
            effect.to_x,
            effect.to_y,
        )),
        EF_FLASH => Some(ugaris_protocol::packet::ceffect_flash(
            effect_id as i32,
            effect_character_id(effect)?.0 as i32,
        )),
        EF_WARCRY => Some(ugaris_protocol::packet::ceffect_warcry(
            effect_id as i32,
            effect_character_id(effect)?.0 as i32,
            effect.stop_tick,
        )),
        EF_BLESS => Some(ugaris_protocol::packet::ceffect_bless(
            effect_id as i32,
            effect_character_id(effect)?.0 as i32,
            effect.start_tick,
            effect.stop_tick,
            effect.strength,
        )),
        EF_HEAL => Some(ugaris_protocol::packet::ceffect_heal(
            effect_id as i32,
            effect_character_id(effect)?.0 as i32,
            effect.start_tick,
        )),
        EF_FREEZE => Some(ugaris_protocol::packet::ceffect_freeze(
            effect_id as i32,
            effect_character_id(effect)?.0 as i32,
            effect.start_tick,
            effect.stop_tick,
        )),
        EF_STRIKE => Some(ugaris_protocol::packet::ceffect_strike(
            effect_id as i32,
            effect
                .target_character
                .map(|character_id| character_id.0 as i32)
                .unwrap_or_default(),
            effect.x,
            effect.y,
        )),
        EF_BURN => Some(ugaris_protocol::packet::ceffect_burn(
            effect_id as i32,
            effect_character_id(effect)?.0 as i32,
            effect.stop_tick,
        )),
        EF_POTION => Some(ugaris_protocol::packet::ceffect_potion(
            effect_id as i32,
            effect_character_id(effect)?.0 as i32,
            effect.start_tick,
            effect.stop_tick,
            effect.strength,
        )),
        EF_PULSE => Some(ugaris_protocol::packet::ceffect_pulse(
            effect_id as i32,
            effect.start_tick,
        )),
        EF_PULSEBACK => Some(ugaris_protocol::packet::ceffect_pulseback(
            effect_id as i32,
            effect_character_id(effect)?.0 as i32,
            effect.x,
            effect.y,
        )),
        EF_FIRERING => Some(ugaris_protocol::packet::ceffect_firering(
            effect_id as i32,
            effect_character_id(effect)?.0 as i32,
            effect.start_tick,
        )),
        EF_EXPLODE => Some(ugaris_protocol::packet::ceffect_explode(
            effect_id as i32,
            effect.start_tick,
            effect.base_sprite,
        )),
        EF_MIST => Some(ugaris_protocol::packet::ceffect_mist(
            effect_id as i32,
            effect.start_tick,
        )),
        EF_EARTHRAIN => Some(ugaris_protocol::packet::ceffect_earthrain(
            effect_id as i32,
            effect.strength,
        )),
        EF_EARTHMUD => Some(ugaris_protocol::packet::ceffect_earthmud(effect_id as i32)),
        EF_EDEMONBALL => Some(ugaris_protocol::packet::ceffect_edemonball(
            effect_id as i32,
            effect.start_tick,
            effect.base_sprite,
            effect.from_x,
            effect.from_y,
            effect.to_x,
            effect.to_y,
        )),
        EF_CURSE => Some(ugaris_protocol::packet::ceffect_curse(
            effect_id as i32,
            effect_character_id(effect)?.0 as i32,
            effect.start_tick,
            effect.stop_tick,
            effect.strength,
        )),
        EF_CAP => Some(ugaris_protocol::packet::ceffect_cap(
            effect_id as i32,
            effect_character_id(effect)?.0 as i32,
        )),
        EF_LAG => Some(ugaris_protocol::packet::ceffect_lag(
            effect_id as i32,
            effect_character_id(effect)?.0 as i32,
        )),
        EF_BUBBLE => Some(ugaris_protocol::packet::ceffect_bubble(
            effect_id as i32,
            effect.strength,
        )),
        _ => None,
    }
}

pub(crate) fn effect_character_id(effect: &Effect) -> Option<CharacterId> {
    effect.target_character.or(effect.caster)
}

pub(crate) fn effect_visible_to_viewer(
    effect: &Effect,
    world: &World,
    viewer: &Character,
    view_distance: usize,
) -> bool {
    let (x, y) = match effect.effect_type {
        EF_BALL | EF_FIREBALL | EF_EDEMONBALL => (effect.x / 1024, effect.y / 1024),
        EF_STRIKE | EF_PULSE | EF_EXPLODE | EF_MIST | EF_EARTHRAIN | EF_EARTHMUD | EF_BUBBLE => {
            (effect.x, effect.y)
        }
        EF_MAGICSHIELD | EF_FLASH | EF_WARCRY | EF_BLESS | EF_HEAL | EF_FREEZE | EF_BURN
        | EF_POTION | EF_CURSE | EF_CAP | EF_LAG | EF_PULSEBACK | EF_FIRERING => {
            let Some(character_id) = effect_character_id(effect) else {
                return false;
            };
            let Some(character) = world.characters.get(&character_id) else {
                return false;
            };
            (i32::from(character.x), i32::from(character.y))
        }
        _ => return false,
    };
    let (Ok(x), Ok(y)) = (usize::try_from(x), usize::try_from(y)) else {
        return false;
    };
    map_position_in_diamond(x, y, viewer.x, viewer.y, view_distance)
}
