use super::*;

/// C `give_exp(cn, val)` (`src/system/tool.c:1371-1423`). Thin wrapper
/// around the canonical `World::give_exp` (`ugaris-core/src/world/exp.rs`),
/// which now owns the full algorithm (multipliers read from
/// `world.settings.exp_modifier`/`hardcore_exp_bonus`, the `CF_NOLEVEL`
/// exp-band clamp, and the `check_levelup` tail call) so it is usable both
/// from server-crate call sites and from `ugaris-core` item drivers, which
/// only ever have `&mut World` available. Kept as a named wrapper (rather
/// than inlining `world.give_exp` at every call site) purely so call sites
/// read the same as their C `give_exp(cn, val)` counterparts.
pub(crate) fn give_exp_with_runtime_modifiers(
    world: &mut World,
    character_id: CharacterId,
    base_exp: i64,
    area_id: u32,
) {
    world.give_exp(character_id, base_exp, area_id);
}

pub(crate) fn parse_exp_command_target(
    world: &World,
    character_id: CharacterId,
    rest: &str,
) -> (CharacterId, String, i64) {
    let mut text = rest.trim_start();
    if text.is_empty() || text.as_bytes().first().is_some_and(u8::is_ascii_digit) {
        let name = world
            .characters
            .get(&character_id)
            .map(|character| character.name.clone())
            .unwrap_or_default();
        return (character_id, name, legacy_atoi_prefix(text));
    }

    let mut split = text.splitn(2, char::is_whitespace);
    let name = split.next().unwrap_or_default();
    text = split.next().unwrap_or_default();
    let target_id = find_online_character_by_name(world, name).unwrap_or(CharacterId(0));
    (target_id, name.to_string(), legacy_atoi_prefix(text))
}
