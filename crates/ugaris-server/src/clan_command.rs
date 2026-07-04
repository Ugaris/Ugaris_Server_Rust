//! `/clan`, `/relation`, and `/clanpots` text commands: read-only clan
//! status display.
//!
//! Ports `showclan`/`show_clan_relation`/`show_clan_pots`
//! (`src/system/clan.c:128-357,1426-1455`), dispatched from
//! `command.c:5974-5980,6001-6011` (`cmdcmp(ptr, "clanpots", 5)`/
//! `cmdcmp(ptr, "clan", 0)`/`cmdcmp(ptr, "relation", 0)`).
//!
//! `showclan`'s "--- Dungeon Guards ---" section and "Dungeon points: X /
//! 400" line (`clan.c:169-197`) are still skipped - the guard counts
//! (`struct clan_dungeon`'s `warrior`/`mage`/`seyan`/`teleport`/`fake`/
//! `key` fields) are part of the still-unported dungeon-guard economy
//! (see the "Clan system" P3 task's REMAINING note) and aren't part of
//! [`ClanEconomy`]. `/clanpots` only needs the potion stockpile
//! (`alc_pot`/`simple_pot`) which *is* part of [`ClanEconomy`] now, even
//! though nothing feeds it yet (every clan reads all-zero until the
//! alchemy-potion economy's `add_alc_potion`/`add_simple_potion` call
//! sites are ported), same as a freshly-founded C clan would show.

use super::*;

use ugaris_core::clan::{
    bonus_name, score_to_level, ClanEconomy, ClanRegistry, MAX_BONUS, MAX_CLAN,
};
use ugaris_core::text::{COL_HEADING, COL_LINK};

fn colored_line(color: &[u8], text: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(color.len() + text.len() + COL_RESET.len());
    out.extend_from_slice(color);
    out.extend_from_slice(text.as_bytes());
    out.extend_from_slice(COL_RESET);
    out
}

/// C `showclan`'s clan-list loop (`clan.c:129-141`).
fn clan_list_lines(registry: &ClanRegistry) -> Vec<Vec<u8>> {
    let mut lines = Vec::new();
    for n in 1..MAX_CLAN as u16 {
        let Some(identity) = registry.identity(n) else {
            continue;
        };
        let jewels = identity.economy.treasure.jewels - identity.economy.treasure.debt / 1000;
        let (raid_color, raid_text) = if identity.economy.raid {
            (COL_LIGHT_RED, "ON")
        } else if identity.economy.raid_on_start != 0 {
            (COL_YELLOW, "PENDING")
        } else {
            (COL_DARK_GRAY, "OFF")
        };
        let level = score_to_level(identity.economy.training_score);

        let mut line = Vec::new();
        line.extend_from_slice(COL_YELLOW);
        line.extend_from_slice(format!("#{n}").as_bytes());
        line.extend_from_slice(COL_RESET);
        line.extend_from_slice(b" ");
        line.extend_from_slice(COL_LIGHT_GREEN);
        line.extend_from_slice(identity.name.as_bytes());
        line.extend_from_slice(COL_RESET);
        line.extend_from_slice(format!(" - {jewels} jewels, Raiding: ").as_bytes());
        line.extend_from_slice(raid_color);
        line.extend_from_slice(raid_text.as_bytes());
        line.extend_from_slice(COL_RESET);
        line.extend_from_slice(format!(", Level: +{level}").as_bytes());
        lines.push(line);
    }
    lines
}

/// C `showclan`'s "Your Clan" section (`clan.c:143-233`) once a caller has
/// already resolved `cnr = get_char_clan(cn)` to `Some`. `rank` is
/// `character.clan_rank` clamped to `0` if out of the valid `0..=4` range
/// (C: `elog(...)` then `rank = 0`; the server-only diagnostic log is not
/// reproduced here, same precedent as elsewhere in this codebase).
fn your_clan_lines(registry: &ClanRegistry, cnr: u16, rank: usize, now: i64) -> Vec<Vec<u8>> {
    let identity = registry
        .identity(cnr)
        .expect("caller already validated cnr exists");
    let mut lines = Vec::new();

    lines.push(b" ".to_vec());
    lines.push(colored_line(
        COL_HEADING,
        &format!("=== Your Clan: {} (#{cnr}) ===", identity.name),
    ));
    let mut rank_line = Vec::new();
    rank_line.extend_from_slice(b"Your rank:");
    rank_line.extend_from_slice(COL_YELLOW);
    rank_line.extend_from_slice(format!(" {}", identity.rank_names[rank]).as_bytes());
    rank_line.extend_from_slice(COL_RESET);
    lines.push(rank_line);

    if rank > 0 {
        let treasure = &identity.economy.treasure;
        lines.push(b" ".to_vec());
        lines.push(colored_line(COL_HEADING, "--- Treasury ---"));

        let mut treasury_line = Vec::new();
        treasury_line.extend_from_slice(b"Jewels:");
        treasury_line.extend_from_slice(COL_YELLOW);
        treasury_line.extend_from_slice(format!(" {}", treasure.jewels).as_bytes());
        treasury_line.extend_from_slice(COL_RESET);
        treasury_line.extend_from_slice(b" | Weekly cost:");
        treasury_line.extend_from_slice(COL_YELLOW);
        treasury_line.extend_from_slice(
            format!(" {:.1}", treasure.cost_per_week as f64 / 1000.0).as_bytes(),
        );
        treasury_line.extend_from_slice(COL_RESET);
        treasury_line.extend_from_slice(b" | Debt:");
        treasury_line.extend_from_slice(COL_YELLOW);
        treasury_line
            .extend_from_slice(format!(" {:.1}", treasure.debt as f64 / 1000.0).as_bytes());
        treasury_line.extend_from_slice(COL_RESET);
        treasury_line.extend_from_slice(b" | Gold:");
        treasury_line.extend_from_slice(COL_YELLOW);
        treasury_line.extend_from_slice(format!(" {}G", identity.economy.depot_money).as_bytes());
        treasury_line.extend_from_slice(COL_RESET);
        lines.push(treasury_line);

        // C's "--- Dungeon Guards ---" section and "Dungeon points: X /
        // 400" line (`clan.c:157-192`) are intentionally skipped - see
        // the module doc comment.

        let mut training_line = Vec::new();
        training_line.extend_from_slice(b"Training: score");
        training_line.extend_from_slice(COL_YELLOW);
        training_line.extend_from_slice(format!(" {}", identity.economy.training_score).as_bytes());
        training_line.extend_from_slice(COL_RESET);
        training_line.extend_from_slice(b" (guard bonus:");
        training_line.extend_from_slice(COL_YELLOW);
        training_line.extend_from_slice(
            format!(" +{}", score_to_level(identity.economy.training_score)).as_bytes(),
        );
        training_line.extend_from_slice(COL_RESET);
        let next_update_minutes = 60 - (now - identity.economy.last_training_update) / 60;
        training_line
            .extend_from_slice(format!("), next update in {next_update_minutes}m").as_bytes());
        lines.push(training_line);
    }

    lines.push(b" ".to_vec());
    lines.push(colored_line(COL_HEADING, "--- Clan Info ---"));
    if !identity.website.is_empty() {
        let mut line = Vec::new();
        line.extend_from_slice(b"Website: ");
        line.extend_from_slice(COL_LINK);
        line.extend_from_slice(identity.website.as_bytes());
        line.extend_from_slice(COL_RESET);
        lines.push(line);
    }
    if !identity.message.is_empty() {
        lines.push(format!("Message: {}", identity.message).into_bytes());
    }

    let mut has_bonus = false;
    for (n, level) in identity
        .economy
        .bonus_level
        .iter()
        .enumerate()
        .take(MAX_BONUS)
    {
        if *level == 0 {
            continue;
        }
        if !has_bonus {
            lines.push(b" ".to_vec());
            lines.push(colored_line(COL_HEADING, "--- Active Bonuses ---"));
            has_bonus = true;
        }
        let mut line = Vec::new();
        line.extend_from_slice(COL_YELLOW);
        line.extend_from_slice(bonus_name(n as i32).as_bytes());
        line.extend_from_slice(COL_RESET);
        line.extend_from_slice(format!(" (#{n}): Level {level}").as_bytes());
        lines.push(line);
    }

    lines.push(b" ".to_vec());
    if identity.economy.raid {
        let mut line = Vec::new();
        line.extend_from_slice(b"Raiding: ");
        line.extend_from_slice(COL_LIGHT_RED);
        line.extend_from_slice(b"ENABLED");
        line.extend_from_slice(COL_RESET);
        line.extend_from_slice(b" - Your clan can be attacked!");
        lines.push(line);
    } else if identity.economy.raid_on_start != 0 {
        let hours_left = ((60 * 60 * 48) - (now - identity.economy.raid_on_start)) as f64 / 3600.0;
        let mut line = Vec::new();
        line.extend_from_slice(b"Raiding: ");
        line.extend_from_slice(COL_YELLOW);
        line.extend_from_slice(b"PENDING");
        line.extend_from_slice(COL_RESET);
        line.extend_from_slice(format!(" - Activates in {hours_left:.1} hours").as_bytes());
        lines.push(line);
    } else {
        let mut line = Vec::new();
        line.extend_from_slice(b"Raiding: ");
        line.extend_from_slice(COL_LIGHT_GREEN);
        line.extend_from_slice(b"DISABLED");
        line.extend_from_slice(COL_RESET);
        lines.push(line);
    }

    lines
}

/// C `showclan` (`clan.c:128-233`), the `/clan` command.
fn showclan_lines(registry: &ClanRegistry, character: &mut Character, now: i64) -> Vec<Vec<u8>> {
    let mut lines = Vec::new();
    lines.push(colored_line(COL_HEADING, "=== Clan List ==="));
    lines.push(b" ".to_vec());
    lines.extend(clan_list_lines(registry));

    if let Some(cnr) = registry.get_char_clan(character) {
        let rank = character.clan_rank as usize;
        let rank = if rank > 4 { 0 } else { rank };
        lines.extend(your_clan_lines(registry, cnr, rank, now));
    }

    lines
}

/// C `show_clan_relation` (`clan.c:311-357`), the `/relation` command.
fn show_clan_relation_lines(registry: &ClanRegistry, cnr: u16, now: i64) -> Vec<Vec<u8>> {
    if cnr < 1 || cnr as usize >= MAX_CLAN {
        return Vec::new();
    }
    let Some(name) = registry.name(cnr) else {
        return vec![format!("No clan by that number ({cnr}).").into_bytes()];
    };

    let mut lines = vec![format!("{name} relations:").into_bytes()];
    let relations = registry.relations();
    for n in 1..MAX_CLAN as u16 {
        if n == cnr || !registry.exists(n) {
            continue;
        }
        let crel = relations.current_relation(cnr, n);
        let wrel = relations.want_relation(cnr, n);
        let orel = relations.want_relation(n, cnr);
        let wdiff = now - relations.want_date(cnr, n);
        let odiff = now - relations.want_date(n, cnr);
        let other_name = registry.name(n).unwrap_or("");
        lines.push(
            format!(
                "{n}: {other_name}: {} ({} [{:02}:{:02}] - {} [{:02}:{:02}])",
                crel.display_name(),
                wrel.display_name(),
                (wdiff / 60) / 60,
                (wdiff / 60) % 60,
                orel.display_name(),
                (odiff / 60) / 60,
                (odiff / 60) % 60,
            )
            .into_bytes(),
        );
    }
    lines
}

/// C `show_clan_pots`'s potion-tier name table (`clan.c:1428`).
const CLAN_POT_SIZES: [&str; 3] = ["Small", "Medium", "Big"];

/// C `show_clan_pots` (`clan.c:1426-1455`), the `/clanpots` command.
/// `character` is only used for its `clan`/`clan_rank` fields (via
/// [`ClanRegistry::get_char_clan`], same as `showclan_lines`).
fn show_clan_pots_lines(registry: &ClanRegistry, character: &mut Character) -> Vec<Vec<u8>> {
    let Some(cnr) = registry.get_char_clan(character) else {
        return vec![b"Only for clan members.".to_vec()];
    };
    if character.clan_rank < 1 {
        return vec![b"Not of sufficient rank.".to_vec()];
    }
    let economy: &ClanEconomy = &registry
        .identity(cnr)
        .expect("get_char_clan already validated cnr exists")
        .economy;

    let mut lines = Vec::with_capacity(6 + 6 + 3 + 3 + 3);
    for (n, count) in economy.alc_pot[0].iter().enumerate() {
        lines.push(format!("Attack, Parry, Immunity+{}: \x0e{count}", n * 4 + 4).into_bytes());
    }
    for (n, count) in economy.alc_pot[1].iter().enumerate() {
        lines
            .push(format!("Flash, Magic Shield, Immunity+{}: \x0e{count}", n * 4 + 4).into_bytes());
    }
    for (size, count) in CLAN_POT_SIZES.iter().zip(economy.simple_pot[0].iter()) {
        lines.push(format!("{size} healing potions: \x0e{count}").into_bytes());
    }
    for (size, count) in CLAN_POT_SIZES.iter().zip(economy.simple_pot[1].iter()) {
        lines.push(format!("{size} mana potions: \x0e{count}").into_bytes());
    }
    for (size, count) in CLAN_POT_SIZES.iter().zip(economy.simple_pot[2].iter()) {
        lines.push(format!("{size} combo potions: \x0e{count}").into_bytes());
    }
    lines
}

/// Dispatches `/clan`, `/relation`, and `/clanpots`
/// (`command.c:5974-5980,6001-6011`).
pub(crate) fn apply_clan_command(
    world: &mut World,
    character_id: CharacterId,
    command: &str,
    now: i64,
) -> Option<KeyringCommandResult> {
    let (verb, rest) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    let lower = verb.to_ascii_lowercase();

    if lower.len() >= 3 && "relation".starts_with(&lower) {
        let arg = rest.trim();
        let requested = legacy_atoi_prefix(arg);
        let cnr = if requested != 0 {
            requested as u16
        } else {
            world.characters.get(&character_id)?.clan
        };
        return Some(KeyringCommandResult {
            message_bytes: show_clan_relation_lines(&world.clan_registry, cnr, now),
            ..Default::default()
        });
    }

    // C checks `cmdcmp(ptr, "clanpots", 5)` before `cmdcmp(ptr, "clan", 0)`
    // (`command.c:5974,5978`) so a typed word long enough to disambiguate
    // (5+ chars) resolves to `/clanpots` before falling through to the
    // shorter `/clan` prefix match.
    if lower.len() >= 5 && "clanpots".starts_with(&lower) {
        let character = world.characters.get_mut(&character_id)?;
        return Some(KeyringCommandResult {
            message_bytes: show_clan_pots_lines(&world.clan_registry, character),
            ..Default::default()
        });
    }

    if lower.len() >= 3 && "clan".starts_with(&lower) {
        let character = world.characters.get_mut(&character_id)?;
        return Some(KeyringCommandResult {
            message_bytes: showclan_lines(&world.clan_registry, character, now),
            ..Default::default()
        });
    }

    None
}
