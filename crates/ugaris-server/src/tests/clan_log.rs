use super::*;

use crate::clan_log::{
    apply_clan_log_command, format_clan_log_entries, parse_clan_log_args, ClanLogParseOutcome,
};
use ugaris_db::ClanLogEntry;

const NOW: i64 = 1_000_000;

fn tester(world: &mut World, clan: u16) -> CharacterId {
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.clan = clan;
    world.add_character(character);
    character_id
}

#[test]
fn no_args_defaults_to_last_24_hours_priority_20_and_no_filters() {
    let mut world = World::default();
    let character_id = tester(&mut world, 0);
    let character = world.characters.get(&character_id).unwrap();

    let outcome =
        parse_clan_log_args(&world, character, "", NOW).expect("empty args should produce a query");
    let ClanLogParseOutcome::Query {
        query,
        leading_messages,
    } = outcome
    else {
        panic!("expected a query outcome");
    };

    assert!(leading_messages.is_empty());
    assert_eq!(query.clan, 0);
    assert_eq!(query.serial, 0);
    assert_eq!(query.character_id, 0);
    assert_eq!(query.prio, 20);
    assert_eq!(query.from_time, NOW - 60 * 60 * 24);
    assert_eq!(query.to_time, NOW);
}

#[test]
fn dash_i_sets_internal_priority_and_own_clan_for_a_member() {
    let mut world = World::default();
    let nr = world.clan_registry.found_clan("Iron Wolves", 0).unwrap();
    let character_id = tester(&mut world, nr);
    let character = world.characters.get(&character_id).unwrap();

    let outcome =
        parse_clan_log_args(&world, character, "-i", NOW).expect("-i should produce a query");
    let ClanLogParseOutcome::Query { query, .. } = outcome else {
        panic!("expected a query outcome");
    };

    assert_eq!(query.clan, nr);
    assert_eq!(query.prio, 50);
}

#[test]
fn dash_i_rejects_a_non_clan_member() {
    let mut world = World::default();
    let character_id = tester(&mut world, 0);
    let character = world.characters.get(&character_id).unwrap();

    let outcome =
        parse_clan_log_args(&world, character, "-i", NOW).expect("-i should still be handled");
    let ClanLogParseOutcome::Messages(messages) = outcome else {
        panic!("expected a message outcome");
    };
    assert_eq!(
        messages,
        vec!["Only clan members may set a priority greater than 20.".to_string()]
    );
}

#[test]
fn priority_above_20_forces_own_clan_and_reports_the_change() {
    let mut world = World::default();
    let own = world.clan_registry.found_clan("Own Clan", 0).unwrap();
    let other = world.clan_registry.found_clan("Other Clan", 0).unwrap();
    let character_id = tester(&mut world, own);
    let character = world.characters.get(&character_id).unwrap();

    let outcome = parse_clan_log_args(&world, character, &format!("-x 30 -c {other}"), NOW)
        .expect("should produce a query");
    let ClanLogParseOutcome::Query {
        query,
        leading_messages,
    } = outcome
    else {
        panic!("expected a query outcome");
    };

    assert_eq!(query.clan, own);
    assert_eq!(query.prio, 30);
    assert_eq!(leading_messages, vec![format!("Changed clan to {own}.")]);
}

#[test]
fn dash_c_validates_the_clan_range() {
    let mut world = World::default();
    let character_id = tester(&mut world, 0);
    let character = world.characters.get(&character_id).unwrap();

    let outcome = parse_clan_log_args(&world, character, "-c 0", NOW).unwrap();
    assert_eq!(
        outcome,
        ClanLogParseOutcome::Messages(vec!["Clan number out of bounds".to_string()])
    );

    let outcome = parse_clan_log_args(&world, character, "-c 32", NOW).unwrap();
    assert_eq!(
        outcome,
        ClanLogParseOutcome::Messages(vec!["Clan number out of bounds".to_string()])
    );
}

#[test]
fn dash_x_validates_the_priority_range() {
    let mut world = World::default();
    let character_id = tester(&mut world, 0);
    let character = world.characters.get(&character_id).unwrap();

    let outcome = parse_clan_log_args(&world, character, "-x 0", NOW).unwrap();
    assert_eq!(
        outcome,
        ClanLogParseOutcome::Messages(vec!["Priority out of bounds".to_string()])
    );

    let outcome = parse_clan_log_args(&world, character, "-x 101", NOW).unwrap();
    assert_eq!(
        outcome,
        ClanLogParseOutcome::Messages(vec!["Priority out of bounds".to_string()])
    );
}

#[test]
fn dash_s_and_dash_e_compute_hours_ago_and_validate_bounds() {
    let mut world = World::default();
    let character_id = tester(&mut world, 0);
    let character = world.characters.get(&character_id).unwrap();

    let outcome = parse_clan_log_args(&world, character, "-s 48 -e 24", NOW).unwrap();
    let ClanLogParseOutcome::Query { query, .. } = outcome else {
        panic!("expected a query outcome");
    };
    assert_eq!(query.from_time, NOW - 48 * 3600);
    assert_eq!(query.to_time, NOW - 24 * 3600);

    // Hours further back than "now" itself is out of bounds (C:
    // `hours < 0 || hours > time_now`).
    let huge_hours = NOW / 3600 + 10;
    let outcome = parse_clan_log_args(&world, character, &format!("-s {huge_hours}"), NOW).unwrap();
    assert_eq!(
        outcome,
        ClanLogParseOutcome::Messages(vec!["Hours out of bounds".to_string()])
    );
}

#[test]
fn start_after_end_is_rejected() {
    let mut world = World::default();
    let character_id = tester(&mut world, 0);
    let character = world.characters.get(&character_id).unwrap();

    let outcome = parse_clan_log_args(&world, character, "-s 1 -e 48", NOW).unwrap();
    assert_eq!(
        outcome,
        ClanLogParseOutcome::Messages(vec![
            "Start time may not be greater than end time.".to_string()
        ])
    );
}

#[test]
fn dash_p_resolves_an_online_character_and_filters_by_id() {
    let mut world = World::default();
    let character_id = tester(&mut world, 0);
    let other_id = CharacterId(9);
    let other = login_character(other_id, &login_block("Ishtar"), 1, 10, 10);
    world.add_character(other);
    let character = world.characters.get(&character_id).unwrap();

    let outcome = parse_clan_log_args(&world, character, "-p Ishtar", NOW).unwrap();
    let ClanLogParseOutcome::Query { query, .. } = outcome else {
        panic!("expected a query outcome");
    };
    assert_eq!(query.character_id, other_id.0);
}

#[test]
fn dash_p_with_an_unknown_name_is_not_recognized() {
    let mut world = World::default();
    let character_id = tester(&mut world, 0);
    let character = world.characters.get(&character_id).unwrap();

    // C: `clanlog_player` sets `repeat=1` and `cmd_clanlog` returns 0
    // without printing anything - not a message, "command unrecognized".
    assert_eq!(
        parse_clan_log_args(&world, character, "-p Nobody", NOW),
        None
    );
}

#[test]
fn unknown_flag_or_non_dash_text_shows_help() {
    let mut world = World::default();
    let character_id = tester(&mut world, 0);
    let character = world.characters.get(&character_id).unwrap();

    let ClanLogParseOutcome::Messages(messages) =
        parse_clan_log_args(&world, character, "garbage", NOW).unwrap()
    else {
        panic!("expected help messages");
    };
    assert_eq!(messages[0], "=== Clan Log Help ===");

    let ClanLogParseOutcome::Messages(messages) =
        parse_clan_log_args(&world, character, "-z", NOW).unwrap()
    else {
        panic!("expected help messages");
    };
    assert_eq!(messages[0], "=== Clan Log Help ===");

    let ClanLogParseOutcome::Messages(messages) =
        parse_clan_log_args(&world, character, "-h", NOW).unwrap()
    else {
        panic!("expected help messages");
    };
    assert_eq!(messages[0], "=== Clan Log Help ===");
}

fn sample_entry(clan: u16, serial: u32, time_unix: i64, content: &str) -> ClanLogEntry {
    ClanLogEntry {
        time_unix,
        clan,
        serial,
        character_id: CharacterId(1),
        prio: 1,
        content: content.to_string(),
    }
}

#[test]
fn format_entries_reports_no_matches_when_empty() {
    let world = World::default();
    let lines = format_clan_log_entries(&[], &world, NOW);
    assert_eq!(lines.len(), 1);
    assert!(String::from_utf8_lossy(&lines[0]).contains("No matching clan log entries."));
}

#[test]
fn format_entries_shows_the_current_clan_name_when_the_serial_matches() {
    let mut world = World::default();
    let nr = world.clan_registry.found_clan("Iron Wolves", 42).unwrap();
    let serial = world.clan_registry.serial(nr);

    let entries = vec![sample_entry(nr, serial, 0, "Clan was founded by Tester")];
    let lines = format_clan_log_entries(&entries, &world, NOW);

    // First line is the "=== Clan Log ===" heading; second is the entry.
    assert_eq!(lines.len(), 2);
    let entry_line = String::from_utf8_lossy(&lines[1]);
    assert!(entry_line.contains("Iron Wolves"));
    assert!(entry_line.contains("Clan was founded by Tester"));
}

#[test]
fn format_entries_shows_former_clan_when_the_serial_no_longer_matches() {
    let mut world = World::default();
    let nr = world.clan_registry.found_clan("Iron Wolves", 42).unwrap();
    let stale_serial = world.clan_registry.serial(nr).wrapping_add(1);

    let entries = vec![sample_entry(nr, stale_serial, 0, "Some old entry")];
    let lines = format_clan_log_entries(&entries, &world, NOW);

    let entry_line = String::from_utf8_lossy(&lines[1]);
    assert!(entry_line.contains(&format!("Former clan {nr}")));
}

#[test]
fn format_entries_shows_a_cutoff_hint_when_more_than_fifty_rows_are_fetched() {
    let world = World::default();
    let mut entries: Vec<ClanLogEntry> = (0..51)
        .map(|i| sample_entry(1, 0, i * 3600, "entry"))
        .collect();
    // 51st (index 50) row's timestamp drives the "-s N" hint.
    entries[50].time_unix = NOW - 5 * 3600;

    let lines = format_clan_log_entries(&entries, &world, NOW);
    // heading + 50 displayed entries + cutoff hint
    assert_eq!(lines.len(), 52);
    let hint_line = String::from_utf8_lossy(lines.last().unwrap());
    assert!(hint_line.contains("Not all entries displayed. Use -s 5 to continue."));
}

#[tokio::test]
async fn apply_clan_log_command_ignores_unrelated_verbs() {
    let mut world = World::default();
    let character_id = tester(&mut world, 0);

    let result =
        apply_clan_log_command(&mut world, &None, character_id, NOW, "/tell foo bar").await;
    assert_eq!(result, None);
}

#[tokio::test]
async fn apply_clan_log_command_reports_unavailable_without_a_repository() {
    let mut world = World::default();
    let character_id = tester(&mut world, 0);

    let result = apply_clan_log_command(&mut world, &None, character_id, NOW, "/clanlog")
        .await
        .expect("command should be recognized");
    assert_eq!(
        result.messages,
        vec!["The clan log is currently unavailable.".to_string()]
    );
}

#[tokio::test]
async fn apply_clan_log_command_reports_parse_errors_even_without_a_repository() {
    let mut world = World::default();
    let character_id = tester(&mut world, 0);

    let result = apply_clan_log_command(&mut world, &None, character_id, NOW, "/clanlog -c 99")
        .await
        .expect("command should be recognized");
    assert_eq!(
        result.messages,
        vec!["Clan number out of bounds".to_string()]
    );
}

#[tokio::test]
async fn apply_clan_log_command_returns_none_when_dash_p_name_is_unresolved() {
    let mut world = World::default();
    let character_id = tester(&mut world, 0);

    let result =
        apply_clan_log_command(&mut world, &None, character_id, NOW, "/clanlog -p Nobody").await;
    assert_eq!(result, None);
}

#[tokio::test]
async fn apply_clearclanlog_requires_god() {
    let mut world = World::default();
    let character_id = tester(&mut world, 0);

    let result =
        apply_clan_log_command(&mut world, &None, character_id, NOW, "/clearclanlog 1").await;
    assert_eq!(result, None);
}

#[tokio::test]
async fn apply_clearclanlog_validates_range_before_touching_the_repository() {
    let mut world = World::default();
    let character_id = tester(&mut world, 0);
    world
        .characters
        .get_mut(&character_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);

    let result = apply_clan_log_command(&mut world, &None, character_id, NOW, "/clearclanlog 0")
        .await
        .expect("command should be recognized");
    assert_eq!(
        result.messages,
        vec!["Invalid clan number. Range is 1-31".to_string()]
    );
}

#[tokio::test]
async fn apply_clearclanlog_reports_unavailable_without_a_repository() {
    let mut world = World::default();
    let character_id = tester(&mut world, 0);
    world
        .characters
        .get_mut(&character_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);

    let result = apply_clan_log_command(&mut world, &None, character_id, NOW, "/clearclanlog 1")
        .await
        .expect("command should be recognized");
    assert_eq!(
        result.messages,
        vec!["The clan log is currently unavailable.".to_string()]
    );
}
