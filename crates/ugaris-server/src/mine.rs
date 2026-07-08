//! Server-side glue for Area 12's (`src/area/12/mine.c`) diggable-wall
//! reward cascade (`handle_mining_result`, wired from
//! `tick_item_use_minewall::dispatch_minewall_outcome` whenever a
//! `MineWallDig` outcome carries `opened: true`): the parts that need
//! `ZoneLoader` (instantiating "silver"/"gold"/orb stack items and golem
//! characters) or `PlayerRuntime`/achievement-repository access (military
//! mission silver tracking, the mined-amount achievement ladders, the
//! gold-earned/exp/military-point tails of the orb and artifact-relic
//! branches). The pure event-roll/amount-roll/cave-in math lives in
//! `ugaris-core`'s `world/mining.rs`.

use super::*;
use ugaris_core::legacy::INVENTORY_START_INVENTORY;
use ugaris_core::text::{
    expand_color_sentinels, COL_STR_AQUA, COL_STR_DARK_GRAY, COL_STR_HIDDEN_LINK,
    COL_STR_LIGHT_BLUE, COL_STR_LIGHT_GREEN, COL_STR_LIME, COL_STR_PINK, COL_STR_RESET,
    COL_STR_TAN, COL_STR_YELLOW,
};
use ugaris_core::world::{CaveInResult, MilitaryMissionSilverProgress, MiningEvent};

/// C `handle_mining_result`'s dispatch tail (`mine.c:229-275`): rolls the
/// weighted event and applies whichever branch fired. Called once per
/// wall that just reached `drdata[3] == 8` (`opened: true`).
#[allow(clippy::too_many_arguments)]
pub(crate) async fn apply_mine_wall_reward(
    world: &mut World,
    zone_loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    area_id: u16,
    item_id: ItemId,
    character_id: CharacterId,
    feedback: &mut Vec<(CharacterId, String)>,
) {
    match world.roll_mining_event() {
        MiningEvent::Silver => {
            apply_mine_silver_find(
                world,
                zone_loader,
                runtime,
                achievement_repository,
                item_id,
                character_id,
                feedback,
            )
            .await;
        }
        MiningEvent::Gold => {
            apply_mine_gold_find(
                world,
                zone_loader,
                runtime,
                achievement_repository,
                item_id,
                character_id,
                feedback,
            )
            .await;
        }
        MiningEvent::Golem => {
            apply_mine_golem_spawn(world, zone_loader, runtime, item_id, character_id, feedback);
        }
        MiningEvent::Orb => {
            apply_mine_orb_find(world, zone_loader, character_id);
        }
        MiningEvent::CaveIn => {
            apply_mine_cave_in(world, item_id, character_id, feedback);
        }
        MiningEvent::Artifact => {
            apply_mine_artifact_find(
                world,
                runtime,
                achievement_repository,
                area_id,
                character_id,
            )
            .await;
        }
        // C's own fall-through ("nothing of value") is also a documented
        // no-op (the message text is commented out in C itself - "too
        // spammy").
        MiningEvent::Nothing => {}
    }
}

/// C `handle_silver_find` (`mine.c:290-302`).
async fn apply_mine_silver_find(
    world: &mut World,
    zone_loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    item_id: ItemId,
    character_id: CharacterId,
    feedback: &mut Vec<(CharacterId, String)>,
) {
    let Some(amount) = world.roll_mining_silver_amount(item_id, character_id) else {
        return;
    };
    let amount = amount.max(0) as u32;
    let Some(result) = give_mine_item(
        world,
        zone_loader,
        character_id,
        StackKind::SilverUnit,
        amount,
    ) else {
        return;
    };
    push_give_mine_item_feedback(feedback, character_id, &result);
    apply_military_mission_silver_check(world, runtime, character_id, amount as i32);
    award_silver_mined_achievement(world, runtime, achievement_repository, character_id, amount)
        .await;
}

/// C `handle_gold_find` (`mine.c:304-318`): identical to the silver
/// branch except gated on `amount > 0` and counting gold double for the
/// military mission check (`check_military_silver(cn, amount * 2)`).
async fn apply_mine_gold_find(
    world: &mut World,
    zone_loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    item_id: ItemId,
    character_id: CharacterId,
    feedback: &mut Vec<(CharacterId, String)>,
) {
    let Some(amount) = world.roll_mining_gold_amount(item_id, character_id) else {
        return;
    };
    if amount <= 0 {
        return;
    }
    let amount = amount as u32;
    let Some(result) = give_mine_item(
        world,
        zone_loader,
        character_id,
        StackKind::GoldUnit,
        amount,
    ) else {
        return;
    };
    push_give_mine_item_feedback(feedback, character_id, &result);
    apply_military_mission_silver_check(world, runtime, character_id, (amount * 2) as i32);
    award_gold_mined_achievement(world, runtime, achievement_repository, character_id, amount)
        .await;
}

/// C `handle_cave_in` (`mine.c:362-403`).
pub(crate) fn apply_mine_cave_in(
    world: &mut World,
    item_id: ItemId,
    character_id: CharacterId,
    feedback: &mut Vec<(CharacterId, String)>,
) {
    let Some(result) = world.apply_mining_cave_in(item_id, character_id) else {
        return;
    };
    match result {
        CaveInResult::Avoided => {
            feedback.push((
                character_id,
                "Your mining expertise helped you avoid a cave-in!".to_string(),
            ));
        }
        CaveInResult::Collapsed {
            endurance_loss_units,
            unreduced_loss_units,
            now_exhausted,
        } => {
            let message = if let Some(unreduced) = unreduced_loss_units {
                format!(
                    "The mine wall suddenly collapses! Thanks to your athletic prowess, you swiftly escape, losing only {endurance_loss_units} endurance instead of {unreduced}!"
                )
            } else {
                format!(
                    "The mine wall suddenly collapses! You barely escape, losing {endurance_loss_units} endurance!"
                )
            };
            feedback.push((character_id, message));
            if now_exhausted {
                feedback.push((
                    character_id,
                    "You're exhausted from the cave-in. Be careful!".to_string(),
                ));
            }
        }
    }
}

/// C `handle_orb_find` (`mine.c:328-360`): rolls one of five skills
/// (`V_IMMUNITY`/`V_ATTACK`/`V_PARRY`/`V_FLASH`/`V_MAGICSHIELD`) and
/// instantiates a `+5` orb of it via the same `"empty_orb"` template/
/// `drdata` layout as `create_orb_with_value`/[`grant_created_orb`],
/// placed with the plain (non-"smart") [`World::give_char_item`] -
/// digging requires an empty cursor (see `world/mining.rs`'s module doc
/// comment), so this always lands on the cursor in practice, matching
/// C's `give_char_item(cn, in2)`. A real C quirk reproduced exactly: the
/// success/failure message branches purely on whether `in2` (item
/// creation) succeeded, *not* on `give_char_item`'s own return value -
/// if placement somehow failed (both cursor and inventory full), C still
/// reports success and orphans the item; here the orb is still added to
/// `world.items` (so it exists exactly as it would in C, just
/// unreachable) even if `give_char_item` returns `false`.
fn apply_mine_orb_find(world: &mut World, loader: &mut ZoneLoader, character_id: CharacterId) {
    // C's local `skills[]` table (`mine.c:329-336`). Every one of its
    // display names is identical to the shared `CHARACTER_VALUE_NAMES`
    // entry for that skill (including `V_FLASH`'s "Lightning"), so no
    // separate name table is needed - `CHARACTER_VALUE_NAMES[skill as
    // usize]` serves both this function's own message text and the
    // orb's `create_orb_with_value`-style item name.
    const ORB_SKILLS: [CharacterValue; 5] = [
        CharacterValue::Immunity,
        CharacterValue::Attack,
        CharacterValue::Parry,
        CharacterValue::Flash,
        CharacterValue::MagicShield,
    ];
    let skill = ORB_SKILLS[world.roll_legacy_random(ORB_SKILLS.len() as u32) as usize];
    let value_name = CHARACTER_VALUE_NAMES[skill as usize];

    let Ok(mut orb) = loader.instantiate_item_template("empty_orb", Some(character_id)) else {
        world.queue_system_text(
            character_id,
            "Alas! Though fortune smiled upon thee, some foul magic hath intervened. The orb slipped through thy fingers like sand.".to_string(),
        );
        return;
    };
    orb.name = format!("Orb of 5 {value_name}");
    ensure_drdata_len(&mut orb, 2);
    orb.driver_data[0] = skill as u8;
    orb.driver_data[1] = 5;
    let orb_id = orb.id;
    world.add_item(orb);
    world.give_char_item(character_id, orb_id);

    world.queue_system_text_bytes(
        character_id,
        expand_color_sentinels(&format!(
            "{COL_STR_TAN}Odds bodkins!{COL_STR_RESET} Thou art blessed by Ishtar's fortune this day! Amidst the common stones, thou hast unearthed a {COL_STR_PINK}mystical orb of cerulean radiance{COL_STR_RESET}. Though its purpose eludes thee, 'tis surely a prize worth keeping. The orb now resides in thy possession."
        )),
    );
    world.queue_system_text_bytes(
        character_id,
        expand_color_sentinels(&format!(
            "Thou hast received: {COL_STR_LIGHT_BLUE}Orb of {value_name}{COL_STR_RESET} +5"
        )),
    );
}

/// C `Sirname(cn)` (`src/system/tool.c:1538-1546`) - same local-copy
/// precedent as `world/npc/area11/islena.rs::islena_sirname` (no shared
/// helper exists yet, and this is the only other call site).
fn mine_sirname(character: &Character) -> &'static str {
    if character.flags.contains(CharacterFlags::MALE) {
        "Sir"
    } else if character.flags.contains(CharacterFlags::FEMALE) {
        "Lady"
    } else {
        "Neuter"
    }
}

/// C `handle_artifact_find`'s `ye_olde_artifacts[]` table (`mine.c:406-
///427`), transcribed digit-for-digit including its embedded `COL_*`
/// markers (via the `COL_STR_*` sentinel convention, expanded at the
/// call site).
fn mine_artifact_description(index: usize) -> String {
    match index {
        0 => format!(
            "a {COL_STR_PINK}petrified trencher{COL_STR_RESET} from the {COL_STR_LIGHT_BLUE}Age of Seyan I{COL_STR_RESET}"
        ),
        1 => format!(
            "an {COL_STR_PINK}ancient Astonian soldier's ration box{COL_STR_RESET}, still bearing a most questionable Luctim-infused fruit{COL_STR_RESET}"
        ),
        2 => format!(
            "a {COL_STR_PINK}fossilized wedge of Cristalim cheese{COL_STR_RESET}, nigh indistinguishable from common stone"
        ),
        3 => format!(
            "a {COL_STR_PINK}prehistoric Seyan'Du training tool{COL_STR_RESET}, or mayhap 'tis but a pointy stick{COL_STR_RESET}"
        ),
        4 => format!(
            "the {COL_STR_PINK}Empire's most ancient scrying orb{COL_STR_RESET}, hewn from solid Elohil crystal{COL_STR_RESET}"
        ),
        5 => format!(
            "a {COL_STR_PINK}stone tablet{COL_STR_RESET} bearing the inscription 'Gone to Battle Demons{COL_STR_RESET}'"
        ),
        6 => format!(
            "a perfectly preserved {COL_STR_PINK}Ishtar follower's beard-trimming implement{COL_STR_RESET} (verily, 'tis but a magically sharpened rock{COL_STR_RESET})"
        ),
        7 => format!(
            "the {COL_STR_PINK}first Astonian multi-tool{COL_STR_RESET}, in truth naught but a {COL_STR_LIGHT_GREEN}rock tied to a stick{COL_STR_RESET} with enchanted twine"
        ),
        8 => format!(
            "an {COL_STR_PINK}antique Labyrinth explorer's helm{COL_STR_RESET}, or perchance merely a {COL_STR_HIDDEN_LINK}bowl-shaped rock{COL_STR_RESET}"
        ),
        9 => format!(
            "the {COL_STR_PINK}oldest known pair of Mage's robes{COL_STR_RESET}, now {COL_STR_DARK_GRAY}perfectly crystallized{COL_STR_RESET} by ancient magicks"
        ),
        10 => format!(
            "a {COL_STR_PINK}mystical amulet{COL_STR_RESET} that grants the power of {COL_STR_AQUA}excessive resistance to Demon rhinorrhea{COL_STR_RESET}"
        ),
        _ => format!(
            "the {COL_STR_PINK}lost treasure map{COL_STR_RESET} to the fabled {COL_STR_LIGHT_GREEN}Vault of Eternal Luctim{COL_STR_RESET}"
        ),
    }
}

/// C `handle_artifact_find` (`mine.c:405-504`): picks one of 12 ye-olde-
/// artifact flavor descriptions, then rolls a rarity tier deciding the
/// reward (50% a pittance of exp, 30% 50-4950 silver, 15% up to 10
/// military points, 5% both a larger exp grant and 150-9950 silver -
/// note the "gold" local variable throughout is actually a silver
/// amount, matching C's own `give_money(cn, gold, ...)`/`gold / 100`
/// display convention), plus an independent 5% chance for a trailing
/// self-aware punchline. The commented-out 30%-chance "extra lines"
/// block (`mine.c:477-493`) stays a no-op, matching C's own disabled
/// code.
async fn apply_mine_artifact_find(
    world: &mut World,
    runtime: &mut ServerRuntime,
    achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    area_id: u16,
    character_id: CharacterId,
) {
    let Some(character) = world.characters.get(&character_id) else {
        return;
    };
    let level = character.level;

    let artifact_index = world.roll_legacy_random(12) as usize;
    let artifact = mine_artifact_description(artifact_index);
    let rarity = world.roll_legacy_random(100) as i32;

    world.queue_system_text_bytes(
        character_id,
        expand_color_sentinels(&format!(
            "{COL_STR_TAN}Hark!{COL_STR_RESET} Thou hast unearthed a relic from the {COL_STR_LIGHT_BLUE}Age of Seyan{COL_STR_RESET}! It appeareth to be {artifact}."
        )),
    );

    if rarity < 50 {
        let exp = level_value(level) / 750;
        world.give_exp(character_id, i64::from(exp), u32::from(area_id));
        world.queue_system_text_bytes(
            character_id,
            expand_color_sentinels(&format!(
                "Ishtar smiles upon thy discovery, granting thee {COL_STR_LIGHT_GREEN}experience{COL_STR_RESET}, though 'tis but a pittance compared to the Labyrinth Quest."
            )),
        );
    } else if rarity < 80 {
        let silver = 50 + world.roll_legacy_random(50) * 100;
        let mut feedback_bytes = Vec::new();
        achievement::give_money(
            world,
            runtime,
            achievement_repository,
            character_id,
            silver,
            &mut feedback_bytes,
        )
        .await;
        for (recipient, message) in feedback_bytes {
            world.queue_system_text_bytes(recipient, message);
        }
        world.queue_system_text_bytes(
            character_id,
            expand_color_sentinels(&format!(
                "The coffers of ancient Aston favor thee! Thou findest {COL_STR_YELLOW} {} gold coins{COL_STR_RESET} amidst the ruins of the Empire.",
                silver / 100
            )),
        );
    } else if rarity < 95 {
        let pts = (i32::try_from(level).unwrap_or(0) / 3).min(10);
        world.give_military_pts(character_id, pts, 1, u32::from(area_id));
        let name = mine_sirname(&world.characters[&character_id]);
        world.queue_system_text_bytes(
            character_id,
            expand_color_sentinels(&format!(
                "{COL_STR_TAN}Huzzah!{COL_STR_RESET} The {COL_STR_LIGHT_BLUE}Seyan'Du{COL_STR_RESET} would be most proud of thine achievement this day, {COL_STR_LIGHT_GREEN}{name}{COL_STR_RESET}. Verily, finding such {artifact} is a feat worthy of the Imperial records... or at least a whisper in the Labyrinth's echoing halls."
            )),
        );
    } else {
        let exp = level_value(level) / 250;
        let silver = 150 + world.roll_legacy_random(100) * 100;
        world.give_exp(character_id, i64::from(exp), u32::from(area_id));
        let mut feedback_bytes = Vec::new();
        achievement::give_money(
            world,
            runtime,
            achievement_repository,
            character_id,
            silver,
            &mut feedback_bytes,
        )
        .await;
        for (recipient, message) in feedback_bytes {
            world.queue_system_text_bytes(recipient, message);
        }
        world.queue_system_text_bytes(
            character_id,
            // C's source has a stray non-ASCII byte (`0xED`) mid-word here
            // ("Cristal\xEDm") - reproduced as U+00ED ('i' with acute,
            // matching the other artifact entries' "Cristalim" spelling)
            // since the raw byte alone isn't valid standalone UTF-8 and
            // this is flavor text, not a formula/constant.
            expand_color_sentinels(&format!(
                "{COL_STR_LIME}By Ishtar's light!{COL_STR_RESET} 'Tis a discovery most extraordinary! Thou receivest {COL_STR_LIGHT_GREEN}experience{COL_STR_RESET} and {COL_STR_YELLOW}{} gold{COL_STR_RESET}, as if blessed by the Cristal\u{ed}m himself!",
                silver / 100
            )),
        );
    }

    if world.roll_legacy_random(100) < 5 {
        let name = mine_sirname(&world.characters[&character_id]);
        world.queue_system_text_bytes(
            character_id,
            expand_color_sentinels(&format!(
                "{COL_STR_TAN}Psst!{COL_STR_RESET} Between thee and me, good {name}, methinks the Mage who enchanted these artifacts had imbibed too much of Ishtar's mystical brew."
            )),
        );
    }
}

/// Result of [`give_mine_item`] (C `give_mine_item`, `mine.c:506-557`).
#[derive(Debug)]
pub(crate) enum GiveMineItemResult {
    /// Merged into an existing pile in the digger's inventory
    /// (`give_mine_item`'s "found a same pile" branch).
    MergedIntoPile {
        amount: u32,
        total: u32,
        name: String,
    },
    /// Placed fresh on the digger's cursor (either the 2%-chance roll, or
    /// no matching pile was found).
    Cursor { amount: u32, name: String },
}

fn push_give_mine_item_feedback(
    feedback: &mut Vec<(CharacterId, String)>,
    character_id: CharacterId,
    result: &GiveMineItemResult,
) {
    match result {
        GiveMineItemResult::MergedIntoPile {
            amount,
            total,
            name,
        } => {
            feedback.push((
                character_id,
                format!(
                    "You found {amount} units of {name}. You now have a total of {total} {name} units."
                ),
            ));
        }
        GiveMineItemResult::Cursor { amount, name } => {
            feedback.push((character_id, format!("You found {amount} units of {name}.")));
        }
    }
}

/// C `give_mine_item(in, cn, item_name, amount)` (`mine.c:506-557`):
/// creates a fresh "silver"/"gold" stack item worth `amount` units, then
/// either merges it into a matching pile already in the digger's
/// inventory (slots `INVENTORY_START_INVENTORY..`) or places it on their
/// (guaranteed-empty, since digging requires an empty cursor) cursor -
/// with a 2% chance to always go straight to the cursor regardless of any
/// existing pile. Returns `None` only if the "silver"/"gold" template
/// fails to instantiate (C: `elog` + early return, no
/// `check_military_silver`/achievement tail).
pub(crate) fn give_mine_item(
    world: &mut World,
    loader: &mut ZoneLoader,
    character_id: CharacterId,
    kind: StackKind,
    amount: u32,
) -> Option<GiveMineItemResult> {
    let add_to_cursor = world.roll_legacy_random(100) < 2;

    let mut new_item = loader
        .instantiate_item_template(stack_template(kind), Some(character_id))
        .ok()?;
    new_item.value = new_item.value.saturating_mul(amount);
    set_stack_count(&mut new_item, amount, kind);
    let new_item_id = new_item.id;
    let item_name = new_item.name.clone();

    if !add_to_cursor {
        let existing = world.characters.get(&character_id)?.inventory[INVENTORY_START_INVENTORY..]
            .iter()
            .flatten()
            .find(|id| {
                world
                    .items
                    .get(id)
                    .is_some_and(|item| stack_kind(item) == Some(kind))
            })
            .copied();
        if let Some(existing_id) = existing {
            let added_value = new_item.value;
            let existing_item = world.items.get_mut(&existing_id)?;
            existing_item.value = existing_item.value.saturating_add(added_value);
            let total = stack_count(existing_item).saturating_add(amount);
            set_stack_count(existing_item, total, kind);
            let name = existing_item.name.clone();
            if let Some(character) = world.characters.get_mut(&character_id) {
                character.flags.insert(CharacterFlags::ITEMS);
            }
            // C `destroy_item(in2)`: the freshly-created item is never
            // inserted into the world at all.
            return Some(GiveMineItemResult::MergedIntoPile {
                amount,
                total,
                name,
            });
        }
    }

    let character = world.characters.get_mut(&character_id)?;
    character.cursor_item = Some(new_item_id);
    character.flags.insert(CharacterFlags::ITEMS);
    world.add_item(new_item);
    Some(GiveMineItemResult::Cursor {
        amount,
        name: item_name,
    })
}

/// C `check_military_silver(cn, amount)` (`mine.c:102-134`), applied
/// after a successful silver/gold grant: resends the questlog display
/// (`sendquestlog`) whenever there's any active unsolved mission, and
/// pushes the C `log_char` progress/completion text via the queued
/// system-text mechanism (matching `military::apply_military_mission_
/// kill_check`'s established pattern for this same PPD).
pub(crate) fn apply_military_mission_silver_check(
    world: &mut World,
    runtime: &mut ServerRuntime,
    character_id: CharacterId,
    amount: i32,
) {
    let Some(player) = runtime.player_for_character_mut(character_id) else {
        return;
    };
    let outcome = player.check_military_silver(amount);
    let questlog_payload = (!matches!(outcome, MilitaryMissionSilverProgress::NoMission))
        .then(|| legacy_questlog_payload(player));

    let message: Option<&'static str> = match outcome {
        MilitaryMissionSilverProgress::NoMission
        | MilitaryMissionSilverProgress::NotSilverMission => None,
        MilitaryMissionSilverProgress::Progress { .. } => None,
        MilitaryMissionSilverProgress::Solved => {
            Some("You solved your mission. Talk to the governor to claim your reward.")
        }
    };

    if let Some(payload) = questlog_payload {
        for (session_id, _) in runtime.sessions_for_character(character_id) {
            runtime.send_to_session(session_id, payload.clone());
        }
    }
    if let MilitaryMissionSilverProgress::Progress { remaining } = outcome {
        world.queue_system_text(
            character_id,
            format!("You fulfilled part of your mission, you still need {remaining} silver."),
        );
    } else if let Some(message) = message {
        world.queue_system_text(character_id, message.to_string());
    }
}

/// C `handle_golem_spawn` (`mine.c:320-326`).
pub(crate) fn apply_mine_golem_spawn(
    world: &mut World,
    loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    item_id: ItemId,
    character_id: CharacterId,
    feedback: &mut Vec<(CharacterId, String)>,
) {
    let golem_type = world
        .items
        .get(&item_id)
        .and_then(|item| item.driver_data.get(2).copied())
        .unwrap_or_default();
    if world.roll_mining_golem_rare() {
        if spawn_rare_golem(world, loader, runtime, item_id, golem_type) {
            feedback.push((
                character_id,
                "A rare, stronger golem has appeared!".to_string(),
            ));
        }
    } else {
        spawn_normal_golem(world, loader, runtime, item_id, golem_type);
    }
}

/// Shared silver-vs-gold drop template selection
/// (`spawn_normal_golem`/`spawn_rare_golem`'s `it[in].drdata[2] <=
/// get_max_silver_golem_type()` check, `mine.c:589,627`).
fn golem_drop_stack_kind(world: &World, golem_type: u8) -> StackKind {
    if i32::from(golem_type) <= world.settings.max_silver_golem_type {
        StackKind::SilverUnit
    } else {
        StackKind::GoldUnit
    }
}

/// Golem loot-drop instantiation shared by [`spawn_normal_golem`]/
/// [`spawn_rare_golem`] (`mine.c:585-601`/`:623-639`): places a fresh
/// silver/gold stack item worth `amount` directly into the golem's
/// `golem_inventory_slot`.
fn grant_golem_drop(
    world: &mut World,
    loader: &mut ZoneLoader,
    character_id: CharacterId,
    golem_type: u8,
    amount: u32,
) {
    let kind = golem_drop_stack_kind(world, golem_type);
    let Ok(mut drop_item) =
        loader.instantiate_item_template(stack_template(kind), Some(character_id))
    else {
        return;
    };
    drop_item.value = amount;
    set_stack_count(&mut drop_item, amount, kind);
    let drop_item_id = drop_item.id;
    let slot = world.settings.golem_inventory_slot.max(0) as usize;
    let Some(character) = world.characters.get_mut(&character_id) else {
        return;
    };
    if slot >= character.inventory.len() {
        return;
    }
    character.inventory[slot] = Some(drop_item_id);
    world.add_item(drop_item);
}

/// C `spawn_normal_golem` (`mine.c:571-607`).
pub(crate) fn spawn_normal_golem(
    world: &mut World,
    loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    item_id: ItemId,
    golem_type: u8,
) -> bool {
    let Some((x, y)) = world.items.get(&item_id).map(|item| (item.x, item.y)) else {
        return false;
    };
    let template = format!("miner{golem_type}");
    let character_id = runtime.allocate_character_id();
    let Ok((mut golem, inventory_items)) =
        loader.instantiate_character_template(&template, character_id)
    else {
        return false;
    };
    golem.dir = Direction::RightDown as u8;
    golem.hp = i32::from(golem.values[0][CharacterValue::Hp as usize]) * POWERSCALE;
    golem.endurance = i32::from(golem.values[0][CharacterValue::Endurance as usize]) * POWERSCALE;
    golem.mana = i32::from(golem.values[0][CharacterValue::Mana as usize]) * POWERSCALE;
    let level = golem.level;
    if !world.spawn_character(golem, usize::from(x), usize::from(y)) {
        return false;
    }
    for item in inventory_items {
        world.items.insert(item.id, item);
    }

    if (world.roll_legacy_random(100) as i32) < world.settings.normal_drop_chance {
        let amount = world
            .calculate_golem_drop_amount(level as i32, false)
            .max(0) as u32;
        grant_golem_drop(world, loader, character_id, golem_type, amount);
    }
    true
}

/// C `spawn_rare_golem` (`mine.c:609-647`): unlike the normal variant,
/// only `hp` is explicitly (re-)set (scaled by both `POWERSCALE` and
/// `get_rare_golem_hp_multiplier()`) - `endurance`/`mana` are left at
/// whatever `create_char`/`instantiate_character_template` already
/// initialized them to (the template's full `value[0][V_ENDURANCE]`/
/// `value[0][V_MANA]`, same as the normal golem's redundant explicit
/// assignment would produce anyway). The level boost is applied *after*
/// `hp` is computed from the template's pre-boost `value[0][V_HP]` (a
/// real C quirk: `update_char`/value recompute is never re-run after the
/// level bump, so the boosted level has no effect on the golem's own
/// combat stats here - only on `calculate_drop_amount`'s loot roll,
/// which reads the already-boosted `ch[co].level`).
pub(crate) fn spawn_rare_golem(
    world: &mut World,
    loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    item_id: ItemId,
    golem_type: u8,
) -> bool {
    let Some((x, y)) = world.items.get(&item_id).map(|item| (item.x, item.y)) else {
        return false;
    };
    let template = format!("miner{golem_type}");
    let character_id = runtime.allocate_character_id();
    let Ok((mut golem, inventory_items)) =
        loader.instantiate_character_template(&template, character_id)
    else {
        return false;
    };
    golem.dir = Direction::RightDown as u8;
    golem.hp = i32::from(golem.values[0][CharacterValue::Hp as usize])
        * POWERSCALE
        * world.settings.rare_golem_hp_multiplier;
    golem.level = golem
        .level
        .saturating_add(world.settings.rare_golem_level_boost.max(0) as u32);
    let level = golem.level;
    if !world.spawn_character(golem, usize::from(x), usize::from(y)) {
        return false;
    }
    for item in inventory_items {
        world.items.insert(item.id, item);
    }

    if (world.roll_legacy_random(100) as i32) < world.settings.rare_drop_chance {
        let amount = world.calculate_golem_drop_amount(level as i32, true).max(0) as u32;
        grant_golem_drop(world, loader, character_id, golem_type, amount);
    }
    true
}

/// C `keyholder_door`'s golem-spawn tail (`mine.c:1196-1208`), called from
/// `tick_item_use_keyassembly::dispatch_keyassembly_outcome` once
/// `World::apply_item_driver_outcome` has already teleported the player
/// into the room and returned `MineKeyDoorOpened { golem_nr, room_x,
/// room_y, .. }`. `room_x`/`room_y` are the player's own teleport target
/// (`2 + (n%3)*8 + 1, 231 + (n/3)*8 + 3`); the golem spawns 4 tiles east
/// of that, at `2 + (n%3)*8 + 5, 231 + (n/3)*8 + 3` (`mine.c:1187,1204-
/// 1207`).
pub(crate) fn spawn_keyholder_golem(
    world: &mut World,
    loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    player_id: CharacterId,
    golem_nr: u8,
    room_x: u16,
    room_y: u16,
) {
    let template = format!("keyholder_golem{golem_nr}");
    let golem_id = runtime.allocate_character_id();
    let Ok((mut golem, inventory_items)) =
        loader.instantiate_character_template(&template, golem_id)
    else {
        return;
    };
    let golem_x = room_x + 4;
    let golem_y = room_y;
    // C `ch[co].dir = DX_LEFTUP;` (`mine.c:1203`).
    golem.dir = Direction::LeftUp as u8;
    golem.hp = i32::from(golem.values[0][CharacterValue::Hp as usize]) * POWERSCALE;
    golem.endurance = i32::from(golem.values[0][CharacterValue::Endurance as usize]) * POWERSCALE;
    golem.mana = i32::from(golem.values[0][CharacterValue::Mana as usize]) * POWERSCALE;
    // C `ch[co].tmpx/tmpy` (`mine.c:1204-1205`), read back by
    // `keyhold_fight_driver`'s `secure_move_driver(cn, ch[cn].tmpx,
    // ch[cn].tmpy, ...)` "return to post" call: no dedicated `tmpx`/`tmpy`
    // field exists yet, so `rest_x`/`rest_y` stand in, same substitution
    // `gate_enter_test_spawn_room` already made for `CDR_GATE_FIGHT`.
    golem.rest_x = golem_x;
    golem.rest_y = golem_y;
    // C never sends the golem an explicit victim message (unlike
    // `CDR_GATE_FIGHT`'s `NT_NPC`/`NTID_GATEKEEPER`) - see
    // `world::npc::area12::golemkeyholder`'s module doc comment for why
    // setting `victim` directly here reproduces the same observable
    // "attacks the summoning player" behavior.
    golem.driver_state = Some(CharacterDriverState::GolemKeyhold(GolemKeyholdDriverData {
        victim: Some(player_id),
        ..Default::default()
    }));
    if !world.spawn_character(golem, usize::from(golem_x), usize::from(golem_y)) {
        return;
    }
    for item in inventory_items {
        world.items.insert(item.id, item);
    }
}
