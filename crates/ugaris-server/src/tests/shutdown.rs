use super::*;

fn connect_player(runtime: &mut ServerRuntime, session_id: u64, character_id: CharacterId) {
    let (commands, _rx) = mpsc::channel(16);
    runtime.connect(session_id, commands, 0);
    if let Some(player) = runtime.players.get_mut(&session_id) {
        player.character_id = Some(character_id);
    }
}

// `COL_LIGHT_RED` embeds a raw `\xb0` color-marker byte that is not valid
// UTF-8 on its own, so tests work with the lossy-decoded text (fine for
// `contains`/`starts_with` checks against the plain-ASCII message content)
// plus the raw bytes (for verifying the color-marker prefix itself).
fn drain_shutdown_texts(world: &mut World) -> Vec<(CharacterId, Vec<u8>)> {
    world
        .drain_pending_system_text_bytes()
        .into_iter()
        .map(|event| (event.character_id, event.message))
        .collect()
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

// C `/shutdown` (`command.c:6068-6086`, `cmdcmp(ptr, "shutdown", 8)`) is
// `CF_GOD`-gated with no abbreviation accepted.
#[test]
fn shutdown_command_requires_god_and_exact_word() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    world.add_character(login_character(
        character_id,
        &login_block("Tester"),
        1,
        10,
        10,
    ));
    let mut runtime = ServerRuntime::default();

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/shutdown 15 20", 1)
            .is_none(),
        "non-god must not be able to schedule a shutdown"
    );
    assert_eq!(runtime.shutdown_at, 0);

    world
        .characters
        .get_mut(&character_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);

    // C `minlen` for "shutdown" is 8 (the full word) - an abbreviation like
    // "/shutdow" must fall through to whatever else (nothing here) rather
    // than being recognized.
    assert!(
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/shutdow 15", 1)
            .is_none(),
        "abbreviations of /shutdown must not be recognized"
    );
    assert_eq!(runtime.shutdown_at, 0);
}

// C `start_shutdown`/`shutdown_bg` (`command.c:541-557`, `system/tool.c:
// 3152-3164`): scheduling immediately broadcasts the countdown to every
// online player, defaults a zero/omitted downtime to 15 minutes, and the
// message is light-red colored.
#[test]
fn shutdown_command_schedules_and_broadcasts_immediately() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.flags.insert(CharacterFlags::GOD);
    world.add_character(character);
    let mut runtime = ServerRuntime::default();
    connect_player(&mut runtime, 1, character_id);

    let before = i64::from(current_realtime_seconds());
    apply_admin_character_command(&mut world, &mut runtime, character_id, "/shutdown 15 20", 1)
        .expect("god /shutdown should be recognized");

    assert!(runtime.shutdown_at >= before + 15 * 60);
    assert_eq!(runtime.shutdown_down_minutes, 20);
    assert!(
        !runtime.nologin,
        "15 minutes out should not yet block logins"
    );

    let texts = drain_shutdown_texts(&mut world);
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0].0, character_id);
    let message = lossy(&texts[0].1);
    assert!(
        message.contains("The server will go down in 15 minutes. Expected downtime: 20 minutes."),
        "unexpected message: {message}",
    );
    assert!(texts[0].1.starts_with(COL_LIGHT_RED));
}

// C `start_shutdown`: `if (!down) down = 15;` - an omitted/zero downtime
// argument defaults to 15 minutes.
#[test]
fn shutdown_command_defaults_missing_downtime_to_fifteen_minutes() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.flags.insert(CharacterFlags::GOD);
    world.add_character(character);
    let mut runtime = ServerRuntime::default();
    connect_player(&mut runtime, 1, character_id);

    apply_admin_character_command(&mut world, &mut runtime, character_id, "/shutdown 5", 1)
        .expect("god /shutdown should be recognized");

    assert_eq!(runtime.shutdown_down_minutes, 15);
    let texts = drain_shutdown_texts(&mut world);
    assert!(lossy(&texts[0].1).contains("Expected downtime: 15 minutes."));
}

// C `shutdown_bg`'s cancel branch (`t == 0`, `system/tool.c:3158-3164`): a
// bare `/shutdown` while nothing is scheduled is a silent no-op.
#[test]
fn shutdown_command_with_no_args_is_noop_when_nothing_scheduled() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.flags.insert(CharacterFlags::GOD);
    world.add_character(character);
    let mut runtime = ServerRuntime::default();
    connect_player(&mut runtime, 1, character_id);

    apply_admin_character_command(&mut world, &mut runtime, character_id, "/shutdown", 1)
        .expect("god /shutdown should be recognized");

    assert_eq!(runtime.shutdown_at, 0);
    assert!(drain_shutdown_texts(&mut world).is_empty());
}

// C `shutdown_bg`'s cancel branch again, this time with a shutdown already
// pending: cancels and broadcasts "Shutdown has been cancelled." to every
// online player.
#[test]
fn shutdown_command_with_no_args_cancels_pending_shutdown() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.flags.insert(CharacterFlags::GOD);
    world.add_character(character);
    let mut runtime = ServerRuntime::default();
    connect_player(&mut runtime, 1, character_id);

    apply_admin_character_command(&mut world, &mut runtime, character_id, "/shutdown 15 20", 1)
        .expect("god /shutdown should be recognized");
    drain_shutdown_texts(&mut world);

    apply_admin_character_command(&mut world, &mut runtime, character_id, "/shutdown", 1)
        .expect("god /shutdown should be recognized");

    assert_eq!(runtime.shutdown_at, 0);
    assert!(!runtime.nologin);
    let texts = drain_shutdown_texts(&mut world);
    assert_eq!(texts.len(), 1);
    assert!(lossy(&texts[0].1).contains("Shutdown has been cancelled."));
}

// C's `while (isdigit(*ptr)) ptr++;` in `command.c:6076` does not step over
// a leading `-` sign, so a negative `diff` leaves `down` parsed from the
// exact same substring - a real, reproducible C quirk, not a Rust bug.
#[test]
fn shutdown_command_negative_diff_reparses_down_from_same_text() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.flags.insert(CharacterFlags::GOD);
    world.add_character(character);
    let mut runtime = ServerRuntime::default();
    connect_player(&mut runtime, 1, character_id);

    apply_admin_character_command(&mut world, &mut runtime, character_id, "/shutdown -5 20", 1)
        .expect("god /shutdown should be recognized");

    // down_minutes also resolved to -5 (not 20), matching C's quirk, and
    // since -5 != 0 the "default to 15" branch does not trigger either.
    assert_eq!(runtime.shutdown_down_minutes, -5);
}

// C `shutdown_warn` (`system/tool.c:3120-3149`): re-broadcasts only when
// the remaining-minutes value changes, and sets `nologin` once the
// countdown drops under 3 minutes.
#[test]
fn tick_shutdown_scheduler_rebroadcasts_only_on_minute_change_and_blocks_logins_under_three_minutes(
) {
    let mut world = World::default();
    let character_id = CharacterId(7);
    world.add_character(login_character(
        character_id,
        &login_block("Tester"),
        1,
        10,
        10,
    ));
    let mut runtime = ServerRuntime::default();
    connect_player(&mut runtime, 1, character_id);

    let now = i64::from(current_realtime_seconds());
    runtime.shutdown_at = now + 150; // 2.5 minutes out -> rounds to 3.
    runtime.shutdown_down_minutes = 10;
    runtime.shutdown_warned_minutes = 999;

    assert!(!tick_shutdown_scheduler(&mut world, &mut runtime));
    let texts = drain_shutdown_texts(&mut world);
    assert_eq!(texts.len(), 1);
    assert!(lossy(&texts[0].1).contains("The server will go down in 3 minutes."));
    assert!(!runtime.nologin, "still 3 minutes out, logins stay open");

    // No time has passed and the warned-minutes value is unchanged, so a
    // second call must not re-broadcast.
    assert!(!tick_shutdown_scheduler(&mut world, &mut runtime));
    assert!(drain_shutdown_texts(&mut world).is_empty());

    // Move the deadline to under 3 minutes: now nologin engages.
    runtime.shutdown_at = now + 100;
    runtime.shutdown_warned_minutes = 999;
    assert!(!tick_shutdown_scheduler(&mut world, &mut runtime));
    assert!(runtime.nologin);
}

// C `shutdown_warn`: `if (realtime >= shutdown_at) quit = 1;` and the "will
// go down NOW" message (singular, no minute count) once the deadline has
// arrived.
#[test]
fn tick_shutdown_scheduler_signals_quit_and_now_message_at_deadline() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    world.add_character(login_character(
        character_id,
        &login_block("Tester"),
        1,
        10,
        10,
    ));
    let mut runtime = ServerRuntime::default();
    connect_player(&mut runtime, 1, character_id);

    let now = i64::from(current_realtime_seconds());
    runtime.shutdown_at = now - 5;
    runtime.shutdown_down_minutes = 10;
    runtime.shutdown_warned_minutes = 999;

    assert!(tick_shutdown_scheduler(&mut world, &mut runtime));
    let texts = drain_shutdown_texts(&mut world);
    assert_eq!(texts.len(), 1);
    assert!(
        lossy(&texts[0].1).contains("The server will go down NOW. Expected downtime: 10 minutes.")
    );
}

// `shutdown_at == 0` means "nothing scheduled" - the periodic check must be
// a complete no-op (no broadcast, no quit signal).
#[test]
fn tick_shutdown_scheduler_is_noop_when_nothing_scheduled() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    assert!(!tick_shutdown_scheduler(&mut world, &mut runtime));
    assert!(drain_shutdown_texts(&mut world).is_empty());
}
