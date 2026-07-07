//! Completed-action-outcome handling: the special-consumables and
//! reading-material family of `ItemDriverOutcome` variants (holiday
//! lollipops/Christmas pops, the "special potion" fun-effect line-up,
//! and the generic book/bookcase text drivers shared across areas -
//! including area 17's two-city bookcase puzzle wired to the same
//! `twocity_goodtile`/`twocity_solved_library` state as `ColorTile`).
//! Split out of the giant `match outcome { ... }` block that still
//! lives inline in `main.rs`'s `tick.tick()` arm (P0.5 "Finish main()
//! phase decomposition" - REMAINING note: the completed-action-outcome
//! handling needs splitting by completed-action-kind family across
//! several files, not just relocation, because the whole match is too
//! large to move verbatim into one file). Warp, chests, dungeon,
//! ice/palace, Teufel, skel-raise, edemon/fdemon, transport, clan/lq/
//! arena, shrines, burndown, xmas/swamp, Caligar, key-assembly,
//! labyrinth, mine-wall, and forest-spade/junkpile/pick-door were
//! sliced first; this is the eighteenth family slice. The rest of the
//! match (single-variant orb/nomad/two-city handlers, ...) is still
//! inline in `main.rs` pending further slices.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) fn dispatch_books_potions_outcome(
    world: &mut World,
    runtime: &mut ServerRuntime,
    area_id: u16,
    outcome: ugaris_core::item_driver::ItemDriverOutcome,
    feedback: &mut Vec<(CharacterId, String)>,
    feedback_bytes: &mut Vec<(CharacterId, Vec<u8>)>,
    special_feedback: &mut Vec<(CharacterId, bytes::BytesMut)>,
    area_feedback: &mut Vec<(CharacterId, String, u16)>,
    executed: &mut i32,
    blocked: &mut i32,
) {
    match outcome {
        ugaris_core::item_driver::ItemDriverOutcome::LollipopLicked { character_id, .. } => {
            area_feedback.push((character_id, lollipop_area_message(world, character_id), 10));
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::LollipopMemories { character_id, .. } => {
            feedback.push((character_id, "Ahh memories, sweet memories.".to_string()));
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::ChristmasPopInspected {
            character_id, ..
        } => {
            for message in christmas_pop_inspection_messages() {
                feedback.push((character_id, message.to_string()));
            }
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::SpecialPotionDrunk {
            character_id,
            kind,
            ..
        } => {
            if let Some(message) = special_potion_fun_message(world, character_id, kind) {
                area_feedback.push((character_id, message, 16));
            }
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::SpecialPotionAntidote {
            character_id,
            poison_removed,
            ..
        } => {
            feedback.push((
                character_id,
                if poison_removed {
                    "You feel better."
                } else {
                    "It didn't have any effect."
                }
                .to_string(),
            ));
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::SpecialPotionInfravision {
            character_id,
            ..
        } => {
            feedback.push((character_id, "Your eyes start to itch.".to_string()));
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::SpecialPotionSecurity {
            character_id,
            used,
            ..
        } => {
            feedback.push((
                character_id,
                if used {
                    "You feel secure."
                } else {
                    "You don't feel like drinking this potion now."
                }
                .to_string(),
            ));
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::SpecialPotionProfessionReset {
            character_id,
            used,
            ..
        } => {
            if !used {
                feedback.push((
                    character_id,
                    "You don't feel like drinking this potion now.".to_string(),
                ));
            }
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::SpecialPotionBug { character_id, .. } => {
            feedback.push((character_id, "Please report bug #1734.".to_string()));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::BookText {
            character_id,
            kind,
            demon_value,
            ..
        } => {
            let lines = if kind == ugaris_core::item_driver::BOOK_NOOK_JOKES {
                ugaris_core::item_driver::book_nook_joke_line_bytes(runtime_random_below(5) as u32)
            } else {
                ugaris_core::item_driver::book_text_line_bytes_for_reader_id(
                    kind,
                    demon_value,
                    character_id.0,
                )
            };
            for line in lines {
                feedback_bytes.push((character_id, line));
            }
            if let Some(special_type) = ugaris_core::item_driver::book_special_effect(kind) {
                special_feedback.push((
                    character_id,
                    bytes::BytesMut::from(
                        &ugaris_protocol::packet::special(special_type, 0, 0)[..],
                    ),
                ));
            }
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::BookcaseText {
            character_id, kind, ..
        } => {
            let mut random_index = runtime_random_below(26) as u8;
            let mut color = 1;
            let mut solved_library = false;
            let mut grant_library_exp = false;
            if let Some(player) = runtime.player_for_character_mut(character_id) {
                let colors =
                    player.ensure_twocity_goodtile_with(|| runtime_random_below(6) as u8 + 1);
                color = match kind {
                    2..=6 => colors[usize::from(kind - 2)],
                    _ => 1,
                };
                solved_library = player.twocity_solved_library;
                if kind == 1 && !player.twocity_solved_library {
                    player.twocity_solved_library = true;
                    grant_library_exp = true;
                }
            }
            if grant_library_exp {
                // C `bookcase` (`area/17/two.c:2622`) grants the
                // library-solved exp via `give_exp(cn, ...)`, not a
                // raw mutation.
                if let Some(level) = world
                    .characters
                    .get(&character_id)
                    .map(|character| character.level)
                {
                    let exp_added = ugaris_core::item_driver::bookcase_library_exp(level);
                    world.give_exp(character_id, i64::from(exp_added), u32::from(area_id));
                }
            }
            if kind != 0 {
                random_index = 0;
            }
            feedback_bytes.push((
                character_id,
                ugaris_core::item_driver::bookcase_text_line_bytes(
                    kind,
                    random_index,
                    color,
                    solved_library,
                ),
            ));
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::BookcaseLocked { character_id, .. } => {
            for line in ugaris_core::item_driver::bookcase_locked_text_lines() {
                feedback.push((character_id, line.to_string()));
            }
            *blocked += 1;
        }
        _ => {}
    }
}
