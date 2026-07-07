//! Area 3 Clara quest NPC (`CDR_SWAMPCLARA`) dialogue state helpers.

#[allow(unused_imports)]
use crate::world::*;

// ---- legacy driver registry surface (moved from character_driver.rs) ----

#[allow(unused_imports)]
use crate::character_driver::*;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ClaraDriverData {
    pub last_talk_tick: i32,
    pub current_victim: Option<CharacterId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaraDialogueContext<'a> {
    pub player_name: &'a str,
    pub clara_name: &'a str,
    pub army_rank: &'a str,
    pub kelly_state: i32,
    pub clara_state: i32,
    pub has_hardkill_item: bool,
    pub hardkill_ritual_progress: u8,
    pub questlog_21_count: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaraDialogueOutcome {
    pub clara_state: i32,
    pub text: Option<String>,
    pub open_questlog: Option<u16>,
    pub complete_questlog: Option<u16>,
    pub military_points: i32,
    pub military_exp: i32,
}

pub fn clara_dialogue_step(context: ClaraDialogueContext<'_>) -> ClaraDialogueOutcome {
    let mut state = context.clara_state;
    let mut open_questlog = None;
    let mut complete_questlog = None;
    let mut military_points = 0;
    let mut military_exp = 0;
    let text = match state {
        0 => {
            state += 1;
            Some(format!(
                "Greetings, {}! I am {}, First Sergeant of the Seyan'Du and commander of this outpost.",
                context.player_name, context.clara_name
            ))
        }
        1 if context.kelly_state >= 15 => {
            state += 1;
            clara_dialogue_step_text_after_fallthrough(&mut state, context)
        }
        1 => None,
        2 => clara_dialogue_step_text_after_fallthrough(&mut state, context),
        3 => {
            state += 1;
            Some(
                "Under the current circumstances, I do not recommend sending reinforcements to secure the road. We cannot afford to bind our forces here. Now go back to Aston and deliver this report."
                    .to_string(),
            )
        }
        4 => {
            state += 1;
            Some(format!(
                "Afterwards come back here, I have more work for thee. That will be all, {}. Dismissed!",
                context.army_rank
            ))
        }
        5 if context.kelly_state >= 18 => {
            state += 1;
            open_questlog = Some(21);
            state += 1;
            Some(format!(
                "I have a difficult mission for thee, {}. The main reason we had to retreat to this camp was one huge swamp beast. It seemed to be immune to our attacks.",
                context.player_name
            ))
        }
        5 => None,
        6 => {
            open_questlog = Some(21);
            state += 1;
            Some(format!(
                "I have a difficult mission for thee, {}. The main reason we had to retreat to this camp was one huge swamp beast. It seemed to be immune to our attacks.",
                context.player_name
            ))
        }
        7 => {
            state += 1;
            Some(
                "I want thee to find a way to slay it. I have heard rumors about a man who used to live with the swamp beasts north-east of this camp. Mayhap he knows a way to injure this beast."
                    .to_string(),
            )
        }
        8 => {
            state += 1;
            Some(format!(
                "Dismissed, {}. And good luck. Thou wilt need it.",
                context.army_rank
            ))
        }
        9 if context.has_hardkill_item => {
            if context.questlog_21_count == 0 {
                military_points = 4;
                military_exp = EXP_AREA15_HARDKILL;
            }
            state += 1;
            clara_hardkill_report_text(&mut state, context)
        }
        9 => None,
        10 => clara_hardkill_report_text(&mut state, context),
        11 if context.has_hardkill_item && context.hardkill_ritual_progress >= 36 => {
            state += 1;
            state += 1;
            Some("Now that thou knowest how to kill that beast, please go and do it.".to_string())
        }
        11 => None,
        12 => {
            state += 1;
            Some("Now that thou knowest how to kill that beast, please go and do it.".to_string())
        }
        13 => None,
        14 => {
            complete_questlog = Some(21);
            if context.questlog_21_count == 1 {
                military_points = 8;
                military_exp = 1;
            }
            state += 1;
            Some(format!("Well done indeed, {}!", context.player_name))
        }
        15 => {
            state += 1;
            Some(format!(
                "The swamp will be safer now, but more dangers await thee on thy travels. May Ishtar be with thee, {}.",
                context.player_name
            ))
        }
        _ => None,
    };

    ClaraDialogueOutcome {
        clara_state: state,
        text,
        open_questlog,
        complete_questlog,
        military_points,
        military_exp,
    }
}

fn clara_dialogue_step_text_after_fallthrough(
    state: &mut i32,
    context: ClaraDialogueContext<'_>,
) -> Option<String> {
    *state += 1;
    Some(format!(
        "I assume thou hast been sent from Aston, {}, to report on our status. The road through the swamp is no longer secure and we have been under attack from beasts emerging from the swamp.",
        context.army_rank
    ))
}

fn clara_hardkill_report_text(
    state: &mut i32,
    context: ClaraDialogueContext<'_>,
) -> Option<String> {
    *state += 1;
    if context.has_hardkill_item && context.hardkill_ritual_progress < 36 {
        Some(format!(
            "So that is how one can kill them. Thou wilt need to find all three stone circles and perform the ritual in each one, then, {}.",
            context.player_name
        ))
    } else {
        Some("So that is how one can kill them.".to_string())
    }
}

pub fn clara_replay_state_after_text_analysis(clara_state: i32, didsay: i32) -> i32 {
    if didsay != 2 {
        return clara_state;
    }
    match clara_state {
        ..=5 => 0,
        6..=9 => 6,
        10..=11 => 10,
        12..=13 => 12,
        15..=16 => 15,
        _ => clara_state,
    }
}

pub fn clara_state_after_swamp_monster_death(
    clara_state: i32,
    killer_is_player: bool,
    monster_is_hardkill: bool,
) -> i32 {
    if killer_is_player && monster_is_hardkill && (12..=13).contains(&clara_state) {
        14
    } else {
        clara_state
    }
}
