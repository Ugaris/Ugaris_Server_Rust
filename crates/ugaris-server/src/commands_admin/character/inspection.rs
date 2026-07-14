use std::ops::ControlFlow;

use super::*;

pub(super) fn dispatch_inspection(
    world: &mut World,
    _runtime: &mut ServerRuntime,
    character_id: CharacterId,
    _area_id: u32,
    lower: &str,
    rest: &str,
) -> ControlFlow<Option<KeyringCommandResult>> {
    // C `/look <name>` (`command.c:8990-9019`), `CF_GOD|CF_STAFF`-gated,
    // full-word only (`cmdcmp`'s `minlen` is 4, the full length of
    // "look", no abbreviation accepted). Unlike `/punish`'s `take_legacy_
    // alpha_name`, C passes its *entire*, untokenized trimmed remainder
    // to `lookup_name` (no alpha-only prefix extraction) - see `World::
    // queue_look_command`'s doc comment for why that's safe to reproduce
    // as a plain `trim_start()`. Always returns a `default()` result
    // immediately; every reply line arrives later via `World::
    // queue_system_text` (same fire-and-forget async pattern as
    // `/punish`/`/unpunish` above).
    if lower == "look" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
        {
            return ControlFlow::Break(None);
        }
        world.queue_look_command(character_id, rest.trim_start());
        return ControlFlow::Break(Some(KeyringCommandResult::default()));
    }

    // C `/klog` (`command.c:9022-9024` -> `karmalog`), `CF_GOD|CF_STAFF`-
    // gated, full-word only (`cmdcmp`'s `minlen` is 4, the full length of
    // "klog"). Takes no argument at all. Always returns a `default()`
    // result immediately; every reply line arrives later via `World::
    // queue_system_text` (same fire-and-forget async pattern as
    // `/look` above).
    if lower == "klog" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
        {
            return ControlFlow::Break(None);
        }
        world.queue_klog_command(character_id);
        return ControlFlow::Break(Some(KeyringCommandResult::default()));
    }

    // C `/values <name>` (`command.c:8391-8399` -> `look_values`,
    // `command.c:501-519`), `CF_GOD|CF_STAFF`-gated, full-word only
    // (`cmdcmp`'s `minlen` is 6, the full length of "values", no
    // abbreviation accepted - same idiom as `/look`/`/klog` above, not
    // `/showvalues`'s open-to-everyone abbreviation gate below). Trims
    // leading whitespace, then passes the entire untokenized remainder
    // to `World::queue_values_command` (see `world/values.rs`'s module
    // doc comment for the contrast with `/showvalues`'s caller/target
    // role swap - `/values` keeps the caller as the caller, showing the
    // resolved target's own stats). Always returns a `default()` result
    // immediately; every reply line arrives later via `World::
    // queue_system_text` (same fire-and-forget async pattern as
    // `/look`/`/klog`/`/showvalues` above).
    if lower == "values" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
        {
            return ControlFlow::Break(None);
        }
        world.queue_values_command(character_id, rest.trim_start());
        return ControlFlow::Break(Some(KeyringCommandResult::default()));
    }

    // C `/showvalues <name>` (`command.c:8401-8409` -> `show_values`,
    // `command.c:521-537`), no permission gate - unlike `/values`/`/look`/
    // `/klog`, any player can use this. Full-word *abbreviation*
    // (`cmdcmp(ptr, "showvalues", 4)`'s `minlen` is 4, only the length of
    // "show" - same idiom as the already-ported `/showattack` above,
    // `starts_with` rather than an exact `lower ==` match) - trims
    // leading whitespace, then passes the *entire* untokenized remainder
    // to `World::queue_showvalues_command` (see `world/values.rs`'s
    // module doc comment for the full behavior, including the caller/
    // target role swap between `show_values` and `show_values_bg`).
    // Always returns a `default()` result immediately; every reply line
    // arrives later via `World::queue_system_text` (same fire-and-forget
    // async pattern as `/look`/`/klog` above).
    if lower.len() >= 4 && "showvalues".starts_with(lower) {
        world.queue_showvalues_command(character_id, rest.trim_start());
        return ControlFlow::Break(Some(KeyringCommandResult::default()));
    }

    // C `/allow <name>` (`command.c:8371-8378` -> `allow_body`,
    // `src/system/death.c:1013-1029`), no permission gate - any player
    // can use this, same as `/showvalues` above. Full-word
    // *abbreviation* (`cmdcmp(ptr, "allow", 3)`'s `minlen` is 3, "all"
    // up to "allow" all match - same idiom as `/showvalues`'s
    // `starts_with` check) - trims leading whitespace, then passes the
    // entire untokenized remainder to `World::queue_allow_command` (see
    // `world/allow.rs`'s module doc comment for the full behavior:
    // grants the resolved target access to every grave the caller owns,
    // never the caller's own kills). Always returns a `default()` result
    // immediately; every reply line arrives later via `World::
    // queue_system_text` (same fire-and-forget async pattern as
    // `/look`/`/klog`/`/showvalues` above).
    if lower.len() >= 3 && "allow".starts_with(lower) {
        world.queue_allow_command(character_id, rest.trim_start());
        return ControlFlow::Break(Some(KeyringCommandResult::default()));
    }

    // C `/showflags` (`command.c:8798-8805`, `cmd_show_flags`,
    // `command.c:4839-5061`), `CF_GOD`-gated, full-word only (`cmdcmp`'s
    // `minlen` is 9, the full length of "showflags", so no abbreviation
    // is accepted - matched with `lower == "showflags"`, not
    // `starts_with`). Target is resolved by scanning every currently
    // loaded character (`getfirst_char`/`getnext_char`, no `CF_PLAYER`
    // filter - reused via `find_online_character_by_name`), by the
    // `isalpha`-only name token (`command.c:4845-4847`, trailing
    // non-alpha text is simply ignored). Every set bit is reported, one
    // per line, in C's exact `if (flags & CF_X)` declaration order - note
    // `CF_SPY` is (deliberately, matching C) never checked here.
    if lower == "showflags" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        let (name, _remainder) = take_legacy_alpha_name(rest.trim_start());
        let Some(target_id) = find_online_character_by_name(world, name) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {name} around.")],
                ..Default::default()
            }));
        };
        let target_flags = world.characters[&target_id].flags;
        let mut messages = vec![format!("Flags for player {name}:")];
        for (flag, label) in SHOW_FLAGS_ORDER {
            if target_flags.contains(*flag) {
                messages.push((*label).to_string());
            }
        }
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages,
            ..Default::default()
        }));
    }

    // C `/toggleflag` (`command.c:8807-8814`, `cmd_toggle_flag`,
    // `command.c:4784-4837`), `CF_GOD`-gated, full-word only (`minlen`
    // 10 == "toggleflag".len()). Name token is the same `isalpha`-only
    // scan as `/showflags`; the flag-name token that follows is C's
    // `!isspace`-only scan (`command.c:4799`, so it may contain digits
    // or punctuation, unlike the name), resolved case-insensitively via
    // [`character_flag_by_name`] (C `get_flag_by_name`,
    // `command.c:4590-4782` - also never maps `CF_SPY`). C additionally
    // calls `update_char(co)` when the toggled bit is `CF_UPDATE`,
    // `CF_ITEMS`, or `CF_PROF`, forcing an immediate client refresh
    // regardless of the toggle's new on/off state; this port only
    // toggles the in-memory bit (which the normal per-tick update
    // pipeline already consumes whenever it becomes set), so an
    // immediate refresh on the *clearing* transition is a known,
    // accepted gap for this rarely-used raw-flag debug command.
    if lower == "toggleflag" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        let rest = rest.trim_start();
        let (name, remainder) = take_legacy_alpha_name(rest);
        let flag_name = remainder.split_whitespace().next().unwrap_or("");
        let Some(target_id) = find_online_character_by_name(world, name) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {name} around.")],
                ..Default::default()
            }));
        };
        let Some(flag) = character_flag_by_name(flag_name) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Sorry, unknown flag: {flag_name}")],
                ..Default::default()
            }));
        };
        let target = world.characters.get_mut(&target_id).expect("just resolved");
        target.flags.toggle(flag);
        let state = if target.flags.contains(flag) {
            "ON"
        } else {
            "OFF"
        };
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages: vec![format!("Flag {flag_name} turned {state} for {name}")],
            ..Default::default()
        }));
    }

    ControlFlow::Continue(())
}

/// C `cmd_show_flags`'s exact `if (flags & CF_X)` declaration order
/// (`command.c:4871-5059`). `CF_SPY` is genuinely never checked by C
/// here (nor mapped by `get_flag_by_name`), so it is intentionally
/// absent from both this table and [`character_flag_by_name`].
pub(crate) const SHOW_FLAGS_ORDER: &[(CharacterFlags, &str)] = &[
    (CharacterFlags::USED, "USED"),
    (CharacterFlags::IMMORTAL, "IMMORTAL"),
    (CharacterFlags::GOD, "GOD"),
    (CharacterFlags::PLAYER, "PLAYER"),
    (CharacterFlags::STAFF, "STAFF"),
    (CharacterFlags::INVISIBLE, "INVISIBLE"),
    (CharacterFlags::SHUTUP, "SHUTUP"),
    (CharacterFlags::KICKED, "KICKED"),
    (CharacterFlags::UPDATE, "UPDATE"),
    (CharacterFlags::RESERVED0, "RESERVED0"),
    (CharacterFlags::RESERVED1, "RESERVED1"),
    (CharacterFlags::DEAD, "DEAD"),
    (CharacterFlags::ITEMS, "ITEMS"),
    (CharacterFlags::RESPAWN, "RESPAWN"),
    (CharacterFlags::MALE, "MALE"),
    (CharacterFlags::FEMALE, "FEMALE"),
    (CharacterFlags::WARRIOR, "WARRIOR"),
    (CharacterFlags::MAGE, "MAGE"),
    (CharacterFlags::ARCH, "ARCH"),
    (CharacterFlags::RESERVED2, "RESERVED2"),
    (CharacterFlags::NOATTACK, "NOATTACK"),
    (CharacterFlags::HASNAME, "HASNAME"),
    (CharacterFlags::QUESTITEM, "QUESTITEM"),
    (CharacterFlags::INFRARED, "INFRARED"),
    (CharacterFlags::PK, "PK"),
    (CharacterFlags::ITEMDEATH, "ITEMDEATH"),
    (CharacterFlags::NODEATH, "NODEATH"),
    (CharacterFlags::NOBODY, "NOBODY"),
    (CharacterFlags::EDEMON, "EDEMON"),
    (CharacterFlags::FDEMON, "FDEMON"),
    (CharacterFlags::IDEMON, "IDEMON"),
    (CharacterFlags::NOGIVE, "NOGIVE"),
    (CharacterFlags::PLAYERLIKE, "PLAYERLIKE"),
    (CharacterFlags::RESERVED3, "RESERVED3"),
    (CharacterFlags::PAID, "PAID"),
    (CharacterFlags::PROF, "PROF"),
    (CharacterFlags::ALIVE, "ALIVE"),
    (CharacterFlags::DEMON, "DEMON"),
    (CharacterFlags::UNDEAD, "UNDEAD"),
    (CharacterFlags::HARDKILL, "HARDKILL"),
    (CharacterFlags::NOBLESS, "NOBLESS"),
    (CharacterFlags::AREACHANGE, "AREACHANGE"),
    (CharacterFlags::LAG, "LAG"),
    (CharacterFlags::RESERVED4, "RESERVED4"),
    (CharacterFlags::THIEFMODE, "THIEFMODE"),
    (CharacterFlags::NOTELL, "NOTELL"),
    (CharacterFlags::INFRAVISION, "INFRAVISION"),
    (CharacterFlags::NOMAGIC, "NOMAGIC"),
    (CharacterFlags::NONOMAGIC, "NONOMAGIC"),
    (CharacterFlags::OXYGEN, "OXYGEN"),
    (CharacterFlags::NOPLRATT, "NOPLRATT"),
    (CharacterFlags::ALLOWSWAP, "ALLOWSWAP"),
    (CharacterFlags::LQMASTER, "LQMASTER"),
    (CharacterFlags::HARDCORE, "HARDCORE"),
    (CharacterFlags::NONOTIFY, "NONOTIFY"),
    (CharacterFlags::SMALLUPDATE, "SMALLUPDATE"),
    (CharacterFlags::NOWHO, "NOWHO"),
    (CharacterFlags::WON, "WON"),
    (CharacterFlags::NOEXP, "NOEXP"),
    (CharacterFlags::DEVELOPER, "DEVELOPER"),
    (CharacterFlags::EVENTMASTER, "EVENTMASTER"),
    (CharacterFlags::XRAY, "XRAY"),
    (CharacterFlags::NOLEVEL, "NOLEVEL"),
];

/// C `get_flag_by_name` (`command.c:4590-4782`), used only by
/// `/toggleflag`. Case-insensitive name -> flag-bit lookup; returns
/// `None` for an unknown name (C's `return 0`).
pub(crate) fn character_flag_by_name(name: &str) -> Option<CharacterFlags> {
    SHOW_FLAGS_ORDER
        .iter()
        .find(|(_, label)| label.eq_ignore_ascii_case(name))
        .map(|(flag, _)| *flag)
}
