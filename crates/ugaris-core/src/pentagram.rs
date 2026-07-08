//! Per-player pentagram-quest reward math (`src/area/4/pents.c`'s
//! `add_pentagram_to_player`/`update_player_pentagram_stats`/`check_for_
//! color_combo`/`handle_lucky_pentagram`/`log_pentagram_info`/`check_for_
//! record`/the reset half of `distribute_rewards_to_player`).
//!
//! These are pure functions over [`PentagramDebugData`] (C `struct
//! pentagram_player_data`, `PlayerRuntime::pentagram_debug` in this port)
//! rather than `World` methods: this data lives on the session-owned
//! `PlayerRuntime`, which `ugaris-core`'s `World` has no access to (same
//! architectural split as `world::pents`'s system-wide half - see that
//! module's doc comment). `ugaris-server`'s `pents` module is the
//! orchestrator: it drains `World::drain_pending_pentagram_activations`,
//! calls [`add_pentagram_to_player`] for the activator, and - on a solve -
//! calls [`distribute_rewards_reset`] for every eligible online player
//! (after `World::reset_pentagram_colors` for any of them with a live
//! five-color combo), then applies the resulting exp/achievements/
//! messages itself (needs `World::give_exp`, `World::clan_registry`, and
//! `PlayerRuntime` DB/Steam-sync side effects ugaris-core cannot reach).
//!
//! Not ported: `get_pent_data`'s macro-daemon challenge-room restore (see
//! `world::pents`'s module doc comment).

use crate::game_settings::GameSettings;
use crate::player::PentagramDebugData;
use crate::world::legacy_random_below_from_seed;

/// C `static const char *COLOR_NAMES[] = {"none", "red", "green", "blue"}`.
pub const COLOR_NAMES: [&str; 4] = ["none", "red", "green", "blue"];

fn color_name(color: i32) -> &'static str {
    usize::try_from(color)
        .ok()
        .and_then(|idx| COLOR_NAMES.get(idx))
        .copied()
        .unwrap_or("?")
}

/// Result of [`add_pentagram_to_player`]: the `log_char` lines C emits (in
/// order) plus the flags `ugaris-server` needs to award achievements.
#[derive(Debug, Clone, Default)]
pub struct AddPentagramOutcome {
    pub messages: Vec<String>,
    pub pent_value: i32,
    /// `handle_lucky_pentagram` hit this call -> `ACHIEVEMENT_HAPPY_GO_LUCKY`.
    pub lucky_hit: bool,
    /// Second lucky hit in the same solve run -> `ACHIEVEMENT_FAVORED_BY_FORTUNE`.
    pub second_lucky_hit: bool,
}

/// C `add_pentagram_to_player` (`pents.c:709-749`), inlining `update_
/// player_pentagram_stats`/`check_for_color_combo`/`handle_lucky_
/// pentagram`/`log_pentagram_info`/`check_for_record` in call order. Takes
/// the pentagram's raw item id (C stores `it[item_id]`'s array index
/// directly into `pent_it[]`) rather than a typed `ItemId` so the stored
/// value round-trips through [`World::reset_pentagram_colors`] unchanged.
#[allow(clippy::too_many_arguments)]
pub fn add_pentagram_to_player(
    data: &mut PentagramDebugData,
    settings: &GameSettings,
    seed: &mut u32,
    record: &mut i32,
    record_holder: &mut String,
    item_id_raw: i32,
    level: i32,
    color: i32,
    number: i32,
    is_quest_solved: bool,
    active_pentagrams: i32,
    total_pentagrams: i32,
    player_name: &str,
) -> AddPentagramOutcome {
    let pent_value = level * settings.get_pentagram_value_multiplier() + number;
    let mut messages = Vec::new();

    if is_quest_solved {
        data.bonus += pent_value * 3;
    }

    messages.push(format!(
        "You got a {} Pentagram, value {}. {} of {} Pentagrammas are active.",
        color_name(color),
        pent_value,
        active_pentagrams,
        total_pentagrams
    ));

    update_player_pentagram_stats(data, settings, item_id_raw, color, pent_value);

    let (lucky_hit, second_lucky_hit) =
        handle_lucky_pentagram(data, settings, seed, pent_value, &mut messages);

    data.bonus += level;

    log_pentagram_info(data, &mut messages);

    data.pent_cnt += 1;
    check_for_record(data, record, record_holder, player_name, &mut messages);

    AddPentagramOutcome {
        messages,
        pent_value,
        lucky_hit,
        second_lucky_hit,
    }
}

/// C `update_player_pentagram_stats` (`pents.c:761-806`).
fn update_player_pentagram_stats(
    data: &mut PentagramDebugData,
    settings: &GameSettings,
    item_id_raw: i32,
    color: i32,
    value: i32,
) {
    if data.pent_value[..5].contains(&value) {
        return;
    }

    if data.status == 0 {
        let mut index = 5;
        for (i, existing) in data.pent_value[..5].iter().enumerate() {
            if *existing < value {
                index = i;
                break;
            }
        }
        if index < 4 {
            for i in (index + 1..5).rev() {
                data.pent_it[i] = data.pent_it[i - 1];
                data.pent_color[i] = data.pent_color[i - 1];
                data.pent_value[i] = data.pent_value[i - 1];
                data.pent_worth[i] = data.pent_worth[i - 1];
            }
        }
        if index < 5 {
            data.pent_it[index] = item_id_raw;
            data.pent_color[index] = color;
            data.pent_value[index] = value;
            data.pent_worth[index] = value / settings.get_pentagram_worth_divisor().max(1);
        }
        check_for_color_combo(data);
    } else if data.pent_value[5] < value {
        data.pent_it[5] = item_id_raw;
        data.pent_color[5] = color;
        data.pent_value[5] = value;
        // C: `pent_worth[5] = value;` (not divided by the worth divisor,
        // unlike the `status == 0` branch above - an intentional
        // asymmetry in the original source).
        data.pent_worth[5] = value;
    }
}

/// C `check_for_color_combo` (`pents.c:815-835`). C's own final step
/// (`log_char(player_data->pent_it[0], ...)`) passes an *item* id where a
/// *character* id is expected - a latent C bug (the message clearly means
/// to notify the player). Rust's typed ids make that mistake
/// uncompilable, so this returns whether the combo was newly triggered
/// and the caller (`add_pentagram_to_player`) attributes the message to
/// the actual player instead.
fn check_for_color_combo(data: &mut PentagramDebugData) -> bool {
    let mut same_count = 0;
    let mut last_color = 0;
    for i in 0..5 {
        if data.pent_value[i] == 0 {
            break;
        }
        if last_color == data.pent_color[i] {
            same_count += 1;
        } else {
            same_count = 1;
            last_color = data.pent_color[i];
        }
    }
    if same_count == 5 {
        data.status = 1;
        true
    } else {
        false
    }
}

/// C `handle_lucky_pentagram` (`pents.c:846-860`).
fn handle_lucky_pentagram(
    data: &mut PentagramDebugData,
    settings: &GameSettings,
    seed: &mut u32,
    value: i32,
    messages: &mut Vec<String>,
) -> (bool, bool) {
    let chance = settings.get_lucky_pentagram_chance().max(1) as u32;
    if legacy_random_below_from_seed(seed, chance) != 0 {
        return (false, false);
    }
    messages.push("You got the lucky Pentagram!".to_string());
    let bonus_roll = legacy_random_below_from_seed(seed, (value * 3).max(0) as u32) as i32;
    data.bonus += value + bonus_roll;
    data.lucky_pents_this_solve += 1;
    let second = data.lucky_pents_this_solve >= 2;
    (true, second)
}

/// C `log_pentagram_info` (`pents.c:872-899`).
fn log_pentagram_info(data: &mut PentagramDebugData, messages: &mut Vec<String>) {
    let mut total_worth = 0;

    if data.pent_value[5] != 0 {
        messages.push(format!(
            "#3{}Combo: Pentagram value of {}, color of {}, {} exp.",
            data.pent_color[5],
            data.pent_value[5],
            color_name(data.pent_color[5]),
            data.pent_worth[5]
        ));
        total_worth += data.pent_worth[5];
    } else {
        messages.push("#3".to_string());
    }

    for index in 0..5 {
        if data.pent_value[index] == 0 {
            break;
        }
        messages.push(format!(
            "#{}{}Pentagram value of {}, color of {}, {} exp.",
            index + 4,
            data.pent_color[index],
            data.pent_value[index],
            color_name(data.pent_color[index]),
            data.pent_worth[index]
        ));
        total_worth += data.pent_worth[index];
    }

    messages.push(format!(
        "#90Bonus: {}, total: {}",
        data.bonus,
        total_worth + data.bonus
    ));
}

/// C `check_for_record` (`pents.c:909-925`). C compares
/// `pentagram_record_ID != ch[player_id].ID` (a persistent save-file
/// identity); this port compares by character name instead (matching
/// `World::arena_toplist`'s own name-keyed record precedent - see
/// `PentagramQuestState::pentagram_record_holder`'s doc comment).
fn check_for_record(
    data: &PentagramDebugData,
    record: &mut i32,
    record_holder: &mut String,
    player_name: &str,
    messages: &mut Vec<String>,
) {
    if data.pent_cnt <= *record {
        return;
    }
    if record_holder != player_name {
        messages.push(format!(
            "You broke {record_holder}'s record. New record is now {} pents activated in one run.",
            data.pent_cnt
        ));
    } else if data.pent_cnt % 25 == 0 {
        messages.push(format!(
            "You increased your own record to {} pents in one run.",
            data.pent_cnt
        ));
    }
    *record = data.pent_cnt;
    *record_holder = player_name.to_string();
}

/// C `distribute_rewards_to_player`'s state-reset half (`pents.c:593-673`,
/// minus the exp-granting/clan-bonus/message-formatting tail, which needs
/// `World`/`PlayerRuntime`/the solver's name and stays in `ugaris-server`'s
/// `pents` module). Returns whether the player had a five-color combo
/// (`ACHIEVEMENT_FIVE_IN_A_ROW` gate) and the summed `exp_reward` before
/// the exp-formula clamp. C's `pent_cnt` is deliberately **not** reset
/// here (`pents.c` never zeroes it in this function) - it is the player's
/// lifetime pentagram-activation counter, `check_for_record`'s subject.
pub fn distribute_rewards_reset(data: &mut PentagramDebugData) -> (bool, i32) {
    let had_combo = data.status == 1;
    let mut exp_reward = 0;
    for i in 0..6 {
        exp_reward += data.pent_worth[i];
        data.pent_value[i] = 0;
        data.pent_worth[i] = 0;
        data.pent_it[i] = 0;
        data.pent_color[i] = 0;
    }
    exp_reward += data.bonus;
    data.status = 0;
    data.bonus = 0;
    data.lucky_pents_this_solve = 0;
    (had_combo, exp_reward)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_lucky_settings() -> GameSettings {
        let mut settings = GameSettings::default();
        // Chance denominator large enough that the deterministic LCG seeds
        // used below never happen to roll a 0.
        settings.set_lucky_pentagram_chance(1_000_000);
        settings
    }

    fn always_lucky_settings() -> GameSettings {
        let mut settings = GameSettings::default();
        settings.set_lucky_pentagram_chance(1);
        settings
    }

    #[test]
    fn pent_value_uses_level_times_multiplier_plus_number() {
        let settings = no_lucky_settings();
        let mut data = PentagramDebugData::default();
        let mut seed = 1;
        let mut record = 0;
        let mut holder = "Nobody".to_string();
        let outcome = add_pentagram_to_player(
            &mut data,
            &settings,
            &mut seed,
            &mut record,
            &mut holder,
            42,
            5,
            2,
            3,
            false,
            1,
            10,
            "Hero",
        );
        assert_eq!(
            outcome.pent_value,
            5 * settings.get_pentagram_value_multiplier() + 3
        );
        assert!(!outcome.lucky_hit);
        assert_eq!(data.pent_cnt, 1);
    }

    #[test]
    fn is_quest_solved_adds_triple_bonus() {
        let settings = no_lucky_settings();
        let mut data = PentagramDebugData::default();
        let mut seed = 1;
        let mut record = 0;
        let mut holder = "Nobody".to_string();
        let outcome = add_pentagram_to_player(
            &mut data,
            &settings,
            &mut seed,
            &mut record,
            &mut holder,
            1,
            2,
            1,
            1,
            true,
            12,
            12,
            "Hero",
        );
        // bonus = pent_value*3 (solve) + level (unconditional tail add).
        assert_eq!(data.bonus, outcome.pent_value * 3 + 2);
    }

    #[test]
    fn stats_insert_in_descending_value_order() {
        let settings = no_lucky_settings();
        let mut data = PentagramDebugData::default();
        let mut seed = 1;
        let mut record = 0;
        let mut holder = "Nobody".to_string();
        for (item_id, level, number) in [(1, 1, 1), (2, 5, 1), (3, 3, 1)] {
            add_pentagram_to_player(
                &mut data,
                &settings,
                &mut seed,
                &mut record,
                &mut holder,
                item_id,
                level,
                1,
                number,
                false,
                1,
                10,
                "Hero",
            );
        }
        // Highest value (level 5) first, descending.
        assert!(data.pent_value[0] >= data.pent_value[1]);
        assert!(data.pent_value[1] >= data.pent_value[2]);
    }

    #[test]
    fn duplicate_value_is_ignored() {
        let settings = no_lucky_settings();
        let mut data = PentagramDebugData::default();
        let mut seed = 1;
        let mut record = 0;
        let mut holder = "Nobody".to_string();
        add_pentagram_to_player(
            &mut data,
            &settings,
            &mut seed,
            &mut record,
            &mut holder,
            1,
            4,
            1,
            5,
            false,
            1,
            10,
            "Hero",
        );
        add_pentagram_to_player(
            &mut data,
            &settings,
            &mut seed,
            &mut record,
            &mut holder,
            2,
            4,
            1,
            5,
            false,
            2,
            10,
            "Hero",
        );
        // Same (level, color, number) => same pent_value => second call is
        // a no-op insert (still only one nonzero slot).
        assert_eq!(data.pent_value.iter().filter(|v| **v != 0).count(), 1);
    }

    #[test]
    fn five_matching_colors_sets_combo_status() {
        let settings = no_lucky_settings();
        let mut data = PentagramDebugData::default();
        let mut seed = 1;
        let mut record = 0;
        let mut holder = "Nobody".to_string();
        for (item_id, number) in [(1, 1), (2, 2), (3, 3), (4, 4), (5, 5)] {
            add_pentagram_to_player(
                &mut data,
                &settings,
                &mut seed,
                &mut record,
                &mut holder,
                item_id,
                1,
                2,
                number,
                false,
                1,
                10,
                "Hero",
            );
        }
        assert_eq!(data.status, 1);
    }

    #[test]
    fn lucky_pentagram_forced_awards_bonus_and_counts() {
        let settings = always_lucky_settings();
        let mut data = PentagramDebugData::default();
        let mut seed = 1;
        let mut record = 0;
        let mut holder = "Nobody".to_string();
        let outcome = add_pentagram_to_player(
            &mut data,
            &settings,
            &mut seed,
            &mut record,
            &mut holder,
            1,
            2,
            1,
            1,
            false,
            1,
            10,
            "Hero",
        );
        assert!(outcome.lucky_hit);
        assert!(!outcome.second_lucky_hit);
        assert_eq!(data.lucky_pents_this_solve, 1);

        let outcome2 = add_pentagram_to_player(
            &mut data,
            &settings,
            &mut seed,
            &mut record,
            &mut holder,
            2,
            2,
            1,
            2,
            false,
            2,
            10,
            "Hero",
        );
        assert!(outcome2.second_lucky_hit);
        assert_eq!(data.lucky_pents_this_solve, 2);
    }

    #[test]
    fn new_record_breaks_previous_holder() {
        let settings = no_lucky_settings();
        let mut data = PentagramDebugData {
            pent_cnt: 4,
            ..Default::default()
        };
        let mut seed = 1;
        let mut record = 4;
        let mut holder = "Rival".to_string();
        let outcome = add_pentagram_to_player(
            &mut data,
            &settings,
            &mut seed,
            &mut record,
            &mut holder,
            1,
            1,
            1,
            1,
            false,
            1,
            10,
            "Hero",
        );
        assert_eq!(record, 5);
        assert_eq!(holder, "Hero");
        assert!(outcome
            .messages
            .iter()
            .any(|m| m.contains("You broke Rival's record")));
    }

    #[test]
    fn own_record_message_only_every_25() {
        let settings = no_lucky_settings();
        let mut data = PentagramDebugData {
            pent_cnt: 24,
            ..Default::default()
        };
        let mut seed = 1;
        let mut record = 24;
        let mut holder = "Hero".to_string();
        let outcome = add_pentagram_to_player(
            &mut data,
            &settings,
            &mut seed,
            &mut record,
            &mut holder,
            1,
            1,
            1,
            1,
            false,
            1,
            10,
            "Hero",
        );
        assert_eq!(record, 25);
        assert!(outcome
            .messages
            .iter()
            .any(|m| m.contains("increased your own record to 25")));
    }

    #[test]
    fn distribute_rewards_reset_sums_worth_and_bonus_then_clears() {
        let mut data = PentagramDebugData {
            pent_worth: [10, 20, 0, 0, 0, 5],
            bonus: 7,
            status: 1,
            pent_cnt: 42,
            lucky_pents_this_solve: 2,
            ..Default::default()
        };
        let (had_combo, exp_reward) = distribute_rewards_reset(&mut data);
        assert!(had_combo);
        assert_eq!(exp_reward, 10 + 20 + 5 + 7);
        assert_eq!(data.bonus, 0);
        assert_eq!(data.status, 0);
        assert_eq!(data.lucky_pents_this_solve, 0);
        assert_eq!(data.pent_worth, [0; 6]);
        // pent_cnt is the lifetime counter - never reset here.
        assert_eq!(data.pent_cnt, 42);
    }
}
