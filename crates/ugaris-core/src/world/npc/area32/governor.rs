//! The Area 32 governor's job-board NPC (`CDR_MISSIONGIVE`), "Mister
//! Jones": offers randomly-rolled Alpha/Beta/Gamma kill jobs and runs a
//! "brownie points" reward shop.
//!
//! Ports `src/area/32/missions.c::mission_giver_driver` (`:1297-1788`)
//! plus its `qa[]` small-talk table (`:84-116`), `mis_rew[]` reward table
//! (`:126-173`), `mdtab[]` mission-template table (`:449-633`),
//! `offer_mission`/`offer_mission_sub` (`:635-676`), `mission_reward_list`/
//! `mission_show_reward`/`mission_give_reward` (`:1132-1287`), and
//! `remove_mission_items` (`:1161-1175`).
//!
//! Deviations/gaps (documented, not silent) - this slice also now ports
//! `start_mission`/`build_fighter` (`:678-1130`, `world::npc::area32::
//! mission_start`): "accept job Alpha/Beta/Gamma" (qa codes 7/8/9) plans
//! the 41x41-tile instance-dungeon spawn (busy-slice search, existing-
//! occupant cleanup, procedural fighter stat generation, key-door/chest
//! wiring, `mission_status` HUD lines, `teleport_char_driver`) and defers
//! the actual `ZoneLoader`-needing character creation to `ugaris-server`'s
//! `area32.rs::spawn_mission_fighters` via a new `SpawnMissionFighters`
//! event - see that module's own doc comment for the exact split. The
//! following remain **not yet ported**, tracked in `PORTING_TODO.md`:
//! - `missionchest_driver` (`IDR_MISSIONCHEST`)/`mission_done`/
//!   `mission_fighter_dead`'s kill-counter hook (`CDR_MISSIONFIGHT`, a
//!   thin `CDR_SIMPLEBADDY` wrapper): a mission can now be started and
//!   its dungeon populated, but nothing yet decrements `ppd->kill_*`/
//!   `find_item` on a fighter kill or chest loot, so `mission_done`'s
//!   auto-solve never fires and the reward chest cannot be opened
//!   (`IDR_MISSIONCHEST`'s driver id has no dispatch case yet, a safe
//!   non-crashing no-op).
//! - The rotating procedural "special offer" gear purchase (qa codes
//!   18/19, `dat->next_spec`/`dat->spec_cost`/`ch[cn].item[30]`): needs
//!   `World::create_special_item` wired through a `ZoneLoader`-backed
//!   event, deferred with the rest of this slice. Both phrases are
//!   recognized but produce no response, and "offer"'s trailing "I also
//!   have a special offer..." teaser line is dropped.
//! - The `CTPOT` ("Custom Stat Potion") reward's multi-turn skill-naming
//!   flow (`ppd->statowed`/`statcnt`/`stat[]`, `find_skill_text`,
//!   `mis_potionbase`'s `mod_index`/`mod_value` fields): no
//!   `Item::mod_index`/`mod_value` fields exist anywhere in this port yet
//!   (a deeper, cross-cutting gap, not specific to this NPC). `CTPOT`'s
//!   preview text (`mission_show_reward`) is still ported faithfully
//!   (canned description), but buying it falls through to the generic
//!   item-template path, which reports "Oops, I've run out of stock"
//!   (no `CTPOT` item template exists) instead of granting anything - a
//!   safe, non-crashing, clearly worse-than-C degradation, not a silent
//!   one. `RNORB` ("Random Orb", needs a `create_orb`-equivalent that
//!   also does not exist in `ugaris-core` yet - see
//!   `crates/ugaris-server/src/commands_admin/grants.rs::
//!   grant_created_orb` for the closest existing analog) degrades the
//!   same way. Every other reward (12 rings, `MEXP1-3`, `GOLD1-4`,
//!   `CBPOT`, `FGPOT`, `SCPOT` - 22 of the 24 table entries) is fully
//!   functional.
//! - `mis_rew[]` is stored pre-sorted by `value` ascending (matching the
//!   sorted state C's own `qsort(mis_rew, ...)`/`init_done` one-time
//!   sort settles into on first use) rather than modeling the sort as
//!   runtime state - behaviorally identical, avoids an unnecessary
//!   `World`-level "have we sorted yet" flag for a `const` table.
//! - Raw C tab-stop/column-alignment control bytes (`\006`/`\007`/`\020`/
//!   `\022`/`\024`) inside `log_char` calls are dropped; message *text*
//!   (including keyword `COL_LIGHT_BLUE`/`COL_LIGHT_GREEN` emphasis, via
//!   [`crate::text::COL_STR_LIGHT_BLUE`]/[`crate::text::COL_STR_LIGHT_GREEN`])
//!   is preserved digit-for-digit; only the legacy client's chat-window
//!   column-tab-stop bytes are not reproduced (a cosmetic-only gap, no
//!   established Rust equivalent exists anywhere in this codebase yet).
//! - The `NT_GIVE` handler (giving Mister Jones an item) uses the
//!   established "hand it right back once, destroy on failure"
//!   simplification of `world::npc::area29::grinnich`'s own `NT_GIVE`
//!   handler, rather than C's `dat->amgivingback`-counted `give_driver`
//!   retry-across-ticks loop (`:1748-1767`).
//! - `dlog` audit-log calls throughout have no Rust equivalent anywhere
//!   in this codebase (dev-only diagnostic, consistently unported) and
//!   are not reproduced here either.

use std::collections::HashMap;

use crate::character_driver::{
    analyse_text_qa, tokenize_text_words, CharacterDriverMessage, TextAnalysisOutcome, TextQaEntry,
    CDR_LOSTCON, CDR_MISSIONGIVE,
};
use crate::drvlib::offset2dx;
use crate::player::{MissionPpd, SingleMission};
use crate::text::{COL_STR_LIGHT_BLUE, COL_STR_LIGHT_GREEN, COL_STR_RESET};
use crate::world::items::give_money_message;
use crate::world::npc::area32::military::army_rank_for_points;
use crate::world::npc::area32::mission_start::{
    mission_status_lines, MissionStartError, MISSION_FIGHTER_DATA,
};
use crate::world::*;

/// C `char_dist(cn, co) > 10` (`missions.c:1424`): the `NT_CHAR`
/// greet-range guard.
const GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`missions.c:201`): `analyse_text_driver`'s
/// own guard, shared by the `NT_TEXT` handler.
const QA_DISTANCE: i32 = 12;
/// C `TICKS * 3` (`missions.c:1407`).
const TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 3;
/// C `TICKS * 5` (`missions.c:1412`).
const TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 30` (`missions.c:1781`): idle "return to post" threshold.
const RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `realtime - ppd->lastseenmissiongiver > 30` (`missions.c:1433`):
/// wall-clock seconds before the dialogue state resets to greeting.
const LASTSEEN_RESET_SECONDS: u64 = 30;
/// C `#define MAXDIFF 1000` (`missions.c:52`).
pub const MAX_DIFFICULTY: i32 = 1000;

/// C `struct mission_data` (`missions.c:449-470`), trimmed to the fields
/// [`offer_mission`]/[`offer_mission_sub`]/the `solved`-reward payout
/// need. The remaining C fields (`basename`/`bossname`/`bigbossname`/
/// `sprite`s/`strengthname`/`temp`s/`itemtemp`/`itemname`/`itemdesc`/
/// `area`/`char_flags`) only feed `build_fighter`/`start_mission`, which
/// this slice does not port yet (see the module doc comment) - add them
/// here when that slice lands.
pub struct MissionTemplate {
    pub title: &'static str,
    pub description: &'static str,
    /// C `int type` (`MISS_TYPE_KILL` = 1; every one of the 7 templates
    /// uses it).
    pub mission_type: i32,
    /// C `float diff_mult`.
    pub diff_mult: f32,
}

/// C `struct mission_data *mdtab[]` (`missions.c:632-633`), same order:
/// thief, spy, beast, ruffian, vampire, graverobber, hide-and-seek.
pub const MISSION_TEMPLATES: [MissionTemplate; 7] = [
    MissionTemplate {
        title: "Stolen Documents",
        description: "A bold thief named 'Sacewan' from the famous gang 'The Pickers' has stolen some vitally important documents from a friend of mine. Please retrieve the documents, and kill Sacewan and his whole gang.",
        mission_type: 1,
        diff_mult: 0.8,
    },
    MissionTemplate {
        title: "Silence the Spy",
        description: "A spy has stolen a important secret from my boss. He's fled into the wilderness while his messengers are trying to find a buyer for the information. See to it the deal is never made.",
        mission_type: 1,
        diff_mult: 0.75,
    },
    MissionTemplate {
        title: "Swamp Beast Invasion",
        description: "A group of swamp beasts has invaded one of our outposts. Please free the outpost of their presence.",
        mission_type: 1,
        diff_mult: 0.75,
    },
    MissionTemplate {
        title: "Dispatch Ruffians",
        description: "A group of ruffians has taken camp in my superior's summer house. Please see to it that they leave. Be careful, their boss, one 'Gorinion', is said to be a powerful mage.",
        mission_type: 1,
        diff_mult: 0.75,
    },
    MissionTemplate {
        title: "Vampire Infection",
        description: "There have been rumors about a group of vampires in a town nearby. Please investigate the town's graveyard for any signs of vampires. Make all vampires you find 'see the light'.",
        mission_type: 1,
        diff_mult: 1.2,
    },
    MissionTemplate {
        title: "Graverobber",
        description: "This time I have a delicate mission for you. I have discovered the location of an ancient tomb in old writings, which is said to hold the 'Magical Spoon of Doom'. Please enter the tomb and retrieve the spoon.",
        mission_type: 1,
        diff_mult: 1.25,
    },
    MissionTemplate {
        title: "Hide and Seek",
        description: "A wizard-friend of my boss has hidden a ring in a dungeon which my boss is to retrieve. Some kind of bet I heard. Go there, get the ring, and kill anything that comes into your way.",
        mission_type: 1,
        diff_mult: 1.50,
    },
];

/// C `struct reward` (`missions.c:118-123`).
pub struct MissionReward {
    pub code: &'static str,
    pub desc: &'static str,
    pub value: i32,
    pub itmtmp: &'static str,
}

/// C `struct reward mis_rew[]` (`missions.c:126-173`), pre-sorted by
/// `value` ascending (see the module doc comment's note on `qsort`/
/// `init_done`). `itmtmp` of `"GOLD"`/`"MEXP"`/`"CTPOT"`/`"RNORB"` are
/// C's own special-cased sentinel strings (`mission_give_reward`/
/// `mission_show_reward`'s `strcmp` branches), not real item templates;
/// every other `itmtmp` names a real zone item template
/// (`ugaris_data/zones/32/mission.itm`).
pub const MIS_REWARDS: [MissionReward; 24] = [
    MissionReward {
        code: "GOLD1",
        desc: "Money 1",
        value: 10,
        itmtmp: "GOLD",
    },
    MissionReward {
        code: "CBPOT",
        desc: "Huge Combo Potion",
        value: 25,
        itmtmp: "mis_combopot",
    },
    MissionReward {
        code: "LNROS",
        desc: "Leonid's Ring of Skirmish",
        value: 30,
        itmtmp: "mis_ring1",
    },
    MissionReward {
        code: "CRROS",
        desc: "Carisah's Ring of Skirmish",
        value: 30,
        itmtmp: "mis_ring4",
    },
    MissionReward {
        code: "LNROB",
        desc: "Leonid's Ring of Battle",
        value: 100,
        itmtmp: "mis_ring2",
    },
    MissionReward {
        code: "CRROB",
        desc: "Carisah's Ring of Battle",
        value: 100,
        itmtmp: "mis_ring5",
    },
    MissionReward {
        code: "MEXP1",
        desc: "Military Experience 1",
        value: 100,
        itmtmp: "MEXP",
    },
    MissionReward {
        code: "GOLD2",
        desc: "Money 2",
        value: 100,
        itmtmp: "GOLD",
    },
    MissionReward {
        code: "AZROS",
        desc: "Arcazor's Ring of Skirmish",
        value: 200,
        itmtmp: "mis_ring7",
    },
    MissionReward {
        code: "WCROS",
        desc: "Wicala's Ring of Skirmish",
        value: 200,
        itmtmp: "mis_ring10",
    },
    MissionReward {
        code: "FGPOT",
        desc: "Forgetfulness Potion",
        value: 200,
        itmtmp: "forgetfulness_potion",
    },
    MissionReward {
        code: "LNROW",
        desc: "Leonid's Ring of War",
        value: 250,
        itmtmp: "mis_ring3",
    },
    MissionReward {
        code: "CRROW",
        desc: "Carisah's Ring of War",
        value: 250,
        itmtmp: "mis_ring6",
    },
    MissionReward {
        code: "CTPOT",
        desc: "Custom Stat Potion",
        value: 250,
        itmtmp: "CTPOT",
    },
    MissionReward {
        code: "SCPOT",
        desc: "Potion of Security",
        value: 300,
        itmtmp: "security_potion",
    },
    MissionReward {
        code: "RNORB",
        desc: "Random Orb",
        value: 400,
        itmtmp: "RNORB",
    },
    MissionReward {
        code: "AZROB",
        desc: "Arcazor's Ring of Battle",
        value: 750,
        itmtmp: "mis_ring8",
    },
    MissionReward {
        code: "WCROB",
        desc: "Wicala's Ring of Battle",
        value: 750,
        itmtmp: "mis_ring11",
    },
    MissionReward {
        code: "MEXP2",
        desc: "Military Experience 2",
        value: 1000,
        itmtmp: "MEXP",
    },
    MissionReward {
        code: "GOLD3",
        desc: "Money 3",
        value: 1000,
        itmtmp: "GOLD",
    },
    MissionReward {
        code: "AZROW",
        desc: "Arcazor's Ring of War",
        value: 2000,
        itmtmp: "mis_ring9",
    },
    MissionReward {
        code: "WCROW",
        desc: "Wicala's Ring of War",
        value: 2000,
        itmtmp: "mis_ring12",
    },
    MissionReward {
        code: "MEXP3",
        desc: "Military Experience 3",
        value: 10000,
        itmtmp: "MEXP",
    },
    MissionReward {
        code: "GOLD4",
        desc: "Money 4",
        value: 10000,
        itmtmp: "GOLD",
    },
];

/// C `struct qa qa[]` (`missions.c:84-116`), same order. Codes `7`/`8`/
/// `9` ("accept job ..."), `18`/`19` ("special offer"/"buy the special
/// offer") and `20`/`21`/`22` ("one/two/three skill(s)") are matched but
/// intentionally produce no action - see the module doc comment.
pub const MISSIONGIVE_QA: &[TextQaEntry] = &[
    TextQaEntry {
        words: &["how", "are", "you"],
        answer: Some("I'm fine!"),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["hello"],
        answer: Some("Hello, %s!"),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["hi"],
        answer: Some("Hi, %s!"),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["greetings"],
        answer: Some("Greetings, %s!"),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["hail"],
        answer: Some("And hail to you, %s!"),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["what's", "up"],
        answer: Some("Everything that isn't nailed down."),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["what", "is", "up"],
        answer: Some("Everything that isn't nailed down."),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["repeat"],
        answer: None,
        answer_code: 2,
    },
    TextQaEntry {
        words: &["restart"],
        answer: None,
        answer_code: 2,
    },
    TextQaEntry {
        words: &["please", "repeat"],
        answer: None,
        answer_code: 2,
    },
    TextQaEntry {
        words: &["please", "restart"],
        answer: None,
        answer_code: 2,
    },
    TextQaEntry {
        words: &["job", "alpha"],
        answer: None,
        answer_code: 4,
    },
    TextQaEntry {
        words: &["job", "beta"],
        answer: None,
        answer_code: 5,
    },
    TextQaEntry {
        words: &["job", "gamma"],
        answer: None,
        answer_code: 6,
    },
    TextQaEntry {
        words: &["accept", "job", "alpha"],
        answer: None,
        answer_code: 7,
    },
    TextQaEntry {
        words: &["accept", "job", "beta"],
        answer: None,
        answer_code: 8,
    },
    TextQaEntry {
        words: &["accept", "job", "gamma"],
        answer: None,
        answer_code: 9,
    },
    TextQaEntry {
        words: &["fail"],
        answer: None,
        answer_code: 10,
    },
    TextQaEntry {
        words: &["offer"],
        answer: None,
        answer_code: 11,
    },
    TextQaEntry {
        words: &["increase"],
        answer: None,
        answer_code: 12,
    },
    TextQaEntry {
        words: &["decrease"],
        answer: None,
        answer_code: 13,
    },
    TextQaEntry {
        words: &["gimme100"],
        answer: None,
        answer_code: 14,
    },
    TextQaEntry {
        words: &["gimme1000"],
        answer: None,
        answer_code: 15,
    },
    TextQaEntry {
        words: &["gimme10000"],
        answer: None,
        answer_code: 16,
    },
    TextQaEntry {
        words: &["gimme100000"],
        answer: None,
        answer_code: 17,
    },
    TextQaEntry {
        words: &["special", "offer"],
        answer: None,
        answer_code: 18,
    },
    TextQaEntry {
        words: &["buy", "the", "special", "offer"],
        answer: None,
        answer_code: 19,
    },
    TextQaEntry {
        words: &["new", "job"],
        answer: None,
        answer_code: 2,
    },
    TextQaEntry {
        words: &["reset", "me"],
        answer: None,
        answer_code: 3,
    },
    TextQaEntry {
        words: &["one", "skill"],
        answer: None,
        answer_code: 20,
    },
    TextQaEntry {
        words: &["two", "skills"],
        answer: None,
        answer_code: 21,
    },
    TextQaEntry {
        words: &["three", "skills"],
        answer: None,
        answer_code: 22,
    },
];

/// Per-player facts [`World::process_mission_giver_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MissionGivePlayerFacts {
    /// `PlayerRuntime::governor`.
    pub ppd: MissionPpd,
}

/// A side effect [`World::process_mission_giver_actions`] could not apply
/// directly because it touches `PlayerRuntime` (`UpdatePpd`) or needs a
/// `ZoneLoader` (`GiveItemReward`/`ShowItemReward`/`SpawnMissionFighters`,
/// all `ugaris-server`-only capabilities `World` cannot reach). See the
/// module doc comment.
#[derive(Debug, Clone, PartialEq)]
pub enum MissionGiveOutcomeEvent {
    /// Write a full new `governor` ppd snapshot back.
    UpdatePpd {
        player_id: CharacterId,
        ppd: MissionPpd,
    },
    /// C `mission_give_reward`'s generic branch (`missions.c:1212-1233`):
    /// create `MIS_REWARDS[reward_index].itmtmp`, give it to `player_id`,
    /// and only on success deduct points and announce it (`npc_id`
    /// speaks). Applied server-side since it needs a `ZoneLoader`.
    GiveItemReward {
        player_id: CharacterId,
        npc_id: CharacterId,
        reward_index: usize,
    },
    /// C `mission_show_reward`'s generic branch (`missions.c:1272-1281`):
    /// create, preview (`legacy_item_look_text`), and destroy
    /// `MIS_REWARDS[reward_index].itmtmp`. Applied server-side for the
    /// same reason as `GiveItemReward`.
    ShowItemReward {
        player_id: CharacterId,
        npc_id: CharacterId,
        reward_index: usize,
    },
    /// C `start_mission`'s `build_fighter` calls (`missions.c:1030-1115`):
    /// spawn each planned fighter. Applied server-side since it needs a
    /// `ZoneLoader`/`ServerRuntime::allocate_character_id` - see
    /// `mission_start`'s module doc comment.
    SpawnMissionFighters {
        fighters: Vec<crate::world::npc::area32::mission_start::FighterSpawnSpec>,
    },
}

impl World {
    /// C `ch_driver`'s `CDR_MISSIONGIVE` dispatch (`missions.c:1885-
    /// 1887`), driving every live `mission_giver_driver` NPC.
    pub fn process_mission_giver_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, MissionGivePlayerFacts>,
        area_id: u16,
        now: u64,
    ) -> Vec<MissionGiveOutcomeEvent> {
        let giver_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_MISSIONGIVE
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for giver_id in giver_ids {
            self.process_mission_giver_messages(giver_id, player_facts, area_id, now, &mut events);
        }
        events
    }

    #[allow(clippy::too_many_arguments)]
    fn process_mission_giver_messages(
        &mut self,
        giver_id: CharacterId,
        player_facts: &HashMap<CharacterId, MissionGivePlayerFacts>,
        area_id: u16,
        now: u64,
        events: &mut Vec<MissionGiveOutcomeEvent>,
    ) {
        let Some(giver_name) = self
            .characters
            .get(&giver_id)
            .map(|giver| giver.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::MissionGiver(mut data)) = self
            .characters
            .get(&giver_id)
            .and_then(|giver| giver.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&giver_id)
            .map(|giver| std::mem::take(&mut giver.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.mission_giver_handle_char_message(
                    giver_id,
                    &mut data,
                    message,
                    player_facts,
                    now,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.mission_giver_handle_text_message(
                    giver_id,
                    &giver_name,
                    &mut data,
                    message,
                    player_facts,
                    area_id,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => self.mission_giver_handle_give_message(giver_id, message),
                _ => {}
            }
        }

        if let Some(giver) = self.characters.get_mut(&giver_id) {
            giver.driver_state = Some(CharacterDriverState::MissionGiver(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`missions.c:1777-1779`).
        if let (Some(giver), Some((tx, ty))) =
            (self.characters.get(&giver_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(giver.x), i32::from(giver.y), tx, ty) {
                if let Some(giver_mut) = self.characters.get_mut(&giver_id) {
                    let _ = turn(giver_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { secure_move_driver(
        // cn, ch[cn].tmpx, ch[cn].tmpy, DX_RIGHT, ret, lastact); }`
        // (`missions.c:1781-1785`).
        let last_talk = if let Some(giver) = self.characters.get(&giver_id) {
            match giver.driver_state.as_ref() {
                Some(CharacterDriverState::MissionGiver(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(giver) = self.characters.get(&giver_id) else {
                return;
            };
            let (post_x, post_y) = (giver.rest_x, giver.rest_y);
            self.secure_move_driver(
                giver_id,
                post_x,
                post_y,
                Direction::Right as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `mission_giver_driver`'s `NT_CHAR` branch (`missions.c:1390-
    /// 1499`).
    #[allow(clippy::too_many_arguments)]
    fn mission_giver_handle_char_message(
        &mut self,
        giver_id: CharacterId,
        data: &mut MissionGiverDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, MissionGivePlayerFacts>,
        now: u64,
        events: &mut Vec<MissionGiveOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(giver) = self.characters.get(&giver_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        if tick < data.last_talk + TALK_MIN_TICKS {
            return;
        }
        if tick < data.last_talk + TALK_VICTIM_TICKS && data.current_victim != Some(player_id) {
            return;
        }
        if giver_id == player_id || !char_see_char(&giver, &player, &self.map, self.date.daylight) {
            return;
        }
        if char_dist(&giver, &player) > GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };
        let mut ppd = facts.ppd;

        // C `if (realtime - ppd->lastseenmissiongiver > 30) ppd->
        // missiongive_state = 0;` (`missions.c:1433-1435`).
        if now.saturating_sub(ppd.lastseenmissiongiver) > LASTSEEN_RESET_SECONDS {
            ppd.missiongive_state = 0;
        }
        ppd.lastseenmissiongiver = now;

        let mut didsay = false;
        match ppd.missiongive_state {
            // C `case 0:` (`missions.c:1439-1484`).
            0 => {
                if ppd.solved != 0 {
                    let md_idx = ppd.md_idx.clamp(0, MISSION_TEMPLATES.len() as i32 - 1) as usize;
                    let md = &MISSION_TEMPLATES[md_idx];
                    let slot_idx = (ppd.solved - 1).clamp(0, 2) as usize;
                    let difficulty = ppd.sm[slot_idx].difficulty;
                    let pts = ((13 + difficulty / 6) as f32 * md.diff_mult) as i32;
                    ppd.points += pts;
                    ppd.solved = 0;
                    ppd.dif_kill = (ppd.dif_kill + 2).min(MAX_DIFFICULTY);
                    ppd.sm = [SingleMission::default(); 3];
                    self.npc_quiet_say(
                        giver_id,
                        &format!(
                            "Congratulations on doing a good job, {}. You earned {pts} brownie points in my book, for a total of {} points. Feel free to ask me for an {COL_STR_LIGHT_BLUE}offer{COL_STR_RESET} anytime. You can also ask me to {COL_STR_LIGHT_BLUE}increase{COL_STR_RESET} or {COL_STR_LIGHT_BLUE}decrease{COL_STR_RESET} the difficulty of your jobs (this must be done before asking for a new job, otherwise you'll be changing the difficulty of the following job offer, not the current one; multiple commands will stack). Or do you want a {COL_STR_LIGHT_BLUE}new job{COL_STR_RESET}?",
                            player.name, ppd.points
                        ),
                    );
                    self.mission_giver_remove_labitems(player_id);
                    ppd.missiongive_state = 2;
                    didsay = true;
                } else if ppd.active != 0 {
                    self.npc_quiet_say(
                        giver_id,
                        &format!("You still have a job. Do you want to {COL_STR_LIGHT_BLUE}fail{COL_STR_RESET} it?"),
                    );
                    ppd.missiongive_state = 2;
                    didsay = true;
                } else {
                    self.npc_quiet_say(
                        giver_id,
                        &format!(
                            "Hello {}. Looking for a job, I bet. Well, let me see...",
                            player.name
                        ),
                    );
                    ppd.missiongive_state = 1;
                    didsay = true;
                }
            }
            // C `case 1:` (`missions.c:1485-1489`).
            1 => {
                self.mission_giver_offer_mission(giver_id, player_id, &mut ppd);
                ppd.missiongive_state = 2;
                didsay = true;
            }
            // C `case 2: break;` (`missions.c:1490-1491`): waiting for the
            // player.
            2 => {}
            _ => {}
        }

        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }

        events.push(MissionGiveOutcomeEvent::UpdatePpd { player_id, ppd });
    }

    /// C `offer_mission` (`missions.c:651-676`).
    fn mission_giver_offer_mission(
        &mut self,
        giver_id: CharacterId,
        player_id: CharacterId,
        ppd: &mut MissionPpd,
    ) {
        const MISS_NAMES: [&str; 3] = ["Alpha", "Beta", "Gamma"];
        self.npc_quiet_say(
            giver_id,
            "Please choose one of the following jobs (this will display job details, you get a chance to accept or refuse the job later):",
        );
        for n in 0..3usize {
            if ppd.sm[n].mission_type == 0 {
                let cmp1 = if n == 0 { 1 } else { 0 };
                let cmp2 = if n == 1 { 2 } else { 1 };
                loop {
                    let mdidx = self.roll_legacy_random(MISSION_TEMPLATES.len() as u32) as i32;
                    if mdidx != ppd.sm[cmp1].mdidx && mdidx != ppd.sm[cmp2].mdidx {
                        ppd.sm[n].mdidx = mdidx;
                        ppd.sm[n].mission_type = MISSION_TEMPLATES[mdidx as usize].mission_type;
                        break;
                    }
                }
                if ppd.sm[n].mission_type == 1 {
                    ppd.sm[n].difficulty = ppd.dif_kill + self.roll_legacy_random(10) as i32;
                }
            }
            let md = &MISSION_TEMPLATES[ppd.sm[n].mdidx as usize];
            self.queue_system_text(
                player_id,
                format!(
                    "{COL_STR_LIGHT_BLUE}Job {}{COL_STR_RESET} {} (Kill), Difficulty {}",
                    MISS_NAMES[n], md.title, ppd.sm[n].difficulty
                ),
            );
        }
    }

    /// C `offer_mission_sub` (`missions.c:635-649`).
    fn mission_giver_offer_detail(&mut self, player_id: CharacterId, ppd: &MissionPpd, idx: usize) {
        const MISS_NAMES: [&str; 3] = ["Alpha", "Beta", "Gamma"];
        let slot = ppd.sm[idx];
        let md =
            &MISSION_TEMPLATES[slot.mdidx.clamp(0, MISSION_TEMPLATES.len() as i32 - 1) as usize];
        self.queue_system_text(
            player_id,
            format!(
                "{COL_STR_LIGHT_GREEN}Job {} {}{COL_STR_RESET} Kill Difficulty {}",
                MISS_NAMES[idx], md.title, slot.difficulty
            ),
        );
        self.queue_system_text(player_id, md.description.to_string());
        let other1 = if idx == 0 { 1 } else { 0 };
        let other2 = if idx == 2 { 1 } else { 2 };
        self.queue_system_text(
            player_id,
            format!(
                "Do you {COL_STR_LIGHT_BLUE}accept Job {}{COL_STR_RESET}? Accepting the job will teleport you to the job area immediately. Or do you want to look at {COL_STR_LIGHT_BLUE}Job {}{COL_STR_RESET} or {COL_STR_LIGHT_BLUE}Job {}{COL_STR_RESET} instead?",
                MISS_NAMES[idx], MISS_NAMES[other1], MISS_NAMES[other2]
            ),
        );
    }

    /// C `mission_giver_driver`'s `NT_TEXT` branch (`missions.c:1502-
    /// 1745`).
    #[allow(clippy::too_many_arguments)]
    fn mission_giver_handle_text_message(
        &mut self,
        giver_id: CharacterId,
        giver_name: &str,
        data: &mut MissionGiverDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, MissionGivePlayerFacts>,
        area_id: u16,
        events: &mut Vec<MissionGiveOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);
        let Some(text) = message.text.as_deref() else {
            return;
        };
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };
        if !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        if giver_id == speaker_id {
            return;
        }
        let Some(giver) = self.characters.get(&giver_id).cloned() else {
            return;
        };
        if char_dist(&giver, &speaker) > QA_DISTANCE
            || !char_see_char(&giver, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let Some(facts) = player_facts.get(&speaker_id) else {
            return;
        };
        let mut ppd = facts.ppd;
        let mut didsay = false;
        let mut give_item_reward: Option<usize> = None;
        let mut show_item_reward: Option<usize> = None;

        match analyse_text_qa(text, giver_name, &speaker.name, MISSIONGIVE_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(giver_id, &reply);
                didsay = true;
            }
            // C `case 2:` ("repeat"/"restart"/"new job").
            TextAnalysisOutcome::Matched(2) => {
                ppd.missiongive_state = 0;
                data.last_talk = 0;
                didsay = true;
            }
            // C `case 3:` ("reset me", `CF_GOD`-only).
            TextAnalysisOutcome::Matched(3) => {
                if speaker.flags.contains(CharacterFlags::GOD) {
                    self.npc_say(giver_id, "reset done");
                    ppd = MissionPpd::default();
                    data.last_talk = 0;
                }
                didsay = true;
            }
            TextAnalysisOutcome::Matched(code @ 4..=6) => {
                if ppd.active == 0 && ppd.solved == 0 {
                    self.mission_giver_offer_detail(speaker_id, &ppd, (code - 4) as usize);
                } else {
                    self.npc_quiet_say(
                        giver_id,
                        &format!("You still have a job. Do you want to {COL_STR_LIGHT_BLUE}fail{COL_STR_RESET} it?"),
                    );
                }
                didsay = true;
            }
            // C `case 7/8/9:` ("accept job ...").
            TextAnalysisOutcome::Matched(code @ 7..=9) => {
                let idx = (code - 7) as usize;
                if ppd.sm[idx].mission_type != 0 && ppd.active == 0 && ppd.solved == 0 {
                    match self.plan_start_mission(idx, &mut ppd) {
                        Ok(plan) => {
                            let md_idx =
                                ppd.md_idx.clamp(0, MISSION_FIGHTER_DATA.len() as i32 - 1) as usize;
                            let title = MISSION_TEMPLATES[md_idx].title;
                            for line in
                                mission_status_lines(&ppd, title, &MISSION_FIGHTER_DATA[md_idx])
                            {
                                self.queue_system_text(speaker_id, line);
                            }
                            self.teleport_char_driver(speaker_id, plan.entry.0, plan.entry.1);
                            events.push(MissionGiveOutcomeEvent::SpawnMissionFighters {
                                fighters: plan.fighters,
                            });
                        }
                        Err(MissionStartError::AllSlicesBusy) => {
                            self.npc_quiet_say(
                                giver_id,
                                &format!(
                                    "I'm sorry, {}, but it appears that this job is unavailable right now. Please choose a different one.",
                                    speaker.name
                                ),
                            );
                        }
                    }
                } else {
                    self.npc_quiet_say(giver_id, "I haven't offered you that job yet.");
                }
                didsay = true;
            }
            // C `case 10:` ("fail").
            TextAnalysisOutcome::Matched(10) => {
                if ppd.active != 0 {
                    self.npc_quiet_say(giver_id, "Don't take on things you cannot handle, kid.");
                    let slot_idx = (ppd.active - 1).clamp(0, 2) as usize;
                    let pts = ppd.sm[slot_idx].difficulty / 10;
                    ppd.points = (ppd.points - pts).max(0);
                    ppd.active = 0;
                    ppd.dif_kill = (ppd.dif_kill - 20).max(0);
                    if pts != 0 {
                        self.npc_quiet_say(
                            giver_id,
                            &format!(
                                "You lost {pts} brownie points for a new total of {} points.",
                                ppd.points
                            ),
                        );
                    }
                    self.mission_giver_remove_labitems(speaker_id);
                    ppd.sm = [SingleMission::default(); 3];
                }
                didsay = true;
            }
            // C `case 11:` ("offer").
            TextAnalysisOutcome::Matched(11) => {
                self.mission_giver_show_reward_list(speaker_id, &ppd);
                didsay = true;
            }
            // C `case 12:` ("increase").
            TextAnalysisOutcome::Matched(12) => {
                ppd.dif_kill = (ppd.dif_kill + 10).min(MAX_DIFFICULTY);
                self.npc_quiet_say(
                    giver_id,
                    "Alright, bigmouth. Let's see how you handle this.",
                );
                didsay = true;
            }
            // C `case 13:` ("decrease").
            TextAnalysisOutcome::Matched(13) => {
                ppd.dif_kill = (ppd.dif_kill - 10).max(0);
                self.npc_quiet_say(
                    giver_id,
                    "Alright, little girl. You'll get the easy ones now.",
                );
                if speaker.flags.contains(CharacterFlags::FEMALE) {
                    self.npc_quiet_say(
                        giver_id,
                        "Oops. Sorry. Old habit from my military days, ma'am. Alright, lady, you'll get the easy ones now.",
                    );
                }
                didsay = true;
            }
            // C `case 14-17:` ("gimme..."), `CF_GOD`-only.
            TextAnalysisOutcome::Matched(code @ 14..=17) => {
                if speaker.flags.contains(CharacterFlags::GOD) {
                    self.npc_say(giver_id, "I hate freeloaders!");
                    ppd.points += match code {
                        14 => 100,
                        15 => 1000,
                        16 => 10000,
                        _ => 100000,
                    };
                }
                didsay = true;
            }
            // C `case 18/19:` (special offer show/buy), deferred - see
            // the module doc comment.
            TextAnalysisOutcome::Matched(18..=19) => {
                didsay = true;
            }
            // C `case 20-22:` (custom stat potion skill-count select),
            // deferred - see the module doc comment. `statowed` never
            // becomes non-zero in this port (only the also-deferred
            // `CTPOT` give-flow would set it), so C's own `if (ppd->
            // statowed < 1)` guard always fires here in practice.
            TextAnalysisOutcome::Matched(20..=22) => {
                if ppd.statowed < 1 {
                    self.npc_quiet_say(giver_id, "You did not buy a stat potion.");
                }
                didsay = true;
            }
            TextAnalysisOutcome::Matched(code) if code >= 2000 => {
                give_item_reward = Some((code - 2000) as usize);
                didsay = true;
            }
            TextAnalysisOutcome::Matched(code) if code >= 1000 => {
                show_item_reward = Some((code - 1000) as usize);
                didsay = true;
            }
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {
                // C's own fallback loop over `mis_rew[]` for `"show
                // <code>"`/`"ibuy <code>"` (`missions.c:277-285`), run
                // only once the fixed `qa[]` table above missed.
                if let Some(words) = tokenize_text_words(text, giver_name) {
                    if words.len() >= 2 {
                        let idx = MIS_REWARDS
                            .iter()
                            .position(|reward| reward.code.eq_ignore_ascii_case(&words[1]));
                        if let Some(idx) = idx {
                            if words[0] == "show" {
                                show_item_reward = Some(idx);
                                didsay = true;
                            } else if words[0] == "ibuy" {
                                give_item_reward = Some(idx);
                                didsay = true;
                            }
                        }
                    }
                }
            }
        }

        let show_reward_index =
            show_item_reward.and_then(|idx| self.mission_giver_show_reward(giver_id, &ppd, idx));
        let give_reward_index = give_item_reward.and_then(|idx| {
            self.mission_giver_give_reward(giver_id, speaker_id, &mut ppd, idx, area_id)
        });

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`missions.c:1740-1743`) - note this does *not* touch `dat->
        // last_talk` (except `case 2`'s own explicit reset above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }

        // `UpdatePpd` is pushed *before* `ShowItemReward`/`GiveItemReward`
        // below - `apply_mission_giver_events` (`ugaris-server`) applies
        // events in order and `GiveItemReward`'s own point deduction
        // mutates `PlayerRuntime` directly (it isn't known yet whether the
        // generic item-template create/give will even succeed when this
        // `ppd` snapshot is taken), so this snapshot write must land first
        // or it would clobber that later deduction.
        events.push(MissionGiveOutcomeEvent::UpdatePpd {
            player_id: speaker_id,
            ppd,
        });
        if let Some(idx) = show_reward_index {
            events.push(MissionGiveOutcomeEvent::ShowItemReward {
                player_id: speaker_id,
                npc_id: giver_id,
                reward_index: idx,
            });
        }
        if let Some(idx) = give_reward_index {
            events.push(MissionGiveOutcomeEvent::GiveItemReward {
                player_id: speaker_id,
                npc_id: giver_id,
                reward_index: idx,
            });
        }
    }

    /// C `mission_reward_list` (`missions.c:1136-1159`).
    fn mission_giver_show_reward_list(&mut self, player_id: CharacterId, ppd: &MissionPpd) {
        let count = MIS_REWARDS.len();
        let mut n = 0usize;
        while n < count - 1 {
            if MIS_REWARDS[n + 1].value >= ppd.points {
                break;
            }
            n += 1;
        }
        let last = (n + 3).min(count);
        let first = last.saturating_sub(5);

        self.queue_system_text(
            player_id,
            format!("{COL_STR_LIGHT_GREEN}Code Cost Description{COL_STR_RESET}"),
        );
        for reward in &MIS_REWARDS[first..last] {
            self.queue_system_text(
                player_id,
                format!(
                    "{COL_STR_LIGHT_BLUE}show {}{COL_STR_RESET} {} {}",
                    reward.code, reward.value, reward.desc
                ),
            );
        }
        self.queue_system_text(player_id, format!("You have: {} points.", ppd.points));
        self.queue_system_text(
            player_id,
            "You'll get a more detailed description of the offer when you choose it.",
        );
    }

    /// C `mission_show_reward` (`missions.c:1239-1287`). Returns
    /// `Some(reward_index)` if the generic (needs-`ZoneLoader`) item-
    /// preview branch should run server-side; `None` if already fully
    /// handled here (canned text, or an out-of-range index).
    fn mission_giver_show_reward(
        &mut self,
        giver_id: CharacterId,
        ppd: &MissionPpd,
        idx: usize,
    ) -> Option<usize> {
        let reward = MIS_REWARDS.get(idx)?;
        match reward.itmtmp {
            "CTPOT" => self.npc_quiet_say(
                giver_id,
                "A custom potion which will enhance one of your stats by 50 or two of your stats by 30 or three of your stats by 20.",
            ),
            "RNORB" => self.npc_quiet_say(
                giver_id,
                "A randomly chosen orb, used to enhance one modifier on an item by one.",
            ),
            "MEXP" => {
                let text = match reward.value {
                    100 => "The equivalent of a two Privates in Military Experience.",
                    1000 => "The equivalent of a Lance Corporal in Military Experience.",
                    10000 => "The equivalent of a Staff Sergeant in Military Experience.",
                    _ => return None,
                };
                self.npc_quiet_say(giver_id, text)
            }
            "GOLD" => {
                let text = match reward.value {
                    10 => "50 gold coins, fresh from the press.",
                    100 => "500 gold coins, fresh from the press.",
                    1000 => "5000 gold coins, fresh from the press.",
                    10000 => "50000 gold coins, fresh from the press.",
                    _ => return None,
                };
                self.npc_quiet_say(giver_id, text)
            }
            _ => return Some(idx),
        };
        self.npc_quiet_say(
            giver_id,
            &format!(
                "This could be yours for {} points (you have {} points). Say {COL_STR_LIGHT_BLUE}ibuy {}{COL_STR_RESET} to buy it.",
                reward.value, ppd.points, reward.code
            ),
        );
        None
    }

    /// C `mission_give_reward` (`missions.c:1177-1237`). Returns
    /// `Some(reward_index)` if the generic (needs-`ZoneLoader`) item-give
    /// branch should run server-side; `None` if already fully handled
    /// here (insufficient points, army-rank gate, `GOLD`/`MEXP`, or an
    /// out-of-range index).
    fn mission_giver_give_reward(
        &mut self,
        giver_id: CharacterId,
        player_id: CharacterId,
        ppd: &mut MissionPpd,
        idx: usize,
        area_id: u16,
    ) -> Option<usize> {
        let reward = MIS_REWARDS.get(idx)?;
        if reward.value > ppd.points {
            self.npc_quiet_say(
                giver_id,
                &format!(
                    "{} costs {} points, but you only have {} points.",
                    reward.code, reward.value, ppd.points
                ),
            );
            return None;
        }
        let player_name = self.characters.get(&player_id)?.name.clone();
        match reward.itmtmp {
            "MEXP" => {
                let military_points = self.characters.get(&player_id)?.military_points;
                if army_rank_for_points(military_points) == 0 {
                    self.npc_quiet_say(
                        giver_id,
                        "I'm sorry, I can't do that. You are not part of the army.",
                    );
                    return None;
                }
                self.give_military_pts_from_npc(
                    player_id,
                    giver_id,
                    reward.value / 40,
                    1,
                    u32::from(area_id),
                );
                ppd.points -= reward.value;
            }
            "GOLD" => {
                let amount = (reward.value as u32).saturating_mul(500);
                if let Some(player) = self.characters.get_mut(&player_id) {
                    player.gold = player.gold.saturating_add(amount);
                    player.flags.insert(CharacterFlags::ITEMS);
                }
                self.queue_system_text_bytes(player_id, give_money_message(amount));
                ppd.points -= reward.value;
            }
            // `CTPOT`/`RNORB`: no real item template - falls through to
            // the generic branch below, which will gracefully report
            // "out of stock" (see the module doc comment).
            _ => return Some(idx),
        }
        self.npc_quiet_say(
            giver_id,
            &format!(
                "Here you go, {player_name}, one {} ({}) for {} points. You now have {} points left.",
                reward.code, reward.desc, reward.value, ppd.points
            ),
        );
        None
    }

    /// C `remove_mission_items` (`missions.c:1161-1175`): strips every
    /// `IF_LABITEM`-flagged item from the cursor and main inventory
    /// (slots `30..`).
    fn mission_giver_remove_labitems(&mut self, character_id: CharacterId) {
        let Some(character) = self.characters.get(&character_id) else {
            return;
        };
        let mut to_destroy = Vec::new();
        if let Some(item_id) = character.cursor_item {
            if self
                .items
                .get(&item_id)
                .is_some_and(|item| item.flags.contains(ItemFlags::LABITEM))
            {
                to_destroy.push(item_id);
            }
        }
        for item_id in character
            .inventory
            .iter()
            .skip(INVENTORY_START_INVENTORY)
            .flatten()
            .copied()
        {
            if self
                .items
                .get(&item_id)
                .is_some_and(|item| item.flags.contains(ItemFlags::LABITEM))
            {
                to_destroy.push(item_id);
            }
        }
        for item_id in to_destroy {
            self.destroy_item(item_id);
        }
    }

    /// C `mission_giver_driver`'s `NT_GIVE` branch (`missions.c:1748-
    /// 1767`): simplified to the same "hand it right back once, destroy
    /// on failure" precedent as `world::npc::area29::grinnich`'s own
    /// `NT_GIVE` handler - see the module doc comment.
    fn mission_giver_handle_give_message(
        &mut self,
        giver_id: CharacterId,
        message: &CharacterDriverMessage,
    ) {
        let giver_of_item_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&giver_id)
            .and_then(|giver| giver.cursor_item.take())
        else {
            return;
        };
        self.npc_quiet_say(
            giver_id,
            "Thou hast better use for this than I do. Well, if there is use for it at all.",
        );
        if !self.give_char_item(giver_of_item_id, item_id) {
            self.destroy_item(item_id);
        }
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

/// C `struct mission_giver_data`, trimmed to the fields this slice needs
/// (`missions.c:1289-1295`'s `last_talk`/`current_victim`; `amgivingback`
/// is not needed by the simplified `NT_GIVE` handler, and `next_spec`/
/// `spec_cost` back the still-deferred "special offer" flow - see the
/// module doc comment).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MissionGiverDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}
