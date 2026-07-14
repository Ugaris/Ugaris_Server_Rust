use std::ops::ControlFlow;

use super::*;

pub(super) fn dispatch_ppd_misc(
    world: &mut World,
    runtime: &mut ServerRuntime,
    character_id: CharacterId,
    _area_id: u32,
    lower: &str,
    rest: &str,
) -> ControlFlow<Option<KeyringCommandResult>> {
    // C `/showppd <name> <ppd>` (`command.c:8790-8837` dispatch,
    // `cmdcmp(ptr, "showppd", 7)` - `minlen` == `strlen("showppd")`, exact
    // word only, `CF_GOD`-gated) + `cmd_showppd` (`command.c:275-346`): an
    // online-only (not `lookup_name`-backed, unlike most other by-name
    // debug commands) `CF_GOD` debug dump of one named `struct` PPD block
    // for a target character. Only two PPD names are recognized in the C
    // source (verified by reading the whole function): `area1` prints
    // every field of `struct area1_ppd`, `area3` prints only
    // `kassim_state` out of `struct area3_ppd` (the other 17 fields of
    // that struct are simply never read by this command). Name/ppd-name
    // parsing mirrors C's own `isalpha`/`isalpha-or-isdigit` scan loops
    // exactly (`take_legacy_alpha_name`/`take_legacy_alnum_name`), and the
    // "not found"/"which ppd"/"no ppd by that name" messages are checked
    // in the same order C does: online-name lookup first, then the
    // remaining-argument-empty check, then the ppd-name match.
    if lower == "showppd" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        let (name, remainder) = take_legacy_alpha_name(rest.trim_start());
        let Some(target_id) = find_online_character_by_name(world, name) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!(
                    "Sorry, no player by the name {name} online (offline chars not possible)."
                )],
                ..Default::default()
            }));
        };
        let ppd_rest = remainder.trim_start();
        if ppd_rest.is_empty() {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Which ppd?".to_string()],
                ..Default::default()
            }));
        }
        let (ppd_name, _) = take_legacy_alnum_name(ppd_rest);
        let target_name = world
            .characters
            .get(&target_id)
            .map(|character| character.name.clone())
            .unwrap_or_default();
        let messages = if ppd_name.eq_ignore_ascii_case("area1") {
            match runtime.player_for_character(target_id) {
                Some(player) => vec![
                    format!("Area1 ppd of {target_name}"),
                    format!(
                        "Yoakin state: {}, Yoakin seen timer: {}, Greeter state: {}, Greeter seen timer: {}",
                        player.area1_yoakin_state(),
                        player.area1_yoakin_seen_timer(),
                        player.area1_greeter_state(),
                        player.area1_greeter_seen_timer(),
                    ),
                    format!(
                        "AClerk state: {}, AClerk seen timer: {}, Cameron Hermit state: {}, Cameron Hermit seen timer: {}, Cameron Hermit kill count: {}",
                        player.area1_aclerk_state(),
                        player.area1_aclerk_seen_timer(),
                        player.area1_camhermit_state(),
                        player.area1_camhermit_seen_timer(),
                        player.area1_camhermit_kills(),
                    ),
                    format!(
                        "Jessica state: {}, Jessica seen timer: {}, Gwendolyn state: {}, Gwendolyn seen timer: {}",
                        player.area1_jessica_state(),
                        player.area1_jessica_seen_timer(),
                        player.area1_gwendy_state(),
                        player.area1_gwendy_seen_timer(),
                    ),
                    format!(
                        "Gerewin state: {}, Gerewin seen timer: {}, Lydia state: {}, Lydia seen timer: {}",
                        player.area1_gerewin_state(),
                        player.area1_gerewin_seen_timer(),
                        player.area1_lydia_state(),
                        player.area1_lydia_seen_timer(),
                    ),
                    format!(
                        "Asturin state: {}, Asturin seen timer: {}, Guiwynn state: {}, Guiwynn seen timer: {}",
                        player.area1_asturin_state(),
                        player.area1_asturin_seen_timer(),
                        player.area1_guiwynn_state(),
                        player.area1_guiwynn_seen_timer(),
                    ),
                    format!(
                        "Logain state: {}, Logain seen timer: {}, Brithildie state: {}, Brithildie seen timer: {}",
                        player.area1_logain_state(),
                        player.area1_logain_seen_timer(),
                        player.area1_brithildie_state(),
                        player.area1_brithildie_seen_timer(),
                    ),
                    format!(
                        "Jiu state: {}, Jiu seen timer: {}, Nook state: {}, Darkin state: {}",
                        player.area1_jiu_state(),
                        player.area1_jiu_seen_timer(),
                        player.area1_nook_state(),
                        player.area1_darkin_state(),
                    ),
                    format!(
                        "Terion state: {}, Shrike state: {}, Shrike fails: {}",
                        player.area1_terion_state(),
                        player.area1_shrike_state(),
                        player.area1_shrike_fails(),
                    ),
                    format!(
                        "Reskin state: {}, Reskin seen timer: {}, Reskin got bits: {}",
                        player.area1_reskin_state(),
                        player.area1_reskin_seen_timer(),
                        player.area1_reskin_got_bits(),
                    ),
                    format!(
                        "James state: {}, James flags: {}",
                        player.area1_james_state(),
                        player.area1_flags(),
                    ),
                ],
                None => vec![format!("Reading PPD {ppd_name} failed.")],
            }
        } else if ppd_name.eq_ignore_ascii_case("area3") {
            match runtime.player_for_character(target_id) {
                Some(player) => vec![format!("Kassim state: {}", player.area3_kassim_state())],
                None => vec![format!("Reading PPD {ppd_name} failed.")],
            }
        } else {
            vec![format!("Sorry, no ppd by the name {ppd_name}.")]
        };
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages,
            ..Default::default()
        }));
    }

    // C `/noarch` (`command.c:9049-9057`, `CF_GOD`-gated, `cmdcmp(ptr,
    // "noarch", 6)` - `minlen == strlen("noarch")`, exact word only) plus
    // `cmd_noarch` (`command.c:3163-3192`): looks up an online character by
    // (case-insensitive) name - no self-fallback, a bare `/noarch` with no
    // name resolves an empty-string lookup that never matches any real
    // character name, reporting "Sorry, no one by the name  around." with
    // C's characteristic double space (`name` is empty, and its own
    // `log_char` format string has a single literal space before `%s`) -
    // then caps every one of the target's `value[1][0..=V_IMMUNITY]`
    // entries (indices `0..=37`, i.e. `CharacterValue::Hp` through
    // `CharacterValue::Immunity` inclusive) at `50` and clears `CF_ARCH`.
    // Unlike every other admin command in this file, C sends no
    // confirmation message at all on success - only the not-found error is
    // ever logged, and only to the caller.
    if lower == "noarch" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        let (name, _) = take_legacy_alpha_name(rest.trim_start());
        let Some(target_id) = find_online_character_by_name(world, name) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {name} around.")],
                ..Default::default()
            }));
        };
        let Some(target) = world.characters.get_mut(&target_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {name} around.")],
                ..Default::default()
            }));
        };
        for value in target.values[1]
            .iter_mut()
            .take(CharacterValue::Immunity as usize + 1)
        {
            if *value > 50 {
                *value = 50;
            }
        }
        target.flags.remove(CharacterFlags::ARCH);
        return ControlFlow::Break(Some(KeyringCommandResult::default()));
    }

    // C `/noprof` (`command.c:9226-9235`, `CF_GOD`-gated, `cmdcmp(ptr,
    // "noprof", 6)`, exact word only): unlike `/noarch` above, this takes
    // no argument at all and never advances `ptr` past the matched word,
    // so it always operates on the caller (`ch[cn]`) itself, never a named
    // target - resets every one of the caller's own `prof[0..P_MAX]`
    // entries (`PROFESSION_COUNT` = 20 here) to `0` and sets `CF_PROF`
    // (client refresh flag, a no-op here since this codebase has no
    // separate "dirty" flag propagation for professions). No message is
    // sent to the caller on success, matching C exactly.
    if lower == "noprof" {
        let Some(caller) = world.characters.get_mut(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        for profession in caller.professions.iter_mut() {
            *profession = 0;
        }
        return ControlFlow::Break(Some(KeyringCommandResult::default()));
    }

    // C `/fixit` (`command.c:9058-9066`, `CF_GOD`-gated, `cmdcmp(ptr,
    // "fixit", 5)` - exact word only) plus `cmd_reset_questlog`
    // (`command.c:3194-3218`): looks up an *online* character by name
    // (alpha-only prefix, matching `take_legacy_alpha_name`; C's
    // `strcasecmp` requires an exact match against the full character
    // name, no self-fallback), reports "Sorry, no one by the name %s
    // around." on failure, otherwise wipes the target's entire quest log
    // PPD (`del_data`, reproduced as `QuestLog::default()`), fully
    // re-derives it (`questlog_init`, reproduced as
    // `PlayerRuntime::init_questlog`, which now actually runs since the
    // sentinel was just cleared by the wipe) and resends the fresh quest
    // log to the TARGET (`sendquestlog(co, ch[co].player)` - unlike
    // `/questfix` right below, this one operates on the right character
    // throughout).
    if lower == "fixit" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        let (name, _) = take_legacy_alpha_name(rest.trim_start());
        let Some(target_id) = find_online_character_by_name(world, name) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {name} around.")],
                ..Default::default()
            }));
        };
        if let Some(target_player) = runtime.player_for_character_mut(target_id) {
            target_player.quest_log = QuestLog::default();
            target_player.init_questlog();
            let payload = legacy_questlog_payload(target_player);
            for (session_id, _) in runtime.sessions_for_character(target_id) {
                runtime.send_to_session(session_id, payload.clone());
            }
        }
        return ControlFlow::Break(Some(KeyringCommandResult::default()));
    }

    // C `/questfix` (`command.c:9067-9075`, `CF_GOD`-gated, `cmdcmp(ptr,
    // "questfix", 8)` - exact word only) plus `cmd_reset_last_quest`
    // (`command.c:3221-3251`): shares `/fixit`'s name-lookup/not-found
    // path above, but its action is a genuine C bug - `set_data` is
    // called with the ACTING character `cn`, not the looked-up target
    // `co`, so it clears the CALLER's own quest-log init-complete
    // sentinel (`quest[MAXQUEST - 1].done = 0`), then calls
    // `questlog_init(co)` on the target (almost always a no-op, since an
    // online character's sentinel is virtually always already set), and
    // finally resends the CALLER's own now-desynced quest log
    // (`sendquestlog(cn, ch[cn].player)`). The practical effect: the
    // named argument only serves as an online-character existence check;
    // the caller's own quest log gets marked for full re-derivation on
    // their *next* login (the immediate resend still reflects the
    // unchanged pre-existing entries, since `init_questlog` is never
    // called on `cn` here). Reproduced verbatim, bug and all.
    if lower == "questfix" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        let (name, _) = take_legacy_alpha_name(rest.trim_start());
        let Some(target_id) = find_online_character_by_name(world, name) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {name} around.")],
                ..Default::default()
            }));
        };
        if let Some(target_player) = runtime.player_for_character_mut(target_id) {
            target_player.init_questlog();
        }
        if let Some(caller_player) = runtime.player_for_character_mut(character_id) {
            caller_player.quest_log.clear_init_complete();
            let payload = legacy_questlog_payload(caller_player);
            for (session_id, _) in runtime.sessions_for_character(character_id) {
                runtime.send_to_session(session_id, payload.clone());
            }
        }
        return ControlFlow::Break(Some(KeyringCommandResult::default()));
    }

    // C `#ls <name> <dir>` / `#cat <name> <file>` (`command.c:9237-9253`
    // dispatch, `CF_GOD`-gated, `cmdcmp(ptr, "#ls", 3)`/`cmdcmp(ptr,
    // "#cat", 4)` - both exact-word only, no abbreviation) plus
    // `cmd_ls`/`cmd_cat` (`command.c:2794-2845`): a debug feature that
    // asks the TARGET character's own game client to list a directory
    // (`#ls`) or dump a file's contents (`#cat`) from the *client's*
    // local disk, not the server's - `plr_ls`/`plr_cat`
    // (`src/system/player.c:3750-3789`) just forward a raw `SV_LS`/
    // `SV_CAT` request packet to the target's connection; any actual
    // listing/content comes back later as a separate client-originated
    // packet this codebase does not yet parse (out of scope here, same
    // as the C dispatcher itself which never processes a reply). The
    // target name is matched by C's `getfirst_char`/`getnext_char` loop
    // with no `CF_PLAYER` filter (`find_online_character_by_name`
    // already replicates this - NPCs are valid targets too, they just
    // never have a live connection to actually receive anything), parsed
    // via `isalpha`-only `take_legacy_alpha_name` exactly like
    // `/fixit`/`/questfix` above. Unlike those two, the not-found message
    // here is `"Sorry, no one by the name {name} around."` (matches this
    // pair's own `log_char`, not `/clearppd`'s distinct "Player '...' not
    // found." text). The remainder after the name and its trailing
    // whitespace is the `dir`/`file` argument verbatim (may itself
    // contain spaces, never re-tokenized in C). C unconditionally logs
    // `"ls {dir} scheduled on {target}."` / `"cat {file} scheduled on
    // {target}."` to the caller once a target is found, even when
    // `plr_ls`/`plr_cat` internally no-ops (target has no live client
    // connection, i.e. `ch[co].player == 0` - modeled here as
    // `sessions_for_character` returning empty - or `dir`/`file` exceeds
    // the 200-byte cutoff `remote_fs_request` enforces) - reproduced by
    // sending the packet only when a session exists and the byte-count
    // check passes, but always returning the confirmation message
    // regardless.
    if lower == "ls" || lower == "cat" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        let (name, after_name) = take_legacy_alpha_name(rest.trim_start());
        let Some(target_id) = find_online_character_by_name(world, name) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {name} around.")],
                ..Default::default()
            }));
        };
        let target_name = world.characters[&target_id].name.clone();
        let dir = after_name.trim_start();
        let mut builder = PacketBuilder::new();
        let sent = if lower == "ls" {
            builder.ls_request(dir)
        } else {
            builder.cat_request(dir)
        };
        if sent {
            let payload = builder.into_payload();
            for (session_id, _) in runtime.sessions_for_character(target_id) {
                runtime.send_to_session(session_id, payload.clone());
            }
        }
        let verb_word = if lower == "ls" { "ls" } else { "cat" };
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages: vec![format!("{verb_word} {dir} scheduled on {target_name}.")],
            ..Default::default()
        }));
    }

    ControlFlow::Continue(())
}

pub(super) fn dispatch_clearppd(
    world: &mut World,
    runtime: &mut ServerRuntime,
    character_id: CharacterId,
    _area_id: u32,
    lower: &str,
    rest: &str,
) -> ControlFlow<Option<KeyringCommandResult>> {
    // C `/clearppd <ppdname> [player]` (`command.c:10144-10146` dispatch,
    // `CF_GOD | CF_STAFF`-gated, `cmdcmp(ptr, "clearppd", 8)` - exact word
    // only; `cmd_clearppd`, `command.c:4214-4288`). A raw, PPD-name-
    // agnostic admin wipe over C's generic `del_data(co, ppd_id)` linked-
    // list removal - unlike every other command in this file, it performs
    // NO resend of the cleared data to either party (verified by reading
    // the whole C function body: no `send*`/`log_char` other than the
    // three messages reproduced below). Supports exactly three PPD names
    // (`keyring`, `questlog`, `alias`), matched case-insensitively.  An
    // optional second, whitespace-separated argument targets an online
    // *player* character (`ch[co].flags & CF_PLAYER`, so - unlike most
    // name-lookup commands in this file - NPCs never match) by exact
    // case-insensitive full-string match against the ENTIRE remaining
    // text (C's `strcasecmp(ch[co].name, ptr)`, not just a leading name
    // token - so any trailing text after a valid name breaks the match, a
    // genuine quirk reproduced here by using the raw trimmed remainder
    // rather than `take_legacy_alpha_name`); the miss message is "Player
    // '%s' not found." (deliberately distinct from every other command's
    // "Sorry, no one by the name %s around." - copied letter for
    // letter). Self-targets when no second argument is given. Since Rust
    // keeps these three PPDs as always-present plain fields rather than
    // lazily-allocated `del_data` blocks, "the PPD existed" (`del_data`'s
    // nonzero return) is modeled as "the field is currently non-default"
    // - exactly the set of players for whom C would actually have called
    // `set_data` at least once - so the found/not-found message split
    // matches observable behavior.
    if lower == "clearppd" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
        {
            return ControlFlow::Break(None);
        }
        let caller_name = caller.name.clone();

        let rest = rest.trim_start();
        if rest.is_empty() {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![
                    "Usage: #clearppd <ppdname> [player]".to_string(),
                    "Available PPDs: keyring, questlog, alias".to_string(),
                ],
                ..Default::default()
            }));
        }

        let mut parts = rest.splitn(2, char::is_whitespace);
        let ppd_name = parts.next().unwrap_or("").to_ascii_lowercase();
        let player_arg = parts.next().unwrap_or("").trim_start();

        let (target_id, target_name) = if player_arg.is_empty() {
            (character_id, caller_name.clone())
        } else {
            let found = world.characters.values().find(|character| {
                character.flags.contains(CharacterFlags::PLAYER)
                    && character.name.eq_ignore_ascii_case(player_arg)
            });
            let Some(target) = found else {
                return ControlFlow::Break(Some(KeyringCommandResult {
                    messages: vec![format!("Player '{player_arg}' not found.")],
                    ..Default::default()
                }));
            };
            (target.id, target.name.clone())
        };

        let ppd_display_name = match ppd_name.as_str() {
            "keyring" => "keyring",
            "questlog" => "questlog",
            "alias" => "alias",
            _ => {
                return ControlFlow::Break(Some(KeyringCommandResult {
                    messages: vec![
                        format!("Unknown PPD: {ppd_name}"),
                        "Available PPDs: keyring, questlog, alias".to_string(),
                    ],
                    ..Default::default()
                }));
            }
        };

        let Some(target_player) = runtime.player_for_character_mut(target_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Failed to get player data.".to_string()],
                ..Default::default()
            }));
        };

        let existed = match ppd_display_name {
            "keyring" => !target_player.keyring.is_empty(),
            "questlog" => !target_player.quest_log.is_empty(),
            _ => !target_player.aliases.is_empty(),
        };
        if existed {
            match ppd_display_name {
                "keyring" => target_player.keyring.clear(),
                "questlog" => target_player.quest_log = QuestLog::default(),
                _ => target_player.aliases.clear(),
            }
        }

        let mut result = KeyringCommandResult::default();
        if existed {
            result
                .messages
                .push(format!("Cleared {ppd_display_name} PPD for {target_name}."));
            if target_id != character_id {
                result.other_messages.push((
                    target_id,
                    format!("Your {ppd_display_name} data has been cleared by {caller_name}."),
                ));
            }
        } else {
            result.messages.push(format!(
                "No {ppd_display_name} PPD found for {target_name}."
            ));
        }
        return ControlFlow::Break(Some(result));
    }

    ControlFlow::Continue(())
}
