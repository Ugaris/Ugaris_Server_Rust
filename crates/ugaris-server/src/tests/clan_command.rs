use super::*;

use crate::clan_command::apply_clan_command;
use ugaris_core::clan::ClanRelation;

const NOW: i64 = 1_000_000;

fn tester(world: &mut World, clan: u16, clan_rank: u8) -> CharacterId {
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    if clan != 0 {
        world
            .clan_registry
            .add_member(&mut character, clan)
            .unwrap();
    }
    character.clan_rank = clan_rank;
    world.add_character(character);
    character_id
}

fn feedback_lines(result: &KeyringCommandResult) -> Vec<String> {
    result
        .message_bytes
        .iter()
        .map(|bytes| String::from_utf8_lossy(bytes).to_string())
        .collect()
}

#[test]
fn unrelated_command_is_not_handled() {
    let mut world = World::default();
    let character_id = tester(&mut world, 0, 0);
    assert!(apply_clan_command(&mut world, character_id, "/say hi", NOW).is_none());
}

#[test]
fn clan_lists_every_founded_clan() {
    let mut world = World::default();
    world.clan_registry.found_clan("Iron Wolves", NOW).unwrap();
    world.clan_registry.found_clan("Silver Hawks", NOW).unwrap();
    let character_id = tester(&mut world, 0, 0);

    let result = apply_clan_command(&mut world, character_id, "/clan", NOW).unwrap();
    let lines = feedback_lines(&result);

    assert!(lines[0].contains("Clan List"));
    assert!(lines.iter().any(|line| line.contains("Iron Wolves")
        && line.contains("0 jewels")
        && line.contains("Raiding")
        && line.contains("OFF")
        && line.contains("Level: +0")));
    assert!(lines.iter().any(|line| line.contains("Silver Hawks")));
    // Not a clan member: no "Your Clan" section.
    assert!(!lines.iter().any(|line| line.contains("Your Clan")));
}

#[test]
fn clan_abbreviation_matches_like_c_cmdcmp() {
    let mut world = World::default();
    world.clan_registry.found_clan("Iron Wolves", NOW).unwrap();
    let character_id = tester(&mut world, 0, 0);

    let result = apply_clan_command(&mut world, character_id, "/cla", NOW).unwrap();
    assert!(feedback_lines(&result)[0].contains("Clan List"));
}

#[test]
fn clan_member_sees_their_clan_section_with_rank() {
    let mut world = World::default();
    let nr = world.clan_registry.found_clan("Iron Wolves", NOW).unwrap();
    let character_id = tester(&mut world, nr, 0);

    let result = apply_clan_command(&mut world, character_id, "/clan", NOW).unwrap();
    let lines = feedback_lines(&result);

    assert!(lines
        .iter()
        .any(|line| line.contains(&format!("Your Clan: Iron Wolves (#{nr})"))));
    assert!(lines
        .iter()
        .any(|line| line.contains("Your rank:") && line.contains("Member")));
    // Rank 0 (plain Member): no Treasury section.
    assert!(!lines.iter().any(|line| line.contains("Treasury")));
    assert!(lines.iter().any(|line| line.contains("Clan Info")));
    assert!(lines
        .iter()
        .any(|line| line.contains("Raiding: ") && line.contains("DISABLED")));
}

#[test]
fn clan_member_with_rank_sees_treasury_and_training() {
    let mut world = World::default();
    let nr = world.clan_registry.found_clan("Iron Wolves", NOW).unwrap();
    let character_id = tester(&mut world, nr, 2);

    let result = apply_clan_command(&mut world, character_id, "/clan", NOW).unwrap();
    let lines = feedback_lines(&result);

    assert!(lines
        .iter()
        .any(|line| line.contains("Your rank:") && line.contains("Recruiter")));
    assert!(lines
        .iter()
        .any(|line| line.contains("Treasury") && !line.contains("Guards")));
    assert!(lines.iter().any(|line| line.contains("Jewels:")
        && line.contains("Weekly cost:")
        && line.contains("Debt:")
        && line.contains("Gold:")));
    assert!(lines
        .iter()
        .any(|line| line.contains("Training: score") && line.contains("guard bonus")));
    // The unported dungeon-guard economy section must not appear.
    assert!(!lines.iter().any(|line| line.contains("Dungeon Guards")));
    assert!(!lines.iter().any(|line| line.contains("Dungeon points")));
}

#[test]
fn out_of_range_clan_rank_is_clamped_to_member() {
    let mut world = World::default();
    let nr = world.clan_registry.found_clan("Iron Wolves", NOW).unwrap();
    let character_id = tester(&mut world, nr, 200);

    let result = apply_clan_command(&mut world, character_id, "/clan", NOW).unwrap();
    let lines = feedback_lines(&result);
    assert!(lines
        .iter()
        .any(|line| line.contains("Your rank:") && line.contains("Member")));
}

#[test]
fn raiding_states_are_reported() {
    let mut world = World::default();
    let nr = world.clan_registry.found_clan("Iron Wolves", NOW).unwrap();
    world.clan_registry.set_clan_raid(nr, true, NOW).unwrap();
    let character_id = tester(&mut world, nr, 0);

    let result = apply_clan_command(&mut world, character_id, "/clan", NOW).unwrap();
    let lines = feedback_lines(&result);
    assert!(lines.iter().any(|line| line.contains("Raiding: ")
        && line.contains("PENDING")
        && line.contains("hours")));

    world.clan_registry.set_clan_raid_god(nr, true).unwrap();
    let result = apply_clan_command(&mut world, character_id, "/clan", NOW).unwrap();
    let lines = feedback_lines(&result);
    assert!(lines.iter().any(|line| line.contains("Raiding: ")
        && line.contains("ENABLED")
        && line.contains("can be attacked")));
}

#[test]
fn active_bonuses_are_listed() {
    let mut world = World::default();
    let nr = world.clan_registry.found_clan("Iron Wolves", NOW).unwrap();
    world.clan_registry.set_bonus_level(nr, 2, 3).unwrap();
    let character_id = tester(&mut world, nr, 0);

    let result = apply_clan_command(&mut world, character_id, "/clan", NOW).unwrap();
    let lines = feedback_lines(&result);
    assert!(lines.iter().any(|line| line.contains("Active Bonuses")));
    assert!(lines
        .iter()
        .any(|line| line.contains("Merchant") && line.contains("Level 3")));
}

#[test]
fn relation_with_no_argument_defaults_to_own_clan() {
    let mut world = World::default();
    let own = world.clan_registry.found_clan("Own Clan", NOW).unwrap();
    let other = world.clan_registry.found_clan("Other Clan", NOW).unwrap();
    world
        .clan_registry
        .relations_mut()
        .set_relation(own, other, ClanRelation::Alliance, NOW)
        .unwrap();
    let character_id = tester(&mut world, own, 0);

    let result = apply_clan_command(&mut world, character_id, "/relation", NOW).unwrap();
    let lines = feedback_lines(&result);
    assert_eq!(lines[0], "Own Clan relations:");
    assert!(lines[1].starts_with(&format!("{other}: Other Clan: Neutral (Alliance")));
}

#[test]
fn relation_with_explicit_clan_number_ignores_own_clan() {
    let mut world = World::default();
    let own = world.clan_registry.found_clan("Own Clan", NOW).unwrap();
    let other = world.clan_registry.found_clan("Other Clan", NOW).unwrap();
    let character_id = tester(&mut world, own, 0);

    let result =
        apply_clan_command(&mut world, character_id, &format!("/relation {other}"), NOW).unwrap();
    let lines = feedback_lines(&result);
    assert_eq!(lines[0], "Other Clan relations:");
}

#[test]
fn relation_unknown_clan_number_reports_no_clan() {
    let mut world = World::default();
    let character_id = tester(&mut world, 0, 0);

    let result = apply_clan_command(&mut world, character_id, "/relation 5", NOW).unwrap();
    let lines = feedback_lines(&result);
    assert_eq!(lines, vec!["No clan by that number (5).".to_string()]);
}

#[test]
fn relation_with_no_clan_and_no_argument_shows_nothing() {
    let mut world = World::default();
    let character_id = tester(&mut world, 0, 0);

    let result = apply_clan_command(&mut world, character_id, "/relation", NOW).unwrap();
    assert!(result.message_bytes.is_empty());
}

#[test]
fn clanpots_requires_clan_membership() {
    let mut world = World::default();
    let character_id = tester(&mut world, 0, 0);

    let result = apply_clan_command(&mut world, character_id, "/clanpots", NOW).unwrap();
    assert_eq!(
        feedback_lines(&result),
        vec!["Only for clan members.".to_string()]
    );
}

#[test]
fn clanpots_requires_sufficient_rank() {
    let mut world = World::default();
    let nr = world.clan_registry.found_clan("Iron Wolves", NOW).unwrap();
    let character_id = tester(&mut world, nr, 0);

    let result = apply_clan_command(&mut world, character_id, "/clanpots", NOW).unwrap();
    assert_eq!(
        feedback_lines(&result),
        vec!["Not of sufficient rank.".to_string()]
    );
}

#[test]
fn clanpots_reports_freshly_founded_clan_as_all_zero() {
    let mut world = World::default();
    let nr = world.clan_registry.found_clan("Iron Wolves", NOW).unwrap();
    let character_id = tester(&mut world, nr, 1);

    let result = apply_clan_command(&mut world, character_id, "/clanpots", NOW).unwrap();
    let lines = feedback_lines(&result);

    assert_eq!(lines.len(), 6 + 6 + 3 + 3 + 3);
    assert_eq!(lines[0], "Attack, Parry, Immunity+4: \u{e}0");
    assert_eq!(lines[5], "Attack, Parry, Immunity+24: \u{e}0");
    assert_eq!(lines[6], "Flash, Magic Shield, Immunity+4: \u{e}0");
    assert_eq!(lines[11], "Flash, Magic Shield, Immunity+24: \u{e}0");
    assert_eq!(lines[12], "Small healing potions: \u{e}0");
    assert_eq!(lines[13], "Medium healing potions: \u{e}0");
    assert_eq!(lines[14], "Big healing potions: \u{e}0");
    assert_eq!(lines[15], "Small mana potions: \u{e}0");
    assert_eq!(lines[16], "Medium mana potions: \u{e}0");
    assert_eq!(lines[17], "Big mana potions: \u{e}0");
    assert_eq!(lines[18], "Small combo potions: \u{e}0");
    assert_eq!(lines[19], "Medium combo potions: \u{e}0");
    assert_eq!(lines[20], "Big combo potions: \u{e}0");
}

#[test]
fn clanpots_abbreviation_needs_five_chars_like_c_cmdcmp() {
    let mut world = World::default();
    let nr = world.clan_registry.found_clan("Iron Wolves", NOW).unwrap();
    let character_id = tester(&mut world, nr, 1);

    // 4 chars: falls through to "/clan" (showclan), not "/clanpots".
    let result = apply_clan_command(&mut world, character_id, "/clan", NOW).unwrap();
    assert!(feedback_lines(&result)[0].contains("Clan List"));

    // 5+ chars: resolves to "/clanpots".
    let result = apply_clan_command(&mut world, character_id, "/clanp", NOW).unwrap();
    assert_eq!(
        feedback_lines(&result)[0],
        "Attack, Parry, Immunity+4: \u{e}0"
    );
}
