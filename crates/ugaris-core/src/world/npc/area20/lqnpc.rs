//! `CDR_LQNPC` (`src/area/20/lq.c::lqnpc`, `:2744-2898`): the per-tick
//! dialogue/movement driver every admin-authored Live Quest NPC runs.
//!
//! Ports the message loop (`NT_CHAR` greeting, `NT_GOTHIT` hurt-mark plus
//! `standard_message_driver`'s aggressive-mode self-defense, `NT_TEXT`
//! trigger/reply dialogue plus the "followme"/"stopfollow" admin-mirroring
//! commands, `NT_GIVE` quest-item turn-in) and the movement/fight/regen
//! tail. `lqnpc_died`'s respawn-scheduling/mark-setting death hook lives
//! in `ugaris-server`'s `world_events::death_hooks` (needs `PlayerRuntime`,
//! which core `World` has no access to - same split as every other
//! `apply_*_death_from_hurt_event` hook).
//!
//! Two deliberate, documented gaps, both parts of C's `usurp`
//! god/LQMaster-possession mechanic (`dat->usurp`/`pdat->usurp`): the
//! `domirror` movement-mirroring branch (`lq.c:2855-2868`) and the
//! `#usurp`/`#follow`/`#stop`/`#exit` admin commands that are the *only*
//! C code path that ever writes `dat->usurp` are both part of the
//! not-yet-ported `CDR_LQPARSER` admin command table
//! (`special_driver`/`cmd_usurp`, `lq.c:2505-2742`) - `dat->usurp` can
//! never be non-`None` without it, so the branch is unreachable dead code
//! in this port exactly as it would be in a stock C server with no admin
//! ever issuing `#usurp`. The `NT_TEXT` "followme"/"stopfollow" mechanic
//! (`dat->follow`, driven by any `CF_LQMASTER` player's own speech, no
//! admin command needed) *is* ported - see [`LqNpcDriverData::follow`].

use crate::character_driver::{mem_add_driver, mem_check_driver, CDR_LQNPC};
use crate::world::*;

/// C `mem_add_driver(cn, co, 7)`/`mem_check_driver(cn, co, 7)`
/// (`lqnpc`'s `NT_CHAR` handler, `lq.c:2769-2771`).
const LQNPC_GREET_MEMORY_SLOT: usize = 7;

/// C `struct lq_npc_data` (`lq.c:167-186`, `DRD_LQ_NPC_DATA`): the live
/// per-instance state a spawned `CDR_LQNPC` character carries, copied from
/// its [`crate::world::LqNpcState`] template at spawn time
/// (`ugaris-server`'s `spawn_lq_npc_character`, mirroring C's own
/// `spawn_npc`, `lq.c:1801-1834`). `thrallname`/the `isthrall` spawn path
/// (`spawn_npc`'s second, admin-`#nspawn`-summoned-thrall branch) is not
/// modeled - only the `special_driver` admin command table can trigger it
/// and that table is not ported (see the module doc comment).
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LqNpcDriverData {
    /// C `dat->n` - the index into `lq_npc[]`/[`crate::world::World::lq_npcs`]
    /// this live character was spawned from (`lqnpc_died`'s respawn-
    /// scheduling identity check).
    pub slot: usize,
    pub mode: u8,
    pub greeting: String,
    pub trigger: [String; 5],
    pub reply: [String; 5],
    pub want_key_id: u32,
    pub reward_item: LqItemSpec,
    pub reward_mark_id: u32,
    pub kill_mark_id: u32,
    pub hurt_mark_id: u32,
    pub dir: u8,
    /// C `dat->follow` - a `CF_LQMASTER` player this NPC is currently
    /// following (set/cleared by its own "followme"/"stopfollow" speech,
    /// `lq.c:2787-2792`). `None` is C's `0` ("nobody").
    pub follow: Option<CharacterId>,
}

/// Side effects of [`World::process_lqnpc_actions`] that need
/// `ZoneLoader`/`PlayerRuntime`, applied by `ugaris-server::area20::
/// apply_lqnpc_events`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LqNpcOutcomeEvent {
    /// C `pdat->mark[dat->hurt_markID] = 1;` (`lqnpc`'s `NT_GOTHIT`
    /// handler, `lq.c:2775-2781`).
    SetPlayerMark {
        player_id: CharacterId,
        mark_id: u32,
    },
    /// C `create_lq_item(&dat->reward_item)` + `give_char_item(co, in2)`
    /// (`lqnpc`'s `NT_GIVE` handler, `lq.c:2832-2837`).
    GiveRewardItem {
        receiver_id: CharacterId,
        item: LqItemSpec,
    },
}

/// C `is_valid_enemy(cn, co, -1)` (`drvlib.c:897-927`), used by
/// `standard_message_driver`'s `NT_CHAR` case (`dat->mode == 'a'`).
fn lqnpc_is_valid_enemy(
    character: &Character,
    target: &Character,
    map: &MapGrid,
    daylight: i32,
) -> bool {
    character.id != target.id
        && character.group != target.group
        && can_attack(character, target, map)
        && char_see_char(character, target, map, daylight)
}

impl World {
    /// C `lqnpc`'s per-tick body (`lq.c:2744-2898`).
    pub fn process_lqnpc_actions(&mut self, area_id: u16) -> Vec<LqNpcOutcomeEvent> {
        let lqnpc_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_LQNPC
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for lqnpc_id in lqnpc_ids {
            self.process_lqnpc_tick(lqnpc_id, area_id, &mut events);
        }
        events
    }

    fn process_lqnpc_tick(
        &mut self,
        lqnpc_id: CharacterId,
        area_id: u16,
        events: &mut Vec<LqNpcOutcomeEvent>,
    ) {
        self.process_lqnpc_messages(lqnpc_id, events);

        // C `if (!domirror && (co = dat->follow) && (ch[co].flags &
        // (CF_GOD | CF_LQMASTER))) { if (move_driver(cn, ch[co].x,
        // ch[co].y, 2)) return; ch[cn].tmpx = ch[cn].x; ch[cn].tmpy =
        // ch[cn].y; }` (`lq.c:2870-2876`) - `domirror` (the unported
        // `usurp` branch) is always false here, see the module doc
        // comment.
        let follow = match self
            .characters
            .get(&lqnpc_id)
            .and_then(|character| character.driver_state.as_ref())
        {
            Some(CharacterDriverState::LqNpc(data)) => data.follow,
            _ => return,
        };
        if let Some(target_id) = follow {
            let target_ok = self.characters.get(&target_id).is_some_and(|target| {
                target
                    .flags
                    .intersects(CharacterFlags::GOD | CharacterFlags::LQMASTER)
            });
            if target_ok {
                let Some((target_x, target_y)) =
                    self.characters.get(&target_id).map(|t| (t.x, t.y))
                else {
                    return;
                };
                if self.setup_walk_toward(
                    lqnpc_id,
                    usize::from(target_x),
                    usize::from(target_y),
                    2,
                    area_id,
                    false,
                ) {
                    return;
                }
            }
        }

        // C `if (fight_driver_attack_visible(cn, 0)) return; if
        // (!domirror && fight_driver_follow_invisible(cn)) return;`
        // (`lq.c:2878-2883`).
        let Some(lqnpc) = self.characters.get(&lqnpc_id).cloned() else {
            return;
        };
        let mut seed = self.legacy_random_seed;
        let attacked = self.fight_driver_attack_visible_and_follow(
            lqnpc_id,
            &lqnpc,
            area_id,
            FightDriverSuppressions::default(),
            true,
            &mut |below| legacy_random_below_from_seed(&mut seed, below),
        );
        self.legacy_random_seed = seed;
        if attacked {
            return;
        }

        // C `if (!domirror && secure_move_driver(cn, ch[cn].tmpx,
        // ch[cn].tmpy, dat->dir, ret, lastact)) return;` (`lq.c:2885-2887`)
        // - `ch[cn].tmpx`/`tmpy` is the spawn-time home position, modeled
        // as `Character::rest_x`/`rest_y` (same precedent as `world::npc::
        // area19::madhermit`/`nomad`).
        let Some(lqnpc) = self.characters.get(&lqnpc_id) else {
            return;
        };
        let (rest_x, rest_y) = (lqnpc.rest_x, lqnpc.rest_y);
        let dir = match lqnpc.driver_state.as_ref() {
            Some(CharacterDriverState::LqNpc(data)) => data.dir,
            _ => return,
        };
        if self.secure_move_driver(lqnpc_id, rest_x, rest_y, dir, 0, 0, area_id) {
            return;
        }

        // C `if (regenerate_driver(cn)) return; if (spell_self_driver(cn))
        // return; do_idle(cn, TICKS/2);` (`lq.c:2889-2897`) - the final
        // `do_idle` isn't modeled, same precedent as every other
        // stationary/idle NPC in this codebase.
        if self.regenerate_simple_baddy(lqnpc_id) {
            return;
        }
        self.spell_self_simple_baddy(lqnpc_id);
    }

    fn process_lqnpc_messages(
        &mut self,
        lqnpc_id: CharacterId,
        events: &mut Vec<LqNpcOutcomeEvent>,
    ) {
        let Some(lqnpc) = self.characters.get_mut(&lqnpc_id) else {
            return;
        };
        let messages = std::mem::take(&mut lqnpc.driver_messages);
        if messages.is_empty() {
            return;
        }

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.lqnpc_handle_char_message(lqnpc_id, message),
                NT_GOTHIT => self.lqnpc_handle_gothit_message(lqnpc_id, message, events),
                NT_TEXT => self.lqnpc_handle_text_message(lqnpc_id, message),
                NT_GIVE => self.lqnpc_handle_give_message(lqnpc_id, message, events),
                _ => {}
            }
        }
    }

    /// C `lqnpc`'s own `NT_CHAR` handler (`lq.c:2765-2773`, the greeting)
    /// plus `standard_message_driver`'s `NT_CHAR` case (`drvlib.c:2470-
    /// 2476`, `dat->mode == 'a'` aggressive self-defense) - both reached
    /// for the same message via `standard_message_driver(cn, msg, dat
    /// ->mode == 'a', 0)` at the bottom of C's loop iteration
    /// (`lq.c:2846`).
    fn lqnpc_handle_char_message(
        &mut self,
        lqnpc_id: CharacterId,
        message: &CharacterDriverMessage,
    ) {
        let seen_id = CharacterId(message.dat1.max(0) as u32);
        if seen_id.0 == 0 {
            return;
        }
        let Some(lqnpc) = self.characters.get(&lqnpc_id).cloned() else {
            return;
        };
        let Some(seen) = self.characters.get(&seen_id).cloned() else {
            return;
        };
        let (greeting, mode) = match lqnpc.driver_state.as_ref() {
            Some(CharacterDriverState::LqNpc(data)) => (data.greeting.clone(), data.mode),
            _ => return,
        };

        if !greeting.is_empty()
            && lqnpc_id != seen_id
            && seen.flags.contains(CharacterFlags::PLAYER)
            && char_dist(&lqnpc, &seen) < 12
            && char_see_char(&lqnpc, &seen, &self.map, self.date.daylight)
            && !mem_check_driver(&lqnpc.driver_memory, LQNPC_GREET_MEMORY_SLOT, seen_id.0)
        {
            self.npc_say(lqnpc_id, &greeting);
            if let Some(lqnpc_mut) = self.characters.get_mut(&lqnpc_id) {
                mem_add_driver(
                    &mut lqnpc_mut.driver_memory,
                    LQNPC_GREET_MEMORY_SLOT,
                    seen_id.0,
                );
            }
        }

        if mode == b'a' && lqnpc_is_valid_enemy(&lqnpc, &seen, &self.map, self.date.daylight) {
            let tick = self.tick.0 as i32;
            if let Some(lqnpc_mut) = self.characters.get_mut(&lqnpc_id) {
                let _ = add_simple_baddy_enemy_unchecked(lqnpc_mut, seen_id, 0, tick);
            }
        }
    }

    /// C `lqnpc`'s own `NT_GOTHIT` handler (`lq.c:2775-2781`, hurt-mark
    /// setting) plus `standard_message_driver`'s `NT_GOTHIT` case
    /// (`drvlib.c:2512-2538`, self-defense) - both reached for the same
    /// message.
    fn lqnpc_handle_gothit_message(
        &mut self,
        lqnpc_id: CharacterId,
        message: &CharacterDriverMessage,
        events: &mut Vec<LqNpcOutcomeEvent>,
    ) {
        // C `fight_driver_note_hit(cn);` (`drvlib.c:2514`).
        let tick = self.tick.0 as i32;
        if let Some(lqnpc) = self.characters.get_mut(&lqnpc_id) {
            lqnpc
                .fight_driver
                .get_or_insert_with(FightDriverData::default)
                .last_hit = tick;
        }

        let attacker_id = CharacterId(message.dat1.max(0) as u32);
        if attacker_id.0 == 0 {
            return;
        }
        let Some(lqnpc) = self.characters.get(&lqnpc_id).cloned() else {
            return;
        };
        let Some(attacker) = self.characters.get(&attacker_id).cloned() else {
            return;
        };
        let hurt_mark_id = match lqnpc.driver_state.as_ref() {
            Some(CharacterDriverState::LqNpc(data)) => data.hurt_mark_id,
            _ => return,
        };

        if attacker.flags.contains(CharacterFlags::PLAYER)
            && (1..MAXLQMARK as u32).contains(&hurt_mark_id)
        {
            events.push(LqNpcOutcomeEvent::SetPlayerMark {
                player_id: attacker_id,
                mark_id: hurt_mark_id,
            });
        }

        // C `if (ch[cn].group == ch[co].group) break; if (!can_attack(cn,
        // co)) break;` (`drvlib.c:2523-2528`).
        if lqnpc.group == attacker.group {
            return;
        }
        if !can_attack(&lqnpc, &attacker, &self.map) {
            return;
        }
        if let Some(lqnpc_mut) = self.characters.get_mut(&lqnpc_id) {
            let _ = add_simple_baddy_enemy_unchecked(lqnpc_mut, attacker_id, 1, tick);
        }
    }

    /// C `lqnpc`'s own `NT_TEXT` handler (`lq.c:2783-2825`):
    /// "followme"/"stopfollow" admin-mirroring commands, then (players
    /// only, in sight, within 10 tiles) substring-matched trigger/reply
    /// dialogue and the `tabunga` debug-stat-dump easter egg.
    fn lqnpc_handle_text_message(
        &mut self,
        lqnpc_id: CharacterId,
        message: &CharacterDriverMessage,
    ) {
        let Some(text) = message.text.as_deref() else {
            return;
        };
        let speaker_id = CharacterId(message.dat3.max(0) as u32);
        let Some(lqnpc) = self.characters.get(&lqnpc_id).cloned() else {
            return;
        };
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };
        let lower = text.to_ascii_lowercase();

        // C `if (strcasestr(ptr, "followme") && (ch[co].flags &
        // CF_LQMASTER) && char_dist(cn, co) < 20) dat->follow = co;`
        // (`lq.c:2787-2789`) - unconditional, before the player-only
        // filter below.
        if lower.contains("followme")
            && speaker.flags.contains(CharacterFlags::LQMASTER)
            && char_dist(&lqnpc, &speaker) < 20
        {
            self.set_lqnpc_follow(lqnpc_id, Some(speaker_id));
        }
        if lower.contains("stopfollow")
            && speaker.flags.contains(CharacterFlags::LQMASTER)
            && char_dist(&lqnpc, &speaker) < 20
        {
            self.set_lqnpc_follow(lqnpc_id, None);
        }

        // C `if (!(ch[co].flags & CF_PLAYER)) continue; if (!co ||
        // !char_see_char(cn, co) || cn == co) { remove_message(cn, msg);
        // continue; } if (char_dist(cn, co) > 10) { remove_message(cn,
        // msg); continue; }` (`lq.c:2795-2809`).
        if !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        if speaker_id.0 == 0
            || !char_see_char(&lqnpc, &speaker, &self.map, self.date.daylight)
            || lqnpc_id == speaker_id
        {
            return;
        }
        if char_dist(&lqnpc, &speaker) > 10 {
            return;
        }

        // C trims `ptr` to after the first `:` then the first `"` (the
        // `"<name>: \"<text>\""` wire wrapper) before matching - this
        // port's `text` is already the unwrapped spoken text, so no
        // trimming step is needed.
        let (triggers, replies) = match lqnpc.driver_state.as_ref() {
            Some(CharacterDriverState::LqNpc(data)) => (data.trigger.clone(), data.reply.clone()),
            _ => return,
        };
        for (trigger, reply) in triggers.iter().zip(replies.iter()) {
            if !trigger.is_empty() && lower.contains(&trigger.to_ascii_lowercase()) {
                self.npc_say(lqnpc_id, reply);
            }
        }

        // C `tabunga(cn, co, (char *)msg->dat2);` (`lq.c:2824`).
        self.apply_tabunga_text_notification(lqnpc_id, speaker_id, text);
    }

    /// C `lqnpc`'s own `NT_GIVE` handler (`lq.c:2827-2844`): a quest-item
    /// turn-in check against `dat->want_keyID`, then unconditional
    /// destruction of whatever's on the cursor.
    fn lqnpc_handle_give_message(
        &mut self,
        lqnpc_id: CharacterId,
        message: &CharacterDriverMessage,
        events: &mut Vec<LqNpcOutcomeEvent>,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(lqnpc) = self.characters.get(&lqnpc_id).cloned() else {
            return;
        };
        let Some(item_id) = lqnpc.cursor_item else {
            return;
        };
        let Some(item) = self.items.get(&item_id) else {
            return;
        };
        let (want_key_id, reward_item) = match lqnpc.driver_state.as_ref() {
            Some(CharacterDriverState::LqNpc(data)) => (data.want_key_id, data.reward_item.clone()),
            _ => return,
        };

        if item.template_id == make_lq_item_template_id(want_key_id) {
            if !reward_item.base.is_empty() {
                events.push(LqNpcOutcomeEvent::GiveRewardItem {
                    receiver_id: giver_id,
                    item: reward_item,
                });
            }
            self.npc_say(lqnpc_id, "Thanks, that's what I wanted.");
        }
        // C `destroy_item(ch[cn].citem); ch[cn].citem = 0;`
        // (`lq.c:2841-2842`) - unconditional.
        self.destroy_item(item_id);
        if let Some(lqnpc_mut) = self.characters.get_mut(&lqnpc_id) {
            lqnpc_mut.cursor_item = None;
        }
    }

    fn set_lqnpc_follow(&mut self, lqnpc_id: CharacterId, follow: Option<CharacterId>) {
        if let Some(CharacterDriverState::LqNpc(data)) = self
            .characters
            .get_mut(&lqnpc_id)
            .and_then(|character| character.driver_state.as_mut())
        {
            data.follow = follow;
        }
    }
}
