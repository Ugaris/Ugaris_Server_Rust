//! Gate-welcome greeter NPC driver data (`CDR_GATE_WELCOME`).
//! World-side pass currently lives in `ugaris-server`.

#[allow(unused_imports)]
use crate::world::*;

// ---- legacy driver registry surface (moved from character_driver.rs) ----

#[allow(unused_imports)]
use crate::character_driver::*;

/// C `struct gate_welcome_driver_data` (`src/system/gatekeeper.c:411-415`):
/// the gatekeeper-welcome NPC's own driver memory (`CDR_GATE_WELCOME`,
/// distinct from the per-player `gate_ppd` in `crate::player::PlayerRuntime`
/// - see `world::gatekeeper`'s module doc comment for the split).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GateWelcomeDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
    pub amgivingback: i32,
}

//-----------------------
// Gatekeeper welcome dialogue (`src/system/gatekeeper.c::gate_welcome_driver`,
// `struct gate_ppd`'s `welcome_state` switch, lines 475-542).
//
// Pure state-machine port modeled on [`clara_dialogue_step`]: the caller
// (not yet wired - see `PORTING_TODO.md`'s "Gatekeeper NPC" task) is
// responsible for the message-loop plumbing (distance/visibility checks,
// the every-10-seconds throttle, `notify_char`/`say`) and for resolving
// `needs_lab` via `teleport_next_lab(co, 0)` before calling this.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GateWelcomeContext<'a> {
    pub player_name: &'a str,
    pub welcome_state: i32,
    /// C `teleport_next_lab(co, 0)` truthiness at the time of the call.
    pub needs_lab: bool,
    pub flags: CharacterFlags,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateWelcomeOutcome {
    pub welcome_state: i32,
    pub text: Option<String>,
}

/// C `gate_welcome_driver`'s `switch (ppd->welcome_state)` (`gatekeeper.c:
/// 475-542`), states `0..=6`. Text is `None` for the terminal "waiting for
/// answer" state (`6`) and for the labyrinth-still-needed wait (state `3`
/// re-checked with `needs_lab` still true).
pub fn gate_welcome_dialogue_step(context: GateWelcomeContext<'_>) -> GateWelcomeOutcome {
    let mut state = context.welcome_state;
    let text = match state {
        0 => {
            state = 1;
            Some(format!(
                "Be greeted, {}. These are the halls of Ishtar. Only the greatest fighters and magic users come here, to take the final test and fight the Gatekeeper.",
                context.player_name
            ))
        }
        1 => {
            state = 2;
            Some(
                "Those who succeed in this test will be able to enhance their abilities further. They may either choose to learn more about their profession than any other mortal being, or to start again as one who can learn all arts."
                    .to_string(),
            )
        }
        2 => {
            // C `case 2:` (`gatekeeper.c:491-500`) never `break`s, so it
            // always falls through into `case 3` in the same call.
            let mut text = None;
            if context.needs_lab {
                state = 3;
                text = Some(
                    "Before thou mayest engage the Gatekeeper, thou must solve the Labyrinth built by Ishtar. Thou canst enter the labyrinth through the door to the east."
                        .to_string(),
                );
            } else {
                state = 4;
            }
            if !gate_case3_stops(&mut state, context.needs_lab) {
                text = gate_case4(
                    &mut state,
                    context.needs_lab,
                    context.flags,
                    context.player_name,
                );
            }
            text
        }
        3 => {
            if gate_case3_stops(&mut state, context.needs_lab) {
                None
            } else {
                gate_case4(
                    &mut state,
                    context.needs_lab,
                    context.flags,
                    context.player_name,
                )
            }
        }
        4 => gate_case4(
            &mut state,
            context.needs_lab,
            context.flags,
            context.player_name,
        ),
        5 => {
            state = 6;
            Some(
                "Name the class thou wishest to become to begin the test. Each try will cost thee 100 gold coins."
                    .to_string(),
            )
        }
        _ => None,
    };

    GateWelcomeOutcome {
        welcome_state: state,
        text,
    }
}

/// C `gate_welcome_driver`'s `case 2:` of the `analyse_text_driver` switch
/// (`gatekeeper.c:565-570`): a `"repeat"`/`"restart"` answer resets the
/// dialogue to `0`, but only while `welcome_state <= 6` (a fully advanced
/// test-in-progress conversation is left alone).
pub fn gate_welcome_state_after_repeat(welcome_state: i32) -> i32 {
    if welcome_state <= 6 {
        0
    } else {
        welcome_state
    }
}

/// C `case 3:` body (`gatekeeper.c:501-506`): `if (!teleport_next_lab(co,
/// 0)) { welcome_state++; } else { break; }`. Returns `true` when C would
/// `break` (stop, no fallthrough into case 4).
fn gate_case3_stops(state: &mut i32, needs_lab: bool) -> bool {
    if needs_lab {
        true
    } else {
        *state += 1;
        false
    }
}
/// C `case 4:` body (`gatekeeper.c:508-533`). Mutates `state`/`text` the
/// same way C mutates `ppd->welcome_state`/calls `say` - note the two
/// non-arch branches do a plain `welcome_state++` off whatever value is
/// already in `state` when this runs, which is *not* always the same
/// number depending on whether case 4 was reached by falling through
/// from case 2 (fast path, ends at `6`) or from a separate call that
/// entered directly at case 3 after the labyrinth got solved later (slow
/// path, ends at `5` - an extra `case 5` "name the class" message gets
/// shown on the next call that the fast path skips entirely). This is a
/// faithfully-preserved legacy quirk, not a Rust bug.
fn gate_case4(
    state: &mut i32,
    needs_lab: bool,
    flags: CharacterFlags,
    player_name: &str,
) -> Option<String> {
    if needs_lab {
        *state = 2;
        return None;
    }
    if flags.contains(CharacterFlags::ARCH) {
        let class_name = if flags.contains(CharacterFlags::WARRIOR) {
            if flags.contains(CharacterFlags::MAGE) {
                "Seyan'Du"
            } else {
                "Warrior"
            }
        } else {
            "Mage"
        };
        *state = 6;
        Some(format!(
            "There is nothing I can do for thee, {player_name}, though, since thou art already an Arch-{class_name}."
        ))
    } else if flags.contains(CharacterFlags::MAGE) && flags.contains(CharacterFlags::WARRIOR) {
        *state += 1;
        Some(
            "Since thou art already a Seyan'Du, thy only choice is to become Arch-Seyan'Du."
                .to_string(),
        )
    } else {
        let path = if flags.contains(CharacterFlags::WARRIOR) {
            "Warrior"
        } else {
            "Mage"
        };
        *state += 1;
        Some(format!(
            "The choice is hard, and so is the test. If thou wishest to take the test, decide which path to follow. That of the Arch-{path}, or that of the Seyan'Du."
        ))
    }
}
