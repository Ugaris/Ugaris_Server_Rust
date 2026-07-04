# Rust Rewrite Porting Ledger

The C source inventory is tracked with:

```bash
git ls-files "src/**/*.c" "src/**/*.h"
```

Current tracked C/C header file count: 302.

## Rust Module Architecture

The former monolithic files were split into module directories in July 2026.
New porting work must land in the matching domain module, not in a giant
file:

- `crates/ugaris-core/src/item_driver/` - item-driver registry. `dispatch.rs`
  owns `use_item`/`execute_item_driver*`, `types.rs` owns the context/outcome
  types, `ids.rs` owns `IDR_*`/`IID_*`/`BOOK_*` constants, and per-domain
  files mirror the C sources (`doors.rs`, `chests.rs`, `potions.rs`,
  `alchemy.rs`, `area6_edemon.rs`, `area25_warped.rs`, ...). Tests live in
  `item_driver/tests/<domain>.rs`.
- `crates/ugaris-core/src/world/` - live world state. `mod.rs` owns the
  `World` struct; impl blocks are split by legacy system: `actions.rs`
  (do.c/act.c), `combat.rs` + `hurt.rs` + `death.rs` (act.c/death.c),
  `spells.rs` (spell paths + tool.c timers + poison.c), `effects.rs` +
  `effect_tick.rs` (effect.c), `npc_fight.rs`/`npc_idle.rs`/`npc_messages.rs`
  (drvlib.c fight driver + simple_baddy.c), `merchant.rs` (merchants),
  `doors.rs`, `items.rs`, `light.rs`, `spawn.rs`, `text.rs`, `teleport.rs`,
  `lq.rs`, `lab2_undead.rs`, `area_mech.rs`, `assembly.rs`,
  `traps_hazards.rs`, `item_outcomes.rs` (driver outcome/timer application).
  Tests live in `world/tests/<domain>.rs`.
- `crates/ugaris-server/src/` - the binary. `main.rs` keeps `Args`,
  `ServerRuntime`, and the tick loop; everything else is split by concern:
  `login.rs`, `snapshots.rs`, `map_sync.rs`, `effects_sync.rs`,
  `player_actions.rs`, `commands_admin.rs`, `commands_chat.rs`,
  `commands_player.rs`, `item_apply.rs`, `area_apply.rs`, `chests.rs`,
  `keyring.rs`, `stacks.rs`, `spawns.rs`, `merchants.rs`, `depot.rs`,
  `containers.rs`, `inventory.rs`, `transport.rs`, `weather.rs`, `xmas.rs`,
  `world_events.rs`, `zone.rs`, `rng.rs`, `constants.rs`. Tests live in
  `src/tests/<domain>.rs`.

Remaining oversized files worth splitting during future work:
`crates/ugaris-core/src/player.rs` (PlayerRuntime + PPD codecs) and the
`main.rs` tick loop itself (a single ~4.5K-line async fn).

## Coverage Rules

- A file is `Ported` only when its externally observable behavior is represented in Rust and covered by tests or a documented integration check.
- Constants-only headers may be marked `Ported` when all public constants and structs are represented in Rust.
- Large behavior files move through `Mapped`, `Partial`, then `Ported`.
- The C server remains the behavioral oracle until every file is `Ported` and client playtests pass.

## Ported

| C File | Rust Location | Notes |
|---|---|---|
| `src/common/balance.h` | `crates/ugaris-core/src/combat.rs` | Damage/balance constants ported with tests. |
| `src/common/client.h` | `crates/ugaris-protocol/src/client.rs`, `crates/ugaris-protocol/src/packet.rs` | Legacy `CL_*`, `SV_*`, command sizes, tick packet builders, map delta packet primitives, origin/scroll packets, basic `SV_MAP11` tile update body encoding, basic `SV_MAP10` character update body encoding, character identity/action packets, stat/resource packets, item/inventory packets, and container packet builders ported with tests. Full runtime map diff integration remains. |
| `src/common/color.h` | `crates/ugaris-core/src/text.rs` | Color/link marker constants ported with byte-level tests. |
| `src/common/direction.h` | `crates/ugaris-core/src/direction.rs` | Direction IDs and deltas ported with tests. |
| `src/common/fight.h` | `crates/ugaris-core/src/combat.rs` | Fight driver data shape ported. |
| `src/config/game_settings.h` / `src/config/game_settings.c` | `crates/ugaris-core/src/game_settings.rs` | Runtime game setting defaults, special item probability constants, and C compatibility-style accessors ported with tests. |
| `src/system/item_utils.h` / `src/system/item_utils.c`, `src/system/map.c` carried-item helpers | `crates/ugaris-core/src/item_ops.rs` | Result codes, inventory-space helpers, money item handling, inventory/cursor give behavior, consume behavior, carried-item removal, and carried-item replacement ported with tests. Ground drop integration remains. |
| `src/system/do.c` `use_item` dispatch boundary, `src/module/base.c` `potion_driver` non-template path, simple `food_driver` path, `teleport_driver` core path, `recall_driver` core path, `door_driver` core path, `teleport_door_driver` core path, and normal `chest_driver` slices | `crates/ugaris-core/src/item_driver.rs`, `crates/ugaris-core/src/world.rs`, `crates/ugaris-core/src/player.rs`, `crates/ugaris-server/src/main.rs` | Legacy account-depot-first, container open, depot open, item-driver dispatch ordering, `IDR_POTION` resource restoration/caps/consumption for potions that do not require empty-bottle template creation, typed deferred outcome for empty-bottle template creation, `IDR_FOOD` simple food consumption for kinds 0/1, `IDR_TELEPORT` drdata decoding/access checks/same-area drop-char-extended teleport, `IDR_RECALL` level/arena/dying checks plus same-area recall/scroll consumption, `IDR_DOOR` core open/close state with item flags, tile flags, sprite toggles, closed-state flag storage, blocked-doorway close refusal, and legacy extended-door neighboring foreground-sprite shifts, `IDR_TELE_DOOR` direction/level checks plus exact opposite-side same-area teleport, and `IDR_CHEST` treasure-index decoding, `treasure_N` template instantiation to cursor, per-player last-access runtime state, `drdata[5..7]` hour cooldowns, exact-key and skeleton-key chest access via cursor/inventory slots 30+, runtime keyring exact-key chest checks, death-gated access via `drdata[7]`, key-use feedback, legacy system feedback text, and PostgreSQL snapshot PPD load/save hookup for keyring plus treasure-chest cooldown runtime state ported with tests. Account depot loading, container permission model, empty-potion item template creation, special food/event handling, cross-area transfer execution, chest achievement persistence/protocol sending, random chest persistent PPD/RNG parity, door auto-close timers, remaining multi-tile door edge cases, tele-door clan-area messaging/guards, Brannington transport PPD gate, teleport/recall logging, and remaining concrete item driver effects remain. |
| `src/module/base.c` `usetrap` / `steptrap` core scheduling paths, `balltrap` dispatch boundary, area 2 `spiketrap_driver` / `flamethrow_driver` / `extinguish_driver` core paths | `crates/ugaris-core/src/item_driver.rs`, `crates/ugaris-core/src/world.rs`, `crates/ugaris-protocol/src/packet.rs`, `crates/ugaris-server/src/main.rs` | `IDR_USETRAP` delayed target-item scheduling with the using character, `IDR_STEPTRAP` zero-character timer target discovery using the legacy 1/3/5/7 direction scan and distance 1 then 2, character-triggered delayed zero-character target scheduling, `IDR_BALLTRAP` non-player/timer guards plus legacy `drdata[0..2]` projectile start/target/power decoding to a typed outcome, `IDR_SPIKETRAP` one-shot armed-state sprite toggle, legacy `drdata[1] * POWERSCALE` damage unit, one-second reset timer, timer reset state, `IDR_FLAMETHROW` timer-only fire countdown, lit/unlit sprite/light modifier transitions, direction target scan for one/two tiles, one-tick active rescheduling, `drdata[3]` idle interval scheduling, C `burn_char` duplicate suppression/one-minute `EF_BURN` lifecycle with direct legacy-unit damage, `IDR_EXTINGUISH` burn removal/no-burn handling, legacy extinguish feedback text, and `SV_CEFFECT` burn body encoding ported with focused core/protocol/server tests. Actual projectile/effect creation for ball traps, exact `hurt` armor/shield reduction/death handling, and light/sector invalidation callbacks remain. |
| `src/area/3/area3.c` `onofflight_driver` / `gate_driver` palace lamp-gate slice | `crates/ugaris-core/src/item_driver.rs`, `crates/ugaris-core/src/world.rs` | `IDR_ONOFFLIGHT` timer registration, player light toggling, sprite/light modifier mutation, switched-on/off palace lamp counters, all-lamps-on keep-open window, `IDR_PALACEGATE` zero-character timer dispatch, gate open/close tile/item flag mutation, blocked close refusal, dirty-sector marking, and startup scheduling for existing palace lamps/gates ported with focused core/world tests. Exact area 3 character/dialogue quest flow around lamp ghosts and palace story state remains. |
| `src/server.h` | `crates/ugaris-core/src/entity.rs`, `crates/ugaris-core/src/effect.rs`, `crates/ugaris-core/src/legacy.rs`, `crates/ugaris-core/src/map.rs` | Core flags, values, map/item/character/effect shapes including character sprite, item template ID, death counter, inventory ranges, version, tick constants ported. |
| `src/module/base.c` `stat_scroll_driver`, `src/system/skill.c` `raise_value_exp` scroll path, `src/system/player.c` `cl_raise` / `src/system/skill.c` `raise_value` (`CL_RAISE`) | `crates/ugaris-core/src/item_driver.rs`, `crates/ugaris-core/src/world.rs`, `crates/ugaris-server/src/main.rs` | `IDR_STATSCROLL` dispatch, carried-only and `/noexp` blocking, C skill-cost/start-factor/max checks needed by stat scrolls, XP grant/spend, bare/effective value raise, consume-on-success behavior, and runtime executed-outcome classification ported with focused tests. `CL_RAISE` now spends already-unspent exp (`raise_value`, distinct from the exp-granting `raise_value_exp` scroll path) through `World::raise_skill`, sending a single-value `SV_SETVAL0/1` + exp/exp_used feedback packet on success and staying silent on failure like C. Both raise paths now call `update_character`: `World::raise_skill` since iteration 17 and `World::apply_item_driver_outcome`'s `StatScrollUsed` arm since iteration 18 (the item driver itself only has `&mut Character`, so the `World`-level outcome handler recomputes once after the scroll's raise loop completes, matching C's per-raise `update_char` since the recompute is idempotent on the final `value[1]` state). As of iteration 20, the `StatScrollUsed` outcome handler also calls `check_levelup` before `update_character`, matching C `raise_value_exp`'s `check_levelup(cn)` call (`raise_value`/`CL_RAISE` never calls `check_levelup` in C, so `World::raise_skill` correctly has no such call). Achievement checks (`achievement_check_skill`/`achievement_check_level`) remain unported for both raise paths. |
| `src/system/act.h` | `crates/ugaris-core/src/legacy.rs` | Action IDs ported. |
| `src/system/questlog.h`, `src/system/questlog.c` (metadata table, `questlog_scale`, `questlog_done`, `questlog_open`/`close`), `src/system/player.c` `sendquestlog` base packet | `crates/ugaris-core/src/quest.rs`, `crates/ugaris-protocol/src/packet.rs`, `crates/ugaris-server/src/main.rs` | Quest IDs, flags, fixed-size quest log behavior, C bitfield quest-entry packing, base `SV_QUESTLOG` payload shape with zeroed random-shrine PPD, and `CL_GETQUESTLOG` response path ported with tests. The 85-entry `struct questlog questlog[]` metadata table (name/level-range/giver/area/nominal-exp/flags, incl. `QLF_XREPEAT`) is now ported verbatim as `QUEST_TABLE`/`quest_meta()`; `questlog_scale`'s repeat-completion decay curve (`scale_exp`) and `questlog_done`'s level taper (`taper_exp_by_level`) are ported as pure functions; `QuestLog::complete_legacy` ports the full `questlog_done` bookkeeping + exp computation (caller still applies `give_exp`/`dlog`/`sendquestlog`); `QuestLog::open`/`close` now match C's exact flag assignment/guard semantics instead of bitwise OR/AND-NOT approximations. `src/area/1/area1.h`'s `struct area1_ppd` and `src/common/nomad_ppd.h`'s `struct nomad_ppd` are now real fixed-layout codecs on `PlayerRuntime` (`area1_ppd`/`nomad_ppd` in `crates/ugaris-core/src/player.rs`, `DRD_AREA1_PPD`/`DRD_NOMAD_PPD` wired into the full ppd-blob decode/encode dispatch), with `questlog_init_area1`/`questlog_init_nomad` ported as pure functions (`init_area1_quests`/`init_nomad_quests` in `crates/ugaris-core/src/quest.rs`) taking a `PlayerRuntime`-built state snapshot. As of iteration 61, `questlog_init_area3`/`questlog_init_staff`/`questlog_init_twocity` are ported the same way (`init_area3_quests`/`init_staff_quests`/`init_twocity_quests` in `quest.rs`, snapshot builders `area3_quest_state`/`staff_quest_state`/`twocity_quest_state` in `player.rs`), with new named accessors added to the pre-existing `area3_ppd`/`staffer_ppd`/`twocity_ppd` raw-byte blocks for every NPC state field those three functions read. This also fixed a real size bug: `LEGACY_AREA3_PPD_SIZE` was `17 * 4` (68) but C `struct area3_ppd` is 18 `int` fields = 72 bytes; corrected to `18 * 4`. Two legacy C quirks are preserved verbatim rather than "fixed": `questlog_init_area3`'s `william_state` ladder has no final `else` (quests 22/23 keep their prior flags when `william_state <= 0`, unlike every other ladder in the function), and `questlog_init_staff`'s `yoatin_state` ladder's "open" branch tests `aristocrat_state` instead of `yoatin_state` (a copy-paste bug in the original C) - both have dedicated regression tests. The `questlog_init` top-level dispatcher (the `quest[MAXQUEST-1].done == 55` sentinel + calling all 5 sub-functions; needs a Rust `DRD_QUESTLOG_PPD` representation of `struct quest[MAXQUEST]`), the per-area `questlog_reopen_qN` reset side effects, `quest_exp.h`'s per-encounter exp/money constants, and wiring from NPC dialogue (which isn't ported yet) remain - see `PORTING_TODO.md` P3 "Questlog initialization & quest state machine" for the itemized gap. |
| `src/system/io.c` / `src/system/io.h` | `crates/ugaris-protocol/src/frame.rs`, `crates/ugaris-net/src/*`, `crates/ugaris-server/src/main.rs` | Legacy tick frame envelope, TCP session skeleton, per-session server command channels, runtime-to-session framed payload sending, listener readiness/error reporting, default info logging, IPv4 plus IPv6 localhost listening for `localhost`, multi-payload login bootstrap queueing, and chunked full-map bootstrap below legacy frame limits ported. Full gameplay send buffering, compression modes, and backpressure policy remain partial. |
| `src/system/map.h` / `src/system/map.c` primitives | `crates/ugaris-core/src/map.rs`, `crates/ugaris-core/src/item_ops.rs` | Legacy map indexing, bounds checks, movement/sight blocker helpers, grid wrapper, item map placement/removal, character map placement/removal, `NOMAGIC` flag sync, simple 3x3 drop order, extended pathfinder-backed drop order, carried-item removal, and carried-item replacement ported with tests. C `set_item_map`'s `IF_TAKE` decay-arming (`set_expire(in, item_decay_time)`) is ported at the `World::complete_drop` call site (`crates/ugaris-core/src/world/actions.rs`) rather than inside `map.rs` itself, since only `World` owns the timer queue - see "Ralph Loop - Ground Item Decay" below. Light/trap/notify callbacks remain. |
| `src/system/los.h` / `src/system/los.c` primitive LOS | `crates/ugaris-core/src/map.rs` | Conservative line-of-sight helper with blocker tests ported. Full per-character cached LOS table remains. |
| `src/system/light.h` / `src/system/light.c` primitives | `crates/ugaris-core/src/light.rs`, `crates/ugaris-core/src/map.rs`, `crates/ugaris-core/src/world.rs` | Legacy light distance, inverse-square light falloff, non-negative tile accumulation, character/item/effect light add/remove gates, takeable-item `MF_NOLIGHT` behavior, lava groundlight sprite table, foreground/sightblock shadow daylight calculation including injectable legacy-randomized flicker path, indoor daylight recomputation, mixed indoor/outdoor reset checks, live world light add/remove wiring for map item insertion, take/drop, timer-driven light item state changes, visible effect map slots, and world-level dirty-sector marking for groundlight/shadow/dlight recomputation ported with tests. Exact global RNG wiring for shadow flicker and bulk LOS-change remove/add around live world characters plus LOS-changing map edits remain. |
| `src/system/path.h` / `src/system/path.c` | `crates/ugaris-core/src/path.rs` | A* path shape, legacy heuristic, first-direction result, node cap, 2/3 movement costs, inner-bound successor checks, and `path_ignore_char`-style character-ignoring movement mode ported with tests. Other custom target callbacks and exact global-state compatibility remain. |
| `src/system/sector.h` / `src/system/sector.c` | `crates/ugaris-core/src/sector.rs` | Dirty sector tick tracking, `skipx_sector`, 8x8 character sector head/links, sound sector flood fill, shout sector flood fill, temporary sound door traversal, and hearing checks ported with tests. Runtime character/map integration remains. |
| `src/system/see.h` / `src/system/see.c` | `crates/ugaris-core/src/see.rs`, `crates/ugaris-core/src/entity.rs` | Character and item visibility rules ported with tests: invisible checks, light/daylight blending, infrared/infravision boosts, light/dark profession boosts, stealth/perception scoring, carried item hiding, takeable item light requirement, and front-wall item side checks. Uses current conservative Rust LOS helper. |
| `src/system/date.h` / `src/system/date.c` | `crates/ugaris-core/src/game_time.rs` | Game date units, moon phases, solstice/equinox flags, sunrise/sunset, area light overrides, and daylight calculation ported with tests. |
| `src/system/do.h` / `src/system/do.c` primitive actions, `src/system/act.c` primitive completions | `crates/ugaris-core/src/do_action.rs`, `crates/ugaris-core/src/entity.rs`, `crates/ugaris-core/src/world/actions.rs`, `crates/ugaris-core/src/world/regen.rs` | Error codes, duration constants, `speed`, `end_cost`, `do_idle`, deterministic `do_walk` movement checks/reservation, terrain/underwater movement costs, fast-mode endurance cost, timed `do_take`/`do_use`/`do_drop`/`do_attack` setup, `act_walk`, `act_take`, `act_drop`, `act_use` validation to typed item-driver request, `act_attack` deterministic-resolution bridge using ported attack formulas, action step readiness/reset, world-backed action completion helpers, minimal world action tick dispatcher including `AC_ATTACK1..3`, C `tile_special_check` slowdeath hazard damage, underwater drowning damage, oxygen bubble creation cadence, and pre-action tick-loop invocation, `turn`, and `act()`'s `regen_ticker` non-idle-action stamp ported with tests. `World::regenerate_characters` (called once per tick from `main.rs`) now ports C `regenerate()` (skill-gated endurance/lifeshield regen throttled per real second via the new `Character.last_regen` field) and the HP/endurance/mana/lifeshield-leak regen from `act_idle()`, gated by `regen_ticker + regen_time` and the `MF_NOREGEN`/area-33 special cases, with focused tests in `world/tests/regen.rs`. Since Rust's tick loop skips characters with `action == 0` entirely (no per-batch idle completion event like C), the idle regen applies continuously once per real tick using the per-tick-equivalent amount instead of C's `act1`-scaled batch amount; the steady-state rate matches, only the batching granularity differs (documented in `regen.rs`). Not yet ported: `reduce_rage`/`increase_rage` (no `rage` field on `Character` yet), the `NT_CHAR` notify-area call at the end of `act_idle` (tracked by the separate P0 "NPC sighting messages" task), and `check_endurance` (tracked by the "Speed mode" P0 task). `World::complete_attack_with_rolls_and_clash_roll` (`world/combat.rs`) now also emits `notify_area(.., NT_CHAR, ..)` from the attacker's position after `apply_legacy_hurt`, gated on `!CF_NONOTIFY` and firing on both hit and miss (matches C `act_attack`, act.c:763-793), with a defensive attacker-still-alive (`!CharacterFlags::DEAD`) guard mirroring C's `if (!ch[cn].flags) return 0`. Weather/effect movement modifiers, full central action dispatcher, exact legacy RNG parity, `sub_surround`/`V_SURROUND` and `increase_rage` (act_attack's remaining side effects - no `rage`/`V_SURROUND` fields on `Character` yet), death/notify/surround integration, actual use-item driver effects, exact tile-special sound fan-out/random underwater sound selection, and spells remain. |
| `src/system/drvlib.c` distance helpers | `crates/ugaris-core/src/drvlib.rs`, `crates/ugaris-core/src/entity.rs` | `map_dist`, `char_dist`, `tile_char_dist`, and `step_char_dist` including `tox`/`toy` target-position handling ported with tests. Broader driver library remains. |
| `src/system/error.h` / `src/system/error.c` | `crates/ugaris-core/src/error.rs` | Legacy `ERR_*` constants, exact error string table, and out-of-bounds fallback behavior ported with tests. |
| `src/system/act.c` / `src/system/do.c` pure attack formulas | `crates/ugaris-core/src/attack.rs`, `crates/ugaris-core/src/do_action.rs`, `crates/ugaris-core/src/world.rs` | Attack chance breakpoint table, strict roll comparison, attack/parry skill math, side/back/assassin bonuses, direct melee damage units, armor/lifeshield reduction helpers, `do_attack` setup/reachability checks, core `can_attack` guard subset, `act_attack` HP/lifeshield damage application, `PAC_KILL` adjacent/path-to-target setup, and timed `AC_ATTACK1..3` world completion ported with tests. Exact legacy RNG parity, full `sub_attack` side effects, rage/surround hits, death/hurt/notify integration, clan/hate/group policy, and player fightback state remain. |
| `src/system/do.c` / `src/system/act.c` pure spell formulas and primitive queued spell bridge, `src/system/tool.c` spell timer core, `src/system/poison.c` poison lifecycle core | `crates/ugaris-core/src/spell.rs`, `crates/ugaris-core/src/do_action.rs`, `crates/ugaris-core/src/world.rs` | Spell constants, effect IDs, spell item driver IDs, spellpower/duration math, variable mana spend formulas, speed modifier formulas, combat spell helper formulas, player queued spell setup for heal/magic shield/pulse/bless/freeze/flash/ball, timed magic-shield lifeshield completion, timed heal target HP completion, no-op pulse completion bridge, timed bless completion that installs/replaces carried `IDR_BLESS` spell items with legacy modifier/drdata layout, timed flash self-spell installation, timed freeze nearby-target `IDR_FREEZE` speed-spell installation using C `freeze_value` math, C `do_ball`/`act_ball` setup plus `EF_BALL` creation/movement/collision/nearby strike-damage cadence and short-lived `EF_STRIKE` target visuals, C `create_spell_timer` timed spell-driver classification, absolute expiry timer scheduling for bless/freeze/flash/poison spell installation and existing carried spell scan, C `remove_spell` serial/slot guarded carried-spell deletion plus `IDR_FREEZE` action duration/step rescaling after speed restoration, `poison_someone` carried `IDR_POISON0..3` item creation with legacy drdata layout/duration, periodic `poison_callback` tick weakening/rescheduling plus C `hurt(cn, POWERSCALE / 3, 0, 1, 0, 50)` armor/lifeshield/death/message side effects, and `remove_poison`/`remove_all_poison` helpers ported with tests. Exact `hurt` side effects/death handling for ball ticks, notifications/sounds, ice-demon curse side effects, and exact visibility/help/attack policy remain. |
| `src/module/base.c` `enchant_item`, regular `anti_enchant_item` paths | `crates/ugaris-core/src/item_driver.rs`, `crates/ugaris-core/src/world.rs`, `crates/ugaris-server/src/main.rs` | `IDR_ENCHANTITEM`, `IDR_ANTIENCHANTITEM`, and the regular `IDR_SPECIALANTIENCHANTITEM` dispatch boundary are ported for carried-orb/cursor-equipment semantics, wearable/no-enhance/left-hand blocks, +20 max modifier cap, 3 counted-enhancement limit excluding weapon/armor/demon/light, orb consumption, target modifier add/reduce/remove, and server outcome accounting/empty-cursor feedback with tests. Extracting anti-orb item creation, exact success/block text variants, requirement recomputation, item look output, orb spawner PPD/cooldowns, and live-data smoke coverage remain. |
| `src/system/timer.h` / `src/system/timer.c` | `crates/ugaris-core/src/scheduler.rs` | Sorted tick timer queue and due-event drain behavior ported with tests. Function-pointer callbacks represented as named timer events. |
| `src/system/task_scheduler.h` / `src/system/task_scheduler.c` | `crates/ugaris-core/src/scheduler.rs`, `crates/ugaris-server/src/main.rs` | 128-task periodic scheduler, seconds-to-ticks conversion, due-task detection, and server tick-loop integration ported. |
| `src/system/talk.h` / `src/system/talk.c` low-level logging | `crates/ugaris-core/src/log_text.rs`, `crates/ugaris-core/src/world/text.rs`, `crates/ugaris-protocol/src/packet.rs` | `LOG_*` constants, text sanitization, scrollback behavior, raw color-marker preservation, `say`/`shout`/`holler`/`emote`/`whisper`/`murmur`/`quiet_say` message formats, `SV_TEXT` little-endian length layout, and `sound_area` positional player `SV_SPECIAL` fan-out math with legacy talk-sector gating ported with tests. `World::npc_say`/`npc_quiet_say`/`npc_emote`/`npc_murmur` (`world/text.rs`) generalize C's `say`/`quiet_say`/`emote`/`murmur` for NPC drivers via the existing `pending_area_texts` queue, at their respective `say_dist`/`quietsay_dist`/`emote_dist`/`whisper_dist` (murmur reuses whisper's) distances. `holler`/`shout`/`whisper` NPC-side helpers not yet added (no NPC driver calls them; only player local-speech commands in `commands_chat.rs` do). |
| `src/system/player.c` `log_player` | `crates/ugaris-protocol/src/packet.rs`, `crates/ugaris-core/src/log_text.rs` | Text packet framing and scrollback rules ported with tests. |
| `src/system/area.c` randomized `area_sound` ambient effects | `crates/ugaris-core/src/area_sound.rs`, `crates/ugaris-server/src/main.rs` | Wet dungeon, dry dungeon, woods, park, and underwater section-to-sound roll tables ported, including legacy `player_special` option math and server `SV_SPECIAL` packet emission after successful player-driver action completions, with focused core/server tests. Exact call cadence/RNG parity and remaining sound call-site wiring remain. |
| `src/system/player.c` login block and initial client sync scaffold, `kick_player`, `src/module/lostcon.c`, `tick_login()`/`read_login()` reclaim halves of `src/system/database/database_character.c` / `src/system/player.c` | `crates/ugaris-protocol/src/login.rs`, `crates/ugaris-net/src/session.rs`, `crates/ugaris-core/src/world/lostcon.rs`, `crates/ugaris-core/src/character_driver.rs`, `crates/ugaris-core/src/player.rs`, `crates/ugaris-server/src/lostcon.rs`, `crates/ugaris-server/src/main.rs` | Login block size, endian layout, vendor protocol version, password obfuscation, runtime character-id assignment, temporary `new_warrior_m` player-template instantiation with starter equipment/items, optional PostgreSQL `begin_login`/snapshot load when `DATABASE_URL` is configured, DB snapshot PPD decode into player runtime, logout snapshot save with carried items and re-encoded legacy PPD blob, runtime login/bootstrap response (`SV_LOGINDONE`, `SV_TICKER`, `SV_MIRROR`, `SV_PROTOCOL`, `SV_ORIGIN`, full visible diamond `SV_MAP11`, visible character `SV_MAP10`, visible character identity `SV_NAME`, `SV_SETVAL*`, resources, exp, gold, cursor item, initial equipment/inventory `SV_SETITEM`, `SV_TEXT`), C-mapped `SV_SCROLL_*` plus origin, character clear/update, newly visible diamond fringe tile/character/name packets for one-tile walk completions, per-session visible-diamond cache initialized at login/refresh, same-origin non-walk map diff packets for changed tile/character cells, and cached visible-character `SV_NAME` identity packets for newly seen or renamed characters ported with tests. Player spawn/despawn is no longer instant on disconnect: `kick_player`'s `CDR_LOSTCON` linger is ported (`World::enter_lostcon`/`reclaim_lostcon`/`is_lostcon`/`expired_lostcon_characters` plus a `CharacterDriverState::Lostcon(LostconDriverData { deadline })` state slot, `ugaris-server`'s `enter_lostcon_on_disconnect`/`reclaim_lostcon_on_login`/`take_expired_lostcon_characters`, and `PlayerRuntime::reclaim_for_session`) - a disconnecting player's character stays on the map under `CDR_LOSTCON` for `runtime.lagout_time` ticks (attackable, not actively defending itself yet), a reconnect within the window reclaims the same in-memory character in place (skipping a stale DB re-read) with its stashed `PlayerRuntime` (PPD blob, keyring, etc.) restored, and the tick loop saves+despawns it if the window expires unclaimed. Server smoke-tested listening without DB. Password hash verification, robust login rejection/client error flow, character selection beyond direct name lookup, true inventory delta cache, character color/clan/PK identity fields, visibility/light cache parity, full player state machine, the `lostcon_driver` self-defense AI cascade (auto-heal/potion/magicshield/fight-back), lostcon's restarea/arena instant-leave and karma early-exit branches, and duplicate-login kick of a still-connected old session still remain. |
| `src/module/book.c` / `src/module/book.h` `IDR_BOOK` text driver | `crates/ugaris-core/src/item_driver.rs`, `crates/ugaris-server/src/main.rs` | Book driver dispatch, zero-character no-op boundary, C `BOOK_*`/`SIGN_*` text cases, raw color marker preservation, character-specific demon ritual words via the legacy `id_rand`/`demonspeak` formula, Earth Demon sign readability gates using Ancient Knowledge, random Book Nook joke selection, earth-demon diary `player_special` effects, runtime `SV_TEXT`/`SV_SPECIAL` emission, and C-compatible item-driver return code behavior ported with focused tests. Exact global RNG parity for joke selection remains. |
| `src/system/player_driver.h` / `src/system/player_driver.c` action setters and primitive runtime bridge, `src/system/area.c` look-section/walk-section slices | `crates/ugaris-core/src/player.rs`, `crates/ugaris-core/src/world.rs`, `crates/ugaris-core/src/area_section.rs`, `crates/ugaris-server/src/main.rs` | `player_driver_stop`, `halt`, direct action setters, serial-preserving item/character actions, teleport, spell queue insertion/last-slot overwrite behavior, server-side use of driver setters for direct/spell client actions, and primitive tick-loop setup/completion for idle, walk-dir including diagonal wall-slide fallback, `PAC_MOVE`, adjacent/path-to-item take, adjacent/path-to-target drop, adjacent/path-to-item use including front-wall pathing, `PAC_TELEPORT` as facing item-use with legacy `spec = teleport + 1`, immediate `PAC_LOOK_MAP` turn/LOS/request handling plus server `SV_TEXT` feedback for hidden targets, C `show_section` section-name/level difficulty text for all non-empty legacy area-sector tables, coordinate fallback, and rest/clan/arena/peace flags, C `walk_section_msg` per-player section tracking with dark-gray `Now entering`/`Now leaving` feedback after successful walks, `PAC_GIVE` adjacent/path-to-recipient setup plus `AC_GIVE` cursor-item transfer, and `PAC_KILL` adjacent/path-to-target setup plus timed attack completion, and the `PAC_KILL` pre-switch stale-target-serial guard (C's `ch[player[nr]->act1].serial != player[nr]->act2` check) plus live-traffic serial capture for Kill/Give/character-targeted spells (see "Ralph Loop - Serial Validation Everywhere" below) ported with tests. Queued spell priority execution, actual item use effects beyond potion, full combat/death/fightback side effects, wall-use/door interaction during movement, music/special sounds for section changes, and action error side effects remain. |
| Zone template/map parser scaffolding from `src/system/create.c` / `src/system/map.c` | `crates/ugaris-core/src/zone.rs` | Legacy token parsing, `.itm`/`.chr` template record parsing including item `ID`, `.map` directive parsing with origin offsets, live item template ID retention, and tiny sample application into `World` ported with tests. Production zone validation, startup integration, full character template fields, item-driver creation side effects, and respawn/random-loot behavior remain. |
| `src/system/death.c` `kill_char`/`die_char`/`god_save_char`/`respawn_callback`/`kill_score_level`/`death_loss`/`drop_grave` core, `src/system/respawn.c` boundary | `crates/ugaris-core/src/world/death.rs`, `crates/ugaris-core/src/world/hurt.rs`, `crates/ugaris-core/src/attack.rs`, `crates/ugaris-server/src/spawns.rs`, `crates/ugaris-server/src/main.rs` | `apply_legacy_hurt` now ports the C `hurt()` fatal-blow decision point for player `saves`: a non-PK death with `saves > 0` calls `World::god_save_character` (decrement+cap saves, `got_saved++`, hp reset, poison/burn removal, Ishtar feedback text, same-area rest transfer) instead of the normal kill path, exactly like C calling `god_save_char` before `kill_char` ever runs. Lethal hurt otherwise runs the C `kill_char` follow-up: death-driver dispatch and NT_DEAD fan-out (already ported) plus respawn-timer registration keyed by template/spawn tile, killer kill-score experience with the exact C level-taper table, hardcore kill bonus, LAG caps, queued server-side `give_exp` routing through runtime EXP modifiers, and the timed `AC_DIE` action (duration 12, act1 = killer, act2 = ispk). `AC_DIE` completion ports `die_char`: map/effect removal, C body rules (`CF_NOBODY` given-item drops, `CF_ITEMDEATH` slot-30 drop, `dead_body` items with the legacy sprite formula/description/player color drdata), extended-drop grave placement, body decay expire timers via a generic `expire_item` timer, loot containers as `contained_in` items (inventory + cursor + gold money item with the C sprite ladder; worn equipment kept except two shuffle-selected pieces; spells destroyed), player exp loss with the C newbie/used-exp taper and hardcore quarter, PK no-loss branch, resource restore, rest-position return, and NPC destruction. `respawn_callback` re-instantiates the stored zone template server-side with resource init and ten-second blocked-tile retries. Characters now carry serde-defaulted `template_key`/`respawn_ticks`, zone characters stamp spawn tiles into `rest_x/rest_y` like C `tmpx/tmpy`, and dying players can no longer cancel `AC_DIE` with new actions. Focused core tests cover the kill metadata, kill-score table, body/loot/money drops, NOBODY/ITEMDEATH branches, respawn scheduling/retry, player exp/PK/rest behavior, body expiry, and money sprites. Remaining gaps: death-mode loot tables (`loot.c`, currently unreferenced by zone data), `CDR_LOSTCON` exp cap, cross-area rest transfers, first-kill/military/achievement kill hooks, and exact global RNG parity for equipment loss. |
| `src/module/merchants/store.c`, `src/module/merchants/merchant.c` core, merchant view slices of `src/system/player.c` / `src/system/act.c` `check_merchant`, `src/system/database/database_merchant.c`, `src/system/auction/auction_db.c` | `crates/ugaris-core/src/world/merchant.rs`, `crates/ugaris-core/src/character_driver.rs`, `crates/ugaris-server/src/merchants.rs`, `crates/ugaris-server/src/commands_chat.rs`, `crates/ugaris-server/src/main.rs`, `crates/ugaris-db/src/merchant.rs`, `migrations/0005_merchant_stores.sql`, `crates/ugaris-db/src/auction.rs`, `migrations/0006_auction_house.sql` | `CDR_MERCHANT = 6` now has a typed driver-state (`MerchantDriverData`) parsed from C `merchant_driver_parse` args at zone load. Merchant NPCs lazily create stores from carried inventory 30+ (beyond `ignore`) as `always` stock with `pricemulti` defaulting to 400, greet visible players once per legacy 12-hour memory window with the C greeting/say format and Fred's extended range, react to `"<name> ... trade"` NT_TEXT speech by setting the speaker's `ch.merchant`, and destroy given items. Plain player `say` speech now fans out as NT_TEXT driver messages to nearby NPC drivers. C `salesprice`/`buyprice` formulas (barter + trader profession + 400 divisor, money exemption) are ported with tests, `sell`/`buy` port cursor-based buying/selling with always-stock preservation, sold-out/gold-low/cursor guards, ware stacking via `store_items_equal`, quest/nodepot/bond/lab/money stocking exclusions, and store gold accounting. The server sends C `con_type 2` store views (`SV_CONNAME`, `SV_CONCNT`, `SV_CONTAINER` sprites, `SV_PRICE`, `SV_ITEMPRICE`, `SV_CPRICE`), routes `CL_CONTAINER`/`CL_LOOK_CONTAINER` merchant-first like `cl_container` with `check_merchant` validation, formats the legacy bought/sold/too-expensive feedback, supports fast-buy inventory storing (`store_citem`), pushes view updates when the active merchant changes, and closes the view with a `con_type 0` packet. `CL_FASTSELL` (`cl_fastsell`, `src/system/player.c:877`) now quick-sells straight from an inventory slot: `apply_fast_sell` in `crates/ugaris-server/src/merchants.rs` reuses the existing simplified `swap` (`inventory_swap_slot`) to pick the slot item onto the cursor, re-validates with `check_merchant`, blocks quest items with the exact C hold-SHIFT message (leaving the item on the cursor, matching C's early return after the swap already ran), and otherwise reuses `merchant_store_sell`/`buyprice` for the trade. `database_merchant.c`'s `load_merchant_inventory`/`save_merchant_inventory` are now ported as `PgMerchantRepository::load_store`/`save_store` (`crates/ugaris-db/src/merchant.rs`), keyed like C by `(merchant_name, merchant_x, merchant_y)` but storing the whole ware list as one `jsonb` array per merchant instead of one row per ware; `main.rs` loads on first store creation (diffing `world.merchant_stores` keys before/after `process_merchant_actions()` each tick, since `ensure_merchant_store` only creates once) or saves an initial snapshot when nothing was persisted yet, and both the buy (`Container`) and fast-sell (`FastSell`) command paths re-save the full store after a successful trade. Focused core tests cover prices, arg parsing, store creation, trade activation, buy/sell mutations, quest-item exclusion, busy/distance clearing, and greeting memory; server tests cover the snapshot<->store conversion helpers; db tests cover JSON round-tripping and (behind `DATABASE_URL`) a live save/load round trip against Postgres. `add_special_store`/`create_special_item` (`src/module/merchants/store.c:229-323`, `src/system/tool.c:2620-2789`) are now ported as `World::create_special_item`/`World::add_special_store`/`World::refresh_special_stores` (`crates/ugaris-core/src/world/special_item.rs`): the full 76-entry weighted `special_item[]` enchant table, the 21-entry base-item roll, the potion branch, `lowhi_random`, and `set_item_requirements_sub` are all transcribed from C, and a `special`-flagged merchant now seeds five special wares on first store creation and one more every 12 real-time hours, matching `merchant_driver`'s C timing exactly; `main.rs` persists the merchant's store whenever `refresh_special_stores` reports a change. Remaining gaps: `create_special_item` is not yet wired to chest/loot generation (`create.c:1102`'s `special_prob`/`special_str`/`special_base` template fields), day/night shop movement/door handling (so persistence currently keys off `character.x/y` at store-creation time rather than C's `tmpx/tmpy`), C's incremental per-item `merchant_tasks.c` task queue (Rust always does a full-store upsert instead), the periodic `save_all_merchants`/admin `#saveall` full-DB sweep, and exact global RNG parity. `merchant.c::analyse_text_driver`'s `qa[]` small-talk table is now ported behind a reusable, driver-agnostic matcher: `character_driver::{TextQaEntry, TextAnalysisOutcome, tokenize_text_words, analyse_text_qa}` tokenizes spoken text into lowercase words (C's `' ' ',' ':' '?' '!' '"' '.'` delimiter set, own-name filtering, 20-word cap, 250-byte-per-word bailout) and scans a qa table for an exact-length ordered match, returning `Said(text)` for entries with a canned `%s`-templated answer or `Matched(code)` for entries that only report an `answer_code` (C's `who are you`/`what's your name` style). `world::merchant::merchant_qa_reply` wires the `MERCHANT_QA` transcription of `merchant.c`'s table through it with the C guard clauses (`CF_PLAYER`, `char_dist <= 12`, `char_see_char`) and emits the reply via a new `merchant_quiet_say` helper using `settings.quietsay_dist` (C `quiet_say`'s `log_area(..., quietsay_dist, ...)` - the existing greet-on-sight message still uses `SAY_DIST`, which is a pre-existing mismatch left alone here). Remaining: `gwendylon.c`/`bank.c`/`base.c`/`military.c`/`forest.c`/`area3.c`/`arkhata.c`/`orb_bank_npc.c` each have their own `qa[]` tables and drivers that still need porting onto `analyse_text_qa`. `merchant.c::aclerk_driver` (`CDR_ACLERK = 4`, the Cameron arena clerk) is now ported in `crates/ugaris-core/src/world/aclerk.rs`: typed `AclerkDriverData`/`parse_aclerk_driver_args` parsed at zone load, store creation shared with `ensure_merchant_store`/`refresh_special_stores` (generalized to accept either `CharacterDriverState::Merchant` or `::Aclerk`, since C's `create_store`/`add_special_store` calls are identical between the two drivers), a once-per-visible-player "Welcome to the Cameron Arena!" greeting within 5 tiles (memory slot 7, 12h clear timer, matching only the *first* of C's three copy-pasted `NT_CHAR` `quiet_say` blocks - the other two are unreachable dead code behind an unconditional `continue`), the hardcoded `abuser()` persistent-ID list reacting to `"<name> ... trade"` speech with a murmur/emote/murmur `RANDOM(3)` (checked against the raw runtime `CharacterId`, the same simplification already used for `TraderDriverData::c1_id`/`c2_id`), given-item vanishing, and an 11-line idle-murmur table including two lines with an embedded period in the C format string that doubles up with `emote()`'s own trailing period (`"...forest.."`, `"...himself up.."`) - copied exactly. Unlike `merchant_driver`, the arena clerk's trade-request handler never sets `ch[co].merchant = cn`, so saying "<clerk>, trade" never actually opens its store in C, and this port matches that. Remaining gaps: day/night shop movement/door handling (same gap as `CDR_MERCHANT`), and the separate `src/system/auction/*.c` + `database_merchant.c` auction-house subsystem the arena clerk driver itself never calls (the community client has no auction UI at all per `render.c`, so that slice may end up N/A once someone audits it directly). 13 focused tests in `world/tests/aclerk.rs`. Continuation (iteration 48): `src/system/auction/auction_db.c`'s full DB layer (create/update/get/delete/search/player-auctions/count-active auctions, plus delivery create/pending-list/claim/summary and expired-auction cleanup) is now ported as `AuctionRepository`/`PgAuctionRepository` in `crates/ugaris-db/src/auction.rs` with `migrations/0006_auction_house.sql`, storing the auctioned item as `jsonb` instead of C's raw-BLOB-plus-`SUBSTRING`-offset approach. Nothing calls this repository yet - `auction_house.c`'s business logic and the `/ah` command in `auction_cmd.c` are still unported, so this is DB-storage plumbing only. 9 new db tests (5 unit + 4 live, the live ones verified against a real ephemeral Postgres 16 container, not just compiled). |
| Area terrain startup loading from `ugaris_data/zones/<area>/*.map` | `crates/ugaris-server/src/main.rs`, `crates/ugaris-core/src/zone.rs` | Server startup resolves `UGARIS_ZONE_ROOT` or default `ugaris_data/zones` / `../ugaris_data/zones`, loads generic and area `.itm`/`.chr` templates best-effort, loads the first area `.map`, accepts signed legacy sprite IDs, tolerates missing item/character templates while preserving terrain, sanitizes `from/to` range copies so live item/character IDs and temporary item blockers are not duplicated across terrain ranges, reports load counts, keeps the loader alive for runtime template instantiation, and chooses an open spawn tile. Area 1 `above1.map` smoke-tested: 65,533 ground tiles, 16,969 blocked tiles, 1,780 item templates, 188 character templates, 2,236 placed items, and 446 placed characters. Process-level legacy login smoke confirmed map bootstrap payloads after loading real area data. Full `.pre` expansion/generator parity, complete template metadata, respawn/random loot, and all object driver side effects remain. |
| `src/module/bank.c`, `src/module/bank.h` | `crates/ugaris-core/src/world/bank.rs`, `crates/ugaris-core/src/character_driver.rs`, `crates/ugaris-core/src/player.rs`, `crates/ugaris-server/src/world_events.rs`, `crates/ugaris-server/src/main.rs` | `CDR_BANK = 22` now has a typed driver state (`BankDriverData`) parsed from C `bank_driver_parse` args at zone load. `World::process_bank_actions` ports the full `bank_driver` body: `NT_CHAR` greeting is a periodic nearby-player scan (same simplification `world/merchant.rs` already established for its own greeting, rather than reacting to `notify_area`'s `NT_CHAR` broadcasts), small talk via the shared `analyse_text_qa` matcher against a transcribed `BANK_QA` table (including the verbatim "I'm just a merchant" copy-paste quirk from `merchant.c`), `deposit`/`withdraw`/`balance` text commands (raw substring search matching C's `strcasestr`, including the "explain deposit" double-reply quirk), `NT_GIVE` cursor-item destruction, the 16-line idle-murmur table with `RANDOM(25)`/`RANDOM(16)` throttling, the 12h greet-memory-clear timer, and the day/night shop-position/door movement block (`is_closed`/`is_room_empty`/`opening_time` ported fresh here - no prior Rust equivalent existed; `move_driver`/`use_item_at` map onto the existing `World::setup_walk_toward`/`toggle_door`). The persistent `ppd->imperial_gold` balance (`DRD_BANK_PPD`, C `struct bank_ppd`) is ported as `PlayerRuntime::bank_gold` with `encode_legacy_bank_ppd`/`decode_legacy_bank_ppd`; since `World` cannot see `PlayerRuntime`, deposit/withdraw/balance requests that need the persistent balance are queued as `BankEvent`s (`pending_bank_events`/`drain_pending_bank_events`, matching the existing `pending_kill_exp`-style convention) and applied by `world_events.rs::apply_bank_events` from `main.rs`'s tick loop. Documented deviations: the full keyed-door `use_driver` dispatch (`item_driver::door_driver`'s key-requirement gate) is not replicated for the bank's own door (toggles directly, since no zone data is expected to key a bank door); the "account"/"explain deposit/withdraw/balance" qa answers drop their `COL_LIGHT_BLUE`/`COL_RESET` color styling (the shared `&str`-based qa pipeline cannot carry the raw non-UTF8 legacy color marker byte) while keeping the wording byte-for-byte; `NT_GIVE` unconditionally destroys the received item rather than first trying `give_driver`'s give-it-back attempt (same simplification `world/merchant.rs` already made). 17 focused tests in `world/tests/bank.rs` plus 2 PPD round-trip tests in `player.rs`. |

## Partial

| C File | Rust Location | Remaining Work |
|---|---|---|
| `src/system/database/database.h` | `crates/ugaris-db/src/lib.rs` | PostgreSQL pool, module boundary, and database handle ported. |
| `src/system/database/database_area.h` | `crates/ugaris-db/src/area.rs`, `migrations/0001_core_accounts_characters.sql` | Area server records and alive/down/get operations scaffolded. |
| `src/system/database/database_character.h` / `database_character.c` login and snapshot paths, `src/system/player.c` `read_login`/`player_client_exit` reject path, `src/system/badip.c` | `crates/ugaris-db/src/character.rs`, `crates/ugaris-server/src/login.rs`, `crates/ugaris-server/src/constants.rs`, `crates/ugaris-server/src/main.rs`, `migrations/0001_core_accounts_characters.sql`, `migrations/0002_sessions_questlog_anticheat.sql`, `migrations/0003_character_snapshots.sql`, `migrations/0004_bad_passwords.sql` | Login status semantics, character target lookup, legacy plaintext subscriber-password comparison against `accounts.password_hash`, current-area update, login session insert, release semantics, guarded backup save, logout save, Rust character JSON snapshot load/save, character item snapshot rows, server-side optional snapshot load/logout-save integration for PostgreSQL, client-facing login rejection (every non-`Ready` `LoginOutcome`/DB error now sends the exact C `player_client_exit` `SV_EXIT` reject text and disconnects instead of spawning a scaffold character), IP-based bad-password rate limiting (`is_badpass_ip`/`add_badpass_ip`, `>3`/60s, `>8`/1h, `>25`/24h windows, backed by a new `bad_passwords` table) constructing `LoginOutcome::TooManyBadPasswords`, and same-account duplicate-online-character detection (`load_char_dup`) constructing `LoginOutcome::Duplicate` are scaffolded/ported. `clean_badpass_ips` confirmed dead code in C (declared, never called) and intentionally not ported. `begin_login_tx`'s full row-decision branching (unknown name, wrong password + bad-password recording, locked character/account, ip-locked, unfixed, not-paid, `allowed_area <= 0`, duplicate-login reject, `account_id == 1` duplicate exemption, `NewArea` routing, success `Ready` + `login_sessions` insert) now has a `DATABASE_URL`-gated live-Postgres test suite (`crates/ugaris-db/src/character.rs::tests::live_login`, 12 tests, `tokio` dev-dependency added to `ugaris-db`) - skips cleanly with no `DATABASE_URL` set, verified against a throwaway local `postgres:16-alpine` Docker container. Live DB migration verification (actually running `migrations/*.sql` against a fresh Postgres, done manually this iteration but not automated), cross-area `NewArea` redirect (`player_to_server`, deferred to the separate "Cross-area transfer" task), a true end-to-end reject test over a real TCP socket, and full legacy binary blob decode/encode remain. |
| `src/system/database/database_anticheat.h` / `database_anticheat.c` session/event core | `crates/ugaris-db/src/anticheat.rs`, `migrations/0002_sessions_questlog_anticheat.sql` | Session/event schema, typed PostgreSQL repository boundary, session create/character/fingerprint/status/bot-score/counter/end operations, event logging, cleanup, and legacy result/action/risk text mappings ported with tests. Runtime anti-cheat packet integration, live migration verification, player-stat/admin query/signature/IP/hardware tables, and exact legacy MySQL report fan-out remain. |
| `src/system/player.c` / `src/system/player.h` | `crates/ugaris-net`, `crates/ugaris-core`, `crates/ugaris-server` | Player states, `PAC_*`, command recognition, command payload parsing, login parse, runtime registry, session send channels, scaffold character spawn, direct action setters, action queue, and primitive world action bridge exist; full action execution, map cache/client sync, inventory delta sync, text logging, transfer, anti-cheat integration remain. As of iteration 38, the `ClientAction::Nop`/`ClientInfo`/`Log`/`ModPacket` dispatch audit is closed: `cl_nop`/`cl_clientinfo` (true C no-ops) get explicit non-logging match arms in `crates/ugaris-server/src/main.rs`'s per-tick dispatch and `player_actions.rs::apply_player_action`'s immediate dispatch instead of falling through a catch-all; `cl_log` is ported as a `debug!`-logged `charlog`-shaped message via new helper `player_actions::format_client_log_message`; `ModPacket` (`cl_mod1`/`cl_mod3`) is a `debug!`-logged no-op matching C's own "acknowledge for now" handshake stub. |
| `src/system/tell.h` / `src/system/tell.c`, `/tell` slice from `src/system/command.c` / `src/system/chat/chat.c` | `crates/ugaris-core/src/tell.rs`, `crates/ugaris-server/src/main.rs` | C-compatible ten-slot sent-tell tracking, duplicate suppression, first-empty insertion, received-tell removal, strict one-second timeout expiry, `DRD_TELL_DATA` ID, legacy not-listening feedback text, runtime `/tell <name> <text>` parsing, online character lookup, sender acknowledgement/self-tell feedback, staff-name/staff-code formatting, no-tell and ignore blocking with delayed not-listening feedback, recipient received-tell clearing, and spy fan-out are ported with focused tests. Offline cross-server tell delivery, lookup-in-progress retry semantics, dlog/audit logging, and exact chat-service transport remain. |
| `src/system/libload.c`, `src/system/drvlib.h`, module driver switch mapping | `DRIVER_PORTING_PLAN.md` | Static Rust driver registry architecture and prioritized character/item driver port order documented. Implementation remains. |
| `src/module/base.c` `ch_driver` / `ch_died_driver` / `ch_respawn_driver` dispatch boundary, `src/system/libload.h` `CDT_*`, `src/system/drvlib.h` base `CDR_*` IDs, `src/module/base.c` `trader_driver` / `janitor_driver` | `crates/ugaris-core/src/character_driver.rs`, `crates/ugaris-core/src/world/trader.rs`, `crates/ugaris-core/src/world/janitor.rs`, `crates/ugaris-core/src/drvlib.rs`, `crates/ugaris-server/src/world_events.rs` | Legacy dispatch type constants, base `CDR_MACRO`/`CDR_TRADER`/`CDR_JANITOR` IDs, typed character-driver call/outcome shapes, C-compatible handled/unsupported return-code behavior for tick/death/respawn dispatch scaffolded. `CDR_TRADER` (the player-to-player trade middleman NPC) is now fully ported: `World::process_trader_actions` handles the "trade with <name>"/"stop trade"/"accept trade"/"show trade" text-command state machine with exact C string matching, `NT_GIVE` item collection capped at 10 per side with cross-partner notification, the three-minute timeout with item return, deal-swap semantics, a periodic-scan greeting (like `world/bank.rs`/`world/merchant.rs`, but also turning to face the greeted player via the new `drvlib::offset2dx`/`turn` combo since C's `talkdir` mechanic is part of this driver's observable behavior), the 12-line idle-murmur table, and the 12h driver-memory clear timer; a new `TraderEvent`/`pending_trader_events` queue (mirroring `BankEvent`) defers the "show trade" item dump and `NT_GIVE` cross-notify to `crates/ugaris-server/src/world_events.rs::apply_trader_events`, which needs `legacy_item_look_text` (lives in `ugaris-server`, not `ugaris-core`). `CDR_JANITOR` (the lamp-lighting/item-tidying NPC) is now also fully ported: `World::process_janitor_actions` toggles the nearest `IDR_TOYLIGHT` whose on/off state doesn't match the current day/night target, takes the nearest visible `IF_TAKE` junk item on the janitor's town half (the C `NT_ITEM` handler's `y == 192` divide filter) not already on one of the nine fixed home-area tiles, stashes held junk in the `item[30..INVENTORYSIZE]` deep-inventory range (C's own comment on `struct char.item[]`), and drops bagged junk off one at a time at the nine home tiles in C's exact candidate order, plus the idle-murmur table (rolled only right after a successful light-toggle, including the dynamic "N lights I turned on" counter case) - all recomputed directly from `World::items` every tick (`JanitorDriverData` keeps only the `cnt` murmur counter as real persistent state) instead of porting C's `NT_ITEM`-message item-ID cache, the same class of simplification already established for the merchant/bank/trader greeting scans; built entirely on existing `setup_walk_toward`/`setup_walk_toward_use_item`/`do_take`/`do_drop`/`do_use` primitives, no new pathfinding machinery was needed. `CDR_MACRO` behavior, character message queues for drivers other than trader/janitor, and runtime invocation for the rest of the registry remain. |
| `src/area/2/area2.c` `shrine_driver`, `src/system/drvlib.c` `add_bonus_spell` shrine use sites | `crates/ugaris-core/src/item_driver.rs`, `crates/ugaris-core/src/world.rs`, `crates/ugaris-server/src/main.rs` | Area 2 zombie shrine `IDR_SHRINE` dispatch, cursor skull offering gates for bone/silver/gold skulls, legacy craving feedback, offering consumption, random item reward table for skull/torch/potion gifts, direct cursor gift placement, XP reward branches, temporary armor/weapon/HP/mana bonus spell installation for shrine rolls with C spell-slot/timer/modifier semantics, and legacy gift/experience/bonus feedback are ported with focused core/server tests. Exact legacy dlog/audit integration remains. |
| `src/area/22/lab3.c` `lab3_plant` carried yellow/brown berry use paths | `crates/ugaris-core/src/item_driver.rs`, `crates/ugaris-core/src/world.rs`, `crates/ugaris-server/src/main.rs` | `IDR_LAB3_PLANT` dispatch is ported for carried yellow berries and brown berries: yellow berries decode legacy freshness/count oxygen durations, remove existing oxygen spell items before installing the refreshed `IDR_OXYGEN` timed spell, consume only on successful install, and brown berries install the 10-second `IDR_UWTALK` timed spell with duplicate-active blocking and legacy failure feedback. Whiteberry light emission, plant growth/picking/rot timers, area log fan-out, and exact dlog/audit integration remain. |
| `src/area/18/bones.c` `bonebridge` / `boneladder` / `boneholder` / `bonewall` core paths | `crates/ugaris-core/src/item_driver.rs`, `crates/ugaris-core/src/world.rs`, `crates/ugaris-server/src/main.rs` | `IDR_BONEBRIDGE` dispatch now ports the C full-bone cursor gate, area-18 libload guard, target tile fit checks, orientation-specific temporary bridge placement, cursor removal, dirty-sector marking, 60-second cleanup scheduling, timer retry while the bridge tile is temporarily blocked, bridge ageing sprite/drdata increments, and final removal while restoring permanent movement blocking with focused core tests. `IDR_BONELADDER` now ports the area-18 libload guard and paired same-area teleports using the C `drdata[0]` offset table. `IDR_BONEHOLDER` now ports the area-18 dispatch boundary for stand rune insertion gates, rune ID decoding for `rune1`..`rune9`, owner/tick metadata storage in `drdata[8..15]`, owner-only removal guards, 120-second timer expiry clearing, activation-holder classification, and legacy blocked feedback texts with focused core/server compilation coverage. `IDR_BONEWALL` now ports the area-18 libload guard, active/timer dispatch guards, adjacent dormant wall pulse scheduling, opening sprite/drdata progression, temporary map item removal with movement/sight unblock and void/use flag toggles, blocked restore retry, and final wall restoration with focused core/world tests. Partial-bridge add/remove inventory paths, template-backed `create_item("bone")`, exact `Hu?`/bug/does-not-fit feedback, boneholder cursor destruction/recreated-rune item placement/update-holder sprite/foreground application, activation scanning of adjacent holders, and `exec_rune` PPD reward/teleport behavior remain. |
| `src/area/13/dungeon.c` dungeon item-driver boundary | `crates/ugaris-core/src/item_driver.rs`, `crates/ugaris-core/src/world.rs`, `crates/ugaris-server/src/main.rs` | `IDR_DUNGEONTELE`, `IDR_DUNGEONFAKE`, `IDR_DUNGEONKEY`, and `IDR_DUNGEONDOOR` now dispatch only under the legacy area-13 libload guard, decode C little-endian `drdata`, preserve player-only dungeon teleport behavior, consume fake/teleport source items through world item destruction, create `maze_key1`/`maze_key2` cursor keys with the decoded legacy key ID, mark first key take via `drdata[2]`, enforce dungeon-door exact key requirements through carried item IDs, enforce the legacy 20-defender catacomb gate, clear solved door key IDs, mark `drdata[12]`, and attempt the C fallback teleport destinations, with focused core tests plus workspace coverage. Clan jewel theft/state, dungeon master/fighter character drivers, server-chat raid protocol messages, solved-catacomb player/NPC notifications, and exact clan policy remain. |
| `src/system/create.c` `update_char`/`armor_skill_req`/`armor_skill_bonus` | `crates/ugaris-core/src/world/character_values.rs` | `World::update_character(cn)`/`recompute_character_values` ports the full `value[0]` recompute: worn/spell item modifier sum with the seyan (72.5%) vs. single-class (50%) cap and non-warrior bless-item cap, `IF_BEYONDMAXMOD` uncapped bypass, skill-table base-attribute averaging (`skill[]` from `skill.c:27` hardcoded as `skill_base_attributes`), the `value[1]==0` skip for unraised skills, Cold/Demon special cases, Speed Skill/Athlete/Thief/Demon-profession bonuses, Body Control armor/weapon bonuses (with the bare-handed player weapon bonus) vs. the spell-average Armor bonus when Body Control is unraised, `armor_skill_bonus`'s body/head/legs/arms weighted requirement-vs-raised comparison, day/night/clan attribute profession bonuses, and the HP/endurance/mana current-value clamp to the new max. Wired into worn-slot equip/unequip (`crates/ugaris-server/src/inventory.rs::inventory_swap_slot`, `pos < 12` only, matching C `do.c:1294`). 11 focused tests in `crates/ugaris-core/src/world/tests/character_values.rs`. As of iteration 28, `World::character_attached_effect_light` sums `.light` across the character's currently-attached effects (`Effect::target_character`) and `recompute_character_values` adds it into `mod[V_LIGHT]`, matching C's `mod[V_LIGHT] += ef[fn].light` loop (`create.c:1785-1797`); the only remaining documented gap is an intentional approximation of C's fixed four-slot `ch.ef[]` cap (Rust sums the four lowest-id attached effects rather than tracking real slot occupancy, which only differs from C with 5+ simultaneous character-attached effects) and the trivial `player_reset_map_cache` display-cache no-op on infravision toggle (Rust has no client-scroll-diff cache to invalidate). As of iteration 25, `World` now has a real `pub area_id: u16` field (set once from `ServerConfig::area_id` at startup in `main.rs`, since this process is one area server for its whole lifetime) and the `P_CLAN` bonus checks `self.area_id == 13 || tile.flags.contains(MapFlags::CLAN)`, matching C `create.c:1856` (`areaID == 13 || (mmf & MF_CLAN)`) exactly - the catacombs special case is no longer a gap. As of iteration 21, sprite reselection (demon suits, weapon-in-hand offsets) *is* ported as `recompute_character_sprite`, called by `World::update_character` right after the value recompute and marking the character's tile dirty (`mark_dirty_sector`) on an actual sprite change, matching C's `set_sector` call; `reset_name(cn)` (colored-name cache invalidation) remains an intentional no-op since Rust has no such cache. As of iteration 17/18 it is also wired into spell install/expiry (`world/spells.rs`), skill raising (`World::raise_skill`, stat-scroll `apply_item_driver_outcome`), player-death respawn (`World::die_character`), and login (`ugaris-server/src/snapshots.rs` + `main.rs`) - see the "Ralph Loop - `update_char` Stat Recomputation" sections below for the exact call-site history; this row's prose above predates that wiring and is kept for the original algorithm description. |
| `src/system/tool.c` `exp2level`/`level2exp`/`level_value`/`check_levelup` | `crates/ugaris-core/src/world/exp.rs` | `exp2level`/`level2exp`/`level_value` (the `pow(level,4)`/`sqrt(sqrt(exp))` formulas) are now the single canonical copy, replacing three independent duplicates that had accreted in `ugaris-server/src/spawns.rs`, `ugaris-server/src/area_apply.rs`, and `ugaris-core/src/item_driver/helpers.rs` (the latter now delegates to this module; the two server-crate copies were deleted and all call sites repointed). `World::check_levelup(character_id)` ports the level-increment loop over `max(exp, exp_used)`, the "Thou gained a level!" text, save grant/reset (hardcore resets to 0, others +1 capped at 10) with feedback text, the level-20 profession unlock (`value[1][V_PROFESSION] = 1`, guarded on it not already being set), and the `set_sector` dirty-map refresh. Wired into the killer-exp and `/god exp` grant paths via `ugaris-server/src/commands_admin.rs::give_exp_with_runtime_modifiers` (kept in the server crate since its `exp_modifier`/`hardcore_exp_bonus` multipliers are live-tunable `ServerRuntime` fields), gated on `!NOLEVEL` exactly like C. 13 focused tests (`world/tests/exp.rs` + 2 server-crate assertions in `tests/commands_admin.rs`). `World::give_exp(character_id, base_exp, area_id)` (C `give_exp` `tool.c:1371-1423`) is now the single canonical grant entry point in `ugaris-core`, applying the hardcore/global exp multipliers, `CF_NOEXP`/area-21 gate, `CF_NOLEVEL` exp-band clamp, decrease-prevention guard, and `check_levelup` tail call; as of iteration 24 every known exp-grant call site in the tree (killer exp, `/god exp`, `/milexp`, lollipop, demonshrine, the four random/zombie shrines, the warp-bonus reward-sphere/step-trickle grants, bookcase library-solved, staffer animation book, and the stat-scroll driver's `check_levelup`/`update_character` wiring) routes through `give_exp`/`check_levelup` instead of a raw `character.exp` mutation - `scrolls.rs::raise_value_exp` intentionally stays a raw `+=` since C's own `raise_value_exp` (`skill.c:353-354`) does too (it calls `check_levelup` directly, not `give_exp`). 13+ focused tests in `world/tests/exp.rs` plus per-call-site tests across `tests/commands_admin.rs`, `tests/area_apply.rs`, `item_driver/tests/*`. As of iteration 26, the level-10-multiple "Grats" broadcast (C `server_chat(6, ...)`, `tool.c:1347-1350`) is also ported: a new `World::queue_channel_broadcast`/`drain_pending_channel_broadcasts` (`world/text.rs`, `WorldChannelBroadcast { channel, message_bytes }`) queues the exact C byte sequence (`"0000000000"` + `COL_MAUVE` + text), and `ugaris-server`'s new `send_pending_world_channel_broadcasts` (`world_events.rs`, wired into the tick loop in `main.rs`) drains it each tick and fans it out to every session with that chat channel joined, reusing the same join-bit rule `apply_chat_command` uses for player channel messages. 3 new focused tests in `world/tests/exp.rs`. Remaining documented gaps (not silently dropped): `achievement_check_level` has no Rust equivalent (needs a general achievement engine), and `reset_name(cn)` is an intentional no-op (no server-side colored-name cache exists to invalidate). As of iteration 27, the P1 "Experience/level-up side effects" task is closed (`- [x]`): a full workspace re-audit found zero remaining raw `character.exp` grant mutations outside `give_exp`/`check_levelup` (the three raw-`exp` writers that remain - `raise_value_exp`'s bare `+=`, the potion/death exp-loss `saturating_sub`s, and `/setlevel`'s debug override - all correctly bypass `give_exp` because C does too). `achievement_check_level` is tracked separately under the P4 "Achievements" task; `reset_name` stays a documented no-op. |

## Continuation Handoff

Use this section as the starting point for the next session.

### Current Runnable State

- The Rust workspace lives at the repository root (`Ugaris_Server_Rust`); the legacy C oracle lives in the sibling `Ugaris_Server` repository.
- The three former monolithic files (`world.rs`, `item_driver.rs`, `main.rs`) are split into module directories; see `Rust Module Architecture` at the top of this ledger before adding code.
- The Rust server loads real area 1 data from `ugaris_data/zones/1/above1.map` plus generic/area templates via the `ugaris_data` symlink.
- Latest real area 1 smoke counts: 65,533 ground tiles, 16,969 blocked tiles, 1,780 item templates, 188 character templates, 2,236 placed items, and 446 placed characters.
- Default login scaffold uses `generic/player.chr` template `new_warrior_m`, then renames the character to the login name and sends starter equipment/inventory to the client.
- NPC deaths now run the full kill loop: death animation, lootable body containers, decay timers, kill experience, and template respawns.
- `CDR_MERCHANT` NPCs create stores from carried stock, greet players, and trade through the legacy `con_type 2` store view after `"<name>, trade"`.

### How To Run

Build and run the Rust area server from the repository root:

```bash
cargo build -p ugaris-server
target/debug/ugaris-server --bind-addr 0.0.0.0:5556
```

Run the legacy client from the client repo:

```bash
cd /home/eddow/Development/UgarisProjects/astonia_community_client
bin/moac -u Godmode -p test123 -d localhost -t 5556 -o 3141 -c 8000 -k 60 -m 8 -n 0
```

Expected server startup log markers:

```text
legacy TCP listener ready
loaded area zone map ... placed_items=2236 placed_characters=446 ...
initialized scaffold player character id allocator next_character_id=447
entering Rust game loop
legacy login block parsed
login accepted by compatibility scaffold ... payload_count=5
```

### Verification Commands

Run from the repository root:

```bash
cargo fmt --all
cargo test --workspace
cargo build -p ugaris-server
```

Last verified after the playtest fix session (spell queue, one-frame-per-tick
pacing, LoS blacking, door pathing):

- `cargo test --workspace`: passed (1,367 tests).
- `cargo build -p ugaris-server`: passed with zero warnings.
- Process-level area 1 boot smoke: passed with all legacy startup markers.

Continue from `PORTING_TODO.md` (prioritized task list); this ledger stays
the record of completed work.

### Recent Client-Facing Fixes

- NPC duplication was mostly caused by `.map` `from/to` range copies duplicating live `item`/`character` tile IDs. `ZoneLoader` now sanitizes range copies so terrain is copied without live object IDs or temporary object blockers.
- Movement stutter was caused by sending a full visible diamond after every completed walk. One-tile walks now send `SV_SCROLL_*`, `SV_ORIGIN`, old-position character clear, center character update, and newly visible fringe tile/character packets.
- Full visible-diamond refresh is still used for non-walk actions until a proper cached map-diff system is ported.
- `PAC_LOOK_MAP` no longer drops pending output in the server loop. It now sends legacy `SV_TEXT` feedback for hidden targets (`Too far away or hidden.`), area 1 `show_section`-style section names and level-difficulty text, coordinate fallback for areas without ported section tables, and the exact rest/clan/arena/peace zone flag messages.
- The runtime text command loop now handles legacy `/sort`, sorting only inventory slots 30+ with the C ordering (empty slots last, value descending, sprite descending, then first-35-byte name ascending) and refreshing inventory after the command.

### Chest Driver State

Current implemented slices:

- `IDR_CHEST = 5` dispatches in `crates/ugaris-core/src/item_driver.rs`.
- The core driver decodes `drdata[0]` as `treasure_index` and returns `ItemDriverOutcome::ChestTreasure`.
- The server applies `ChestTreasure` by using the retained `ZoneLoader` to instantiate `treasure_<index>` and place it on the character cursor.
- The existing post-action inventory snapshot sends the new cursor item to the client.
- Cursor-occupied chests are blocked.
- Keyed chests check exact legacy item `ID` and skeleton key `ID` against cursor and inventory slots 30+.
- Runtime keyring state stores up to 100 legacy key IDs/names with duplicate/full checks and keyed chests consult it for exact key matches.
- Successful keyed chests send the legacy key-use feedback before the loot feedback.
- Character runtime/snapshot shape now includes a serde-defaulted `deaths` counter.
- Normal chests with `drdata[7]` require at least that many character deaths and otherwise report as empty.
- `PlayerRuntime` now tracks chest-open achievement progress and markers for looter thresholds plus gold-looter chest `63` on successful loot only.
- `PlayerRuntime` now keeps runtime treasure-chest last-access seconds keyed by treasure index.
- Normal chest cooldowns decode legacy `drdata[5..7]` as little-endian hours and use tick-derived realtime seconds.
- Chest feedback sends legacy `SV_TEXT` messages for empty chest, cursor occupied, key required, and successful loot.
- `IDR_RANDCHEST = 34` dispatches to a typed runtime outcome.
- Random chests keep a runtime 100-entry per-player location table and enforce the legacy 24-hour cooldown.
- Random chests now handle cursor-occupied and empty feedback, no-tier 1-in-4 money chance, money item creation, tier potion-template attempts for rolls 21-27, money fallback, cursor placement, and chest achievement increments on successful loot.
- `IDR_KEY_RING = 200` now dispatches to typed show/add-cursor outcomes.
- `PlayerRuntime` keyring entries now store legacy key recreation metadata: ID, name, description, sprite, flags, value, driver, first 16 drdata bytes, and expire serial.
- Runtime keyring add keeps duplicate/full semantics, 100-key cap, and the auto-add setting shape used by legacy `DRD_KEYRING_PPD`.
- Keyring item use now shows legacy-shaped keyring contents through `SV_TEXT`, adds the cursor key into runtime keyring state, consumes the cursor key item, and reports duplicate/full/add feedback with tests.
- Keyring item use and `#keyring addall` now gate additions through the legacy registered-key item-ID list from `src/module/keyring/key_registry.c`, reject non-registered key-like items with legacy feedback, and keep rejected cursor items unconsumed.
- Keyed doors now use carried exact-key/skeleton-key plus runtime keyring lookup and emit the legacy keyring door feedback text.
- `#keyring`/`/keyring` text commands now require a keyring on the cursor, show contents, remove runtime entries, add registered inventory-slot 30+ key candidates to the runtime keyring, toggle auto-add, emit legacy-shaped `SV_TEXT` feedback, and refresh inventory after `addall`, with focused tests.
- `#keyring remove <n>` now recreates the removed key before deleting the runtime keyring entry: it first tries a loaded template by legacy item ID, then the placeholder-key template, then stored key metadata, gives the key to inventory/cursor, preserves the keyring entry on full inventory, and refreshes inventory on success.
- Keyring auto-add now runs after successful `TAKE` action completion for registered keys when enabled, stores full key metadata, consumes the cursor key on success, leaves duplicate/full keys on the cursor/inventory path, and emits legacy-shaped feedback text with focused tests.
- Admin `#keyring addallkeys`/`/keyring addallkeys` now requires `CF_GOD` or `CF_STAFF`, instantiates loaded templates from the registered legacy key-ID list, stores matching key metadata up to the 100-key cap, and emits the legacy-shaped start/summary feedback with focused tests.
- `PlayerRuntime` now has a fixed-layout C-compatible `DRD_KEYRING_PPD` byte codec matching `src/module/keyring/keyring.h`: 100 key slots, 40-byte names, 80-byte descriptions, 16-byte drdata, C struct padding/alignment, one-byte expire serials, and `auto_add`, with layout/truncation/round-trip tests.
- `PlayerRuntime` now decodes and encodes the legacy outer persistent-player-data blob framing for `DRD_KEYRING_PPD`, preserves unknown PPD blocks, skips `DRD_JUNK_PPD`, rejects malformed blobs, and verifies the C `MAKE_DRD` IDs for keyring persistence with tests.
- `PlayerRuntime` now has a fixed-layout C-compatible `DRD_TREASURE_CHEST_PPD` byte codec matching `struct treasure_chest_ppd { int last_access[200]; }`, decodes/encodes it through the same outer PPD blob framing, replaces existing blocks, appends when runtime chest cooldowns exist, and preserves unknown PPD blocks with tests.
- The server now retains DB snapshot `ppd_blob`/subscriber blobs in `PlayerRuntime`, decodes loaded keyring and treasure-chest PPD blocks on optional PostgreSQL-backed login, and re-encodes those blocks into the logout snapshot save request together with carried inventory/cursor items.
- `PlayerRuntime` now has a fixed-layout C-compatible `DRD_RANDCHEST_PPD` byte codec matching `struct randchest_ppd { int ID[100]; int last_used[100]; }`, decodes/encodes it through the same outer PPD blob framing, replaces existing blocks, appends when runtime random chest access exists, preserves unknown PPD blocks, and keeps the legacy random-chest location ID shape `x + (y << 8) + (areaID << 16)` covered by focused tests.
- `IDR_CITY_RECALL = 159` now dispatches from the base item-driver path, maps legacy scroll types 0..12 to fixed city coordinates from `src/module/base.c`, blocks Teufelheim arena use, preserves carried/dying no-op behavior, decrements `drdata[1]` stack counts or consumes the final scroll, teleports same-area destinations, and returns typed cross-area handoff outcomes with focused tests.
- `IDR_ASSEMBLE = 29` now dispatches from the base item-driver path, ports the legacy sun amulet and staff blue/green/red key combination matrix from `src/module/base.c`, emits the legacy no-cursor/does-not-fit/bug feedback messages, instantiates the combined item template, consumes the cursor component, and replaces the used carried item with focused core and server tests.
- `IDR_DOUBLE_DOOR = 187` now dispatches from the base item-driver path, returns a typed double-door outcome, toggles the used door through the existing door state machinery, synchronizes adjacent north/south/east/west doors whose open-state differs, and `IDR_DOOR` now shifts neighboring foreground sprites for legacy extended doors marked by `drdata[7]`, with focused core/world tests. `IDR_DOOR` timer callbacks now also mirror C `cn == 0` handling: `drdata[39]` outstanding timer counters, no-auto-close `drdata[5]`, 10-second auto-close scheduling after opens, five-second blocked-doorway retry scheduling, and delayed nonzero-character item calls distinct from zero-character timer callbacks are covered by focused world tests.
- `IDR_NIGHTLIGHT = 11`, `IDR_TORCH = 12`, and `IDR_TOYLIGHT = 117` now dispatch from the base item-driver path for core state transitions: toy lights toggle `drdata[0]`/sprite/light modifier, nightlights respond to timer/daylight threshold context and request 30-second rescheduling, torches light/extinguish carried items, set `IF_NODECAY`, use the legacy light fade formula, mark modified torches with `min_level = 200`, handle underwater block/extinguish context, send the legacy underwater-lighting failure feedback, request 30-second timer rescheduling, destroy/remove expired torches from carried inventory or map storage through world timer callbacks, extract non-light positive modifiers from carried modified torches into `empty_orb`-based legacy-shaped orbs before light toggling and decrement the torch modifier only after successful give, and send the legacy timer feedback text for underwater hissing and expiration with focused core/server tests. Zone startup now schedules existing nightlights and already-burning torches with legacy `cn=0` timer callback handling. Item-light map invalidation, character stat recomputation, and full no-space drop/destroy feedback parity for torch-extracted orbs remain.
- `IDR_FOOD = 64` now ports C `food_driver` special-food branches for lollipops and Christmas pops: lollipops keep the carried item, increment `drdata[1]` and sprite up to eight licks, grant `max(5, level_value(level) / 750)` XP using the legacy fourth-power level value, change descriptions on first/final lick, emit the legacy nearby `log_area` lollipop text, and send the final memory message once only the stick remains; Christmas pops now return a typed inspected outcome and emit the four legacy inscription lines instead of unsupported. Focused core/server tests cover consumption for kinds 0/1, special-food state transitions for kinds 2/3, and runtime flavor text.
- `IDR_XMASMAKER = 143` runtime application now matches C `xmasmaker`: staff/god use silently creates an `xmaspop` through smart item placement without the extra Rust-only `You received ...` system feedback. Focused server coverage verifies the grant path.
- `IDR_XMASTREE = 142` gift creation now ports the C `xmastree` enhancement slice: generated holiday gifts clear existing modifiers, receive one to three unique random skill modifiers from the legacy valid skill pool with weighted 0..20 values, optionally receive the legacy extra immunity modifier when room remains, and get the personalized `To <name>... Merry Christmas!` description with a random legacy god name. Focused server tests cover gift description and enhancement invariants. Exact global RNG parity and item requirement recomputation remain.
- `IDR_ORBSPAWN = 84` and `IDR_ANTIORBSPAWN = 162` now dispatch from the base item-driver path, enforce empty cursor/min-level/paid-player gates, create legacy-shaped orb and anti-orb items from `empty_orb` / `empty_anti_orb` templates onto the cursor, port the C `create_orb` / `create_anti_orb` 32-value stat selection table, mark extracting anti-orbs through `drdata[2]`, emit legacy-shaped creation/cooldown/no-op feedback, track 100 per-player spawner locations with 30-day cooldowns, and encode/decode `DRD_ORBSPAWN_PPD` through the legacy outer PPD blob with focused core/server tests. Exact legacy global RNG parity, live-data smoke coverage, dlog/audit integration, and configurable respawn-day settings remain.
- `IDR_USETRAP = 6` and `IDR_STEPTRAP = 25` now dispatch from the base item-driver path for C `usetrap`/`steptrap` core scheduling behavior: use traps read `drdata[0..1]`, find the target map item, and schedule that item driver after `TICKS / 2` with the using character; step-trap timer callbacks discover the first nearby non-steptrap item in the legacy 1/3/5/7 direction scan at distance 1 then 2 and store its target coordinates; character-triggered step traps schedule the target item after one tick with `cn=0`. `IDR_BALLTRAP = 3` now dispatches for the C `balltrap` guard/decode boundary: timer calls and players no-op, while non-player triggers decode signed `drdata[0..1] - 128` offsets and `drdata[2]` power into a typed projectile outcome, and the world applies that outcome by creating a retained legacy-shaped `EF_BALL` effect with trap start/target coordinates, power, light 80, no caster, and five-second lifetime. `IDR_SPIKETRAP = 26` now ports the area 2 one-shot armed sprite/state transition, direct legacy-unit damage outcome, and one-second timer reset. `IDR_FLAMETHROW = 24` now ports the area 2 timer-only fire countdown, light/sprite modifier state transitions, one/two-tile directional target scan, one-tick active rescheduling, `drdata[3]` idle interval scheduling, C `burn_char` duplicate suppression, one-minute `EF_BURN` lifecycle, direct legacy-unit burn damage, and visible burn `SV_CEFFECT` records. `IDR_EXTINGUISH = 28` now ports the area 2/Caligar extinguish-driver behavior: character-only use, remove the active burn effect when present, and send the legacy `You extinguish the flames.` / `Ahh. Sweet and refreshing.` feedback. Focused core/world/protocol tests cover typed outcomes, delayed scheduling, timer callback dispatch, target discovery, balltrap projectile decoding/effect creation, spike damage/reset, flamethrower burn state, burn expiry, burn effect packet layout, and extinguish outcomes. Exact `hurt` armor/shield reduction/death handling and light/sector invalidation callbacks remain.
- Area 14 `IDR_TRAPDOOR = 70` now dispatches through the legacy area libload guard for C `trapdoor`: player step-on use attempts the backwards `dx2offset` teleport before opening, sets `drdata[0] = 1`, increments the sprite, applies `MF_TMOVEBLOCK`, schedules the six-second close timer, and queues the legacy step-back feedback; timer callbacks close only the open state and clear the temporary movement block; off-tile use with an `IID_AREA14_STEELBAR` cursor item blocks the trapdoor with `drdata[0] = 2`, sprite `+2`, cursor destruction, and inventory dirtying; busy/no-stick feedback is surfaced through the server runtime. Focused core/world tests cover dispatch, step-back/open/close, timer scheduling, steelbar blocking, and feedback text. Area-14 `IDR_JUNKPILE = 71` now ports C `junkpile` cursor blocking, steelbar/money/nothing roll table, cursor grant, pile destruction, and legacy found feedback. Area-14 `IDR_GASTRAP = 72` now ports C `gastrap` trigger/timer guards, nine-step animation scheduling, nearby foreground-sprite animation, and legacy-unit player damage through the world hurt bridge. Area-14 random-shrine used-state now has C-compatible `DRD_RANDOMSHRINE_PPD` decode/encode and questlog sync coverage; `shrine_security` applies saves/hardcore/secure-already gates, marks the shrine bit only on success, and emits the legacy save-count text; `shrine_jobless` clears profession selections, sets `CF_PROF|CF_UPDATE`, marks the shrine bit only on success, and emits the legacy jobless/already-jobless feedback; `shrine_edge` ports the saves/noexp gates, C level-value XP formula, save clearing, PPD mark, and feedback; `shrine_kindness` clears `CF_PK` only when needed and marks PPD on success; `shrine_death` clears saves, marks PPD, emits the legacy laugh, and routes the lethal effect through the existing Rust legacy-hurt death bridge; and `shrine_braveness` requires prior death-shrine use, grants C level-value XP plus gold, marks item/update flags, and marks PPD on success, with focused server tests. `shrine_vitality` raises HP (warrior)/Mana (non-warrior) toward the seyan-aware 100/115 cap using the legacy `raise_cost` per-point formula and marks `exp_used`; `shrine_continuity` enforces the sequential-level PPD gate, grants level-value XP, and opens the level-99 gate. As of iteration 23, `shrine_edge`/`shrine_vitality`/`shrine_braveness`/`shrine_continuity`'s exp/`update_char` grants route through `World::give_exp`/`World::update_character` from their `main.rs` call sites instead of a raw `character.exp +=` (matching C's `give_exp(cn, ...)`/`update_char(cn)` calls exactly, including the hardcore/exp_modifier multipliers and `CF_NOEXP`/`CF_NOLEVEL` gates the old raw mutation silently skipped). Remaining area-14 random-module gaps are the indecisiveness/bribes/welding random-shrine effect families and exact audit/log side effects.
- Area 15 `IDR_SWAMPWHISP = 74` now dispatches through the legacy area libload guard for C `swampwhisp`: zero-character timer-only execution, initial origin/direction `drdata[1..3]` setup, daylight dark-state transition with light modifier removal/restoration, frame animation from sprites `20934 + drdata[0]`, cardinal movement checkpoints with map placement success/failure branches, circle-left/right random-turn context hooks, two-tick active rescheduling, one-second daylight sleep rescheduling, dirty-sector/map-slot updates, and item-light refresh are covered by focused core/world tests. Area 15 Clara/NPC dialogue, swamp quest PPD, and exact global RNG parity remain.
- Area 15 Clara quest support now exposes C-compatible `area3_ppd` `kelly_state` and `clara_state` accessors at the legacy struct offsets, plus the transient `DRD_CLARADRIVER`/`clara_driver_data` state shape (`last_talk`, `current_victim`) in the Rust character-driver state enum. The C `clara_driver` state-machine core is now represented by pure typed helpers for the Kelly-gated report dialogue, hardkill quest opening/progress/completion text, legacy military-point/EXP reward markers, text-analysis replay resets, and hardkill swamp-monster death transition, with focused tests. Runtime NPC message-loop integration, actual questlog/military reward application, item give-back, rank derivation from military points, and secure movement remain.
- Area 25 `IDR_WARPTELEPORT` now preserves the legacy invalid plain-portal `drdata[1]` feedback path from `warped.c`: it returns a typed bug outcome and the server emits `You found BUG #31as5.` followed by `Target is busy, please try again soon.` instead of treating the item as generic unsupported. Focused core coverage verifies the outcome and server build coverage verifies runtime handling.
- `IDR_ACCOUNT_DEPOT = 148` now has a first-class account-depot open path in `use_item`: it runs before generic container/depot handling like C `do.c`, sets `current_container`, returns `OpenAccountDepot`, and the server runtime treats completed use actions as successful opens instead of permanently passing `account_depot_available=false`. The legacy non-use `ItemDriverRequest::AccountDepot` boundary now returns `AccountDepotOpened` instead of `Unsupported`, with focused core tests. The server runtime now keeps per-character account-depot snapshot slots while open, sends legacy `SV_CONTYPE`/`SV_CONNAME`/`SV_CONCNT`/`SV_CONTAINER` account-depot views, handles parsed `CL_CONTAINER`, `CL_CONTAINER_FAST`, and `CL_LOOK_CONTAINER` commands for account-depot swap/fast-store/look behavior, blocks `IF_QUEST`/`IF_NODEPOT` cursor storage with legacy feedback, and supports `/accountdepotsort` ordering by sprite/value/name with focused server tests. Ordinary item containers with `content_id` now send legacy-shaped container views using the opened item's description/name, support `CL_CONTAINER` cursor/container swaps, `CL_CONTAINER_FAST` first-free-inventory storage for withdrawn items, `CL_LOOK_CONTAINER` text feedback using a C-shaped `look_item` formatter for name/description, holy weapon level, modifiers, requirements, level/class gates, bonding flags, quest/no-enhance/beyond-max flags, flask/beyond-potion/decay durations, gilded/silvered sprite ranges, and demon-suit notes, quest-cursor storage blocking, and explicit empty-slot clears with focused server tests. The runtime now mirrors C `check_container_item` for open container validity on command/refresh paths by clearing `current_container` when the player is busy, the opened item is gone/no longer usable, or a map container is no longer the item the character is facing. `DRD_ACCOUNT_WIDE_DEPOT` raw item-blob compatibility, loading/saving depot snapshots across logout/restart, immediate DB flush behavior, exact color/ITEMDESC marker packet parity for `look_item`, keyring hover contents, persisted generic container slot/order parity, depot PPD handling, and merchant command handling remain.
- `IDR_INFINITE_CHEST = 93` key handling now matches C `infinite_chest`: configured key IDs are checked only against carried inventory slots 30+, keyring entries are ignored, and skeleton keys no longer satisfy the exact `drdata[1..4]` key requirement. Focused core/server tests cover exact-key success, keyring ignoring, and skeleton-key rejection.
- `/accountdepotsort` now matches the C command gate from `src/system/command.c`: it only sorts when the account depot item is the currently open container, not merely when a per-character account-depot runtime snapshot exists. Focused server tests cover the stale-state rejection and valid open-depot sort path.
- `PAC_KILL` now participates in the primitive player action bridge: adjacent targets set up timed `AC_ATTACK1`, distant targets path toward attack range, `do_attack` validates legacy 3x3 target reachability around the faced tile plus dead/peace/no-attack/playerlike guards, and timed `AC_ATTACK1..3` completion applies deterministic hit/miss and direct HP/lifeshield damage through the ported attack formulas. Focused core/world tests cover setup, walking toward targets, strict hit roll behavior, and HP damage. Exact C RNG parity, full `sub_attack` side effects, death/hurt/notify/rage/surround integration, clan/hate/group attack policy, and fightback state remain.
- Player fightback state now ports the C `player_driver` `NT_GOTHIT` slice: nearby attackers (`char_dist < 3`) trigger immediate `PAC_KILL` while idle after the three-second no-fight gate, busy actions store `next_fightback_*`, deferred fightback promotes within the legacy one-second window, `driver_stop`/`driver_halt` continue clearing fightback state, and hurt-event processing wires this into the server runtime before PK-hate accounting with focused core/server tests. Autoturn optimization, target serial validation during `PAC_KILL` setup, and broader combat side effects remain.
- Area 30 `IDR_CLANSPAWN` now ports the legacy clan jewel spawner core path: area-30 libload guard, timer initialization from `drdata[0..7]`, 48-hour default frequency, 30-minute spawn-time rounding, 60-second timer rescheduling, configurable max-jewel context with legacy `<= max_jewel_count` spawn condition, empty/non-empty sprite transitions, level cap checks, adjacent-player contest blocking, god force-add countdown behavior, award-time jewel decrement, runtime `clan_jewel` template creation through inventory placement, and initial jewel expiry timer scheduling are covered by focused core/server verification. Exact legacy RNG parity, clan server-chat/jewel accounting, dlog/audit integration, live configurable `max_jewel_count`, and startup timer naming cleanup remain.
- Area 30 `IDR_CLANVAULT` now matches the legacy clanmaster module dispatch boundary: it is area-30 guarded like libload, returns a handled no-op instead of falling through to unsupported, and preserves the C `it_driver` return code of `1` with focused core tests.
- `src/system/saltmine.c` `IDR_SALTMINE_ITEM` now dispatches from the Rust item-driver registry for the first saltmine item slice: `drdata[0] == 3` door use returns a typed blocked outcome and the server emits the exact legacy non-worker feedback `Thou canst not enter there.` while preserving handled return-code behavior with focused core tests. Runtime ladder cooldown and pending-salt state now decodes/encodes the C-compatible fixed-layout `DRD_SALTMINE_PPD` block (`version/useitemflag/quitflag/gatamastate`, 20 ladder timestamps, pending salt) through the shared outer PPD blob while preserving unknown blocks, with focused player-runtime tests. Ladder startup numbering, saltbag coordinates/rewards, monk-worker removal/use state, and character-driver integration remain.
- Client-visible map updates now keep a per-session visible-diamond cache in the server runtime. Login/full refresh/one-step scroll paths update the cache, and same-origin non-walk completed actions compare cached tile/character packets against the current world to send only changed `SV_MAP11`/`SV_MAP10` cells, including character-clear packets, instead of always replaying the full visible diamond. Visible character identity now sends legacy-shaped `SV_NAME` packets during bootstrap/full refresh, newly visible walk fringe updates, newly visible same-origin diffs, and renamed/title-changed visible characters using a per-session known-name cache. Client-effect slot packets (`SV_CEFFECT`/`SV_UEFFECT`) are now included in login bootstrap and immediate post-action refreshes as well as periodic frames, so visible projectile/spell effects do not wait for the next ticker cycle. Focused server tests cover changed tile diffs, removed-character clears, bootstrap names, diff names, walk-fringe names, and login bootstrap effect slot seeding. Remaining cache parity gaps are exact character color/clan/PK relation fields in `SV_NAME`, exact light/dark/LOS behavior, sector skip optimization, and using cache-driven inventory deltas instead of the coarse post-action inventory snapshot.
- `PAC_LOOK_MAP` section-name/level output now uses the legacy `src/system/area.c` `section[]` and non-empty `area_sector[]` coverage for areas 1-26, 29, 31-34, and 36, keeping coordinate fallback for empty/unknown area tables. Focused core tests cover representative non-area-1 look output.
- Successful walk completions now reuse the same area-section tables for C `walk_section_msg` parity: each `PlayerRuntime` tracks the current section id, sends dark-gray legacy `SV_TEXT` for `Now entering <section>.` and `Now leaving <section>.` only when the section changes, suppresses duplicate same-section messages, and sends the legacy `SV_SPECIAL` looping music packet from `play_music_by_section` on section entry. Focused protocol/server tests cover `player_special` packet layout, entry text/music, duplicate suppression, leaving text without music, and the section music switch.
- Randomized ambient `area_sound` special effects now use the ported section tables and C roll mappings for wet dungeon, dry dungeon, woods, park, and underwater sections, and successful action completions can append the corresponding `SV_SPECIAL` packet for the active player. The reusable `sound_area` primitive now returns per-player `SV_SPECIAL` targets for the legacy 16-tile square, squared-distance attenuation, horizontal pan, and `LOG_TALK` sector-hearing gate. Exact ambient driver-call cadence/RNG parity and wiring every combat/effect/driver sound call site remain.
- Legacy item-driver identity tags (`nr >= 1000` in `src/system/libload.c`) now return a typed handled no-op outcome instead of falling through to unsupported, preserving the C registry-edge return code while keeping the driver ID visible for diagnostics. Focused core coverage verifies direct dispatch and return-code parity.
- `IDR_FLASK` finished-potion use now follows the C `mixer_use` application path instead of the Rust-only unported feedback: shaken alchemy flasks with an empty cursor check item requirements, install an `IDR_POTION_SP` timed spell with the flask modifiers and `drdata[3]` duration, schedule spell removal, emit the potion visual effect, and then reset the same carried flask to the legacy empty-bottle state instead of consuming it. The path shares the legacy failed-requirements/active-potion feedback behavior with beyond potions. Focused core/world tests cover the dispatch boundary, runtime spell installation, and empty-flask reset. Alchemy `mixer_power`/`mixer_duration`/`mixer_mix` formula coverage now includes C-compatible unused modifier slots (`-1`) for one- and two-modifier recipes, preventing accidental `V_HP` modifiers in empty slots. Exact live alchemy data smoke coverage and alchemy achievement/audit integration remain.
- `src/system/drvlib.h` spell identity macros now have Rust equivalents: `IDR_ISSPELL`, `IDR_DONTSAVE`, and `IDR_ONECARRY` are represented by tested predicate helpers, including the legacy non-save item-driver constants for melting keys, back-to-fire markers, palace bombs/caps, and clan jewels.
- `IDR_CALIGARFLAME = 145` now dispatches through the existing typed flamethrower timer path, matching C `caligar_flamethrow` countdown/light/direction/reschedule behavior, and startup light-timer registration includes existing Caligar flame map items. Focused core/world tests cover the legacy ID, timer-only no-op guard, pulse mutation, and startup scheduling. The broader `IDR_CALIGAR` quest/training/weight/gun/key/skelly-door item multiplexer remains unported.
- `IDR_CALIGAR` subtype `11` now ports the C `caligar_extinguish` item path by dispatching through the existing typed extinguish outcome: zero-character timer calls no-op, character use removes an active burn effect through the world applier when present, and the server reuses the legacy extinguish/refreshing feedback text. `IDR_CALIGAR` subtype `12` now ports the C `caligar_skelly_door` core path: non-character calls no-op, the door index is decoded from `drdata[1]`, invalid diagonal/off-map/busy teleports return the legacy busy outcome, successful use teleports the character to the opposite side of the door, flips cardinal facing, and clears active action timing. Focused core/world coverage verifies both subtype dispatch and world mutation. Remaining Caligar item gaps are exact Caligar PPD quest state, logging/dlog parity, and live area-data smoke coverage.
- `IDR_FREAKDOOR = 58` now dispatches from the legacy ice/palace shared item-driver path, decodes C `drdata[8..15]` link metadata, resolves and caches paired freakdoor item IDs in `drdata[10..13]`, delegates off-tile use to the normal door toggler, opens the paired door when the source opens, teleports a character standing on the freakdoor to the paired door, preserves the current movement delta for fake-move continuation, and keeps the C recursion guard byte around partner teleports. Focused core/world tests cover dispatch metadata, partner lookup/cache, teleport, target door opening, and movement continuation. Exact player-driver fake-walk action scheduling beyond target coordinate preservation and live ice/palace data smoke coverage remain.
- `IDR_FLASK` Teufelheim arena blocking now matches C `flask_driver`: finished/unfinished alchemy flask use is blocked only when `areaID == 34` and the current tile is marked arena, instead of treating every arena tile in every area as a no-potion zone. Focused core coverage verifies non-Teufelheim arena use still reaches the normal flask path while area 34 arena use returns the legacy blocked outcome.
- `CL_GETQUESTLOG` now returns a legacy-shaped `SV_QUESTLOG` packet built from `PlayerRuntime.quest_log`: 100 one-byte C bitfield entries (`done:6`, `flags:2`) followed by the C-compatible `struct shrine_ppd` random-shrine used bitset from `DRD_RANDOMSHRINE_PPD`, padded to the legacy 36-byte packet section. `CL_REOPENQUEST` now routes through the server command loop, applies the C repeatable/done-count/done-state gates, emits the legacy blocked feedback text, reopens eligible quests by restoring `QF_OPEN` and clearing `QF_DONE`, and sends an updated `SV_QUESTLOG` on success. Protocol/server/core tests cover packet sizing, padding, bitfield packing, random-shrine bit propagation, and reopen gate behavior. Questlog initialization from area quest state, area-specific PPD reset side effects during reopen, random-shrine effect execution, and mod-protocol questlog extensions remain.
- Unfinished `IDR_FLASK` shaking now ports C `mixer_power`, `mixer_duration`, and `mixer_mix` for normal alchemy flask recipes: ingredient-count recipe matching, fallback attribute mixes, duration divisors, fire/ice/hell stone power/class effects, hour/moon/solstice/equinox/alchemist profession power modifiers, C-style truncating floating division, finished-potion modifier slots, `drdata[2]` shaken flag, `drdata[3]` duration, item value, legacy pre-shake `Contains N parts <ingredient>.` feedback for successful and ruined shake attempts, and legacy `The potion seems finished.` feedback are implemented with focused core/server tests. Exact global date/profession parity now depends on the existing Rust game-time and character profession state; broader alchemy systems such as lab mixing/audit logging remain.
- Primitive queued spell execution now routes `PAC_HEAL`, `PAC_MAGICSHIELD`, `PAC_PULSE`, `PAC_BLESS`, `PAC_FREEZE`, `PAC_FLASH`, and `PAC_WARCRY` through the world action bridge. The core ports C `do_heal`/`act_heal` HP restoration, `do_magicshield`/`act_magicshield` lifeshield restoration, pulse mana/action setup, `do_bless` setup, `act_bless`-style bless item installation/replacement, legacy action IDs/durations, mana/endurance spending, fast endurance cost, target-facing behavior, C `do_flash`/`act_flash` self speed-spell installation, C `do_freeze`/`act_freeze` nearby-target timed speed-spell installation, and primitive C `do_warcry`/`act_warcry` execution: warcried blocking, `AC_WARCRY` setup, sound-sector-reachable target scan, `can_attack` guard, C warcry speed/damage formulas, timed `IDR_WARCRY` spell item installation, target HP damage, and caster lifeshield gain with focused core tests. Bless creates a carried `IDR_BLESS` item with `V_INT`/`V_WIS`/`V_AGI`/`V_STR` modifiers and little-endian expire/start/strength `drdata`; flash/freeze/warcry create carried timed speed-modifier spell items with little-endian expire/start `drdata`; poison creates carried `IDR_POISON0..3` HP-modifier spell items with little-endian expire/start/power/tick `drdata`. `src/system/tool.c` spell timer core now has exact legacy identity-driver constants for the timed spell cases, `is_timed_spell_driver`, absolute expiry scheduling for installed/existing spell items, serial/slot-guarded carried spell removal through the world timer queue, and poison callback scheduling/rescheduling. `src/system/poison.c` core `poison_someone`, periodic damage/tick weakening, `remove_poison`, and `remove_all_poison` behavior is covered by focused world tests. Projectile/effect-backed spells, visible effect creation, exact warcry pathfinder cadence/effects/notifications, freeze-removal action-speed adjustment, ice-demon curse side effects, exact `hurt` side effects/death handling for poison ticks, poison log text, notifications/sounds, and exact policy checks remain.
- Core spell slot admission now ports C `may_add_spell` and `add_same_spell` semantics for legacy spell inventory slots 12..29: duplicate active driver blocking, last-free-slot selection, and the special near-expired `IDR_BLESS` refresh allowance. Spell identity driver constants now cover `IDR_FREEZE`, `IDR_FLASH`, `IDR_WARCRY`, `IDR_CURSE`, `IDR_POISON0..3`, and `IDR_FIRERING` with focused tests. Runtime creation/timer wiring for spell items remains.
- Ice-demon freeze now ports C `ice_curse` side effects: `IDR_CURSE` uses `add_same_spell` slot semantics, creates a carried 30-minute curse spell with negative `V_INT`/`V_WIS`/`V_AGI`/`V_STR` modifiers, stacks existing curse strength up to the C max cap, schedules serial/slot-guarded spell removal, creates or strengthens the retained `EF_CURSE` visual, restores `EF_CURSE` visuals when scheduling existing timed curse spells, queues the exact target system feedback text only when the curse applies, and routes world system-text feedback through the server runtime. Focused world tests cover fresh installation, capped stacking, runtime modifier application, and the C-shaped target feedback.
- Area 26 `IDR_STAFFER = 121` now dispatches through the legacy area-26 libload guard for C `staffer_item` subtypes 1..3: subtype 1 spike traps mutate armed sprite/state, apply `drdata[2] * POWERSCALE` direct damage, and schedule the one-second reset; subtype 2 fireball machines are timer-only, decode the shifted C `drdata[1..4]` direction/power/frequency layout, create a typed fireball-machine projectile, and reschedule when configured; subtype 3 movable blocks delegate to the existing Staffer block-move world applier. Focused core tests cover the area guard, spike trigger/reset, timer-only fireball machine, and block-move dispatch. Remaining area-26 gaps are vault skull PPD/questlog state, vault shelf `vault_ritual`/`vault_journal` template rewards, and the Rouven/smuggler character dialogue drivers.
- Area 28 `IDR_BRANNINGTONFOREST = 123` now dispatches through the legacy area-28 libload guard for C `underwater_berry`: `drdata[0] == 1` requires a real player character, installs a 30-second `IDR_OXYGEN` timed spell through the existing spell slot/timer machinery, and destroys the consumed berry only when the spell is accepted. Focused core/world tests cover the area guard, player-only no-op, duration, spell installation, and item consumption. Remaining area-28 gaps include Brannington forest character dialogue/quest drivers and broader area-specific item subtypes.
- Timed spell runtime bookkeeping now applies/removes carried spell item modifiers for installed bless, curse, beyond-potion, poison, flash, freeze, and warcry items, ports C `speed2` inverse speed math, mirrors `remove_spell` `IDR_FREEZE` expiry rescaling of the current action `duration` and `step`, and refreshes C `update_char`-style driver-derived spell flags for carried `IDR_INFRARED`/`IDR_NONOMAGIC`/`IDR_OXYGEN` spell items on install, expiry, and existing-spell scheduling with focused core tests. Full central `update_char` stat recomputation remains broader work.
- `IDR_LAB3_PLANT = 193` now dispatches from the area 22 Lab 3 plant driver for carried yellow, white, and brown berries. Yellow berries use the C freshness table `{3,8,10,12,15}` seconds times berry count, replace any active oxygen spell, install a variable-duration `IDR_OXYGEN` timed spell, and consume the berry only when installation succeeds. White berries use the C light-power table `{10,30,40,45,50}` times berry count, create or refresh the carried `drdata[0] == 10` Lab 3 whiteberry light item in spell slots 12..29, apply the `V_LIGHT` modifier, schedule 20-second decay callbacks, reduce light by `3/4` per callback, destroy below 8, and refresh live character/map light state. Brown berries install a 10-second `IDR_UWTALK` timed spell through the same serial/slot-guarded timer path and preserve the legacy duplicate-active failure message. Focused item-driver/world tests cover decoding, oxygen replacement, whiteberry light creation/refresh/decay/destruction, consumption, expiry, and underwater-talk timer removal. Plant growth/picking/rot timers and area log fan-out remain.
- `CL_FIREBALL` now routes through the world action bridge for both C `do_fireball` branches: non-self map targets spend fireball mana, validate legacy map bounds, set `AC_FIREBALL1` with target coordinates, half-duration magic action timing, direction, create a retained Rust `EF_FIREBALL` effect record with C `create_fireball` initial fields (`strength`, light 200, from/to coordinates, caster identity, fixed-point start position, one-second stop tick), and then hand off to `AC_FIREBALL2`; self-targeted casts continue as C `AC_FIRERING`, gate on the one-second `IDR_FIRERING` blocker, install the carried blocker spell with legacy expire/start drdata and spell-removal timer, create the C `EF_FIRERING` caster visual with legacy light/strength ordering, create C-shaped short `EF_BURN` visuals on adjacent attackable targets, and apply ported fireball damage to adjacent attackable characters with focused core tests. Character-targeted fireballs now use `PlayerActionCode::FireballCharacter` and port C `fireball_driver` target serial guard plus moving-target ETA prediction before delegating to `do_fireball`. `World::tick_effects` now advances `EF_FIREBALL` projectiles with the C half-tile stepping/two-steps-per-tick movement, map effect-slot updates, `MF_TMOVEBLOCK`/`MF_FIRETHRU` collision rules, one-second expiry, 3x3 explosion damage guarded by LOS/can-attack checks, legacy `IID_REFLECT_FIREBALL` equipment-slot reflection that reduces or destroys stored charges and shoots a weaker fireball back at the caster, and C `CF_EDEMON` shootback that creates a weaker fireball from the earth demon to the caster before applying damage, with focused tests. Remaining gaps are full explosion/sound fan-out coverage, exact `hurt` side effects/death handling, and full notification parity.
- Area 2 `IDR_SHRINE` zombie shrines now dispatch from the Rust item-driver registry, require the exact legacy zombie skull item IDs on the cursor for shrine types 0/1/2, consume valid offerings, apply the C random item/XP reward branches, place item gifts on the cursor, grant XP, install the timed armor/weapon/HP/mana bonus spell reward items with legacy strength/duration values, and emit legacy-shaped offering/gift/experience/bonus feedback with focused tests. Exact legacy dlog/audit integration remains.
- `CL_BALL` now routes through the world action bridge for both C `do_ball` branches: non-self map targets spend flash mana, validate legacy map bounds, set `AC_BALL1` with target coordinates, half-duration magic action timing, direction, create a retained Rust `EF_BALL` effect record with C `create_ball` initial fields (`strength`, light 80, from/to coordinates, caster identity, fixed-point start position, five-second stop tick), and then hand off to `AC_BALL2`; self-targeted casts continue through the existing C `AC_FLASH` path. Character-targeted balls now use `PlayerActionCode::BallCharacter` with target serial guard before delegating to `do_ball`. `World::tick_effects` advances `EF_BALL` with the C eighth-tile stepping movement, map effect-slot updates on tile changes, `MF_TMOVEBLOCK`/`MF_FIRETHRU` collision removal, five-second expiry, nearby LOS/can-attack strike damage every fourth tick, and short-lived `EF_STRIKE` target visuals with focused core tests. Sound fan-out, earth-demon damage reduction, and exact `hurt` side effects/death handling remain.
- `IDR_SPECIAL_POTION` now preserves the C default/bug branch for unknown potion kinds: it returns a typed `SpecialPotionBug` outcome, does not consume the item, and the runtime sends the legacy `Please report bug #1734.` system feedback instead of treating the driver as generic unsupported. Focused core coverage verifies the item remains carried.
- `src/module/transport.c` `IDR_TRANSPORT` now dispatches from the item-driver registry for discovery/UI-open and destination execution paths: valid transport indices `0..25` and clan-exit marker `255` are accepted, invalid indices return the legacy `Nothing happens - BUG (%d,#1).` feedback, `PlayerRuntime` tracks and persists the C `seen` bitmask through fixed-size `DRD_TRANSPORT_PPD` outer PPD blocks, first discovery emits `You have reached a new transportation point.`, achievement threshold markers are tracked for major cities/all teleports/underground teleports, `SV_TELEPORT` 13-byte packet layout (`cmd`, little-endian `seen`, four clan-access bytes) is ported, `CL_TELEPORT` resolves regular and clan destinations, same-area travel moves the character and updates mirror state, unseen/Arches-only/invalid/busy gates emit legacy-shaped feedback, and cross-area targets return typed area handoff results with protocol/core/server tests. Remaining transport gaps are exact clan access policy beyond same-clan checks, actual cross-area server handoff, and exact global RNG parity for random mirror selection.
- Area 25 `IDR_WARPBONUS = 114` now dispatches through the Warped item-driver path under the legacy area-25 libload guard instead of falling through to unsupported: the core boundary preserves zero-character no-op behavior, default base level 40, final-level blocking above base 139, per-location used-at-current-base blocking, the C location id shape `x + (y << 8) + (areaID << 16)`, advancement threshold `base / 4`, required cursor teleport-sphere gate on advancing touches, reward sphere-kind capture from `drdata[0]`, point reset/advance classification, and reward level `min(character.level, base * 0.80)` with focused core tests. Runtime context now reads the fixed-layout `DRD_WARP_PPD` fields, mutates bonus IDs/last-used/base/points/nostepexp, applies step EXP and level-up rewards for EXP/save/military/gold/lollipop spheres, and emits the legacy feedback text. Exact military rank progression formula, dlog/audit integration, and live area-25 data smoke coverage remain.
- Area 25 `IDR_WARPTRIALDOOR = 113` runtime handling now instantiates the `warped_fighter` character template at the decoded trial-room center after a successful player teleport, initializes direction and HP/endurance/mana/lifeshield resources, inserts template inventory items, and emits the legacy `Bug #319i, sorry.` failure text if fighter creation/drop fails. Focused server coverage verifies template instantiation and placement. Full C `warpfighter` driver state (`DRD_WARPFIGHTER` target/room bounds), `warped_raise` level scaling/equipment synthesis, and exact C failure ordering before player teleport remain.
- `src/system/arena.c` `IDR_TOPLIST = 63` now dispatches from the Rust item-driver registry for player use and preserves the C zero-character no-op boundary. The reusable arena-toplist formatter ports the C output ordering: top ten entries, the rank window around the player's score, the no-fights score default of `-2000`, and the final personal win/loss line. The server runtime currently mirrors C's `!tops` behavior by producing no output until real arena rankings are loaded. Remaining arena-toplist gaps are loading/persisting the legacy ranking table, `DRD_ARENA_PPD` decode/encode, and emitting the formatted lines once ranking storage is wired.
- Client-visible transient effect exposure now sends legacy-shaped `SV_CEFFECT` records and `SV_UEFFECT` used-slot masks for the currently retained Rust `EF_FIREBALL`, `EF_BALL`, `EF_STRIKE`, `EF_BURN`, `EF_PULSE`, `EF_PULSEBACK`, and character-attached `EF_MAGICSHIELD`, `EF_FLASH`, `EF_WARCRY`, `EF_BLESS`, `EF_HEAL`, `EF_FREEZE`, `EF_POTION`, and `EF_FIRERING` records. The server keeps a 64-slot per-session effect cache, reuses effect slots by legacy effect id, only retransmits changed records, clears stale slots and the used mask when effects leave visibility, deterministically reuses freed slots for newly visible effects, gates projectile/strike/pulse records by the visible diamond, gates character-attached records by the current target character position, and now sends legacy `SV_MAP01` map-effect pointer packets for visible tile effect slots during bootstrap and same-origin diffs, including clearing removed pointers, with focused protocol/server tests. Runtime retained show-effect creation is now wired for magicshield, heal, pulse/pulseback, flash, firering, bless, freeze, warcry, beyond-potion spell installation, and restored existing timed bless/freeze/warcry/potion spell timers. Remaining effect parity gaps are retained runtime creation for other less common character-attached visual call sites, explosion/mist/earth/bubble effect families, sound fan-out, and exact `hurt` side effects/death handling.
- `IDR_PALACEKEY = 59` now dispatches from the base item-driver path for C `palace_key`: the full legacy sprite-combination table is ported, carried-only/non-character guards are preserved, no-cursor use splits already-combined parts through the `palace_key_part1` template, cursor use requires an `IID_AREA11_PALACEKEYPART`, matching parts combine symmetrically, final sprite `51014` becomes a non-use `Palace Key` with `IID_AREA11_PALACEKEY`, and cursor parts are consumed with inventory dirtying. Focused core/server-path tests cover split/combine/failure outcomes and final-key world mutation. Exact debug-number log lines/dlog parity and live area-data smoke coverage remain.
- Area 11 `IDR_PALACEDOOR = 76` now dispatches from the palace item-driver path for C `palace_door`: player use requires a carried/cursor `IID_AREA11_PALACEKEY` and otherwise emits the legacy `You need a key to open this gate.` feedback, successful use starts the opening animation through `drdata[1] = 3`, timer callbacks advance opening frames with sprite `15196 + drdata[0]`, clear `MF_TMOVEBLOCK` when fully open, wait ten seconds, set `MF_TMOVEBLOCK` before closing, and close frames back to the dormant state with focused core/workspace tests. Remaining area 11 gaps include `IDR_ISLENADOOR`, palace guard/Islena character drivers, exact sector invalidation parity beyond dirty map state, and live palace data smoke coverage.
- `IDR_LABENTRANCE = 101` now dispatches from the gatekeeper item-driver path for C `labentrance` / `teleport_next_lab`: `PlayerRuntime` decodes/encodes the legacy `DRD_LAB_PPD` solved-bit field while preserving the remaining lab PPD payload, the item-driver scans lab levels `0..63` exactly like C, skips solved/missing lab levels, enforces the per-lab minimum levels, returns the fixed area-22 labyrinth destinations for lab levels 10/15/20/25/30, and emits the legacy solved-all / too-low feedback through the server runtime. Focused core/server/workspace tests cover next-lab selection, solved-bit skipping, level gates, all-solved feedback boundary, and PPD blob round trips. Actual cross-area transfer execution, lab completion reward/exit creation, and full gatekeeper character-driver dialogue/fight behavior remain. Iteration 51: see the dedicated "Gatekeeper NPC Welcome Dialogue" section below for the character-driver-side dialogue/precondition slice that has since landed.
- `IDR_LABEXIT = 102` now dispatches from the base item-driver path for the C labyrinth exit gate core: zero-character timer callbacks animate sprite ranges `1060..` from little-endian `drdata[8]`, reschedule every two ticks, expire/destroy after the closing animation, character use requires the matching creator ID from `drdata[0..3]`, rejected users get the legacy gate-owner feedback, successful use sets the closing frame, returns the solved-lab number from `drdata[4]`, and exposes a typed cross-area exit to area 3 at `(183,199)` with focused core tests. Persisting solved-lab PPD state and actual cross-area transfer execution remain scaffolded.
- `src/system/light.c` now has reusable Rust primitives for legacy light handling: `LIGHTDIST = 20`, center tile accumulation before capped inverse-square falloff, sight-blocked propagation checks through the current conservative LOS helper, non-negative tile light clamping, character/item/effect `MF_NOLIGHT` gates, takeable item suppression on no-light tiles, lava ground tile light emission, indoor `dlight` best-outdoor-tile recomputation, and mixed indoor/outdoor reset scanning with focused tests. `World` now wraps groundlight, shadow, single-tile dlight, and mixed indoor/outdoor dlight reset recomputation so changed light/daylight tiles mark dirty sectors for cache/sector consumers. Remaining work is live world entity/effect bulk remove/add around LOS-changing map edits.
- C `compute_shadow` is now ported into `crates/ugaris-core/src/light.rs`: it scans the same 7x7 foreground sprite shadow tables, applies the left-side sightblock bonus, writes capped daylight values, preserves legacy edge bounds, and exposes `compute_shadow_with_random` so runtime code can inject the eventual global C-compatible RNG. Focused tests cover no-shadow fallback, blocker distance, foreground table/randomized divisor behavior, edge bounds, and world-level dirty-sector marking when daylight changes. Remaining light work is exact global RNG wiring and live world entity/effect bulk remove/add around LOS-changing map edits.
- `src/module/base.c` `special_potion` type 7 now ports the profession-reset potion path: it blocks when total exp is below used exp or no profession choices exist, clears profession selections, lowers `V_PROFESSION` by one third of selected profession points using the existing legacy skill-cost math, adjusts `exp`/`exp_used` like C `lower_value`, consumes the potion on success, and keeps the successful server feedback silent while preserving the legacy blocked text. Focused core tests cover successful reset and blocked no-profession use.
- `src/module/base.c`, `src/module/simple_baddy.c`, and area 11 `src/area/11/palace.c` `ch_driver`/death/respawn dispatch boundaries now have a typed Rust scaffold for `CDR_SIMPLEBADDY`, `CDR_MACRO`, `CDR_PALACEISLENA`, `CDR_TRADER`, and `CDR_JANITOR`, including `CDT_*` compatibility constants and C-compatible return-code behavior for known and unknown drivers. Focused core tests cover tick, death, respawn, and unsupported dispatch. Actual simple-baddy/macro/trader/janitor/Islena behavior remains to be ported once character message queues and driver state are available.
- `IDR_SPECIAL_SHRINE = 147` now dispatches from the base item-driver path for the HC-to-SC special shrine kind `0x0A`: characters carry a serde-defaulted legacy `creation_time`, `PlayerRuntime` tracks the C `hcsc_last_touch` confirmation window, ineligible/non-hardcore/new-hardcore characters receive the legacy "nothing here" result, eligible hardcore characters require a second touch within ten seconds, and confirmation clears `CF_HARDCORE` with legacy-shaped system feedback. Focused core/server compilation tests cover dispatch, confirmation, and eligibility guards. Persistence of `DRD_SPECIAL_SHRINE` as an outer PPD block is not implemented because the state only protects a ten-second confirmation prompt.
- Runtime world mutations now apply the ported light primitives for live map item/effect/character changes: `World::add_item`, completed `TAKE`/`DROP`, timer-driven `IDR_NIGHTLIGHT`/`IDR_TORCH`/`IDR_TOYLIGHT` `LightChanged` outcomes, flamethrower light transitions, effect map-slot set/remove, character spawn/remove, completed walks, same-area teleports, and explicit `World::refresh_character_light_after_value_change` calls update `MapTile.light`. Completed walks also restore the legacy destination `MF_TMOVEBLOCK` reservation. `World` now owns legacy-style dirty sectors and conservatively marks light footprints for lit item changes, character light movement/removal/teleport/value changes, item-driver light mutations, and effect map-slot light changes so cache/sector consumers can skip only unchanged areas. Focused core tests cover timer-driven map item light, lit item take/drop, effect enter/leave light cleanup, character spawn/walk/remove/value-change light cleanup, and dirty-sector updates for item, character, and effect light changes. Remaining light work is wiring this refresh into full `update_char`-style value recomputation/equipment spell change call sites, exact global RNG wiring for shadow flicker, and LOS-changing map-edit bulk remove/add.
- `IDR_DECAYITEM = 132` now dispatches from the base item-driver path, porting C `decaying_item_driver` carried-use toggling, active/inactive modifier value swaps from `drdata[1]`/`drdata[2]`, sprite +/- transitions, two-second active timer scheduling, little-endian `drdata[3..6]` age/max-age counters, timer rescheduling while active, expiry destruction through the world outcome applier, and legacy-shaped expiry feedback text in the server runtime. Focused core tests cover toggle/schedule, timer aging, expiry, and inactive timer no-op. Remaining parity gaps are exact `update_char` recomputation side effects and dlog/audit behavior on expiry.
- `IDR_NOMADSTACK = 96` now dispatches from the base item-driver path, porting C `nomad_stack` split/merge behavior for area 19 salt, wolf-skin, and wolf-skin2 stacks: carried-only use, cursor-empty split sizing thresholds, template-backed split stack creation, proportional value splitting, little-endian count updates, legacy sprite/description refresh, matching-cursor merge, cursor consumption, and legacy feedback text in the server runtime. Focused core/server tests cover dispatch, split, and merge behavior. Remaining parity gaps are live area 19 data smoke coverage and exact bug/audit logging side effects.
- `IDR_NOMADDICE = 95` now dispatches from the area-19 item-driver path for C `nomad_dice`: it preserves the libload area guard, zero-character no-op, carried-only use requirement, and exposes a typed `NomadDice` outcome carrying the legacy `drdata[0]` luck value. The core also ports the C `lucky_die` helper semantics as best-of-`luck + 1` rolls with focused tests. Runtime use now rolls three six-sided lucky dice, emits the legacy `"<name> rolled d1, d2 and d3 for a total of n."` area text to players within eight tiles, and has focused server coverage for the dice roll helper. `NT_NPC/NTID_DICE` notification and exact global RNG wiring remain.
- `IDR_BEYONDPOTION = 133` now dispatches from the base item-driver path, porting C `beyond_potion_driver` core behavior: carried-only use, item requirement checks, Teufelheim arena blocking, duplicate active `IDR_POTION_SP` spell blocking through the legacy spell-slot admission helper, timed carried potion-spell creation with copied modifier slots/values and `IF_BEYONDMAXMOD`, little-endian expire/start `drdata`, spell-remove timer scheduling, source potion consumption, character item/update flags, and legacy-shaped blocked feedback text in the server runtime. Focused core/world tests cover dispatch, requirement/arena blocks, spell installation, consumption, timer data, and duplicate active potion blocking. Remaining parity gaps are template-backed `potion_spell` metadata parity, exact `update_char` recomputation side effects, and dlog/audit behavior on consumption.
- `IDR_DEMONCHIP = 136` now dispatches from the base item-driver path, reusing the stack split/merge runtime path for C `chip_stack`: bronze/silver/gold chip legacy item IDs, template names, proportional value split/merge, little-endian count updates, C split sizing thresholds, chip singular/plural descriptions, sprite offset tables, and the invalid-template `Bug #1445y` feedback distinct from `IDR_NOMADSTACK`'s `Bug #1442y` are ported with focused core/server tests. Remaining parity gaps are live-data smoke coverage and exact audit logging side effects.
- `IDR_INFINITE_CHEST = 93` now dispatches from the base item-driver path, porting C `infinite_chest` core behavior: automatic calls no-op, occupied cursor blocks before key checks, optional little-endian key ID lookup in carried inventory slots 30+ with skeleton-key precedence, rune kinds 1..9 map to `rune1`..`rune9` templates, the server instantiates the rune to the cursor, and legacy-shaped key/cursor/reward/bug feedback is emitted with focused core/server tests. Remaining parity gaps are exact bug-coordinate text, `can_carry` weight/space checks, and dlog/audit behavior.
- `IDR_SHRIKEAMULET = 118` and `IDR_MINEGATEWAYKEY = 126` now dispatch from the base item-driver path, porting the C cursor-component assembly checks and mutations: carried Shrike amulet use requires another Shrike amulet component with non-overlapping bitmask, updates `drdata[0]`, legacy sprite/name/description combinations, consumes the cursor component, and emits legacy no-cursor/does-not-fit feedback; mine gateway keys require a cursor key component, OR the bitmasks, update the legacy partial/final sprite table, final key name/description/template ID/`IF_USE` removal, consume the cursor component, and emit the legacy no-cursor/wrong-item feedback. Focused core/world tests cover dispatch and mutation side effects. Remaining parity gaps are exact item audit logging and live-data smoke coverage for the relevant quest items.
- `IDR_SPECIAL_POTION = 88` now dispatches from the base item-driver path for C `special_potion` drink branches: carried-only use, min/max level gates, long-tunnel and Teufelheim-arena blocking with legacy `You sense that the potion would not work.` feedback, antidote poison removal kinds `0..4` through the timed poison spell inventory, security potion kind `5` legacy save-count increment up to 10 with hardcore/capped blocking and client feedback, infravision potion kind `6` installing a 10-minute `IDR_INFRARED` spell item with legacy expire/start tick data, profession reset kind `7`, legacy kind `8..15` HP/mana/endurance mutations with `POWERSCALE` units and HP caps, legacy `regen_ticker = ticker` stamping for the draining fun-potion kinds `8..11` and `15`, item consumption/destruction, typed runtime accounting, legacy-shaped antidote/security/infravision feedback, and client-visible legacy fun-potion text for kinds `8..15` using `log_area`-style square `maxdist=16` runtime fan-out are covered by focused core/server tests. Remaining parity gaps are gender-aware `hisname`/`himname` pronouns, save-count persistence parity beyond Rust snapshots, and audit logging.
- `IDR_DEMONSHRINE = 35` now dispatches from the base item-driver path, porting C `demonshrine_driver` for non-timer character use, minimum-level blocking, legacy location IDs (`x + (y << 8) + (areaID << 16)`), one-time per-player shrine learning, `V_DEMON` bare-value increment, XP grant formula `min(250 + demon * 100, exp / 25)`, update/item flags, legacy feedback text for success/repeat/too-low-level/full-table bug, and `DRD_DEMONSHRINE_PPD` fixed 100-int outer PPD blob decode/encode with focused core tests. Remaining parity gaps are exact `update_char` recomputation side effects and live-data smoke coverage for the shrine items.
- `IDR_XMASMAKER = 143` now dispatches from the base item-driver path for C `xmasmaker`: zero-character calls and non-staff/non-god users no-op, staff/god users return a typed runtime outcome, and the server instantiates `xmaspop` through a legacy `give_char_item_smart`-style inventory-first/hand/drop fallback path with legacy-shaped receipt feedback. Focused core/server tests cover dispatch gating and inventory/hand placement. Remaining Christmas-driver parity gaps are full `IDR_XMASTREE` event-year PPD handling, random enhanced gift generation, exact global RNG parity, smart-give drop-failure/dlog details, and live-data smoke coverage.
- `IDR_XMASTREE = 142` now dispatches from the base item-driver path for C `xmastree`: runtime Christmas-season gating covers Dec 20 through Jan 7 with January mapped to the previous event year, `DRD_MISC_PPD` is decoded/encoded with the fixed 36-byte C layout while preserving unrelated misc fields, per-area `treedone[8]` annual gift flags reset when `gift_year` changes, cursor holiday-treat checks require an `IDR_FOOD` item with `drdata[0] == 3`, successful gifts use the C template list through the existing smart-give path, the cursor treat is consumed only after successful placement, failed placement rolls back the annual area flag, and legacy-shaped dormant/repeat/treat/no-space/success feedback is emitted with focused core/server tests. Remaining Christmas-driver parity gaps are `enhance_xmas_item` modifier/name/description generation, exact global RNG/god-name parity, smart-give drop-failure/dlog details, and live-data smoke coverage.
- Client-visible transient effect exposure now also includes the C map-anchored `EF_EXPLODE`, `EF_MIST`, `EF_EARTHRAIN`, `EF_EARTHMUD`, and `EF_BUBBLE` `SV_CEFFECT` bodies with legacy `client.h` struct layouts, plus visible-diamond gating through the retained effect map coordinates. Runtime retained creation/ticking now covers C-shaped explosion creation/addition with base sprite and light, generic map effects, mist 24-tick lifetime, bubble y-offset storage, earthrain/earthmud 60-second 3x3 sightblock-filtered placement with duplicate same-type tile suppression, and generic map-effect expiry cleanup that clears map slots/light. Focused protocol/server/core tests cover packet byte layouts, per-session cache emission, retained creation, placement, and expiry cleanup. Remaining runtime gaps are wiring all legacy call sites that create these effect families, earthrain damage cadence/RNG/hurt parity, and exact sound fan-out.
- `src/system/do.c` / `src/system/act.c` EarthRain/EarthMud action setup and completion now has a Rust bridge: `do_earthrain` and `do_earthmud` validate bounds/dead/self targets, apply the legacy HP cost guard and fast-mode endurance cost, encode `act1 = x + y * MAXMAP`, set `AC_EARTHRAIN` / `AC_EARTHMUD`, and timed world action completion creates the retained 3x3 `EF_EARTHRAIN` / `EF_EARTHMUD` map effects. Focused core tests cover setup, rejection, action completion, and retained effect shape. Remaining gaps are player/NPC call-site wiring, earthrain damage cadence/RNG/hurt parity, and exact sound/notify fan-out.
- `src/system/sewers.c` `IDR_RATCHEST` now dispatches from the item-driver registry with the C zero-character no-op and empty-cursor guard, server runtime application tracks per-player 100-entry crate cooldowns with the legacy 23-hour gate, creates money rewards with the existing legacy money item path and feedback, selects level-banded hidden sewer treasure crate coordinates, grants `sewer_ring`/`sewer_amulet` treasure with the C level/class modifier selection, and decodes/encodes the fixed-layout `DRD_RATCHEST_PPD` outer PPD block (`ID[100]`, `last_used[100]`, `treasure_x`, `treasure_y`, `last_treasure`) with focused core/workspace tests. Remaining gaps are exact global RNG parity, `set_item_requirements` recomputation for generated sewer accessories, ratling death hint speech, live sewer data smoke coverage, and dlog/audit integration.
- `EF_EARTHRAIN` runtime ticking now ports the C `ef_earthrain` damage scan for retained mapped fields: player-only targets, `max(0, strength - V_DEMON) * 150` damage, 1-in-10 roll gate via an injectable random source, `CF_UPDATE` marking, and normal effect expiry cleanup are covered by focused core tests. Remaining EarthRain gaps are exact global RNG wiring, full `hurt` side effects/death handling, player/NPC call-site wiring, and exact sound/notify fan-out.
- `src/module/simple_baddy.c` now has the first typed Rust foundation beyond dispatch stubs: `DRD_SIMPLEBADDYDRIVER` compatibility ID, C-shaped `simple_baddy_driver_data` fields, legacy `NT_CREATE` default values, `nextnv`-compatible argument parsing for aggressive/scavenger/helper/distance/day-night/teleport/help/poison/special-potion settings with unknown-argument capture, exact `src/system/notify.h` `NT_*`/`NTID_*` message constants, a serializable `CharacterDriverState::SimpleBaddy` carrier, per-character driver message queues with purge/push helpers, C `NT_CREATE` initialization from loaded character-template driver args for `CDR_SIMPLEBADDY`, including creation tick storage, create-message consumption, and `CF_NOBODY` inventory-slot-30 transformation to `CF_ITEMDEATH`, and live `Character::driver` preservation from parsed character templates are ported in `crates/ugaris-core/src/character_driver.rs`, `crates/ugaris-core/src/entity.rs`, and `crates/ugaris-core/src/zone.rs` with focused tests. Existing serialized character snapshots default missing driver IDs to zero for compatibility. Remaining simple-baddy work is fight-driver integration, potion/spell/combat decisions, movement/pathing, death/respawn behavior, and runtime invocation from NPC ticks.
- Client-visible transient effect exposure now covers the remaining C character-attached `EF_CURSE`, `EF_CAP`, and `EF_LAG` families in the retained effect cache. Rust has legacy-compatible effect IDs, `SV_CEFFECT` body builders for client types 18/19/20, visible-character diamond gating, cache slot emission, and focused protocol/core/server tests. Remaining effect parity gaps are retained runtime creation for the call sites that trigger these less-common visuals, exact sound fan-out, and full `hurt` side effects/death handling.
- `src/module/simple_baddy.c` `NT_GOTHIT` inventory-potion handling now has a typed Rust message-processing helper: simple baddies with `drinkinvpots` enabled consume their message queue, scan carried inventory slots 30+ only, use the first `IDR_POTION` with HP `drdata[1]` when HP is below 50%, and use the first `IDR_POTION` with mana `drdata[2]` when mana is below 25%, returning typed use-item outcomes for later world/runtime application. Focused core tests cover threshold behavior, slot range, wrong-driver rejection, and HP/Mana potion discrimination. Remaining simple-baddy gaps include wiring these typed outcomes into NPC ticks/use-item execution, `standard_message_driver`, helper bless decisions, poison-on-hit, fight-driver integration, movement/pathing, respawn behavior, and runtime invocation cadence.
- `src/module/simple_baddy.c` `NT_GOTHIT` inventory-potion handling is now wired into runtime NPC processing: the server loop scans simple-baddy characters with queued driver messages, calls the ported message processor, and executes returned inventory potion uses through the existing world item-driver bridge so HP/Mana potions mutate resources and consume inventory items. Focused world tests cover enabled and disabled `drinkinvpots` runtime paths. Remaining simple-baddy gaps include broader runtime invocation cadence parity, `standard_message_driver`, helper bless decisions, poison-on-hit, fight-driver integration, movement/pathing, respawn behavior, and exact NPC tick scheduling.
- `src/module/simple_baddy.c` `drinkspecial` poison-clearing flavor now emits the C `emote(cn, "drinks a potion")` feedback through a reusable Rust world area-text queue. The server drains area-text events to all sessions whose characters are inside the legacy square emote distance, and focused core tests verify the emote is queued only when an active `IDR_POISON0` spell triggers `remove_all_poison`. Remaining area-text work is wiring broader legacy `log_area` call sites and exact talk-sector/log-type filtering.
- `src/module/simple_baddy.c` `NT_CHAR` helper bless selection is now ported through the typed simple-baddy message processor: `helper`-enabled NPCs remember the last seen character message as the candidate friend like C, Rust `Character` now carries a serde-defaulted legacy `group` field preserved from parsed character templates, and the world runtime validates same-group/non-self/Bless value/mana/active-spell-slot gates before setting up the existing timed `do_bless` action bridge. Focused core/world tests cover helper-disabled suppression, last-candidate selection, same-group setup with mana spend/`AC_BLESS1`, and other-group rejection. Remaining simple-baddy gaps include broader `standard_message_driver` enemy/helper behavior, poison-on-hit, fight-driver integration, movement/pathing, respawn behavior, and exact NPC tick scheduling.
- `src/module/simple_baddy.c` `NT_CHAR` helper bless selection now also preserves the C `char_see_char(cn, co)` gate: same-group helper targets must be currently visible before the Rust runtime stores them as pending bless friends. Focused world coverage verifies hidden/LOS-blocked friends are rejected while the visible helper-bless flow still reaches `AC_BLESS1`.
- `src/module/simple_baddy.c` helper bless message ordering now matches the C local `friend` selection more closely: each `NT_CHAR` helper candidate is validated in message order by the world layer, invalid later candidates no longer clear an earlier valid friend from the same driver pass, and stale pending helper targets are cleared at the start of each message pass. Focused world coverage verifies the last valid helper target survives a later invalid candidate.
- `src/module/simple_baddy.c` `NT_DIDHIT` poison-on-hit handling now has a typed Rust message outcome and runtime application path: simple baddies with positive `poisonpower` emit a target/power/type/chance poison request only after a positive-damage hit message, and `World` validates the existing `can_attack` policy plus an injectable `RANDOM(100)`-style roll before installing the existing timed poison spell item through `poison_character`. Focused core/world tests cover outcome emission, no-target/no-damage suppression, chance misses, attack-policy rejection, spell installation, and message consumption. Remaining simple-baddy gaps include broader `standard_message_driver` enemy/helper behavior, fight-driver integration, movement/pathing, respawn behavior, exact global RNG parity, and exact NPC tick scheduling.
- `src/module/simple_baddy.c` `NT_DIDHIT` poison-on-hit runtime now consumes `World::legacy_random_seed` through the shared C-shaped `RANDOM(100)` helper on the default message-processing path, instead of using a deterministic tick/character fallback. The injectable random path remains for focused tests, and core coverage verifies default seed advancement plus poison spell installation. Broader simple-baddy exact RNG parity remains for fight-task tie ordering and other unrelated randomized branches.
- `src/system/drvlib.c` `fight_driver_attack_enemy` low-HP flee task is now enabled in the Rust simple-baddy fight-driver path: below-half-HP NPCs add the C high-priority `flee` task, the weighted task executor routes it through the existing C-shaped flee movement/scoring helper, and focused core tests cover task admission plus action setup/speed-mode behavior. Remaining simple-baddy gaps include broader fight-driver parity, movement/pathing edge cases, respawn behavior, exact global RNG parity, and exact NPC tick scheduling.
- `src/module/simple_baddy.c` scavenger idle/wander runtime cadence now consumes the world legacy RNG seed in the bulk NPC noncombat loop instead of using deterministic zero rolls, preserving C-style `RANDOM(2)` idle gating and `RANDOM(8)+1` direction selection while retaining injectable random hooks for focused tests. Remaining simple-baddy gaps include broader `standard_message_driver` enemy/helper behavior, fight-driver integration edge cases, respawn/body-drop behavior, and exact NPC scheduling cadence.
- `src/module/simple_baddy.c` nearby death notifications now feed the Rust simple-baddy fight state: `NT_DEAD` messages emit a typed remove-enemy outcome and the world message applier removes matching targets from the ten-slot enemy table before later attack/pathing passes can chase stale dead targets. Focused character-driver/world tests cover message emission and runtime state cleanup.
- `src/module/simple_baddy.c` `ch_died_driver` earth-demon death retaliation now has a typed character-driver outcome for `CDR_SIMPLEBADDY` death calls, and the world death path routes through that dispatch before applying the existing C-compatible `CF_EDEMON`/visibility gates to create `EF_EARTHMUD` when `V_DEMON > 5` plus `EF_EARTHRAIN` at the killer tile. Focused registry/world tests cover return-code parity and effect creation through the dispatch path. Remaining simple-baddy gaps include broader death/drop/respawn behavior, full fight-driver parity, movement/pathing edge cases, and exact NPC scheduling cadence.
- Area 25 `IDR_WARPKEYSPAWN` now preserves the C dynamic template name shape `warped_teleport_key%d` for all `drdata[0]` values instead of clamping unknown sphere kinds to a Rust-only `warped_teleport_key0`; invalid legacy data now reaches the existing template-creation failure path and emits `It won't come off.` like C. Focused core tests cover valid and invalid sphere kinds, and `cargo build -p ugaris-server` passes.
- Area 25 `IDR_WARPKEYDOOR` direct world execution now builds its runtime context from carried inventory slots only, matching C `has_item(cn, IID_AREA25_DOORKEY)` and preventing cursor-held warped door keys from opening the door. Focused world tests cover inventory-key success and cursor-key rejection.
- Area 22 `IDR_LAB2_WATER = 196` now dispatches through the Lab 2 item-driver path for C `lab2_water`: zero-character initialization classifies wells, altars, bowl templates, water bowls, and holy-water bowls from the legacy sprite table; player well use blocks occupied cursors with the legacy feedback and otherwise creates `lab2_waterbowl` on the cursor; altar use converts cursor and inventory-slot-30+ water bowls into `lab2_holywaterbowl` templates and emits the singular/plural/no-water feedback; direct water/holy-water bowl use emits the legacy `Skoll!` text. `IDR_LAB2_REGENERATE = 194` now ports the C timed regenerate spell item core: timer-only dispatch, C-shaped `drdata` decoding for speed/regen/target/start tick, carried-target guard, missing-HP fractional healing, `CF_NODEATH` set/clear behavior around `startat`, and speed-based rescheduling. `IDR_LAB2_GRAVE = 197` now has reusable core support for the C-described grave table, `DRD_LAB_PPD` grave-version/index offsets, clue-book text formatting for Henry/Eldrick/John/Mariah, player-specific special-grave coordinate matching, one-bit grave-cleared bitsets, and runtime grave-open/close/check outcomes. Focused core/world tests cover initialization, typed use outcomes, legacy regenerate decoding, healing, pre-start nodeath clearing, timer rescheduling, clue text, special-grave matching, bitset layout, and timer close/check behavior; server build coverage validates runtime template creation/replacement. Remaining Lab 2 gaps include undead death reward/PPD count side effects, Arathas wake-all behavior, broader undead movement/combat scripts, exact dlog/audit integration, and live area-data smoke coverage.
- `src/module/simple_baddy.c` `NT_TEXT` handling now preserves an optional Rust text payload on character-driver messages with serde-default compatibility for existing snapshots, ports the C `tabunga` keyword gate for nearby god speakers, and emits the legacy diagnostic NPC `say` lines as area text containing resource totals, combat values, selected professions, rest coordinates, speed mode, and alive/undead flags. Focused character-driver/world tests cover payload preservation, keyword/god/distance gates, and representative diagnostic output. Remaining simple-baddy gaps include exact `tabunga` derived immunity/spell-power helper lines, broader `standard_message_driver` reuse outside simple baddies, full fight-driver scheduling parity, movement/pathing edge cases, respawn behavior, and exact NPC tick scheduling.
- Area 22 `IDR_LAB2_GRAVE = 197` now has the first Lab 2 grave item-driver registry boundary: it is guarded to area 22 like the C module, closed zero-character/timer callbacks return a handled C-compatible no-op (`it_driver` return code `1`), clue-book graves (`drdata[0]` 1..4) return typed outcomes, `DRD_LAB_PPD` preserves the C `graveversion`/`graveindex[4]` offsets, the server emits the matching Henry/Eldrick/John/Mariah described-grave text from the legacy 40-entry table, normal player use spawns `lab2_undead` / `lab2_skeleton` from templates, attaches the described/fixed special grave items, opens/closes the grave with timer/serial guards, and repeated cleared normal graves use the C one-bit-per-grave runtime bitset to emit `This grave is empty` and temporarily open without spawning. Focused core/server/workspace tests cover the area guard, handled closed-timer return code, clue-book dispatch, Lab PPD layout, undead spawn/item attachment, empty-open timer close, live-undead serial checks, and repeated-cleared empty-grave behavior. Remaining Lab 2 grave gaps include undead death-driver bit marking/rewards, Arathas awaken-all fan-out, full undead character-driver behavior, exact random described-grave selection/global RNG parity, and live area-data smoke coverage.
- Area 22 `IDR_LAB2_GRAVE` normal player use now returns a typed open-grave outcome instead of unsupported, preserves fixed special-item kinds from `drdata[0]`, resolves per-player described grave coordinates through the existing Lab PPD grave indices, instantiates `lab2_undead` / `lab2_skeleton` plus Elias/Arathas special item templates, applies the C grave open state in `drdata[4..12]`, sprite increment, dirty-sector marking, and five-second open-check timer scheduling, and spawns Lab 2 grave enemies with `CDR_LAB2UNDEAD`, no respawn flag, down-facing direction, full resources, and special Elias/Arathas names/value tweaks. Focused core/server tests cover the dispatch boundary and template-backed described-grave item attachment. Remaining Lab 2 grave gaps include per-player grave bitset/empty-repeat behavior, Arathas opening nearby graves, undead death reward/gold/PPD handling, holy-water undead driver behavior, and exact RNG parity for ordinary undead-vs-skeleton selection.
- Area 22 `IDR_LAB2_GRAVE` timer handling now also ports the C empty-open-grave close path: timer/zero-character calls with `drdata[4..7]` open marker and `drdata[8..11] == -1` return a typed close outcome, and the world applier decrements the grave sprite, clears the open character/serial fields, and marks the grave sector dirty. Focused core/world tests cover the typed outcome and mutation. Undead spawning, live-undead timer rescheduling/serial checks, grave bitset rewards, and player grave-opening remain.
- Area 22 `IDR_LAB2_GRAVE` live-undead timer handling now mirrors the C open-grave check loop: timer callbacks with a positive stored undead id and serial return a typed check outcome, the world applier reschedules the grave check every five seconds while the matching undead serial still exists, and closes/clears the grave when the undead is gone or stale. Player use against already-open graves now no-ops like C. Focused core/world tests cover typed dispatch, rescheduling, stale-serial closure, and open-use suppression. Undead spawning, grave bitset rewards, and player grave-opening remain.
- `src/module/simple_baddy.c` `simple_baddy_dead` earth-demon death effect behavior now has a Rust world applier wired into the reusable `hurt` kill path: `CDR_SIMPLEBADDY` characters with `CF_EDEMON` that can see the killer create `EF_EARTHRAIN` at the killer position using effective `V_DEMON`, and also create `EF_EARTHMUD` when effective demon value is greater than five. Focused core tests cover the driver gate, visibility/LOS gate, earthmud threshold, effect strengths, map-slot placement, and invocation through `World::apply_legacy_hurt` when damage kills the baddy. Remaining simple-baddy death gaps are broader `kill_char`/respawn/body-drop integration and exact sound/notify fan-out.
- `src/system/act.c` `tile_special_check` now has a Rust world primitive and runtime tick-loop invocation before action advancement like C: player-only `MF_SLOWDEATH` handling applies underwater drowning damage without `CF_OXYGEN`, creates one-tick `EF_BUBBLE` map effects on the legacy oxygen cadence using `ticker + serial * 32`, applies non-underwater slowdeath damage with the lava-sprite higher-damage branch, marks characters for update on damage, and returns typed sound hooks for later runtime fan-out. Focused core tests cover drowning, bubble cadence, slowdeath hazard damage, and invocation for idle/no-action players. Remaining gaps are exact `hurt` death/armor side effects and C `sound_area` fan-out/random underwater sound selection.
- Area 22 `IDR_LAB2_STEPACTION = 195` now dispatches through the Lab 2 item-driver path: timer callbacks clear legacy marker sprites for known step kinds 1/2, daemon-warning triggers require a player facing up and expose a runtime `lab2_daemon` spawn request at `y - 5`, daemon-check triggers require a player and route the C `notify_area(..., NT_NPC, NTID_LAB2_DEAMONCHECK, cn, 0)` through Rust driver message queues, and server runtime attempts the daemon template spawn through the retained zone loader. Focused core/world tests cover timer clearing, player/facing gates, and nearby daemon-check notification. Remaining Lab 2 gaps include graves, daemon/undead character-driver behavior, exact daemon spawn driver state initialization, PPD grave value handling, and exact dlog/audit integration.
- Area 11 `IDR_PALACEBOMB` and `IDR_PALACECAP` now dispatch through the Rust item-driver registry under the legacy area-11 libload guard. The palace bomb boundary ports carried active/inactive toggling, sprite transitions, owner ID storage in `drdata[1..4]`, ground-timer arming to `drdata[0] = 2`, `IF_STEPACTION`/take/use flag mutation, five-second rescheduling, and typed explosion outcomes carrying source coordinates and owner ID. Runtime palace bomb explosions now create the C-shaped `EF_EXPLODE` visual at base sprite `50050`, queue legacy sound type `6`, attach one-minute `EF_BURN` effects with `POWERSCALE * 2` strength to eligible 3x3 targets while preserving the Islena and owner/player filters, and destroy/remove the bomb item. The palace cap boundary preserves the C character-use no-op and timer rescheduling shape as a typed runtime outcome. Focused core tests cover bomb toggling, arming, explosion metadata, area guard behavior through dispatch, explosion effect/sound/burn/runtime removal, and cap timer/no-op boundaries. Remaining area-11 bomb/cap gaps are exact palace-guard perception integration and live palace data smoke coverage.
- Area 11 `IDR_PALACECAP` timer application now ports the C carried-cap active-state loop: zero-character timer callbacks reschedule every quarter-second, require the cap to still be carried in `WN_HEAD`, deactivate active caps while the wearer is inside `regen_ticker + regen_time`, activate idle caps after the regen window by toggling `drdata[0]`/sprite and marking `CF_ITEMS`, and create or refresh the retained `EF_CAP` character visual for the same short stop window as C. Focused world tests cover activation, deactivation, item flagging, and retained effect refresh. Remaining area-11 cap gaps are exact palace-guard perception integration beyond the C-visible active `drdata[0]` state and live palace data smoke coverage.
- `src/module/simple_baddy.c` `NT_TEXT` handling now has a typed Rust boundary outcome: simple-baddy message processing preserves the legacy `tabunga(cn, co, text)` notification's speaker id and raw text token instead of silently discarding it, and the world runtime treats the preserved notification as a handled no-op until the message carrier grows a real text payload. Focused core coverage verifies message consumption and outcome shape. Remaining work is reconstructing/persisting text payloads and wiring actual `tabunga` dialogue behavior.
- `src/module/simple_baddy.c` `NT_NPC` helper alert handling now has a typed Rust message outcome and runtime application path: simple baddies with a matching `helpid` consume NPC alert messages, validate the reporting character is same-group/non-self like C, and record the reported enemy in serializable simple-baddy driver state as the future fight-driver integration point. Focused core/world tests cover help-id filtering, same-group gating, duplicate enemy refresh, and runtime message consumption. Remaining simple-baddy gaps include broader `standard_message_driver` enemy/helper behavior, full movement/pathing, respawn behavior, exact global RNG parity, and exact NPC tick scheduling.
- `src/module/simple_baddy.c` retained-enemy attack selection now has a Rust runtime slice: the server tick loop scans `CDR_SIMPLEBADDY` characters with simple-baddy driver state, prioritizes recorded enemies by priority/last-seen tick, validates live/visible/attackable adjacent targets through the existing `char_see_char`/`can_attack`/`do_attack` bridge, sets up timed `AC_ATTACK1`, and updates `lastfight` on successful setup. Focused core tests cover adjacent visible attack setup. Remaining simple-baddy gaps include broader `standard_message_driver` enemy/helper behavior, following/pathing toward invisible enemies, full fight-driver enemy scoring and attack cadence parity, death/respawn effects, exact global RNG parity, and exact NPC tick scheduling.
- `src/system/drvlib.c` `standard_message_driver` is now partially wired into the simple-baddy message path: aggressive `NT_CHAR` sightings record visible attackable enemies with C priority `0`, `NT_GOTHIT` records valid attackers defensively with priority `1` even when not currently visible, and helper `NT_SEEHIT` records the attacker/victim enemy for same-group friends through the existing simple-baddy enemy list. Focused core/world tests cover typed outcome emission and runtime enemy insertion. Remaining standard-message/fight-driver gaps include exact `is_valid_enemy` clan/hate policy, invisible-follow use of unseen enemies, `fight_driver_note_hit`, broader area-driver reuse, full fight-driver scoring, and exact NPC scheduling.
- `src/system/drvlib.c` `fight_driver_attack_visible` movement fallback now has a Rust runtime slice for simple baddies: recorded visible enemies that are live/attackable but not adjacent cause the NPC to path one step toward attack range through the existing legacy pathfinder/`do_walk` bridge, with character-blocker ignoring as a fallback and `lastfight` updated on successful action setup. Focused core tests cover visible non-adjacent enemy pursuit. Remaining simple-baddy movement gaps include invisible last-known-position following, secure/home-distance movement rules, full `fight_driver_attack_enemy` spell/task scoring, exact sound cadence, and exact NPC scheduling.
- `src/system/drvlib.c` `fight_driver_update` stop-distance handling now matches the C visibility ordering for simple-baddy enemy memory: dead/unattackable enemies are dropped immediately, visible enemies beyond `stopdist` are dropped, but hidden enemies beyond `stopdist` are retained and can still be followed to their last known tile by the existing invisible pursuit path. Focused core tests cover hidden-retained and visible-dropped stop-distance cases. Remaining simple-baddy gaps include broader fight-driver scoring, exact sound cadence, death/respawn/body-drop integration, and exact NPC scheduling.
- `src/module/simple_baddy.c` scavenger home-return direction handling now matches the C branch that clears `dat->dir` before moving/secure-moving back to `tmpx/tmpy`, so a failed or interrupted return does not keep reusing the stale random-wander direction. Focused core tests cover the recent-fight home-return path and existing scavenger behavior. Remaining simple-baddy gaps include broader area-driver reuse, full enemy scoring parity, respawn/body-drop behavior, exact global RNG wiring, and exact NPC scheduling.
- `src/system/drvlib.c` `fight_driver_follow_invisible` now has a Rust runtime slice for simple baddies: recorded enemies track C-style `visible` plus last-known tile fields, tracking is refreshed from `char_see_char` before action selection, visible enemies still attack/path first, invisible enemies path toward their last-known position with the same character-blocker fallback, successful follow actions stamp `lastfight = ticker` like `simple_baddy_driver`, and enemies are dropped once the NPC reaches the last-known tile or cannot path there. Focused core tests cover following an invisible target, the `lastfight` update, and giving up at the last-known tile. Remaining simple-baddy movement gaps include secure/home-distance movement rules, full `fight_driver_attack_enemy` spell/task scoring, exact sound cadence, and exact NPC scheduling.
- `IDR_FORESTSPADE = 77` now dispatches from the base item-driver path for C `spade`: carried-only use, occupied-cursor blocking, forest note dig at area 16 `(205,234)`, collapse teleports at area 16 `(130,219)` and area 1 `(93,36)`, Brannington area 29 annual treasure locations, legacy-shaped success/empty/cooldown/cursor/collapse feedback, `forest_note1` template creation to cursor, money-item creation for treasure digs, `DRD_TREASURE_DIG_PPD` five-int outer PPD blob decode/encode, and successful Brannington digs updating the legacy `DRD_STAFFER_PPD.forestbran_done = dig_index + 1` quest field are covered by focused core/server/player tests. Remaining parity gaps are exact global RNG parity and dlog/audit behavior.
- `src/system/drvlib.c` simple-baddy standard enemy admission now preserves the C `fight_driver_add_enemy` distance exception for enemies that hurt the NPC: `start_dist` and `char_dist` still reject normal aggressive/helper sightings, but `NT_GOTHIT`/hurtme enemies are retained even outside those limits while keeping neutral-zone and attack-policy gates. Focused core coverage prevents regression.
- Area 11 `IDR_ISLENADOOR = 138` now dispatches through the legacy palace libload guard for C `islena_door`: zero-character/timer calls no-op, standing at `(144,56)` exits to `(144,58)`, the room scan over `(138..146,49..57)` blocks when another player is inside, missing Islena and recovering HP/mana states emit the legacy feedback text, and ready state teleports the challenger to `(143,55)` through the existing same-area teleport path. Focused core/server tests cover dispatch, area guard, room-state context scanning, blocked feedback classifications, and teleport destinations. Remaining area 11 gaps include palace guard/Islena character-driver dialogue/combat/death behavior, `DRD_ISLENA_PPD` persistence, exact title/achievement side effects, and live palace data smoke coverage.
- `src/system/drvlib.c` simple-baddy fight-driver distance gates now have a Rust runtime slice: standard enemy insertion honors C `start_dist` and `char_dist` gates for non-hurt aggro using the character respawn/home fallback, and enemy tracking drops visible targets that move beyond `stop_dist` before attack/path selection. Focused core tests cover start/char-distance rejection and stop-distance removal. Remaining simple-baddy movement gaps include explicit `fight_driver_set_home` updates from secure day/night movement, full `fight_driver_attack_enemy` spell/task scoring, exact sound cadence, and exact NPC scheduling.
- `src/system/drvlib.c` `fight_driver_add_enemy` neutral-zone gating is now represented in the simple-baddy standard-message path: `NT_CHAR`/non-hurt helper aggro carries the C `hurtme=0` distinction and refuses targets standing on `MF_NEUTRAL`, while defensive `NT_GOTHIT`/hurt-me entries still record the enemy through neutral tiles. Focused core tests cover both rejection and retaliation paths. Remaining standard-message/fight-driver gaps include exact `is_valid_enemy` clan/hate policy, full enemy array ordering parity, broader area-driver reuse, full `fight_driver_attack_enemy` spell/task scoring, and exact NPC scheduling.
- `src/system/drvlib.c` simple-baddy `firering` task execution now has an explicit Rust dispatch helper instead of sharing the target fireball helper: close-range fireball tasks validate the legacy `IDR_FIRERING` spell-slot blocker, require a useful fireball damage result, cast `do_fireball` at the caster's own tile, preserve `lastfight` stamping, and keep active-blocker rejection covered by focused core tests. Remaining fight-driver gaps include exact `fight_driver_attack_enemy` fallback ordering for edge failures, broader area-driver reuse, exact global RNG/sound cadence, and exact NPC scheduling.
- Client-visible transient effect exposure now covers C `EF_EDEMONBALL` records: the protocol builder emits legacy client effect type `17` with `nr`, `start`, `base`, `frx`, `fry`, `tox`, and `toy` fields, and the server retained-effect cache gates it by projectile fixed-point position and sends it through the same `SV_CEFFECT`/`SV_UEFFECT` slot machinery as fireball/ball effects. `IDR_EDEMONBALL = 36` now dispatches timer callbacks from the area-driver item path, creates retained `EF_EDEMONBALL` projectiles using the C rotating fallback shot table, preserves `drdata[1]` base sprite / `drdata[2]` strength / `drdata[3]` rotation state, reschedules blind fallback shots after the legacy 16-second delay, scans nearby characters before fallback shots, predicts walking targets with C `dist * 1.5` / remaining-step timing, validates the half-tile `can_hit` line against map blockers and the target character, preserves fallback rotation on aimed shots, and reschedules aimed shots after the legacy 8-second delay. Runtime `EF_EDEMONBALL` ticking now ports the C fixed-point quarter-tile movement, character/TMOVEBLOCK/MOVEBLOCK collision rules with `MF_FIRETHRU` passthrough, previous-tile wall explosions, direct impact damage, player/playerlike immunity for base-sprite `2`, green-crystal shield absorption/destruction for base-sprite `0`, and retained explosion creation with focused core tests. Remaining Earth Demon ball work is switch/sector enable-state gates, exact sound fan-out, and full `hurt` side effects/death handling.
- Area 12 `IDR_ENHANCE = 61` now dispatches from the Rust item-driver registry for C `collect_item` carried material use and reuses the existing stack runtime applier for silver/gold unit split/merge behavior. The server recognizes C's metal stack layout (`drdata[0]` kind, little-endian unit count in `drdata[1..4]`), instantiates `silver`/`gold` split stacks, preserves proportional value splitting/merging, updates legacy descriptions, clears matching cursor stacks on merge, and emits the existing split/merge/cannot-mix feedback path with focused core/server tests. The non-matching cursor branch now ports C `enhance_item`: enhanceable sprite mapping, silver-vs-gold material gates for normal/already-silvered items, C unit-price formula, material count/value reduction or consumption, target sprite/value/modifier mutation, armor/weapon/skill requirement modifier increments, unusable-item confirmation via `drdata[8..15]`, and legacy feedback for silver/gold/not-enough/confirm/success are covered by focused server tests. Remaining area-12 mine gaps are `minewall`, `minedoor`, key-holder character behavior, exact `set_item_requirements` recomputation side effects, item look-output parity after enhancement, audit/dlog side effects, and live area-data smoke coverage.
- Area 12 `IDR_MINEGATEWAY = 127` now dispatches from the Rust item-driver registry for C `minegateway`: zero-character and non-player calls no-op, live inventory/cursor context checks the exact assembled gateway key item ID `IID_MINEGATEWAY`, missing keys emit the legacy inscription text, destinations decode little-endian `drdata[0..5]`, invalid coordinates/area produce the legacy bug feedback, same-area destinations teleport through the existing world path, and cross-area destinations keep the target-area-server-down handoff boundary. Area 12 `IDR_MINEKEYDOOR = 125` now dispatches for C `keyholder_door`: it requires a cursor `IDR_ENHANCE` gold-unit stack with `drdata[0] == 2` and little-endian `drdata[1..4] == 2000`, emits the legacy missing-gold/fighting-noises feedback, scans the nine 7x7 keyholder rooms for the first room without takeable items or characters, teleports the player to the C room entry tile, and consumes the cursor gold stack on successful entry. Focused core/world tests cover key gating, destination decoding, invalid destinations, live assembled-key same-area teleport, keyholder-door gold gating, golem-number decoding, room selection, teleport, and cursor consumption. Remaining area-12 mine gaps are actual cross-area transfer execution, full keyholder golem character creation/AI, exact `minewall`/`minedoor` random/timer parity, exact audit/dlog side effects, and live area-data smoke coverage.
- `IDR_EDEMONBALL` timer dispatch now ports the C area-6 fire/sector enable gates: part-1 launchers with `drdata[0] == 0` consult the retained Earth Demon switch fire state, disabled launchers switch to sprite `14160` and retry after one second without creating a projectile, re-enabled launchers restore sprite `14159`, part-5 launchers with section indices `2..=9` consult matching loader section power, disabled/offline sections use sprite `14160` and retry after one second, and powered sections use sprite `14161` before firing. The world timer context derives fire state from loaded `IDR_EDEMONSWITCH` items and section power from loaded `IDR_EDEMONLOADER` items, schedules inactive retries, and marks changed launcher sectors dirty with focused item-driver/world tests. Remaining Earth Demon ball work is exact sound fan-out and full `hurt` side effects/death handling.
- `src/system/drvlib.c` `fight_driver_add_enemy` slot/update semantics are now mirrored in the Rust simple-baddy enemy table: existing enemies are searched only in slots `0..8`, slot `9` remains the C overflow/new-entry slot even for the same target ID, and the stored hurt/priority flag is overwritten on refresh instead of maxed. Focused core tests cover overflow-slot return behavior and priority downgrade parity. Remaining standard-message/fight-driver gaps include exact `is_valid_enemy` clan/hate policy, broader area-driver reuse, exact NPC scheduling, and full death/respawn side effects.
- Area 6 `IDR_EDEMONSWITCH = 37` now dispatches from the item-driver registry for the C `edemonswitch_driver` core lever path: timer callbacks re-enable the fire/light after the stored five-minute cooldown, player use while active disables fire, increments the sprite, clears light, stores the cooldown deadline, schedules the reset callback, and disabled player use emits the legacy `The lever seems stuck.` feedback. Focused core tests cover active use, timer re-enable, and stuck feedback. Remaining Earth Demon switch parity gaps are the original C module-global fire/pause sharing across all switches, exact area 6 sector/power integration, sound fan-out, and live area-data smoke coverage.
- `src/module/simple_baddy.c` scavenger return-home movement now mirrors the C branch more closely: out-of-range scavengers without `notsecure` and without a recent fight use the existing `secure_move_driver` path, including the blocked-use return-code teleport fallback, while recent-fight and `notsecure` cases keep plain pathing. Focused core tests cover secure teleport fallback and recent-fight non-teleport behavior. Remaining simple-baddy movement gaps include full secure movement side effects, exact sound cadence, and exact NPC scheduling.
- Area 6 `IDR_EDEMONLOADER = 39` now dispatches from the item-driver registry for the C `edemonloader_driver` core crystal-loader path: player use accepts only the legacy yellow crystal item ID, decodes cursor `drdata[0]` into loader power, consumes the cursor crystal, starts the seven-tick animation, mutates loader sprite and high-word ground overlay sprites, emits the legacy insert/power-off sounds, timer callbacks decay power/animation once per second, startup schedules existing loaders, and `IDR_EDEMONLIGHT` now derives section power from matching live loaders during timer callbacks. Focused core/world tests cover accept/block paths, timer decay, cursor destruction, map overlay, sounds, scheduling, and powered light behavior. Remaining Earth Demon loader gaps are exact C module-global `sect[]` ordering when multiple loaders share a section, full area 6 door/gate/tube sector integration, dlog parity, and live area-data smoke coverage.
- Area 6 `IDR_EDEMONGATE` spawn bookkeeping now stores and validates the spawned character serial in the same two-byte slot layout as C `edemongate_driver`, and Rust `Character` snapshots carry a serde-defaulted `serial` field for legacy serial-guard call sites. Gate slots with matching character IDs but changed serials are treated as stale and respawnable, with focused world coverage. Remaining Earth Demon gate gaps are exact C character-serial allocation/reuse parity, module-global mode-1 position cache behavior, and live area-data smoke coverage.
- Area 6 `IDR_EDEMONDOOR = 41` now preserves the C carried-key feedback line for exact-key use: the item-driver outcome carries the legacy key name and lock/unlock direction, keyring keys remain rejected for Earth Demon doors, and the server emits `You use <key> to unlock/lock the door.` before counting the successful toggle. Focused core coverage pins the key-name outcome and timer no-key path. Remaining Earth Demon door gaps are full area 6 sector/power propagation parity, dlog/sound fan-out beyond the shared door sound path, and live area-data smoke coverage.
- Area 14 `IDR_RANDOMSHRINE = 69` now dispatches from the item-driver registry for the C random-shrine boundary: zero-character calls no-op, shrine keys use the legacy item ID `MAKE_ITEMID(DEV_ID_DB, 0x5B)` and matching `drdata[0] == shrine drdata[1]`, inventory and cursor key lookup is world-sourced like C, missing-key and already-used cases return typed outcomes, legacy shrine type ranges are classified for indecisiveness/bribes/welding/edge/kindness/vitality/death/braveness/security/jobless/dormant/continuity, and continuity type `255` follows its distinct non-bitset path. Runtime execution covers security, jobless, edge, kindness, death, braveness, vitality, and continuity shrine effects, including C HP/Mana target selection, 5-point/cap handling, raise-cost XP accounting, `/noexp`/cap feedback, continuity sequence gating, C `level_value(min(level + 5, shrine_level)) / 6` EXP, level-99 gate teleport to `(41,250)`, random-shrine PPD marking only on success for bitset shrines, and C-compatible `struct shrine_ppd` persistence with the 32-byte used bitset plus one-byte continuity field. Focused core/world/server tests cover key gating, inventory lookup, repeat blocking, type classification, vitality, continuity sequencing, and random-shrine PPD round trips. Remaining random-shrine gaps are indecisiveness/bribes/welding effect execution, questlog resend after successful effects, exact dlog/audit behavior, and continuity completion achievement side effects.
- Area 6 `IDR_EDEMONGATE = 38` now dispatches from the item-driver registry for the C `edemongate_driver` timer path: zero-character timer callbacks choose the first stale mode-0 demon slot using the legacy seven fixed positions, choose mode-1 `edemon6s` spawn slots from live section-4 `IDR_EDEMONLIGHT` items in stable item-id order, return typed spawn requests for `edemon2s`/`edemon6s`, reschedule at the C 10-second/20-second intervals, and the server runtime instantiates the requested character template, inserts inventory items, and records the spawned character ID in the gate slot table. Focused core/world tests cover dispatch, slot selection, and rescheduling. Remaining Earth Demon gate gaps are exact C character serial tracking, module-static mode-1 state parity, C `item_drop_char` around-gate placement before target-home assignment, full area 6 sector/power integration, dlog/sound fan-out, and live area-data smoke coverage.
- Area 6 `IDR_EDEMONBLOCK = 42` now dispatches from the item-driver registry for the C `edemonblock_driver` movable block path: player use pushes the block one tile in the character's facing direction, validates the target tile's movement blockers, item occupancy, and legacy ground-sprite range `12150..=12158`, moves the map item while updating temporary movement blockers and dirty sectors, stores the last-touch tick, emits the legacy `It won't move.` feedback when blocked, timer callbacks initialize original coordinates, return moved blocks after fifteen minutes of inactivity when the origin is free, and startup schedules existing blocks. Focused core/world tests cover dispatch, touch-tick storage, push movement, blocked targets, timer origin return, and startup scheduling. Remaining Earth Demon block gaps are exact halt/fake-action behavior after pushing, sound/log side effects, and live area-data smoke coverage.
- Area 6 `IDR_EDEMONTUBE = 43` timer overload behavior now ports the C `sect[nr] > 250` pulse path: timer callbacks with overloaded loader power return a typed tube-pulse outcome after target discovery, the world scans used player characters in the legacy 10-tile square, gates them through `char_see_item`, teleports visible players to the remembered loader-adjacent target, queues the legacy `The strange tube teleported you.` system feedback, and reschedules the one-second timer. Focused core/world tests cover the overloaded dispatch and full timer-request teleport side effect. Remaining Earth Demon tube gaps are exact C module-global `sect[]` ordering when multiple loaders share a section, sector-iteration ordering parity, and live area-data smoke coverage.
- `src/system/drvlib.c` `fight_driver_set_home` now has a Rust state/runtime slice for simple baddies: `SimpleBaddyDriverData` stores serde-defaulted explicit home coordinates, `World::set_simple_baddy_home` mirrors the C helper boundary, and start/stop distance gates prefer explicit fight-driver home over respawn/rest fallback when present. Focused core tests cover explicit-home start-distance admission, stop-distance pursuit retention, and explicit-home updates from secure/scavenger/day-night movement outcomes. Remaining simple-baddy movement gaps include broader area-driver reuse, exact global RNG parity, and exact NPC scheduling.
- `src/system/drvlib.c` `fight_driver_attack_value` now uses the C `get_attack_skill`-style calculation for simple-baddy melee task priority instead of raw `V_ATTACK`: right-hand weapon type flags select the matching Hand/Dagger/Staff/Sword/Two-Hand skill, bare or non-weapon hands fall back to `V_HAND`, attack/tactics/spell-average/earth-demon branches share the existing ported attack-skill formulas, and focused core tests cover weapon-skill and bare-hand task values. Remaining parity gaps include carrying the C `rage` field into Rust character state and broader fight-driver NPC scheduling/global RNG parity.
- `src/module/simple_baddy.c` post-combat noncombat driver flow now has a Rust runtime slice: idle live `CDR_SIMPLEBADDY` characters with driver state perform the legacy short creation idle, day/night post selection, optional configured teleport to day/night posts, rest-home walking, scavenger return-to-rest bounds, C `regenerate_driver` one-second idle blocking while below base HP/mana before wander/final idle choices, C `RANDOM(2)` idle gating, random `RANDOM(8)+1` bounded scavenger wandering with retained direction and blocked-direction reset, and explicit fight-driver home updates when reaching, teleporting to, or wandering around a post. The server tick loop now invokes this after message and attack processing, and focused core tests cover creation idle, night-post teleport/home setting, rest-home walking, regeneration-before-random-wander ordering, scavenger idle gating, bounded random wandering, direction reset on blocked walks, helper friend-bless memory/consumption ordering, C `secure_move_driver` walk/teleport/turn behavior including the failed-use `ret == 2` path, weighted `fight_driver_attack_enemy` task selection for currently ported spell/combat/movement tasks, and the 10-second start-combat `sound_area(..., 1)` cadence. Remaining simple-baddy movement gaps include full area-driver reuse, exact global RNG parity for fight-task silliness/tie ordering, remaining unported task side effects such as complete earthrain/flee integration, exact `is_valid_enemy` clan/hate policy, and exact NPC scheduling.
- `src/system/drvlib.c` `spell_self_driver` ordering is now represented in the Rust simple-baddy noncombat runtime path: after `regenerate_driver` and before final idle/wander, simple baddies try self-bless, then magic shield, then self-heal using the existing timed spell setup bridge and legacy mana/active-spell gates. Focused core tests cover self-bless priority, magic-shield fallback, regeneration blocking lower-priority self-spells, and helper friend-bless ordering through the noncombat flow. Remaining simple-baddy movement gaps include broader area-driver reuse, exact global RNG parity, and exact NPC scheduling.
- `src/system/drvlib.c` `fight_driver_attack_enemy` now has a first spell-task runtime slice for simple baddies: visible recorded enemies are checked for C-style fireball skill/mana/damage gates before melee/path fallback, adjacent targets set up self-targeted `AC_FIRERING` through the existing timed spell bridge when the `IDR_FIRERING` blocker slot is available, and non-adjacent visible targets set up targeted `AC_FIREBALL1` using the existing moving-target prediction helper. Focused core tests cover firering priority, targeted fireball setup, mana spend, direction/target encoding, fireball line-of-hit repositioning, distance/attackback/flee task scaffolding, weighted task ordering, and `lastfight` stamping. Remaining fight-driver gaps include exact global RNG parity, broader area-driver reuse, complete side-effect parity for every completed task, and exact NPC scheduling.
- Area 29 `IDR_STAFFER2` now dispatches from the item-driver registry for the C `staffer2_item` animation-book branch (`drdata[0] == 6`): player-only use returns a typed same-area teleport to `(25,114)` plus the legacy XP amount for runtime PPD gating, while preserving the area-29 libload guard and explicit unsupported outcomes for the still-unported staffer book/mine/block/special-door subtypes. Focused core tests cover the typed XP/teleport boundary, non-player no-op, area guard, and unsupported subtypes. Remaining staffer2 gaps are branches 1..5, `DRD_STAFFER_PPD` quest state, exact feedback/logging, mine/block map mutation timers, special-door mechanics, and live area-data smoke coverage.
- Area 29 `IDR_STAFFER2` subtype `1` now ports the C `staffer_book` text-cycle branch: reader ID changes reset book progress, `drdata[1]` advances through the five legacy pages and wraps after the final page, and the server emits the matching legacy continuation/start-over prompt through `SV_TEXT`. Subtype `2` ports the mine-wall digging path, including endurance/miner-cost handling, stage sprites, sightblock removal at stage 3, open-wall `IF_VOID`/map-block removal at stage 8, delayed timer restore, and exhausted feedback. Subtype `3` ports pushable block movement, valid Brannington floor-sprite checks, blocked feedback, original-position storage, and timer return. Subtypes `4`/`5` port the special-door toggle/timer core, blocked-door retry counters, marker-lock checks, stored door flags, auto-close scheduling, and sound/dirty-sector hooks. Focused core/world/server tests cover page cycling, mine dig/restore, block push/return/blocking, and special-door toggle/lock/timer behavior. Remaining staffer2 gaps are full `DRD_STAFFER_PPD` quest state beyond the animation-book shanra marker, exact dlog/audit/logging side effects, exact player dig animation toggles, and live area-data smoke coverage.
- Area 29 `IDR_STAFFER2` animation-book subtype `6` now gates its one-time XP reward through a C-compatible fixed 100-byte `DRD_STAFFER_PPD` block: `staffer_ppd.shanra_state` at offset 64 is decoded/encoded through the legacy outer PPD framing, first use below state 3 marks it to 3 and grants the legacy `min(level_value(60) / 5, level_value(level) / 4)` XP, repeated uses only teleport to `(25,114)`, and core keeps the teleport separate from runtime persistence. Focused core/player/world tests cover the PPD layout, one-time state transition, typed outcome, and teleport-without-core-XP behavior. Remaining Staffer PPD gaps are the broader Brannington/Warr/Rhorun quest fields and exact questlog integration for every `staffer_ppd` state.
- Area 29 `IDR_STAFFER2` subtypes `2`, `3`, `4`, and `5` now port the C `staffer_mine`, `staffer_block_move`, and `staffer_spec_door` core paths: mine timer initialization sprite selection, player digging endurance/miner-profession cost, stage/sprite progression, sightblock removal at stage 3, opening at stage 8 with temporary blocker removal/voiding and delayed restore, blocked restore retry, restore of use/sightblock/map blockers, pushable block direction movement with legacy ground-sprite gates, action halt, original-home capture, idle timer return after two minutes, special-door marker-tile lock checks, open-state storage in `drdata[1]`, timer-counter storage in `drdata[39]`, map/item blocker flag toggling, ten-second auto-close, five-second blocked-close retry, and legacy exhausted/blocked/locked feedback are covered by focused core/server-path tests. Remaining staffer2 gaps are `DRD_STAFFER_PPD` quest state, exact random restore-delay parity, exact logging/dlog side effects, and live area-data smoke coverage.
- `src/system/drvlib.c` `fight_driver_attack_enemy` task admission now preserves the C `(!nomove || cdist == 2)` attack-task gate inside the Rust simple-baddy fight-task builder, with focused tests for no-move suppression at longer range and the distance-two exception. Current simple-baddy runtime still calls the weighted fight task path with movement enabled; the explicit gate is ready for no-move callers. Remaining fight-driver gaps include exact `is_valid_enemy` clan/hate policy, broader area-driver reuse, full enemy array ordering parity, exact global RNG parity, and exact NPC scheduling.
- `src/system/drvlib.c` `fight_driver_attack_enemy` spell-task coverage now includes a deterministic Rust slice for non-fireball simple-baddy spell setup: close visible enemies can trigger `AC_FREEZE` when the ported `freeze_value` modifier would apply, close enemies can trigger `AC_FLASH`, close/weak-shield situations can trigger `AC_WARCRY`, and distant visible enemies can trigger targeted `AC_BALL1` through the existing ball action bridge. Focused core tests cover setup, resource spending, target coordinates, and `lastfight` stamping for each action. The already-ported distance3, distance7, attackback, direct flee helper, and C-commented EarthRain task behavior are now covered by focused tests so the task list does not accidentally grow behavior beyond the C oracle. Remaining fight-driver gaps include the full randomized task scorer/priority ordering, exact fireball line-of-hit repositioning, exact ball intercept/random target offset parity, exact global RNG parity, and exact NPC scheduling.
- Area 17 `IDR_PICKCHEST = 80` now dispatches from the item-driver registry for the C `pick_chest` note branch: the area-17 libload guard is represented, occupied-cursor and missing-lockpick gates return legacy-shaped outcomes, lockpick detection uses carried inventory item ID `IID_AREA17_LOCKPICK`, `drdata[0]` maps to `palace_note1`, `palace_note2`, `palace_note3`, or `merchant_note1`, invalid kinds emit the legacy bug text, and the server grants the selected template to the cursor with the C lock-pick/found feedback. Focused core and workspace tests cover dispatch gates and kind mapping. Remaining Area 17 item gaps include dlog/audit parity and live area-data smoke coverage.
- Area 17 `IDR_PICKDOOR = 79` and `IDR_BURNDOWN = 82` are covered in the Rust item-driver/world paths: pick doors preserve the area-17 guard, player cursor-lockpick requirement, lock-picked feedback outcome, adjacent `NT_NPC/NTID_TWOCITY_PICK` notification fan-out, open/close state mutation, and 20-second auto-close timer; burn-down barrels preserve touch/too-hot/already-burned feedback, lit-torch ignition gate, burn countdown sprite/light/foreground transitions, timer rescheduling, Two-City pick notification fan-out, and the C `DRD_TWOCITY_PPD` thief quest side effect that promotes `thief_state` 13/14 to 14 and increments `thief_killed[0]` on successful ignition. Focused core/world tests cover dispatch gates, state mutation, timers, nearby notification delivery, and fixed-offset PPD mutation. Remaining Area 17 gaps are dlog/audit parity and live area-data smoke coverage.
- Area 17 `IDR_BOOKCASE = 85` now dispatches from the item-driver registry for C `bookcase`: zero-character and non-player calls no-op, library-key gating checks the exact `IID_AREA17_LIBRARYKEY`, locked bookcases emit both legacy feedback lines, random filler books use the 26-title legacy table including the Adygalah recipe special text, special color-puzzle books use per-player Two-City color state initialized as five `RANDOM(6)+1` colors, the solved-library book state flips after the first successful read with legacy green-title/reset text formatting, grants the one-time C library XP reward `min(level_value(level)/5, 80000)`, and `DRD_TWOCITY_PPD` now decodes/encodes the fixed 29-int C layout while preserving unknown Two-City fields and persisting `goodtile[5]` plus `solved_library` through the legacy outer PPD blob. Focused core tests cover locked/read dispatch, text layout, fixed-layout PPD offsets, XP formula, and blob replacement/append behavior. Remaining Area 17 bookcase parity gaps are exact global RNG parity and live area-data smoke coverage.
- Area 17 `IDR_SKELRAISE = 87` no-blood-bowl use now mirrors C `skelraise`: after the legacy dust feedback it clears stored raised-character id/serial bytes, marks the chair active, increments the chair sprite, dirties the sector, and schedules the ten-second zero-character timer so the chair resets through the existing serial-guarded timer path. Focused core/world tests cover dust activation/reset and `cargo build -p ugaris-server` passes. Remaining skelraise gaps are exact dlog/audit parity and live area-data smoke coverage.
- Area 17 `CDR_TWOSKELLY = 70` now has a Rust character-driver registry boundary matching the C `two.c` dispatch: tick, death, and respawn calls return the legacy handled code, `DRD_SKELLYDRIVER` is represented with a typed `last_talk/current_victim/alive` state carrier, and focused core tests pin the C driver/data IDs and return-code behavior. Full raised-skeleton dialogue, reward/item handoff, movement, death side effects, and Two-City PPD transitions remain.
- Area 17 `IDR_COLORTILE = 86` and `IDR_SKELRAISE = 87` now dispatch from the item-driver registry for C `colortile` / `skelraise`: color tiles initialize and compare the per-player Two-City `goodtile` state, wrong tiles randomize the colors and teleport the player to `(5,250)` with legacy feedback, skeleton chairs enforce blood-bowl cursor use, instantiate the legacy raised-skeleton templates, consume the cursor bowl, store active chair state plus spawned character ID/serial in `drdata[2]` and `drdata[4..12]`, poll every ten seconds, and reset the chair only when the spawned skeleton disappears or its serial no longer matches. Focused core/world/server tests cover dispatch gates, feedback, skeleton template selection, active-chair timer boundaries, and serial-guard reset behavior. Remaining Area 17 skeleton gaps are the raised skeleton character/dialogue drivers, exact dlog/audit parity, and live area-data smoke coverage.
- Area 37 `IDR_ARKHATA` subtype `0` now ports the C `pool_driver` item path: zero-character timer calls no-op, missing cursor and wrong-cursor uses emit the legacy feedback without consuming the cursor item, the exact Arkhata scroll item ID gates successful use, valid scrolls are consumed, the C `RANDOM(70)` reward table creates `Red_Scroll` for rolls 22/33, `Buddah_Statue` for roll 42, and otherwise emits the legacy vanished-in-pool text. Subtype `1` now ports the C stopwatch timer path: player use no-ops, zero-character timer callbacks reschedule after ten ticks, carried stopwatch items read the C-compatible `DRD_ARKHATA_PPD` clerk state/time fields, and the runtime emits the legacy active countdown, failed, or blank red feedback text. Focused core/server tests cover dispatch gates, scroll consumption, no-reward, reward creation, stopwatch no-op/reschedule behavior, PPD layout, logout omission, and stopwatch feedback formatting. Remaining Arkhata item gaps are broader Arkhata NPC quest state, exact global RNG parity, and live area-data smoke coverage.
- Area 6 `IDR_EDEMONLIGHT = 40` now dispatches from the item-driver registry for the C `edemonlight_driver` timer path: non-timer/player use no-ops, the driver reads explicit section power from `ItemDriverContext`, switches between legacy sprites `14191`/`14189`, applies `V_LIGHT = 200` only for powered sections below the C `249` threshold, returns the standard light-change outcome, reschedules every second, and existing map lights are included in startup light-timer registration. Focused core tests cover powered/off thresholds, no-op use, and startup scheduling. Remaining Earth Demon section gaps are full C module-global `sect[]` power propagation from loaders/switches/gates, gate/loader/door/block/tube branches, live area-data smoke coverage, and exact sound fan-out.
- `src/system/drvlib.c` `fight_driver_pulse_value` now has a Rust simple-baddy attack slice: visible attackable enemies in the legacy 5x5 pulse field are scored with the C low-resource/low-health/profitability gates, simple baddies set up `AC_PULSE` through the existing `do_pulse` bridge when pulse is worthwhile, and healthy targets fall through to melee/path behavior. Focused core tests cover profitable pulse setup and healthy-target rejection. Remaining fight-driver gaps include the full randomized task scorer/priority ordering, exact fireball line-of-hit repositioning, exact ball intercept/random target offset parity, exact global RNG parity, and exact NPC scheduling.
- `src/system/drvlib.h` item-driver ID surface now has Rust constants for the remaining non-spell legacy `IDR_*` values through `IDR_SKELETON_KEY`, including clan, pent, earth/fire demon, dungeon, swamp, LQ, strategy, warped, staffer, Teufelheim, saltmine, and lab driver IDs. `IDR_BURN = 1008` is now represented in the spell identity constants. Focused core tests pin representative previously missing values against the C header so future driver ports can reference typed Rust constants instead of ad hoc numbers. Behavioral dispatch for those newly named drivers remains unsupported until their modules are ported.
- `src/system/saltmine.c` `IDR_SALTMINE_ITEM` now ports the player-facing ladder and saltbag boundaries in addition to the existing door rejection: ladder use returns a typed runtime outcome with the legacy 20-ladder index, the server tracks the C 24-hour ladder reuse window per player and emits the legacy already-used feedback, saltbag use blocks occupied cursors, emits the C no-earned-salt feedback, and can instantiate a legacy-shaped `salt` cursor item with ounce count, sprite tier, value scaling, and feedback when runtime pending salt exists. Focused core tests cover ladder/saltbag dispatch and cooldown state, and `cargo build -p ugaris-server` passes. Remaining Saltmine gaps are `DRD_SALTMINE_PPD` fixed-layout persistence, monk Gatama/Govida/worker character drivers, worker-driven salt increment/drop-off, ladder startup numbering, exact notify-area fan-out, and dlog/audit/live-data parity.
- `src/area/23_24/strategy.c` item-driver registry boundary now dispatches `IDR_STR_MINE`, `IDR_STR_STORAGE`, `IDR_STR_SPAWNER`, `IDR_STR_DEPOT`, `IDR_STR_TICKER`, and `IDR_NOSNOW` as C-handled no-op outcomes instead of falling through to unsupported, preserving the legacy `it_driver` return code of `1` for the Strategy module while leaving mission ownership, worker spawning, storage/depot resource movement, and snow tile mutation for later strategy-system work. Focused core tests cover dispatch and return-code parity.
- Area 25 `IDR_WARPTELEPORT = 112` now dispatches through the item-driver registry for the C `warpteleport_driver` path: the warped driver family is gated behind the legacy area-25 libload requirement, player use with `drdata[0] == 0` maps `drdata[1]` values 1..5 to the exact fixed destinations `(242,252)`, `(247,66)`, `(251,16)`, `(152,7)`, and `(183,250)`, keyed teleports require cursor `IID_AREA25_TELEKEY`, map the portal/sphere kind pair through the exact 25-entry C target table, teleport same-area, destroy the cursor sphere plus all inventory spheres only after successful teleport, and emit the legacy missing/success feedback. Focused core/server-path tests cover the destination tables, area guard, missing cursor sphere, and sphere consumption. Remaining warped gaps are `IDR_WARPTRIALDOOR`, `IDR_WARPBONUS`, `IDR_WARPKEYSPAWN`, `IDR_WARPKEYDOOR`, warped PPD persistence, exact busy/bug feedback, and broader area-25 NPC/dialogue behavior.
- Area 25 `IDR_WARPKEYDOOR = 116` now dispatches through the warped item-driver path: it preserves the area-25 libload guard, zero-character handled no-op, same-tile bug feedback classification, exact carried `IID_AREA25_DOORKEY` inventory requirement without skeleton-key/keyring fallback, through-door same-area teleport, consumed key item, cardinal facing reversal, and legacy locked/vanished feedback. Focused core/world/server compile checks cover dispatch, return-code classification, teleport, key destruction, and direction flip. Remaining warped gaps are `IDR_WARPTRIALDOOR`, `IDR_WARPBONUS`, full warped PPD persistence, exact failed-teleport `Oops` feedback, and broader area-25 NPC/dialogue behavior.
- `src/system/drvlib.c` simple-baddy warcry task admission now matches C `fight_driver_attack_enemy`: close simple baddies queue `warcry` when the target can accept `IDR_WARCRY` or the caster needs lifeshield, without Rust-only pre-filtering by the computed speed modifier. Focused core coverage verifies weak-but-admissible warcry still sets up `AC_WARCRY`. Remaining fight-driver gaps include exact task RNG/global RNG parity, broader scheduler cadence parity, and full side-effect parity for completed warcry hits.
- `src/system/drvlib.c` simple-baddy visible enemy selection now preserves the C `person_cmp` hurt-me ordering in the runtime attack loop: visible enemies that hurt the baddy are tried before merely sighted visible enemies even when farther away, while arbitrary non-C priority magnitudes remain ignored for target choice. Focused core coverage verifies hurt-me-before-distance selection and the existing visible distance/facing ordering behavior. Remaining fight-driver gaps include exact task RNG/global RNG parity, broader scheduler cadence parity, and full side-effect parity for completed attacks/spells.
- Area 25 fixed `IDR_WARPTELEPORT` busy-target feedback now matches C `warpteleport_driver`: failed same-area movement from a plain portal returns a typed busy outcome instead of collapsing to `Noop`, and the server emits `Target is busy, please try again soon.`. Focused world coverage verifies the blocked destination keeps the player in place and preserves the feedback boundary.
- `src/system/drvlib.c` `fight_driver_remove_enemy` now has a reusable Rust simple-baddy state helper and the existing world invisible-pursuit cleanup path routes through it. The helper preserves the C boundary return shape for present/missing/non-simple-baddy states with focused core tests. Remaining fight-driver gaps include broader area-driver reuse of explicit enemy removal, exact task RNG/global RNG parity, broader scheduler cadence parity, and full side-effect parity for completed attacks/spells.
- Area 25 `IDR_WARPTRIALDOOR = 113` now dispatches through the warped item-driver path under the legacy area-25 libload guard: zero-character calls preserve the C handled no-op return code, the world derives the paired trial-door room bounds by scanning up to 14 tiles through unblocked map cells like C, cached `drdata[2..7]` metadata is honored when present, inside-room use emits the wrong-side classification, non-simple-baddy occupants block opening as fighting-noises busy, successful use returns a typed `warped_fighter` spawn request with room bounds/target coordinates and teleports the player one tile through the source door. Focused core/world tests cover dispatch, wrong-side/busy gates, return-code parity, room discovery, and player teleport. Remaining warped-trial gaps are runtime `warped_fighter` template spawning/raising/equipment, `DRD_WARPFIGHTER` state initialization, exact bug feedback lines, dlog/audit parity, and live area-25 data smoke coverage.
- Area 25 `IDR_WARPTRIALDOOR` room discovery now also mirrors the C side effect that caches discovered bounds and the partner door ID into the source item's `drdata[2..7]` before executing the door logic, so subsequent uses take the cached path just like `warptrialdoor_driver`. Focused world coverage verifies the cache bytes alongside the existing player teleport/spawn boundary.
- `src/system/death.c` `hurt` now has a reusable Rust world primitive for the core damage path: non-negative damage clamping, legacy armor reduction via `V_ARMOR`/armor divisor/armor percent, F-demon back-hit reduction, lifeshield absorption through shield percent, `CF_IMMORTAL` suppression, `CF_NODEATH` survival at one HP, death flag/death-count marking for the default kill path, `CF_UPDATE` marking, and `NT_GOTHIT`/`NT_DIDHIT`/nearby `NT_SEEHIT` driver-message fan-out are covered by focused core tests. Runtime `EF_EARTHRAIN` ticks now use the C `hurt(cn, dam, 0, 8, per, per + 25)` armor/shield percentages, retained ball strike ticks now use the C `hurt(cn, dam, cc, TICKS * 2 * 10 / 4, 30, 85)` path, retained fireball explosions now use C `hurt(co, dam, cn, 10, 50, 70)`, and retained Earth Demon ball impacts now use C `hurt(co, dam * POWERSCALE, cn, 6, 75, 50)` instead of raw HP subtraction. Remaining hurt/death gaps are hardkill weapon gating, magic-shield show-effect creation, area-specific death/save branches, `kill_char` body/drop/respawn behavior, hate/sector side effects, player sound/log fan-out, and routing all remaining direct-damage call sites through the primitive.
- Area 37 `IDR_ARKHATA` now dispatches the C `arkhata_item_driver` key-assembly subtype (`drdata[0] == 2`): carried-only/nonzero-character gates, cursor-required and wrong-piece feedback outcomes, pairwise key-part combinations for `AKEY1 + AKEY2`, `AKEY2 + AKEY3`, and final `AKEY12/AKEY23` combinations, legacy sprite/template-ID mutation, final `Knoger Key 1` name/description, and cursor-piece consumption are covered by focused core/world tests. Remaining Arkhata item-driver gaps are pool subtype `0`, stopwatch subtype `1`, area 37 NPC quest/dialogue drivers, PPD persistence, and exact logging/audit side effects.
- `src/system/poison.c` poison timer damage now routes through the reusable Rust `hurt` primitive with the legacy `hurt(cn, POWERSCALE / 3, 0, 1, 0, 50)` parameters, so poison ticks apply lifeshield absorption, death/update handling, and driver hit messages instead of directly subtracting HP. `src/system/act.c` slowdeath tile damage now uses the same primitive with C `hurt(cn, 50, 0, 1, 0, 0)` underwater and `hurt(cn, 100/250, 0, 1, 25, 66)` hazard parameters, and area 2 `spiketrap_driver`/`burn_char` damage now use C `hurt(cn, drdata[1] * POWERSCALE, 0, 1, 75, 75)` and `hurt(cn, 20 * POWERSCALE, 0, 1, 50, 75)` semantics. Focused core tests cover rescheduling, tick weakening, shield reduction, trap/burn armor-lifeshield reduction, and driver hit messages.
- C `act_attack` direct melee completion now routes damage application through the reusable Rust `hurt` primitive with the legacy `hurt(co, dam * POWERSCALE / ATTACK_DIV, cn, ATTACK_DIV, per, 75 + per / 4)` parameters. The pure `act_attack` resolver now reports the C hit damage parameters without mutating HP, and `World::complete_attack_with_rolls` applies armor/lifeshield reduction, death/update handling, and `NT_GOTHIT`/`NT_DIDHIT` driver messages through `apply_legacy_hurt`. Focused core tests cover the non-mutating resolver and world-level hit notification side effects.
- Area 25 warped persistent state now has a C-compatible `DRD_WARP_PPD` codec for `struct warped_ppd`: base, points, 50 bonus location IDs, 50 bonus last-used/base markers, and `nostepexp` are decoded/encoded in the fixed legacy layout and integrated with the outer PPD blob replacement/append path while preserving unknown blocks. Focused core tests cover fixed offsets, round trips, and outer PPD framing. Remaining warped gaps include `IDR_WARPBONUS` reward execution, `IDR_WARPTRIALDOOR`, full warped PPD runtime mutation, exact busy/bug feedback, and broader area-25 NPC/dialogue behavior.
- `src/system/act.c` spell completion damage for self-targeted fireball/firering, pulse, and warcry now routes through the reusable Rust `hurt` primitive with the legacy `hurt(co, dam, cn, 10, 30, 85)`, `hurt(co, str, cn, 1, 0, 100)`, and `hurt(co, dam, cn, 1, 0, 0)` parameters respectively, so armor/lifeshield reduction, death/update handling, and `NT_GOTHIT`/`NT_DIDHIT` driver messages are applied consistently. Focused core tests cover firering armor/lifeshield reduction, pulse lifeshield absorption, and warcry hit notifications.
- C `hurt` `CF_FDEMON` back-attack gating is now represented in `World::apply_legacy_hurt`: damage is reduced by armor first, fire demons take only one percent of post-armor damage unless the causing character stands on the exact legacy back tile for the target's cardinal facing, and magic-shield absorption then applies to the reduced value like C. Focused core tests cover blocked front damage and full back damage.
- C `hurt` `CF_HARDKILL` special-weapon gating is now represented in `World::apply_legacy_hurt`: hard-to-kill targets take zero post-armor damage unless the causing character has legacy `IID_HARDKILL` equipped in `WN_RHAND` and unsigned `drdata[37]` is at least the target level. Focused core tests cover no weapon, under-leveled weapon, and qualifying weapon damage.
- C `hurt` magic-shield hit visuals are now represented in `World::apply_legacy_hurt`: when lifeshield absorbs damage, the target has an effective `V_MAGICSHIELD`, and no `EF_MAGICSHIELD` visual is already attached, Rust creates the legacy three-tick character-attached shield effect with light `16` and strength `0`. Focused core tests cover creation and duplicate suppression.
- C `hurt` post-damage regeneration delay is now represented in `World::apply_legacy_hurt`: successful hurt calls stamp the target `regen_ticker` from the world tick alongside hit notifications, matching the legacy `ch[cn].regen_ticker = ticker` side effect. Focused core tests cover the tick stamp through the armor/lifeshield damage path.
- C `hurt` death notification fan-out is now represented in `World::apply_legacy_hurt`: kills queue legacy `NT_DEAD` driver messages to characters in the C `notify_area` 32-tile square with killed-character and cause IDs. Focused core coverage pins the inclusive 32-tile boundary and excludes characters outside it. Remaining hurt/death gaps are full `kill_char` body/drop/respawn behavior, save/area-specific death branches, hate/sector side effects, and exact notify-sector iteration parity.
- Area 36 Caligar skeleton-door PPD now has the C `skelly_dead_driver` lock-bit core represented on `PlayerRuntime`: legacy skeleton home coordinates map to `door_flag[0..2]` bits, repeated kills report the already-unlocked classification, the third bit reports the fully-unlocked classification, unmapped coordinates preserve the bug boundary, and the existing fixed-layout `DRD_CALIGAR_PPD` codec persists the bytes. `CDR_CALIGARSKELLY = 124` is represented in the character-driver registry, and lethal Rust hurt events now wire player kills of Caligar skeletons into the same PPD mutation plus legacy feedback messages. Focused core/server tests cover all three result classes, the dual-X third-door mapping, partial unlock feedback, full unlock feedback, and repeated-kill feedback.
- Area 37 Arkhata stopwatch logout persistence now mirrors the C quest warning that stopwatches vanish when leaving/logging off: server logout snapshot construction filters carried/cursor `IDR_ARKHATA` subtype `1` stopwatch items out of saved item rows and clears matching saved inventory, cursor, and current-container references while preserving other Arkhata quest items. Focused server tests cover inventory and cursor snapshot filtering. Remaining Arkhata stopwatch gaps are exact area-leave cleanup during live cross-area transfer and broader Arkhata NPC quest state.
- `IDR_SPECIAL_POTION` fun-potion area feedback now uses the C `hisname`/`himname` pronoun behavior for male, female, and neutral characters instead of hard-coded male text. Focused server tests cover the affected mug and knuckle messages.
- `src/system/drvlib.c` `fight_driver_regen_value` area-33 guard is now represented in the Rust simple-baddy weighted fight-task builder: NPCs that are below max HP/mana no longer add the regeneration fight task while fighting in area 33, matching the C `if (areaID == 33) return 0` branch. Focused core tests cover normal-area regeneration task admission and area-33 suppression.
- C `hurt` player hit/death sound fan-out is now represented in `World::apply_legacy_hurt`: player targets with at least one `POWERSCALE` of post-shield HP damage queue legacy male/female ouch sounds `9`/`32`, and player targets crossing the death threshold queue legacy male/female death sounds `4`/`33`, including the C `CF_NODEATH` case where the sound is emitted before the no-death save. Focused core tests cover male hit, female lethal damage, nearby-player fan-out, and nodeath death-sound behavior.
- C `EF_BURN` effect ticking now routes recurring burn damage through the reusable Rust `hurt` primitive with legacy `hurt(cn, POWERSCALE / 6 + strength, 0, 30, 50, 75)` parameters, removes stale attached burn effects when the target is gone/unusable, and keeps the existing expiration path. Focused core tests cover recurring damage, driver hit messages, and stale-effect cleanup.
- Area 2 `IDR_PARKSHRINE = 23` now dispatches from the Rust item-driver registry for the C `parkshrine_driver` path: non-character calls no-op, valid shrine numbers `1..3` return a typed memorization outcome, invalid `drdata[0]` returns the legacy bug feedback path, `PlayerRuntime` now preserves the fixed 17-int `DRD_AREA3_PPD` layout and updates Kelly shrine-found fields at the C offsets, the outer PPD blob decoder/encoder round-trips/replaces/appends this block, and the server runtime emits the exact memorized/familiar/bug feedback text. Focused core tests cover dispatch, fixed-layout PPD, and outer blob integration.
- Legacy `sound_area` fan-out now has a reusable queued runtime path: `World` can queue already target-resolved `SV_SPECIAL` sound records using the ported distance/pan/sector math, the server tick loop drains and sends them to matching player sessions, targeted fireball and self-targeted firering completions queue C sound type `5`, fireball explosions now create the legacy `EF_EXPLODE` visual with base sprite `50050` and queue sound type `6`, and slowdeath/bubble tile-special hooks queue their legacy sound types. Focused core tests cover queued sound drainage and fireball explosion visual/sound behavior. Remaining sound work is wiring all remaining legacy `sound_area` call sites and exact random sound variants.
- `src/module/alchemy.c` `IDR_FLASK` now dispatches from the Rust item-driver registry for the C flask core gates and ingredient-add path: timer/non-carried calls no-op, Teufelheim arena use returns the legacy no-potion block, finished shaken potions reject additional cursor ingredients, empty unshaken flasks emit the empty-shake feedback, cursor items must have `IID_ALCHEMY_INGREDIENT`, full flasks reject inserts, invalid ingredient types report `BUG # 231...`, valid ingredients update the C unfinished-potion sprite/name/description by flask size and used count, increment `drdata[1]` plus the per-ingredient counter at `drdata[type + 10]`, destroy the cursor ingredient, dirty inventory, and emit the legacy `You put ... into the flask.` feedback with focused core/runtime coverage. Remaining flask/alchemy gaps are the full `mixer` recipe table, `mixer_use` potion drinking path, achievement/dlog side effects, and exact recipe RNG/global parity.
- Direct melee attack completion now queues C `sub_attack` sound types through the same runtime sound path: successful direct hits emit `7`, unarmed/one-sided-weapon misses emit `8`, and weapon-vs-weapon misses emit the legacy clash alternatives `34`/`35` from an explicit `RANDOM(2)`-style clash roll instead of deriving the choice from the d100 attack roll. Focused core tests cover hit/miss sounds and independent clash-roll selection. Remaining melee RNG parity gap is replacing the scaffolded tick/id attack and clash rolls with exact global C RNG wiring.
- `src/system/drvlib.c` `fight_driver_add_enemy` map-coordinate guard is now represented in the Rust simple-baddy standard-message path: hurt and non-hurt enemy admission rejects targets with legacy out-of-map coordinates (`x < 1`, `y < 1`, or `>= MAXMAP`) before storing enemy memory, while valid out-of-sight hurt attackers are still retained with hidden tracking. Focused core tests cover the coordinate rejection and corrected hidden-attacker fixture.
- C `act_warcry` sound parity is restored: Rust no longer emits the bless sound `29` when warcry completes, matching the legacy action path which notifies the spell and applies effects without a `sound_area` call. Focused core coverage now asserts warcry completion leaves the pending sound queue empty.
- Area 18 `IDR_BONEHINT` now dispatches behind the existing libload area guard: carried-only diary use initializes the legacy `drdata[1..3]` hint selector with runtime RNG, returns typed hint output, emits the C `Rune Diary, Page ...` and `Used the rune ...` feedback lines, and keeps zero-character/non-carried calls as no-ops. `PlayerRuntime` now carries the C-shaped `DRD_RUNE_PPD` state (`used[32]` plus `special_exec[25]`), generates `special_exec` values with the C constraints, decodes/encodes the fixed 228-byte block through the legacy outer PPD framing, and preserves high unsigned `used` words. Focused core/server tests cover dispatch, initialization, generation constraints, hint text lookup, and PPD round-trip. Remaining area 18 rune gaps are rune-use quest progression, full rune execution recipes, exact global RNG parity, and adjacent bone ladder/holder/wall quest behavior.
- C spell-completion sound fan-out now covers the shared bless/warcry sound type `29`: successful `AC_BLESS_*` completions queue the legacy sound after installing the bless spell, and `AC_WARCRY` completions now match C by succeeding and queuing sound even when no target is debuffed due to sound blockers. Focused core tests cover bless sound, affected warcry sound, and sound-blocked warcry success without target spell installation.
- C `act_freeze` completion sound/completion semantics are now represented in the Rust action bridge: `AC_FREEZE` queues legacy `sound_area(..., 31)` after passing the no-magic gate and completes successfully even when no nearby target accepts a freeze spell, matching the legacy scan/notify/sound path. Focused core tests cover target-free completion and the queued sound record.
- C `door_driver` sound fan-out is now represented in the world door mutation path: successful character-triggered door toggles queue legacy `sound_area(..., 3)`, zero-character timer/automatic door toggles queue `sound_area(..., 2)`, and synchronized double doors inherit the same per-door behavior through the shared toggle primitive. Focused core tests cover manual open/close sounds and timer auto-close sound emission.
- `src/module/simple_baddy.c` `NT_NPC` helper alert forwarding now preserves the C zero-target boundary: matching `helpid` messages with a valid reporting character are converted to `AddEnemy` outcomes even when `dat3 == 0`, instead of being dropped by a Rust-only target guard. Focused core coverage pins the zero-target outcome; runtime same-group/non-self validation remains in the world applier like the existing helper-alert path.
- C `tile_special_check` underwater oxygen sound variance now matches the legacy `44 + RANDOM(3)` shape instead of always queuing sound `44`, and non-underwater `MF_SLOWDEATH` hazard damage no longer emits the Rust-only sound `66` because the C path only calls `hurt`. Focused core tests cover bubble sound queueing and hazard silence.
- C `ef_ball` strike sound fan-out is now represented in retained ball projectile ticking: when `check_strike_near` finds at least one valid target on a `ticker & 7 == 0` scan, Rust queues legacy sound type `30` at the caster position exactly once for that scan while preserving the existing strike visual/damage cadence. Focused core tests cover sound emission and eighth-tick suppression.
- `src/module/simple_baddy.c` post-combat noncombat flow now also preserves the C `drinkspecial` ordering for NPCs already standing at their day/night post: poison-clearing runs after post-facing/home updates and before regenerate/self-spell/friend-bless/final-idle fallback, instead of being skipped on the at-post branch. Focused core coverage verifies active `IDR_POISON0` removal at the day post.
- Area 15 `CDR_SWAMPCLARA` and `CDR_SWAMPMONSTER` now have Rust character-driver registry coverage matching `src/area/15/swamp.c`: the legacy IDs are represented, tick/death/respawn dispatch returns the C-compatible handled code, and focused core tests pin the boundary. Actual Clara dialogue/quest state, swamp monster death quest progression, and hardkill weapon upgrade side effects remain.
- Area 15 `CDR_SWAMPMONSTER` death side effects now port the C `monster_dead` hardkill/quest slice: killed swamp monsters trigger Clara quest progression from `clara_state` 12/13 to 14 for player killers with the legacy `Well done. Clara will be proud of thee!` feedback, and midnight kills at the three legacy stone-circle rectangles upgrade an eligible right-hand non-driver weapon into `IID_HARDKILL`, increment `drdata[37]` by 12, set the corresponding `drdata[36]` circle bit, mark the item `IF_QUEST`, and emit `Your <item> starts to glow.` with focused core coverage plus server hook compilation. Remaining Area 15 gaps include full Clara dialogue/text-analysis/repeat handling, questlog rewards/done transitions, give-back behavior, exact military point side effects, swamp monster runtime AI parity beyond simple-baddy delegation, and live area-data smoke coverage.
- Area 12 `IDR_MINEDOOR = 62` now has a first Rust slice for C `minedoor`: the area-12 libload guard is represented, player use resolves the paired source/target door by legacy `drdata[0]`/`drdata[1]`/usable-source metadata, computes the opposite-side teleport destination from C direction IDs, applies same-area teleport with the legacy collapse fallback coordinates, and exposes missing-target boundaries with focused core/world tests. Timer callbacks now return a typed mine-door timer outcome, reschedule the 30-second callback, preserve the C surrounding-wall sightblock guard before opening, optionally close the current source door only when its surrounding movement blockers are intact and the legacy `RANDOM(20) == 0` cadence hits, mutate source-door sprites/`IF_USE`/`drdata[3]`, mark affected sectors dirty, and live mine doors are scheduled during zone startup with focused world tests. Remaining mine-door gaps are exact global RNG parity, sector invalidation detail parity, and collapse feedback text.
- `src/module/transport.c` `DRD_TRANSPORT_PPD` persistence and destination dispatch boundary are now represented in Rust: `PlayerRuntime` encodes/decodes the legacy 8-byte `unsigned long long seen` mask under `MAKE_DRD(DEV_ID_DB, 44 | PERSISTENT_PLAYER_DATA)`, outer PPD blob load/save replaces existing transport blocks or appends newly discovered masks while preserving unrelated blocks, and `IDR_TRANSPORT` now distinguishes C `act2 == 0` UI-open/discovery from nonzero destination travel specs through a typed `TransportTravel` outcome. Same-area transport destinations now resolve through the legacy 26-entry table, reject unseen destinations with C-shaped text, enforce the Brannington Arches-only gate, send `SV_MIRROR`, and move the character with the existing same-area drop path; cross-area destinations remain at the handoff boundary with the existing target-area-down feedback. Transport UI packets now include C-shaped four-byte clan access masks for directly represented character clan membership, and clan-hall travel specs `65..96` resolve through the legacy 32-entry clan coordinate table with same-area movement and C-shaped rejection text for non-members. New teleport discovery now marks the same legacy exploration achievement thresholds for major cities, all non-empty Rodney map teleports, and earth underground teleports in the runtime achievement state. Focused core/server tests cover the fixed layout, outer block framing, append path, item-driver travel dispatch, same-area movement, unseen-destination rejection, cross-area handoff, direct clan access masks, clan-hall movement, clan rejection, and achievement threshold bitmasks. Remaining transport gaps are clan repository/alliance access beyond direct membership, achievement protocol/persistence beyond runtime markers, exact mirror state persistence, and real cross-area transfer.
- Production transport mirror fallback now uses the same `RANDOM(26) + 1` shape as C for `CL_TELEPORT` specs with mirror `0` or out of range instead of the earlier deterministic mirror-1 placeholder, while preserving injected deterministic tests for invalid/high mirror specs and clamped test rolls. Remaining transport gaps are clan repository/alliance access beyond direct membership, achievement protocol/persistence beyond runtime markers, exact mirror state persistence, and real cross-area transfer.
- Area 31 Warr Mines `IDR_PICKBERRY` now has runtime application for C `pick_berry`: player flower cooldowns use a fixed-layout C-compatible `DRD_FLOWER_PPD` block (`ID[100]` plus `last_used[100]`), persisted through the legacy outer PPD blob, ripe-time gates honor `P_HERBALIST` thresholds at 24/12/8/4 hours, valid berry/flower kinds instantiate `lizard_brown_berry` / `picked_flower_h` / `picked_flower_i` / `picked_flower_j` to the cursor, cooldown state is marked only after successful cursor placement, and legacy cursor/not-ripe/bug feedback is wired with focused core/server tests. Remaining Warr Mines area-driver gaps are exact dlog/audit integration and broader area 31 NPC driver behavior.
- Area 31 `IDR_PICKBERRY = 129` now has a Rust item-driver boundary matching the C `pick_berry` entry checks: area 31 guard, zero-character no-op, occupied-cursor rejection, picked-kind decoding from `drdata[0]`, and legacy flower-location ID shape `x + (y << 8) + (areaID << 16)` are represented as typed outcomes with focused core tests. Runtime reward creation, herbalist-dependent cooldowns, and `DRD_FLOWER_PPD` persistence are covered by the Warr Mines runtime slice above.
- Same-area transport mirror selection is now retained in `PlayerRuntime`: login initializes the runtime mirror from the configured/DB login mirror, successful same-area `CL_TELEPORT` updates it after movement and before sending `SV_MIRROR`, and logout snapshot saves use the retained transport mirror while preserving the current-area guard mirror. Focused server tests cover fallback configured mirror saving and runtime transport mirror persistence. Remaining transport gaps are clan repository/alliance access beyond direct membership, achievement protocol/persistence beyond runtime markers, and real cross-area transfer.
- `src/system/drvlib.c` `fight_driver_attackback_value` / `attack_back_driver` now has a Rust simple-baddy attack slice: visible attackable targets with cardinal facing can trigger a back-positioning walk when the target is idle long enough or has a character occupying the front tile, the back tile must be in-bounds and unblocked, diagonal-facing targets are ignored like C, and successful setup stamps `lastfight` before the normal adjacent attack/path fallback. Focused core tests cover front-occupied back movement and blocked-back fallback. Remaining fight-driver gaps include full randomized task scorer/priority ordering, distance3/distance7 spacing tasks, fireball line-of-hit repositioning, flee/secure movement details, exact global RNG parity, and exact NPC scheduling.
- `src/module/simple_baddy.c` notsecure day/night movement now preserves the legacy target distinction: when `notsecure` baddies have day/night posts but do not teleport, Rust follows C by walking toward the character's original rest/home tile (`tmpx/tmpy`) with `mindist` instead of walking directly to the day/night post. Focused core coverage verifies the day-post path. Remaining simple-baddy noncombat gaps include broader live area-driver smoke coverage, exact global RNG parity, and exact NPC scheduling cadence.
- `src/system/drvlib.c` simple-baddy fight-task silliness rolls now use retained world RNG state on the default attack-processing path instead of the previous constant-roll scaffold, while preserving injectable deterministic random sources for tests. Focused core tests cover the C-style LCG helper, seed consumption through direct simple-baddy attack processing, and the existing 114-test simple-baddy subset. Remaining simple-baddy fight-driver gaps include exact process-global C RNG parity across all systems, broader area-driver reuse, and exact NPC scheduling cadence.
- Area 15 `IDR_SWAMPSPAWN = 75` now dispatches through the legacy area libload guard for C `swampspawn`: zero-character timer-only execution, one-time editor-sprite replacement with base sprite stored in `drdata[16..20]`, active spawned-character id/serial checks using `drdata[4..12]`, two-minute last-spawn cooldown from `drdata[12..16]`, nearby-player square-distance trigger, ground-sprite-dependent animation stop frames, swamp template selection for `swamp25n`/`swamp27n`/`swamp29n`/`swamp31n`, runtime template instantiation, spawned id/serial/tick bookkeeping, dirty-sector marking, and legacy 3-tick/1-second rescheduling are covered by focused core/world tests plus server build verification. Remaining area 15 gaps include Clara/NPC dialogue, swamp quest PPD, exact global RNG parity for adjacent swamp drivers, and live area-data smoke coverage.
- Area 34 `IDR_TEUFELARENAEXIT = 141` now dispatches through the Teufelheim item-driver boundary for C `teufelarenaexit`: zero-character calls no-op, non-full-health exits return the legacy blocked feedback, successful use teleports same-area characters to `(206,231)`, and Teufelheim item IDs are guarded to area 34 like the legacy module load. Focused core/world tests cover the dispatch gates and teleport side effect. Remaining Teufelheim item gaps include `IDR_TEUFELRATNEST`, rat/gambler character drivers, exact target-busy area feedback, and live area-data smoke coverage.
- Area 34 `IDR_TEUFELDOOR = 137` now dispatches through the Teufelheim item-driver boundary for C `teufeldoor`: zero-character calls no-op, demon-sprite class gates preserve the legacy no-humans/no-beggars/only-nobles feedback, diagonal use no-ops, target tiles are computed as the opposite side of the door with legacy bounds bug feedback, successful same-area teleports use extended drop placement and reverse cardinal facing, and busy destinations return typed feedback. Focused core/world tests cover sprite gates, target computation, teleport, and facing reversal. Remaining Teufelheim item gaps include `IDR_TEUFELRATNEST`, rat/gambler character drivers, exact area-log wording for busy/bug fan-out, and live area-data smoke coverage.
- Area 34 `IDR_TEUFELARENA = 139` now dispatches through the Teufelheim item-driver boundary for C `teufelarena`: zero-character calls no-op, subtype `1` selects the eight legacy arena entry coordinates through a runtime `RANDOM(8)` seam, requires earth-demon sprite `27`, enforces max level 38, blocks worn non-demon-suit equipment with more than one counted enhancement or quest/bound flags, same-area teleports with extended placement, clears active spell slots/effects on successful entry, and emits the legacy suit/level/equipment/spell-removal/target-busy feedback. Focused core/world/server-path tests cover destination selection, gates, equipment rejection, teleport, and spell cleanup. Remaining Teufelheim item gaps include `IDR_TEUFELRATNEST`, rat/gambler character drivers, exact area-log wording for busy/bug fan-out, exact global RNG parity, and live area-data smoke coverage.
- Area 2 `IDR_FIREBALL` fireball-machine item driver now dispatches through the Rust item-driver registry: it decodes C `drdata[0..3]` projectile offsets/power/frequency, creates retained `EF_FIREBALL` projectiles with no caster, C light/strength/start/target/one-second lifetime fields, queues legacy `notify_area(..., NT_SPELL, V_FIREBALL, fn)` driver messages around the machine, and reschedules zero-character timer calls by the configured frequency. Focused core/world tests cover direct decode, timer rescheduling, retained effect shape, and nearby spell notification fan-out. Remaining fireball-machine gap is live area-data smoke coverage.
- Area 17 `IDR_PICKDOOR` lockpick gating now matches C `pick_door`: player door picking requires the Area 17 lockpick on the cursor, not merely somewhere in inventory, while `IDR_PICKCHEST` keeps the existing carried-lockpick gate. Runtime item-driver context now distinguishes carried and cursor lockpicks, and focused core coverage pins inventory-only door rejection plus cursor-lockpick success.
- Area 31 `IDR_LIZARDFLOWER = 130` now dispatches from the Rust item-driver registry with the C `libload.c` area-31 guard and ports `flower_mixer`: carried-only/nonzero-character use, no-cursor and wrong-cursor legacy feedback outcomes, cursor-driver validation, `drdata[0]` bitwise flower mixing, partial scuba-potion sprite/description mutation, completed `Scuba Potion` mutation to sprite `11188` plus `IDR_OXYPOTION`, cursor component destruction, item dirtying, and the legacy bottle/finished feedback flags. Focused core/world tests cover area gating, validation, mutation, and cursor consumption. Remaining Warrmines item gaps include `IDR_PICKBERRY` flower/berry picking PPD cooldowns, exact dlog/audit behavior, and live area-31 data smoke coverage.
- `src/system/drvlib.c` / `src/system/effect.c` simple-baddy direct-ball task selection now ports the C `calc_steps_ball(cn, from, target) > tile_dist * 2 - 5` gate before adding the weighted `ball` task. Rust traces the same fixed-point half-tile projectile path, treats `MF_TMOVEBLOCK` and non-`MF_FIRETHRU` `MF_MOVEBLOCK` tiles as blockers unless occupied by the caster, and preserves the existing C-shaped random target offset when the task is selected. Focused core tests cover successful distant ball setup, random offset use, and suppression when an early wall collision would make the projectile fail. Remaining fight-driver gaps include full randomized task scorer/priority ordering, distance3/distance7 spacing task parity, flee/secure movement details, exact global RNG parity, and exact NPC scheduling.
- `src/system/drvlib.c` `distance_driver` distance-3 spell-spacing now has a Rust simple-baddy attack slice for the C active Flash/Freeze spacing path: visible enemies are checked after offensive spell/pulse attempts and before attack-back/melee fallback, active Flash blocks only when the existing carried `IDR_FLASH` spell prevents adding another Flash, active Freeze uses the existing freeze-value/may-add gates, and already-at-distance-3 cases perform the C `do_idle(TICKS / 4)` fallback while stamping `lastfight`. Focused core tests cover the active-Flash spell-slot gate and already-at-distance idle behavior. Remaining fight-driver gaps include full randomized task scorer/priority ordering, distance7/fireball spacing, fireball line-of-hit repositioning, flee/secure movement details, exact global RNG parity, and exact NPC scheduling.
- `src/system/drvlib.c` fight-driver task ordering now uses the injected runtime RNG for the C `RANDOM(sillyness)` value jitter before sorting tasks, instead of the previous deterministic placeholder. Focused core tests cover task-order jitter changing simple-baddy action choice and preserve the existing ball target-offset random rolls after ordering. Remaining fight-driver gaps include exact global RNG stream wiring, full randomized task scorer parity beyond currently represented tasks, fireball line-of-hit repositioning edge cases, flee/secure movement details, and exact NPC scheduling.
- `src/system/do.c` `char_swap` and `src/system/drvlib.c` `walk_swap_or_use_driver` now have Rust world primitives: failed walk attempts can turn, swap with idle visible `CF_PLAYER`/`CF_PLAYERLIKE`/`CF_ALLOWSWAP` characters while preserving peace/underwater gates and map occupancy, then fall back to item use when no swap is possible. Focused core tests cover successful swaps, invisible-target rejection, and use fallback. Remaining swap-movement gaps are the full `swap_move_driver` pathfinder target callback used by area-specific NPC drivers and exact misc-PPD swapped timestamp/audit logging.
- `src/area/31/warrmines.c` `IDR_OXYPOTION` now dispatches from the area item-driver path with the legacy area-31 guard, carried-item requirement, silent one-minute `IDR_OXYGEN` timed spell installation, source potion destruction, spell-remove timer scheduling, and inventory/update flag mutation covered by focused core/world tests. Remaining Warrmines item gaps include lizard flower/berry PPD cooldowns, flower mixing, exact area driver feedback/audit logging, and live-data smoke coverage.
- `src/system/drvlib.c` `distance_driver` distance-7 fireball-spacing now has a Rust simple-baddy attack slice for the C active Fireball spacing path: after offensive spell/pulse and distance-3 attempts, simple baddies with active Fireball, enough mana, useful fireball damage, an available Flash spell slot, and effective Fireball above effective Flash try to path to seven steps from the target without the distance-3 idle fallback. Focused core tests cover successful distance-seven movement and the Fireball-vs-Flash gate. Remaining fight-driver gaps include full randomized task scorer/priority ordering, fireball line-of-hit repositioning, flee/secure movement details, exact global RNG parity, and exact NPC scheduling.
- `src/system/drvlib.c` in-combat `fight_driver_attack_enemy` self-preservation tasks now have a Rust simple-baddy attack slice: visible-enemy processing tries C-style low-HP self-heal, low-lifeshield magic shield, unblessed self-bless, and half-second regeneration idle before offensive/movement fallback, using the existing timed action bridges and stamping `lastfight`. Focused core tests cover heal priority over fireball, magic shield before melee, self-bless admission, and regeneration idle. Remaining fight-driver gaps include exact randomized task scorer/priority ordering, fireball line-of-hit repositioning, flee/secure movement details, exact global RNG parity, and exact NPC scheduling.
- `src/system/drvlib.c` fireball line-of-hit repositioning inside `fight_driver_attack_enemy` now has a Rust simple-baddy slice: non-adjacent Fireball setup first runs a C-shaped `ishit_fireball`-style half-tile line scan, refuses blocked lines that do not hit the recorded target in the 3x3 blast neighborhood, and tries the legacy right/left/down/up lane search up to four tiles with dead-direction suppression before falling back to other attack tasks. Focused core tests cover successful lane repositioning and blocked-line no-cast behavior. Remaining fight-driver gaps include full randomized task scorer/priority ordering, exact enemy-list blast admission beyond the selected target, flee/secure movement details, exact global RNG parity, and exact NPC scheduling.
- `src/system/drvlib.c` earth-demon `fight_driver_attack_enemy` earthmud task now has a Rust simple-baddy attack slice: `CF_EDEMON` simple baddies with effective demon value `30`, at least half HP, and useful non-sightblocked/non-duplicate target tiles set up the existing timed `AC_EARTHMUD` action against the C-shaped target tile, including walking-target prediction via `tox/toy + tox/toy - x/y`, HP cost, strength storage, direction, and `lastfight` stamping. Focused core tests cover useful earthmud setup and blocked/no-useful-tile fallback. Remaining fight-driver gaps include the full randomized task scorer/priority ordering, exact earth task value weighting relative to other tasks, flee/secure movement details, exact global RNG parity, and exact NPC scheduling.
- `src/system/drvlib.c` simple-baddy direct ball task now preserves the C `do_ball(cn, target.x - 1 + RANDOM(3), target.y - 1 + RANDOM(3))` target scatter through an injectable random seam. The default deterministic runtime keeps center-target behavior until global C-compatible RNG is wired, while focused core coverage verifies low/high roll offsets. Remaining fight-driver gaps include full randomized task scorer/priority ordering, exact global RNG source wiring for ball scatter and task ordering, flee/secure movement details, and exact NPC scheduling.
- `src/system/drvlib.c` `fight_driver_flee` now has a Rust world helper for simple-baddy driver reuse: visible tracked enemies within the legacy distance limit contribute the C direction-score table, escape path scoring scans up to ten tiles with movement blockers and light/daylight penalties, near enemies switch to fast/normal speed according to endurance and distant enemies switch to stealth, successful flee setup uses the existing `do_walk` bridge and stamps `lastfight`. This helper is intentionally not wired into generic simple-baddy attack selection because the C `fight_driver_attack_enemy` flee task is compiled out behind `if (0 && ...)`. Focused core tests cover close flee direction, distant stealth mode, and blocked escape path scoring. Remaining fight-driver gaps include the full randomized task scorer/priority ordering, exact earth task value weighting relative to other tasks, secure movement details, exact global RNG parity, and exact NPC scheduling.
- `src/system/drvlib.c` fight-driver flee task admission now makes the C disabled `if (0 && ...)` branch explicit in Rust with a named false guard: low-HP simple baddies still do not enqueue weighted flee tasks, while the standalone `fight_driver_flee` helper remains available for area-specific callers. Focused core coverage pins the disabled branch so future fight-task work does not accidentally grow behavior beyond the C oracle.
- `src/system/drvlib.c` `fight_driver_note_hit` side effects are now represented in the simple-baddy message path: `NT_GOTHIT` emits a typed internal note-hit outcome, `World::process_simple_baddy_message_actions` stamps serde-defaulted `SimpleBaddyDriverData::last_hit` from the current tick without changing public applied item-driver outcomes, and focused core tests cover both potion-use and defensive-enemy `NT_GOTHIT` paths. Remaining fight-driver gaps include using `last_hit` in the full C randomized regeneration task scorer and exact NPC scheduling.
- `src/system/drvlib.c` `secure_move_driver` final-facing behavior is now represented in the simple-baddy noncombat day/night post path: once the NPC is already on the configured post tile, Rust applies the configured `daydir`/`nightdir` with the existing `turn` primitive before the regeneration/self-spell/idle fallthrough, while preserving home-coordinate updates. Focused core coverage verifies day-post facing and idle continuation. Remaining secure movement gaps are broader reuse by non-simple-baddy area drivers and exact return-code/caller-flow parity.
- `src/system/drvlib.c` `secure_move_driver` now has a reusable Rust world helper with the C return contract: off-target characters walk with `move_driver`-style pathing unless the last action was `AC_USE` returning `2`, then fall back to same-area teleport; on-target characters only turn to the requested facing and return false so callers continue their idle/regeneration flow. The simple-baddy secure post path now uses this helper after the legacy ten-second post-combat delay, while preserving the existing `notsecure` movement path. Focused core tests cover turn-only return semantics, blocked-use teleport fallback, and normal walk-before-teleport behavior. Remaining secure movement gaps are broader reuse by non-simple-baddy area drivers and threading real `ret`/`lastact` through all character-driver runtime invocations.
- `src/system/account_depot.c` `DRD_ACCOUNT_WIDE_DEPOT` raw item serialization now has a Rust compatibility codec and PostgreSQL snapshot integration for the direct `struct item` array payload: fixed C offsets for flags/name/description/value/level/class/owner/modifiers/content/driver/40-byte driver data/legacy template ID/serial/sprite are encoded, volatile map/carried/container/free-list fields are zeroed for persistence, decode ignores the trailing legacy free-list pointer and restores dense account-depot snapshot slots, the legacy subscriber-data outer block `MAKE_DRD(DEV_ID_ED, 6 | PERSISTENT_SUBSCRIBER_DATA)` is decoded from loaded `subscriber_blob`, runtime account-depot snapshots are encoded back on logout save while preserving unrelated subscriber blocks, and empty depots remove the block like C `del_data`. Focused server tests pin the byte layout, outer subscriber-block replacement/preservation, empty-depot deletion, and logout save request integration. Remaining account-depot persistence gaps are immediate flush behavior, live PostgreSQL restart verification, and exact multi-account subscriber ownership semantics beyond the current per-character snapshot row.
- `src/area/3/area3.c` `IDR_ONOFFLIGHT` and `IDR_PALACEGATE` now dispatch from the Rust item-driver registry: zero-character lamp timer callbacks preserve the legacy first-call lamp-registration flag in `drdata[6]` without toggling, lit startup lamps are included in existing light timer scheduling, character use toggles `drdata[0]`, sprite, and light modifier from `drdata[1]`, world light refresh/dirty-sector handling is reused, `World` tracks the C area-global `on`/`off` palace-lamp counters, relighting reports either `%d remaining` or `The light has returned to the palace and the gates open.`, the legacy three-minute palace-gate-open window is recorded, registered lamps are scheduled for one-tick-staggered extinguish callbacks when all lights are restored, palace gates reschedule every ten seconds, open while the keep-open window is active by clearing stored movement/sight/sound/door blockers, close after the window expires by restoring stored flags, and refuse to close while blocked. Focused core/world tests cover registration, toggling, counter updates, gate-window state, extinguish scheduling, gate open/close mutation, blocked close refusal, and recurring gate scheduling. Remaining palace-lamp parity gaps are lampghost AI ownership/pathing behavior and live area-state smoke coverage.
- Area 6 `IDR_EDEMONDOOR` section-power gating now matches the C `if (!sect[nr] && cn)` check for all section indices, including `drdata[6] == 0`; unpowered section-zero doors return the legacy lifeless outcome instead of toggling. Focused item-driver coverage pins the section-zero blocked path.
- Area 37 Arkhata stopwatch vanish handling now has a reusable live-world cleanup primitive in addition to logout snapshot filtering: carried `IDR_ARKHATA` subtype `1` stopwatch items are destroyed from inventory/cursor ownership and clear `current_container` references without touching other Arkhata quest items. Focused server tests cover live inventory and cursor cleanup. Actual invocation during real cross-area transfer remains blocked on implementing cross-area handoff execution; current target-area-down outcomes correctly do not delete the item because the character has not left.
- `src/module/simple_baddy.c` `drinkspecial` poison-cleansing branch is now represented in the Rust simple-baddy noncombat runtime path: parsed `drinkspecial` NPCs scan active spell slots for the legacy `IDR_POISON0` trigger after movement handling, remove all active poison spell items through the existing poison-removal primitive when present, mark inventory/update flags, and continue into regeneration/self-spell/idle flow like C. Focused core tests cover full poison removal and the C-specific no-op when only non-zero poison types are active.
- Area 2 `IDR_CHESTSPAWN = 27` now dispatches from the Rust item-driver registry for C `chestspawn_driver`: character use on inactive kind-0 spawners returns a typed `normal_vampire` spawn request at the spawner tile, successful runtime instantiation sets direction/resources, clears `CF_RESPAWN`, drops the NPC on the spawner tile, marks the spawner active by incrementing sprite and storing the spawned character id, schedules the legacy ten-second poll, and zero-character timer callbacks reset the spawner when the spawned character is gone/dead or reschedule while it remains alive. Focused core/server-build tests cover dispatch, active marking, polling, and reset behavior. Remaining chestspawn gaps are exact serial tracking, live area-2 data smoke coverage, and exact vampire death/quest side effects beyond the existing character-driver/death scaffolding.
- `src/module/simple_baddy.c` helper friend-bless ordering now matches the C driver split between message scanning and action selection: `NT_CHAR` helper messages validate the same-group/visibility/mana/spell-slot gates and store a transient pending friend, while the noncombat driver attempts that bless only after movement, regeneration, and self-spell handling, clearing stale candidates afterward. Focused core tests cover deferred same-group bless setup, self-bless priority, and other-group rejection. Remaining simple-baddy gaps include full randomized fight-task scorer parity, broader area-driver reuse, secure/flee edge cases, exact global RNG parity, and exact NPC scheduling.
- `src/module/simple_baddy.c` visible-combat start sound cadence is now represented in the Rust simple-baddy attack path: successful visible enemy attack/spell/path setup queues legacy `sound_area(..., 1)` only when the previous `lastfight` was more than ten seconds old, then preserves the existing `lastfight` stamp. Focused core coverage verifies sound delivery to nearby players and suppression for recent combat.
- Area 30 `IDR_CLANSPAWNEXIT = 124` now dispatches behind the legacy area-30 libload guard for C `clanspawn_exit`: non-character calls no-op, character use targets the character rest area/tile, same-area exits reset action and use the existing legacy drop-char placement behavior, occupied destinations surface the legacy busy feedback outcome, and cross-area exits remain at the target-area-down handoff boundary in the server runtime. Focused core/world tests cover dispatch, area guard, same-area movement, and busy fallback. Remaining clan-module gaps include `IDR_CLANSPAWN`, clan vault semantics, clan clerk/master character drivers, real cross-area handoff, and exact clan repository persistence/logging.
- `src/system/drvlib.c` `attack_driver` moving-target melee admission is now represented in the Rust simple-baddy attack path: visible enemies whose current tile is not cardinal-adjacent but whose pending `tox`/`toy` movement target is adjacent are attacked immediately using the existing `do_attack` bridge, preserving the C `if he's moving towards us, hit him` branch before pathing. Focused core coverage verifies action setup, direction, target serial, and `lastfight` stamping. Remaining fight-driver gaps include the full randomized task scorer/priority ordering, broader `attack_driver` pathbest/walk-or-use edge cases, exact global RNG parity, and exact NPC scheduling.
- `src/system/drvlib.c` `attack_driver` pathbest fallback is now represented in the Rust simple-baddy attack path: direct-target and moving-target path costs are compared like C, successful movement uses a reusable `walk_or_use_driver` helper that tries `do_walk` before `do_use`, unreachable targets use the pathfinder best-partial direction only when it improves Manhattan distance, and non-player simple baddies idle for `TICKS / 4` when no partial progress is possible. Focused core coverage verifies best-partial walking and unreachable idle fallback. Remaining fight-driver gaps include the full randomized task scorer/priority ordering, broader `attack_driver` door/use live-data parity, exact global RNG parity, and exact NPC scheduling.
- `src/system/drvlib.c` `attack_driver` walk-or-use blocker fallback now has a Rust simple-baddy slice for adjacent usable blockers when normal pathing/pathbest cannot progress: visible-enemy movement scans cardinal neighboring `MF_MOVEBLOCK`/`MF_TMOVEBLOCK` tiles with usable items that reduce target distance, sets up the existing timed `AC_USE` path, and stamps `lastfight`. Focused core coverage verifies a blocked path causes the NPC to use the adjacent blocker instead of idling. Remaining fight-driver gaps include the full randomized task scorer/priority ordering, broader live-data door/use parity, exact global RNG parity, and exact NPC scheduling.
- Area 6 `IDR_EDEMONDOOR = 41` now dispatches from the Earth Demon item-driver path for the C `edemondoor_driver` core gates: timer callbacks decrement the outstanding close counter and no-op when already closed/no-auto-close/other timers remain, player use preserves the exact carried/cursor key requirement without skeleton-key or keyring fallback, section power from matching loaders gates live character use with the legacy lifeless-door feedback, and successful toggles reuse the existing C-shaped door open/close, auto-close, blocked-close retry, and sound behavior. Runtime context now supplies exact-key access for Earth Demon doors and server feedback covers missing-key/lifeless outcomes. Focused core coverage plus workspace tests cover key/power/timer dispatch. Remaining Earth Demon door gaps are exact C module-global `sect[]` aggregation when multiple loaders share a section, live area-6 data smoke coverage, and audit/log side effects.
- `src/system/drvlib.c` `fight_driver_attackback_value` now also preserves the C edge-case guards in the simple-baddy back-positioning slice: NPCs no longer try to move behind a target when they are already standing on the target's front tile, and the attack-back task is suppressed when a same-group ally already occupies the legacy checked side tile. Focused core coverage verifies both guard clauses. Remaining attack-back/fight-driver gaps include exact randomized task scorer/priority ordering, broader live-data door/use parity, exact global RNG parity, and exact NPC scheduling.
- `src/system/drvlib.c` `attack_driver_direct` now has a reusable Rust `World::attack_driver_direct` primitive for area/player-driver reuse: it preserves the C guard order for self/visibility/attack policy, attacks current adjacent target tiles, attacks a moving target's pending `tox`/`toy` tile when adjacent, otherwise takes one `walk_or_use_driver` step along a complete path to the target, and deliberately omits the full `attack_driver` best-partial and non-player idle fallback. Focused core tests cover adjacent attacks, moving-target attacks, normal path steps, and blocked no-path failure without idling.
- `src/system/drvlib.c` `distance_driver` `pathbestdir()` fallback is now represented in the simple-baddy distance-spacing path: when exact distance-3/distance-7 spacing cannot be reached, Rust takes the best partial pathfinder direction through `walk_or_use_driver` instead of giving up, and stamps `lastfight` on successful setup. Focused core coverage pins an unreachable exact distance-7 case behind a movement-blocking wall. Remaining fight-driver gaps include full randomized task scorer/priority ordering, broader live-data door/use parity, exact global RNG parity, and exact NPC scheduling.
- `src/system/drvlib.c` `distance_driver` now has a reusable Rust `World::distance_driver` primitive for area/player-driver reuse: it preserves the legacy guard order for missing/self/invisible targets, returns false when already at the requested step distance, prefers a moving target's `tox`/`toy` path before current position, uses the shared `walk_or_use_driver` movement/use fallback, and exposes the same `pathbestdir()`-style best-partial fallback for arbitrary requested distances such as the dormant distance-8 flee branch. Focused core tests cover moving-target preference, already-there false return, and blocked exact-distance best-partial movement.
- Simple-baddy noncombat runtime now threads prior action completion context into the ported `secure_move_driver` path instead of always passing zero `ret`/`lastact`: the server tick loop supplies the latest same-character completion to noncombat processing, `WorldActionCompletion` carries an explicit C-style legacy return code, applied item-driver dispatch fills that return code before NPC noncombat processing, and door-family blocked/no-progress outcomes preserve the legacy `ret == 2` teleport fallback while ordinary failed use completions keep `ret == 0` and still try pathing. Focused core coverage verifies both secure-move branches. Remaining parity gap is preserving exact legacy character-driver return codes beyond the currently wired door-family item-driver cases.
- Area 8 `IDR_FDEMONWAYPOINT = 48` now dispatches from the Rust item-driver registry with the C area-8 libload guard: player/playerlike use marks the waypoint as enemy-spotted, stores the target runtime character identity in the C `drdata[4..12]` layout, switches to sprite `14200`, fire-demon use or timer callbacks clear the marker and switch to sprite `14202`, dirty the sector, and schedule the next waypoint callback after three seconds. Focused core/world tests cover player marking, fire-demon clearing, timer callback shape, area guard, live item mutation, and timer scheduling. Remaining fire-demon waypoint gaps are exact legacy `ch.ID` serial parity, `notify_area(..., NT_NPC, NTID_FDEMON, MSG_WAYPOINT, ...)` fan-out, and waypoint graph/hunt-driver integration.
- Area 8 `IDR_FDEMONWAYPOINT = 48` now also preserves the dormant legacy `notify_area(..., NT_NPC, NTID_FDEMON, MSG_WAYPOINT, in)` payload as a Rust driver-message fan-out after waypoint mutation, using explicit Rust constants because the C call site is commented and the macros are absent from `notify.h`. Remaining fire-demon waypoint gaps are exact legacy `ch.ID` serial parity and waypoint graph/hunt-driver integration.
- Area 8 `IDR_FDEMONWAYPOINT = 48` now stores the Rust character serial in waypoint `drdata[8..12]`, matching C `ch[cn].ID`, instead of duplicating the runtime character index. Focused item-driver/world tests use intentionally different id/serial values to pin the layout. Remaining fire-demon waypoint gaps are waypoint graph/hunt-driver integration.
- Area 4 `IDR_PENT` and `IDR_PENTBOSSDOOR` now have typed Rust item-driver boundaries for C `handle_pentagram_interaction` / `handle_demon_lord_door`: the area-4 libload guard is represented for both drivers, `IDR_PENT` no longer falls through as unsupported and distinguishes player activation, already-active player touches, and zero-character timer callbacks while decoding legacy `drdata[0]` level, `drdata[1]` status, `drdata[2]` color, and `drdata[4]` area status; `IDR_PENTBOSSDOOR` preserves zero-character no-op, diagonal no-op, recent-solve access timing, exact opposite-side same-area teleport targets, world facing reversal, and blocked-target retry outcomes. Focused core/world tests cover pentagram dispatch gates, activation/timer boundaries, boss-door access timing, target decoding, and facing reversal. Remaining pentagram gaps are applying `IDR_PENT` activation/deactivation world state, solve serial/area serial tracking, demon spawning, player PPD/rewards/records, solve-time runtime wiring into server contexts, and area-4 character drivers.
- `src/system/drvlib.c` `fight_driver_attack_enemy` task-order scaffolding now has a Rust compatibility helper for the C task table shape: all legacy task kinds are represented, low/medium/high priority constants are pinned, `silliness = level / 2 + 5` random bonuses are applied before descending value sort, and focused core tests cover deterministic ordering plus injected random reordering. The helper is not yet wired into the simple-baddy attack cascade; remaining fight-driver gaps include migrating the current fixed-order attack path to weighted task execution, exact global RNG parity, and exact NPC scheduling.
- `src/system/drvlib.c` `fight_driver_attack_visible` visible-enemy target choice now matches the C scoring slice in Rust simple-baddy attack processing: visible tracked enemies are ordered by `(999 - char_dist) * 10` with the legacy +5 facing bonus before trying attack/spell/path actions, so closer/faced enemies beat older or higher-priority entries. Focused core coverage verifies a close visible target is attacked before a farther high-priority target. Remaining fight-driver gaps include migrating the current fixed-order attack path to weighted task execution, exact enemy-array ordering/slot cap parity, exact global RNG parity, and exact NPC scheduling.
- Area 25 `IDR_WARPKEYSPAWN = 115` now dispatches through the warped item-driver path for C `warpkeyspawn_driver`: area-25 libload gating is preserved, zero-character calls no-op, occupied cursors emit the legacy empty-hand feedback, `drdata[0]` selects the `warped_teleport_keyN` template name, runtime instantiation places the glowing half-sphere on the cursor, and missing templates surface the legacy `It won't come off.` feedback. Focused core coverage verifies typed template selection and cursor blocking, and workspace tests cover server/runtime compilation. Remaining warped gaps are `IDR_WARPTRIALDOOR`, `IDR_WARPBONUS`, `IDR_WARPKEYDOOR`, warped PPD persistence, trial fighter character-driver behavior, exact reward side effects, and live area-25 data smoke coverage.
- `src/system/drvlib.c` fight-driver enemy memory now preserves the C fixed ten-entry table behavior for simple baddies: adding an existing enemy refreshes its timestamp without reporting a new entry, adding beyond ten overwrites the final slot instead of growing unbounded, and runtime message/refresh paths sort and cap remembered enemies by the legacy `person_cmp` priorities of visible first, hurt-me/priority first, nearer last-known tile, then facing. Focused character-driver/world tests cover return-code, slot-cap, and ordering behavior. Remaining fight-driver gaps include migrating the current fixed-order attack path to weighted task execution, exact global RNG parity, and exact NPC scheduling.
- `src/system/drvlib.c` `fight_driver_attack_enemy` weighted task execution is now wired into the Rust simple-baddy visible-enemy attack path: Rust builds C-shaped task records for freeze, fireball/firering, ball, flash, warcry, attack, fireball lane movement, regeneration, distance3/distance7, bless, earthmud, heal, magic shield, pulse, and attack-back using the legacy low/medium/high priority formulas already ported, sorts them through the existing silliness-aware task ordering helper, and executes the ordered tasks through the existing typed action setup helpers. The earlier Rust-only self-preservation pre-pass has been removed, so low-HP heal, low-shield magic shield, self-bless, and regeneration now compete by C task value; focused coverage pins heal winning when regeneration is delayed and regeneration winning when its C scorer outranks lower tasks. Remaining fight-driver gaps include exact shared global RNG use for task silliness and projectile scatter, exact enemy-list blast admission beyond the selected target, broader area-driver reuse, and exact NPC scheduling.
- `src/system/effect.c` `ishit_fireball` enemy blast admission is now represented in the Rust simple-baddy fireball line scan: when a fireball line hits a blocking tile, the 3x3 blast neighborhood is accepted if it hits the selected target or any enemy remembered by the NPC fight-driver state, and rejected if it would hit a non-enemy character. Focused core tests cover recorded-enemy splash acceptance and friendly-collateral rejection. Remaining fight-driver gaps include exact shared global RNG use for task silliness and projectile scatter, broader area-driver reuse, and exact NPC scheduling.
- `src/system/drvlib.c` `fight_driver_attack_enemy` attack-back task sequencing now preserves the C sorted-task edge case: an `attackback` task is attempted only when the immediately following sorted task is `attack`; otherwise it is skipped and the next task is tried. Focused core coverage pins the helper behavior for attack-back followed by attack, by another task, and at the end of the task list. Remaining fight-driver gaps include exact shared global RNG use for task silliness and projectile scatter, broader area-driver reuse, and exact NPC scheduling.
- Runtime simple-baddy visible-enemy attack processing now threads a server-supplied random callback through the batch NPC tick path, so C `RANDOM(sillyness)` task jitter and direct-ball target scatter use the runtime RNG seam instead of the deterministic single-action fallback when the server loop invokes `process_simple_baddy_attack_actions`. Focused core coverage pins random propagation through the batch API. Remaining fight-driver RNG gaps are exact global C RNG stream/state parity and exact NPC scheduling cadence.
- Area 17 `IDR_BURNDOWN = 82` now dispatches from the item-driver registry for C `burndown`: area-17 libload guard, zero-character timer no-op/tick boundary, too-hot/already-burned/touch-without-lit-torch feedback outcomes, lit-torch ignition, burn-state countdown, sprite progression, foreground fire overlay, light modifier add/remove, five-second timer rescheduling, and final burned sprite are represented in core/world/server paths with focused tests. Remaining burndown parity gaps are `DRD_TWOCITY_PPD` thief quest mutation, `notify_area(..., NTID_TWOCITY_PICK)`, exact sector/light invalidation parity beyond current dirty-sector/light refresh hooks, and audit logging.
- `src/module/simple_baddy.c` scavenger noncombat ordering now matches the C branch around `drinkspecial`: scavengers run regeneration, self-spells, deferred friend bless, random idle, and bounded wander before falling through to poison-cleansing special-potion handling, so low-HP scavengers no longer remove poison before the legacy `regenerate_driver` early return. Focused core coverage pins regeneration before both random wander and `drinkspecial` poison removal.
- Runtime simple-baddy message processing now respects the C `act.c` character-driver invocation point: queued `NT_*` messages are not consumed and cannot trigger inventory-potion/spell/enemy side effects while the NPC is still executing an action or is dead; they wait until the next idle driver turn. Focused core coverage verifies `NT_GOTHIT` inventory-potion messages remain queued during an active walk action.
- Legacy item-driver return-code classification is now a shared `ugaris-core` helper instead of server-local policy: successful handled outcomes return `1`, unsupported/no-op outcomes return `0`, and blocked `IDR_DOOR`/`IDR_DOUBLE_DOOR` no-op outcomes preserve the C retry code `2` used by character drivers such as `secure_move_driver`. Focused core tests pin the C contract and the server runtime now uses the shared helper when threading completed item-use context into simple-baddy noncombat processing.
- C `can_attack` same-clan suppression is now represented in the shared Rust attack policy: characters with matching nonzero direct clan IDs cannot attack each other, so melee, spell, effect, and simple-baddy target-admission paths inherit the guard. Rust also has an area-aware `can_attack_in_area` wrapper for runtime callers with area context; player-vs-player attacks in area 1 are blocked before adjacent attack or path setup, matching the legacy non-PK town guard. Focused `do_attack`/world coverage pins the resulting legacy `IllegalAttack`/idle behavior. Remaining `can_attack` gaps include alliance/repository clan policy, PK hate-list persistence, arena path gating, and the rest of the area-ID-specific clan/PK rules.
- C `can_attack` arena gating is now represented in the shared Rust attack policy: one-sided arena attacks are blocked, two arena occupants must be connected through `MF_ARENA` tiles, and connected same-arena attacks continue through the existing clan/playerlike/area guards. Focused core coverage pins one-sided, disconnected, and connected arena cases. Remaining `can_attack` gaps include alliance/repository clan policy, PK hate-list persistence, exact legacy pathfinder callback parity for unusual arena maps, and the rest of the area-ID-specific clan/PK rules.
- C `can_attack` automatic/environment attacker handling is now represented in the shared Rust attack policy: attacker character id `0` returns attack-allowed immediately after the same basic defender/self/dead/no-attack guards as C, before `CF_NOPLRATT`, peace/arena, playerlike, PK, group, or clan restrictions. Focused core coverage pins zero-attacker bypass behavior while preserving dead-target rejection.
- Area 37 `IDR_ARKHATA` subtype `1` now ports the C `stopwatch_driver` timer path: zero-character timer calls reschedule every 10 ticks, carried stopwatch items resolve the carrying player, `DRD_ARKHATA_PPD` is represented with the fixed 25-int C layout including `clerk_state`/`clerk_time`, outer PPD decode/encode preserves and appends the block, and runtime feedback emits the legacy `#91 Time: ... Astonian Minutes`, `#92 YOU FAILED!`, or blank `#92 ` messages. Focused core tests cover timer-only dispatch and PPD round-trip. Remaining Arkhata gaps are broader NPC quest/dialogue drivers, live area-data smoke coverage, exact global RNG parity, and audit/logging side effects.
- C `can_attack` guard ordering now preserves two additional shared-policy edges: same nonzero NPC group members cannot attack each other outside arena combat, and connected normal arena combat returns before same-clan suppression while connected clan-arena tiles still block same-clan attacks. Focused core tests cover same-group suppression and the normal-arena/clan-arena same-clan distinction. Remaining `can_attack` gaps include alliance/repository clan policy, PK hate-list persistence, exact legacy pathfinder callback parity for unusual arena maps, and the rest of the area-ID-specific clan/PK rules.
- C `can_attack` player-vs-player prerequisites are now represented in the area-aware Rust attack policy for the currently modeled state: outside area 1, both players must carry `CF_PK` and be within the legacy three-level range before player-vs-player combat is admitted. Focused core tests cover area-1 blocking, missing one/both PK flags, admissible PK combat, and out-of-range rejection. Remaining `can_attack` gaps include PK hate-list persistence, alliance/repository clan policy, exact legacy pathfinder callback parity for unusual arena maps, and the rest of the area-ID-specific clan/PK rules.
- C `can_attack` clan relation ordering now has a Rust policy seam for repository-backed clan state: `can_attack_in_area_with_clan_policy` preserves the C order where connected clan arenas block same-clan or allied clans, clan-area war/feud attacks and outside-clan feud attacks can admit combat before the player-vs-player PK gate, outside feuds keep the legacy three-level range and area-1 block, and ordinary allied clans are suppressed by the shared attack policy. Default runtime callers use `NoClanAttackPolicy` until a real clan repository is wired. Focused core tests cover allied arena blocking, clan-area war before PK gating, outside feud admission, area-1 rejection, and feud level range. Remaining `can_attack` gaps include loading real clan relation state, PK hate-list persistence, exact legacy pathfinder callback parity for unusual arena maps, and the rest of the area-ID-specific clan/PK rules.
- C `can_attack` player-vs-player hate-list admission has an explicit Rust policy seam: after the existing area-1, PK-flag, and three-level-range guards, `can_attack_in_area_with_clan_policy` calls `has_pk_hate(attacker, defender)` so runtime/repository-backed policies can enforce the legacy attacker hate-list requirement without changing combat call sites. The default no-runtime policy now fails closed instead of preserving the old scaffold allow behavior. Focused core tests cover default rejection, policy-admitted hate, level-range rejection after hate, and preserving the PvP-before-same-group ordering.
- `DRD_PK_PPD` persistence is now represented in `PlayerRuntime`: the fixed C layout from `struct pk_ppd` (`kills`, `deaths`, `last_kill`, `last_death`, and 50 `hate` character IDs) has encode/decode helpers, outer PPD blob load/save replacement and append behavior, duplicate/full hate-list admission helpers, and focused core tests for byte layout plus legacy block framing. Remaining PK gaps are wiring a repository/runtime-backed `ClanAttackPolicy` into live player-vs-player checks and exact `del_hate`/audit side effects.
- `SV_NAME` visible-character identity packets now preserve three more C `player.c` fields: the direct clan byte is populated from the target character's legacy clan ID, demon sprite `27` forces all three color words to zero like the C demon-sprite display hack, and the viewer-specific PK relation byte now follows C `get_pk_relation(cn, co)` for neutral PK candidates, viewer-hates-target, target-hates-viewer, and mutual hate using a runtime PK-hate snapshot during login bootstrap, map diffs, movement fringe updates, periodic cache refresh, and command-triggered name-cache refresh. Focused server tests cover normal colors, clan emission, demon color suppression, and PK relation byte values. Remaining identity gaps are clan repository/alliance-derived relation display beyond direct clan IDs, applying relation-specific name-cache invalidation around every future repository-backed relation change, and exact reset-name fan-out side effects.
- Live player direct-attack setup now uses the loaded `PlayerRuntime` PK hate list through the shared `ClanAttackPolicy` seam for `PAC_KILL`: outside area 1, player-vs-player attacks still require both PK flags and level range, and now also require the attacker runtime to contain the defender character ID in `DRD_PK_PPD` hate entries before adjacent attack or path-to-attack setup proceeds. Focused core tests cover missing-hate rejection and admitted attacks with a hate entry. Remaining PK gaps are wiring repository-backed clan/alliance policy into runtime checks beyond the current no-clan policy, applying the same runtime hate policy to every non-`PAC_KILL` PvP spell/effect admission path, and exact `del_hate`/audit side effects.
- `src/system/tool.c` `add_hate`/`del_hate_ID` list semantics are now reflected in `PlayerRuntime`: adding a hated character moves that ID to the front like C, duplicate adds refresh priority without creating another entry, full lists evict the oldest tail entry instead of rejecting the new hated ID, and removal by character ID is available for future command/death call-site wiring. Focused core tests cover ordering, duplicate refresh, removal, full-list eviction, and unchanged fixed-layout `DRD_PK_PPD` encoding. Remaining PK gaps are wiring the helper through every legacy hate-list command/death call site, repository-backed clan/alliance policy, non-`PAC_KILL` PvP spell/effect admission, and exact audit/name-reset side effects.
- Completed player area-spell actions now use the same runtime attack-policy seam as projectile/effect ticks: `tick_basic_actions_with_attack_policy` lets the server apply `DRD_PK_PPD` hate-list and area/clan policy checks while resolving immediate `AC_FIRERING`, `AC_PULSE`, `AC_FREEZE`, and `AC_WARCRY` target scans. The default core tick path preserves the existing broad `can_attack` behavior for tests/NPC callers, while the server runtime removes stale hate entries on legacy policy failures and falls back to area-aware non-player checks. Focused core coverage verifies an otherwise valid pulse target is blocked by the injected policy without suppressing the caster-side pulse visual. Remaining PK gaps are applying repository-backed clan/alliance policy in live runtime, wiring hate-list command/death side effects beyond hit accumulation, and exact audit/name-reset side effects.
- `/clearhate` command handling now matches C `del_all_hate`: it is silent, clears the hate list only for PK characters, and leaves non-PK character runtime state unchanged. Focused server coverage pins the legacy no-feedback and non-PK no-op behavior. Remaining PK gaps are applying repository-backed clan/alliance policy in live runtime, wiring hate-list command/death side effects beyond hit accumulation, and exact audit/name-reset side effects.
- Live `/hate` command handling now uses the C-style `PlayerRuntime::add_pk_hate` semantics end-to-end, including duplicate-add front-priority refresh, lag-flag clearing, and affected name-cache refresh for the attacker/target so client PK relation coloring can update immediately. Focused server tests cover command-level add, duplicate refresh ordering, abbreviations, list, remove, numeric remove, and clear behavior. Remaining PK gaps are repository-backed clan/alliance policy, non-`PAC_KILL` PvP spell/effect admission, exact audit/name-reset side effects, and legacy death-time hate cleanup fan-out.
- Character-targeted player fireball and ball setup now use the same runtime `DRD_PK_PPD` hate-list policy seam as direct `PAC_KILL`: player-vs-player character spell targeting outside area 1 still requires both PK flags and level range, and now also requires the attacker runtime to hate the defender before `AC_FIREBALL1` / `AC_BALL1` setup is admitted. Focused core tests cover fireball blocking without hate, fireball admission with hate, and ball blocking without hate. Remaining PK gaps are wiring repository-backed clan/alliance policy into runtime checks beyond the current no-clan policy, future retained damage-effect families beyond the currently wired fireball/ball paths, and exact `del_hate`/audit/name-reset side effects.
- Area 30 `IDR_CLANJEWEL = 21` now dispatches from the item-driver registry for the C clan-jewel timer lifecycle: zero-character timer callbacks initialize the little-endian creation timestamp in `drdata[0..3]`, reschedule 30-second expiry checks, expire after one hour of tick-derived realtime seconds, destroy carried or ground jewels through the world item-removal path, and emit the legacy carried-item `Your <name> expired.` feedback through the server runtime. Focused core/world tests cover first timer initialization, strict one-hour expiry, direct-use no-op behavior, and carried inventory removal. Remaining clan-system gaps are clan spawner/clerk/NPC flows, exact wall-clock realtime parity, dlog/audit side effects, and live area-30 data smoke coverage.
- Map-targeted player fireball and ball effect damage now use a runtime-supplied attack-policy seam: `World::tick_effects` keeps the default legacy `can_attack` behavior for core tests/NPC callers, while the server tick loop supplies per-caster `PlayerRuntime` PK hate state through `can_attack_in_area_with_clan_policy` before retained `EF_FIREBALL` explosion damage or `EF_BALL` strike damage can affect player targets. Focused core tests cover policy-denied fireball and ball effects leaving player targets undamaged. Remaining PK gaps are wiring repository-backed clan/alliance policy into runtime checks beyond the current no-clan policy, applying the same runtime policy to any future retained player-damage effect families as they are ported, and exact `del_hate`/audit/name-reset side effects.
- C `add_hate` hit-side list side effects now have an explicit Rust helper: `PlayerRuntime::add_pk_hate_from_hit` reuses the fixed-layout hate-list priority/eviction behavior and clears the owning character's `CF_LAG` flag when a nonzero attacker is recorded, matching the legacy `ch[cn].flags &= ~CF_LAG` side effect. Focused core tests cover newly-added, refreshed, and zero-attacker cases. Remaining PK gaps are wiring this helper into every runtime PvP hurt call site, repository-backed clan/alliance policy, `del_hate`/command/death call sites, and exact audit/name-reset side effects.
- Runtime `hurt` PK hate side effects are now wired through a core/server boundary: `World::apply_legacy_hurt` records target/cause hurt events, the server tick drains them after effects/timers/actions/NPC processing, applies the C `check_hate` gates (both sides player+PK, not self, level difference <= 3), updates the target player's `DRD_PK_PPD` hate list, and clears `CF_LAG` on accepted hits. Focused server tests cover valid player hits and the legacy level gate. Remaining PK gaps are repository-backed clan/alliance policy, `del_hate`/command/death call sites, exact audit/name-reset side effects, and ensuring future retained player-damage effect families use the same runtime attack-policy seam.
- PK hate-list removal now mirrors the client-visible part of C `del_hate` / `del_hate_ID` `reset_name` behavior for `/nohate`: online-target removal queues visible `SV_NAME` refreshes for both source and target, numeric-ID removal queues a source refresh, and the existing command feedback remains unchanged. Focused server tests cover named and numeric removal refresh requests. Remaining PK gaps are repository-backed clan/alliance policy, exact audit/log side effects, death/kill cleanup call sites, and applying the same runtime hate policy to future retained player-damage effect families.
- Lethal runtime PvP hurt events now also apply the C `add_pk_kill`/`add_pk_death` side effects through `PlayerRuntime`: accepted player-vs-player PK deaths increment the killer's `DRD_PK_PPD.kills`/`last_kill` and the victim's `DRD_PK_PPD.deaths`/`last_death` using the server realtime tick seam while preserving the existing hate-list update. Focused server tests cover counter and timestamp updates. Remaining PK gaps are repository-backed clan/alliance policy, `del_hate`/command call sites, exact audit/name-reset side effects, and ensuring future retained player-damage effect families use the same runtime attack-policy seam.
- C `can_attack` stale PK hate cleanup is now represented at the Rust player attack setup boundary: direct `PAC_KILL` plus character-targeted fireball/ball admission remove a hated defender from the attacker's `DRD_PK_PPD` hate list when both sides are players but the legacy PK prerequisite check fails outside area 1, while area-1 town blocking preserves the hate entry like C's earlier return. Focused core tests cover out-of-range cleanup and area-1 preservation. Remaining PK gaps are repository-backed clan/alliance policy, offline-name hate-list lookup, exact audit/name-reset side effects, and ensuring future retained player-damage effect families use the same runtime attack-policy seam.
- Runtime `/nohate` now has a `del_hate_ID`-style fallback for persisted/offline PK hate entries when the command supplies a numeric legacy character ID: PK-gated removal clears the matching `DRD_PK_PPD` hate slot and emits the C-shaped `Removed from hate list` feedback while preserving online-name removal behavior. Focused server coverage pins the offline-ID removal path. Remaining PK gaps are repository-backed name lookup for true offline `/nohate <name>`, repository-backed clan/alliance policy, exact audit/name-reset side effects, and ensuring future retained player-damage effect families use the same runtime attack-policy seam.
- `DRD_PK_PPD` hate-list deletion now preserves the C fixed-slot table semantics: removing a hated character zeroes that slot instead of compacting the list, decode/encode retain middle holes while trimming only trailing zero slots, new adds use the C front-insertion/memmove shape over the fixed 50-entry table, and server display/lag-gating filters active nonzero entries. Focused core tests cover removed-slot persistence, active-entry filtering, duplicate priority refresh, eviction, and empty-list trimming. Remaining PK gaps are repository-backed clan/alliance policy, offline `/nohate <name>` lookup by persisted character name, exact audit/name-reset side effects, and ensuring future retained player-damage effect families use the same runtime attack-policy seam.
- Retained player-damage fireball/ball policy denial now applies the same C stale-hate cleanup rules as direct/character-targeted setup: runtime effect attack checks use the attacking player's `DRD_PK_PPD` hate state, remove stale hated defenders when PK prerequisites fail outside area 1, and preserve hate entries for area-1 town blocking. Focused server tests cover out-of-range cleanup and area-1 preservation. Remaining PK gaps are repository-backed clan/alliance policy, offline-name hate-list lookup, exact audit/name-reset side effects, and applying the same runtime seam to future retained player-damage effect families as they are ported.
- Player text commands now cover the online-character PK hate-list slice from `src/system/command.c` / `src/system/tool.c`: `/hate <name>` finds online characters case-insensitively, applies the C player/PK/self/level gates, mutates the fixed-layout `DRD_PK_PPD` runtime hate list, and clears `CF_LAG`; `/nohate <name>` removes online character IDs with legacy feedback; `/nohate <numeric-id>` removes persisted/offline IDs with the C `del_hate_ID` feedback; `/listhate` emits `Hate: <name>` or `List is empty.` for PK players; and `/clearhate` clears the runtime list only for PK characters. Focused server tests cover add/list/remove/clear, numeric-ID removal, and legacy no-feedback cases. Remaining PK command gaps are offline `lookup_name`/`lookup_ID` repository integration for names, reset-name/audit side effects, `/pk` mode transitions, and repository-backed clan/alliance policy.
- Player text commands now cover the C `/playerkiller` and `/iwilldie <id>` PK-mode transition slice: level-10 and paid-player admission gates, ID confirmation, C-shaped confirmation/tired/status feedback, `AC_IDLE` plus three-second regeneration-ticker leave guard, 28-day post-kill leave cooldown, `CF_PK` flag mutation, and `DRD_PK_PPD`-equivalent runtime state clearing on join/leave are ported with focused server tests. Remaining PK command gaps are offline `lookup_name`/`lookup_ID` repository integration, reset-name/audit side effects, command aliases/help parity, repository-backed clan/alliance policy, and exact realtime source persistence semantics.
- `/clearhate` now preserves the legacy command-level feedback from `src/system/command.c`: it always emits `Hate list has been erased.` after invoking the PK-only clear helper, including the non-PK no-op case. Focused server coverage pins both the non-PK no-op plus feedback and PK clear plus feedback paths. Remaining PK command gaps are offline `lookup_name`/`lookup_ID` repository integration, reset-name/audit side effects, command aliases/help parity, repository-backed clan/alliance policy, and exact realtime source persistence semantics.
- Ported PK hate-list command abbreviation parity from C `cmdcmp`: `/hat` routes to `/hate`, `/noh` routes to `/nohate`, and `/li`/longer `listhate` prefixes route to `/listhate`, while `/playerkiller`, `/iwilldie`, and `/clearhate` keep their legacy full-command requirements. Focused server coverage pins accepted and rejected abbreviations. Remaining PK command gaps are offline `lookup_name`/`lookup_ID` repository integration, reset-name/audit side effects, help text parity, repository-backed clan/alliance policy, and exact realtime source persistence semantics.
- The runtime text-command path now recognizes legacy `/help` and emits the C help section for PvP/security commands, including `/playerkiller`, `/iwilldie`, `/hate`, `/nohate`, `/listhate`, and `/clearhate`, with focused server coverage for exact lines and full-command recognition. Remaining PK command gaps are offline `lookup_name`/`lookup_ID` repository integration, reset-name/audit side effects, broader help text parity, repository-backed clan/alliance policy, and exact realtime source persistence semantics.
- The runtime `/help` text now mirrors the non-staff C player help sections from `src/system/command.c`: communication, emote shortcuts, chat channels, character/interaction, aliases, PvP/security, inventory/gold, clan/club, character development, thief, game information, lag control, automation, and miscellaneous command lines are emitted through the existing text-command feedback path with focused server coverage. Remaining help/PK command gaps are staff/admin subhelp commands, offline `lookup_name`/`lookup_ID` repository integration, reset-name/audit side effects, repository-backed clan/alliance policy, and exact realtime source persistence semantics.
- The runtime `/help` text now also appends the C gated staff/event/quest-master/god help sections based on `CF_STAFF`, `CF_EVENTMASTER`, `CF_LQMASTER`, and `CF_GOD`, preserves the Live Quest area note for areas 20/35, preserves the shared C footer, and passes the caller's world flags/area through the command dispatch path. Focused server tests cover player-only suppression, staff-only output, god output, event-master output, and Live Quest master area-note output. Remaining help command gaps are color-marker packet parity.
- Dedicated C admin subhelp commands are now recognized by the Rust runtime command path: `#achelp`/`/achelp` for staff or gods, `/macrohelp` for staff or gods, and `/penthelp` for gods only. Their legacy command lists from `src/module/anticheat/anticheat.c` and `src/system/command.c` are emitted as system feedback without inventory side effects, and focused server tests cover privilege gates plus representative text.
- Help and admin subhelp command feedback now uses the legacy raw color-marker byte protocol when sent to clients: section headings are wrapped with C light-red/reset markers, command keywords with light-blue/reset markers, angle-bracket parameters with light-green/reset markers, and Live Quest notes with orange/reset markers. The command result keeps plain text for internal assertions while the runtime sends `SV_TEXT` through the byte-preserving packet builder, with focused server/protocol tests covering raw `0xb0` marker output.
- Player command aliases from `src/system/command.c` are now represented in Rust: `/alias` and `#alias` accept the legacy two-character abbreviation, list/create/replace/delete aliases with the C 32-slot, 7-byte source, and 55-byte destination limits, `/clearaliases` clears the table with the legacy feedback, normal text commands expand aliases before command dispatch using C-style word boundaries with apostrophes preserved inside words, and `DRD_ALIAS_PPD` fixed-layout outer PPD decode/encode preserves aliases across snapshot load/save while deleting empty alias blocks. Focused core/server tests cover byte layout, outer block replacement/removal, expansion boundaries, command feedback, truncation, and abbreviation handling. Remaining alias gaps are exact in-place partial-match expansion edge cases for overlapping aliases and live client smoke coverage.
- Player text commands now cover the C `/gold <amount>` inventory command from `src/system/command.c`: legacy `atoi`-prefix amount parsing, copper conversion by `* 100`, invalid/insufficient-gold/cursor-occupied guard order and feedback, cursor money item creation, character gold decrement, item flagging, and client refresh including updated `SV_GOLD` are covered by focused server tests. Remaining inventory command gaps include exact dlog/audit side effects and broader command alias persistence.
- Player text commands now also cover the C god-only `/ggold <amount>` inventory/admin helper from `src/system/command.c`: non-god callers do not claim the command, god callers use legacy `atoi`-prefix parsing with copper conversion by `* 100`, mutate character gold, set the item-refresh flag, and emit no feedback like C. Focused server tests cover privilege gating, parsing, flagging, and the no-feedback path. Remaining inventory/admin command gaps include exact dlog/audit side effects and broader command alias persistence.
- Corrected the Rust simple-baddy visible-enemy selector to match `src/system/drvlib.c` `fight_driver_attack_visible`: the C hurt-me preference is commented out, so target choice now uses only `(999 - char_dist) * 10` plus the legacy facing bonus. Focused core coverage verifies a closer visible target is selected before a farther hurt-me/high-priority target. Remaining fight-driver gaps include exact shared global RNG use for task silliness/projectile scatter, broader area-driver reuse, and exact NPC scheduling.
- Player text commands now cover the C god-only `/laugh` helper from `src/system/command.c`: non-god callers do not claim the command, god callers queue the legacy `sound_area(..., 13)` special effect with no feedback, and the live command dispatch drains it through the existing sound-special runtime path. Focused server coverage pins privilege gating and sound type `13` emission.
- `src/system/player.c` `CL_TAKE_GOLD` / `CL_DROP_GOLD` goldbag client actions are now wired in the Rust server runtime: `CL_TAKE_GOLD` deposits an existing money cursor item into carried gold before validating/taking the requested silver amount, refuses non-money cursor items, creates a legacy-shaped money cursor item, decrements gold, and refreshes inventory/gold packets; `CL_DROP_GOLD` deposits only money cursor items and leaves non-money cursor items untouched. Focused server tests cover deposit-before-take and money-only drop semantics. Remaining gold/inventory gaps are exact dlog/audit side effects and broader client inventory delta parity beyond the current snapshot refresh.
- Player text commands now cover the C god-only `/saves <amount>` admin helper from `src/system/command.c`: `/save` and longer prefixes follow the legacy `cmdcmp(..., minlen=4)` shape, non-god callers do not claim the command, god callers use legacy `atoi`-prefix parsing, mutate the existing character save counter, and emit no feedback like C. Focused server tests cover privilege gating, abbreviation, and invalid-number reset-to-zero behavior. Remaining command/admin gaps include exact audit side effects and broader command alias persistence.
- Player text commands now cover the C visibility/admin toggle slice for `/immortal`, `/infrared`, `/invisible`, `/xray`, and `/spy`: the Rust runtime preserves the legacy `is_lqmaster` gate for gods, event masters, and Live Quest masters only in area 20; keeps the exact minimum-command abbreviations for immortal/infrared/invisible/spy and full `/xray`; toggles `CF_IMMORTAL`, `CF_INFRARED`, `CF_INVISIBLE`, `CF_XRAY`, and `CF_SPY`; emits the C-shaped feedback strings; and refreshes the client after x-ray changes. Focused server tests cover area-gated Live Quest toggles, god-only x-ray/spy, and existing `/saves` regression behavior. Remaining command/admin gaps include exact `update_char` side effects for x-ray beyond item refresh, name-cache invalidation for visibility/color-changing commands, and exact audit side effects.
- Player text commands now cover the C god-only `/joinclan <nr>` and `/joinclub <nr>` local identity mutation slice: full command names are required like `cmdcmp(..., minlen=8)`, non-god callers do not claim the commands, valid clan IDs `0..31` set `clan` plus rank `4`, valid club IDs `0..16383` set `clan = nr + 1024` plus rank `2`, invalid IDs are handled silently without mutation, and visible-name refresh is requested for client identity packets. Focused server tests cover clan/club mutation, out-of-range no-op handling, privilege gating, and exact-command recognition. Remaining clan/admin gaps are repository-backed clan/club serial lookup, destructive `/killclan`/`/killclub`, and exact audit/name-reset side effects.
- Player text commands now cover the C character-color slice from `src/system/command.c`: `Character` carries serde-defaulted legacy `c1`/`c2`/`c3` color words, `/col1`, `/col2`, and `/col3` pack RGB arguments with the C `(r << 10) + (g << 5) + b` formula, god-only `/color` reports the current packed words, `SV_NAME` packets include the stored color words, and successful color changes refresh known visible-character name packets for clients whose map cache has already seen that character. Focused server tests cover command packing, god-only readback, packet byte layout, and cache refresh. Remaining color-command gaps are exact legacy `atoi` pointer edge cases for malformed negative/mixed inputs and audit/xlog parity.
- Player text commands now cover the C staff/god `/shutup <name> <minutes>` core slice: full-command recognition, staff/god privilege gating, legacy alphabetic target-name parsing, default 10-minute duration, `atoi`-prefix minute parsing, 0..60 minute range feedback, online player lookup, `CF_SHUTUP` set/clear behavior, and the no-success-feedback path are covered by focused server tests. Remaining shutup gaps are cross-server/offline `lookup_name` scheduling, `server_chat` propagation, scrollback/audit records, and fixed-layout `DRD_SWEAR_PPD` persistence.
- `src/module/book.c` `IDR_BOOK = 16` now dispatches from the Rust item-driver registry for deterministic text book cases: Loisan diary pages, Superior prototype notes, Vampire notes (`drdata[0]` kinds `0..12`), static earth/ice demon lore (`20..30`), rune hints (`32..38`), static bones/Kir/Shrike/Mad Mage/arena text (`40..42`, `45..46`), and Lab 2 diary pages (`100..101`) return a typed `BookText` outcome and the server emits the corresponding legacy `log_char` text lines as client feedback. Focused core tests pin the driver ID, zero-character no-op behavior, `drdata[0]` kind decoding, and representative legacy text for early, later, and high-numbered book kinds. Remaining book-driver gaps are demon-language dynamic text, demon-knowledge sign gates, raw color-marker text cases, `player_special` side effects for two earth-demon diaries, and live-data smoke coverage.
- `src/system/libload.c` hard-coded item-driver area guards are now represented at the Rust item-driver registry edge for `IDR_BONEBRIDGE`, `IDR_BONEHINT`, `IDR_NOMADDICE`, `IDR_STAFFER2`, `IDR_OXYPOTION`, `IDR_LIZARDFLOWER`, `IDR_CALIGAR`, and `IDR_ARKHATA`: outside their legacy areas they return a handled typed outcome and the server emits the C-shaped `This does not work outside its area.` feedback instead of falling through as unsupported. Focused core tests pin the dispatch-critical IDs, required areas, and correct fall-through inside area 18. Remaining work is the actual in-area behavior for the still-unported Bones/Nomad/Brannington/Caligar/Arkhata drivers.
- Player text commands now cover C `/time` from `src/system/command.c` / `src/system/date.c`: the Rust runtime recognizes the legacy `cmdcmp(ptr, "time", 2)` prefix shape, formats the current in-game time/date, sunrise/sunset, moonrise/moonset, moon phase, solstice/equinox event text, next moon phase, and next seasonal event from the ported `GameDate` state, and emits the same system feedback lines with focused server tests. Remaining command gaps include broader player/staff/admin command coverage, exact realtime source parity for all command call sites, and audit/log side effects where applicable.
- Player text commands now cover the C god-only `/sprite <num>` admin helper from `src/system/command.c`: full-command recognition, god privilege gating, legacy `atoi`-prefix parsing, silent character sprite mutation, and visible-name refresh signaling for the existing demon-sprite color packet edge are represented with focused server tests. Remaining command/admin gaps include exact `set_sector`/dirty-sector parity, audit/log side effects, and broader admin command coverage.
- Player text commands now cover C `/hints` from `src/system/command.c`: `PlayerRuntime` preserves the `struct lostcon_ppd.hints` field at the fixed C offset alongside `maxlag`, lostcon PPD encoding/decoding appends/replaces a hints-only block, and the runtime command path toggles the flag with the legacy `cmdcmp(ptr, "hints", 4)` abbreviation plus `Hints turned off/on.` feedback. Focused core/server tests cover byte layout, outer PPD persistence, and command toggle behavior. Remaining lostcon command gaps are the broader lost-connection automation toggles and exact command/help audit side effects.
- `src/module/book.c` raw color-marker book text coverage has been extended for `BOOK_RUNES1`, `BOOK_BONES1`, `BOOK_GWENDYLON`, and `BOOK_MADMAGES_BOOK1`: Rust now includes the missing deterministic lines, exposes a byte-preserving book-line helper for legacy `COL_DARK_GRAY`/`COL_RESET` markers that cannot be represented by UTF-8 `String`, and the server sends book feedback through `SV_TEXT` byte packets for marker preservation. Focused core/protocol/server compilation tests cover the raw marker output path. Remaining book-driver gap is live-data smoke coverage.
- Plain local speech now mirrors the C `command.c` pre-`say()` `demontest` hook: non-command text is treated as local `say`, underwater speech becomes `Blub.` before ritual checks, matching character-specific demon ritual words from `ID_rand` raise bare `V_DEMON` protection up to the spoken ritual cap, set `CF_UPDATE`, and emit the legacy protective-ritual feedback plus the cannot-utilize-full-knowledge warning when applicable. Focused server tests cover plain-say delivery, ritual success, and underwater suppression. Remaining demon-command gaps are the broader `/demon*` command family and exact audit/log side effects.
- `src/module/book.c` `BOOK_NOOK_JOKES` now preserves the C `RANDOM(5)` shape: using the book selects one of the five legacy two-line joke pairs at runtime and sends it through the byte-preserving book feedback path. Focused core coverage pins the joke table and modulo wrapping. Remaining book-driver gap is live-data smoke coverage.
- `src/module/book.c` `BOOK_EDEMON3` / `BOOK_EDEMON4` now preserve their C `player_special(cn, 0, 50287/50305, 0)` side effects: Rust maps book kinds `22` and `23` to the exact legacy `SV_SPECIAL` payloads and sends them after the byte-preserving diary text. Focused core coverage pins the special IDs and the server book path compiles against the runtime payload type. The previously listed book-driver gaps for demon-language dynamic text and demon-knowledge sign gates are already covered by `book_text_line_bytes_for_reader_id`: `BOOK_DEMON1..5` use the C-shaped `id_rand`/`demonspeak` word generator keyed by character id, and `SIGN_EDEMON1..2` gate readable lines on effective Ancient Knowledge. Remaining book-driver gap is live-data smoke coverage.
- `src/module/book.h` book/sign kind constants are now represented in Rust by named `BOOK_*` / `SIGN_*` constants in `item_driver.rs`, including the Lab 2 high-ID diary cases, and the book text helpers/tests now use the named IDs instead of raw dispatch numbers. Focused core coverage pins representative legacy values against the C header. Remaining book-driver gaps are live-data smoke coverage and any future non-text side effects discovered in area data.
- `src/module/book.c` `SIGN_EDEMON1` / `SIGN_EDEMON2` now preserve the C reader-gated earth-demon sign text: `BookText` carries the reader's effective `V_DEMON`, signs below knowledge 1 emit the unreadable text, knowledge 1 emits the partial-recognition text, and knowledge 2+ emits the exact control-room/laboratorium sign lines through the byte-preserving book feedback path. Focused core coverage pins all three gates. Remaining book-driver gap is live-data smoke coverage.
- `src/module/book.c` `BOOK_DEMON1`..`BOOK_DEMON5` now preserve C `demonspeak` dynamic ritual generation: Rust ports `ID_rand`, the syllable/lead tables, and the per-reader character-ID derivation, and the runtime passes the reader ID into the byte-preserving book feedback path. Focused core coverage pins the legacy C comment example and verifies different reader IDs generate different ritual words. Remaining book-driver gap is live-data smoke coverage.
- `src/system/command.c` `/who` now has a Rust runtime command slice: legacy `cmdcmp`-style short-prefix recognition for `who`, area-local listing header, visible-player filtering, invisible suppression, staff/god `CF_NOWHO` suppression, NPC suppression, and C-shaped `Name (AWMlevel)` class/level formatting are covered by focused server tests. Remaining command gaps include full chat/tell/ignore/status/time/weather commands, command aliases, and exact multi-area/session filtering once cross-area runtime transfer is implemented.
- `src/system/command.c` `/whostaff` and `/nowho` now have Rust runtime command slices: staff/god-only `/whostaff` preserves the C `cmdcmp(..., minlen=4)` recognition, filters invisible/non-player/non-staff entries, emits lagging markers for characters with nonzero drivers, and `/nowho` preserves full-command staff/god-only toggling of `CF_NOWHO` with the legacy `NoWho enabled/disabled.` feedback. Focused server tests cover privilege gates, prefix recognition, listing filters, lagging markers, and toggling. Remaining command gaps include full chat/tell/ignore/status/time/weather commands, command aliases beyond covered slices, staff-code display persistence, and exact multi-area/session filtering once cross-area runtime transfer is implemented.
- `src/system/command.c` `/maxlag <seconds>` now has a Rust runtime command slice: legacy `cmdcmp(..., minlen=4)` recognition, C `atoi`-prefix parsing, `3..=20` range enforcement, exact feedback text, serde-defaulted `PlayerRuntime::max_lag_seconds` storage, and fixed-layout `DRD_LOSTCON_PPD` outer PPD decode/encode for the C `struct lostcon_ppd` `maxlag` field are covered by focused tests. Remaining lost-connection work is the actual lost-connection driver behavior and the other `lostcon_ppd` automation toggles.
- `src/system/command.c` `/status` now has a Rust runtime command slice for the represented C `show_lostconppd`/account-status output: legacy `cmdcmp(ptr, "status", 0)` prefix recognition, lag-control header, persisted max-lag line, represented default potion/recall/move/automation toggles, spell-gated option lines, bless-permission line, and paid/trial account status are emitted with focused server tests. Remaining `/status` gaps are account expiration timing, persistence/runtime mutation for the other `lostcon_ppd` automation toggles, and exact command-order conflicts as more short `s*` commands are ported.
- Area 18 `IDR_BONEBRIDGE` now dispatches through the static item-driver registry for the player-visible full-bone placement path from `src/area/18/bones.c`: Rust verifies the full `IID_AREA18_BONE` cursor stack, computes the legacy adjacent bridge target from actor/base positions, requires an empty permanently blocked target tile, places the cursor bone as a temporary non-takeable bridge with horizontal/vertical sprite selection, clears the cursor and inventory dirty flag, schedules the 60-second timer, retries cleanup while temporarily blocked, ages sprite/drdata every cleanup tick, and finally removes the bridge while restoring `MF_MOVEBLOCK`. Focused item-driver/world tests cover dispatch, placement, retry, aging, and final removal. Remaining area-18 bone gaps are partial bridge add/remove with template-backed `create_item("bone")`, exact feedback/log text, and ladder/holder/wall/hint/rune quest behavior.
- `src/system/command.c` `/weather` now has a Rust runtime command slice: full-command recognition, C-shaped current-area weather line, default clear-weather state, indoor protection text from `MF_INDOORS`, outdoor effect feedback for slow/blind/damage/slip flags, and god debug lines for current weather, intensity, effects, transition state/progress, next change, and affected areas are covered by focused server tests. Remaining weather gaps are the actual global/area weather update model, admin weather commands, persistence/synchronization, client `SV_MOD2` weather packets, and live movement/visibility/combat weather effects.
- `src/system/command.c` god-only `/setxmas <value>` now has a Rust runtime command slice: full-command recognition, legacy `atoi`-prefix parsing, C-shaped `Setting christmas special to ..., old value was ...` feedback, a runtime Christmas-special override preserving default date-window behavior until explicitly set, and `IDR_XMASTREE` runtime use consults the override before falling back to the date-based Christmas window. Focused server tests cover on/off overrides and old-value reporting. Remaining Christmas parity gaps are cross-server `server_chat(1035)` propagation, persistence/restart behavior for the global flag, and exact event-system synchronization.
- `src/system/command.c` `/tell` and `/notells` now have a Rust runtime command slice backed by the ported `TellData`: command text is no longer lowercased before dispatch, local online target lookup preserves mixed-case tell bodies, sender feedback/error/self-tell text matches the C command path, staff senders bypass `CF_NOTELL`, delivered local tells acknowledge receipt by clearing the pending tell slot, `CF_NOTELL` recipients block normal tells, and the tick loop drains strict one-second tell timeouts into the legacy `"<name> is not listening."` feedback with focused server tests. Remaining tell/chat gaps are spy forwarding, cross-area/offline lookup/chat routing, raw color/link marker parity, and dlog/audit records.
- `src/system/ignore.c` and the `/ignore` command slice from `src/system/command.c` are now represented in Rust: `PlayerRuntime` carries the fixed 100-entry C `DRD_IGNORE_PPD` ID array, decodes/encodes it through the legacy outer PPD blob while removing empty blocks, `/ignore` uses the legacy three-character abbreviation to list/toggle local online players with C-shaped feedback, `/clearignore` clears the list, normal `/tell` delivery respects recipient ignore lists, and staff/god tell mode bypasses ignore just like C channel `1030`. Focused core/server tests cover byte layout, outer block replacement/removal, command feedback, and tell suppression. Remaining tell/chat gaps are spy forwarding, cross-area/offline lookup/chat routing, raw color/link marker parity for tells, and dlog/audit records.
- `src/module/alchemy.c` `IDR_FLOWER = 33` now dispatches from the Rust item-driver registry for regular alchemy ingredient picking: zero-character calls no-op, occupied cursors return the legacy empty-hand feedback, location IDs use `x + (y << 8) + (areaID << 16)`, the existing C-compatible `DRD_FLOWER_PPD` cooldown state is reused with herbalist ripe-time thresholds, valid kinds `1..20` instantiate the matching `alc_flower*`, `alc_mushroom*`, or `alc_berry*` template to the cursor, and cooldown state is marked only after successful cursor placement. Focused core/server tests cover dispatch, cursor gating, template reward placement, and PPD marking. Remaining alchemy gaps are `IDR_FLASK` mixing/use behavior, exact achievements/dlog/audit side effects, and live alchemy-data smoke coverage.
- `/shutup` runtime state now records a per-player mute expiry in realtime seconds, sends the C light-red target feedback for manual disable/enable, drains expired mutes from the tick loop, clears `CF_SHUTUP`, and sends the C light-red enabled feedback to the target. The fixed-layout C `DRD_SWEAR_PPD` block (`lasttalk[10]`, `bad`, ten 80-byte sentence slots, `last_time[10]`, `last_cnt[10]`, `last_pos`, `banned_till`) is decoded/encoded through `PlayerRuntime`, maps `banned_till` to `shutup_until_seconds`, preserves existing swear counters/sentences, replaces/appends/removes the outer PPD block on snapshot save, and is covered by focused core/server tests. Remaining shutup gaps are cross-server/offline scheduling, server-chat propagation, scrollback, and audit records.
- Chat channel management commands now cover the C `list_chat`/`join_chat`/`leave_chat`/`join_all` slice: `/channels` emits the legacy 0..14/31/32 channel table with C-shaped fixed-width formatting, `/join <nr>` and `/leave <nr>` use legacy `atoi`-prefix parsing and feedback, `/joinall` sets channels 1..13, staff/event/god gates are enforced for channels 31/32, and per-player channel membership is retained in `PlayerRuntime` with serde defaults. Focused server tests cover listing, membership bit updates, duplicate/already-left feedback, `/joinall`, and privilege gates. Remaining chat gaps are actual channel message fan-out, spy forwarding, cross-area chat routing, raw color/link marker parity, and dlog/audit records.
- Chat channel message fan-out now covers the local-server C `cmd_chat`/`write_chat`/`rec_chat` slice: `/cN` and channel-name prefixes resolve through the legacy channel table, empty/too-long/access/not-joined/clan/club guards emit the C-shaped feedback, joined recipients receive byte-preserving `SV_TEXT` chat lines with legacy channel color markers, staff/god/event sender coloring and staff uppercase names are preserved, normal recipient filtering honors joined-channel bits, ignore lists, same-clan private channels, area/mirror gates, and privilege pruning for development/staff/god channels. Focused server tests cover delivery formatting, join/access gates, ignored senders, and clan filtering. Remaining chat gaps are spy forwarding, cross-area/offline chat routing, clan-alliance repository policy, real club membership, swearing/dlog/audit records, and exact staff-code persistence.
- Local chat/tell spy forwarding now mirrors the C `forward_to_spies` slice for represented runtime state: gods with `CF_SPY` receive byte-preserving dark-gray `[SPY/TELL]`, `[SPY/CLAN]`, `[SPY/ALLIANCE]`, `[SPY/CLUB]`, `[SPY/AREA]`, and `[SPY/MIRROR]` system lines when they would not already be sender/recipient or in the matching normal channel scope, and tell spy copies are emitted before recipient `notells`/ignore filtering like C. Focused server tests cover blocked tells and private clan chat spy delivery. Remaining chat gaps are cross-area/offline chat routing, clan-alliance repository policy beyond direct same-clan checks, real club membership, swearing/dlog/audit records, and exact staff-code persistence.
- `src/system/swear.c` / `src/system/game/ppd_structs.h` `DRD_SWEAR_PPD` persistence is now represented for the `/shutup` runtime slice: `PlayerRuntime` decodes/encodes the fixed 932-byte C `struct swear_ppd`, preserves existing recent-talk/sentence/counter fields, maps `banned_till` to the Rust mute expiry, replaces/appends/removes the outer PPD block with other player data, and restores `CF_SHUTUP` when DB snapshot load finds an active persisted mute. Focused core/server tests cover byte layout, outer block behavior, and snapshot restoration. Remaining swear/chat gaps are actual swear/excessive-chat detection, cross-server/offline shutup scheduling, server-chat propagation, scrollback, and audit records.
- `src/system/swear.c` `swearing()` runtime enforcement is now wired into local speech, `/tell`, and chat-channel sends: Rust uses the fixed-layout `DRD_SWEAR_PPD` counters in place, honors active `banned_till` chat blocks, exempts gods only after the active mute check like C, applies the 30-second post-offense block, ports the legacy recent-line flood thresholds, blocked-word list, all-caps check, repeated-long-sentence table including the C `last_pos` timestamp quirk, shifts `lasttalk[]` only after accepted speech, and emits raw light-red client feedback bytes. Focused server coverage pins blocked-word suppression and follow-up `Chat is blocked.` behavior. Remaining swear/chat gaps are cross-server/offline shutup scheduling, server-chat propagation, scrollback/audit records, and exact non-ASCII `ctype` parity for the all-caps/block-word scans.
- `src/system/command.c` `/lag` now has a Rust runtime command slice: full command recognition, arena and non-empty-hate-list guards only when enabling artificial lag, `CF_LAG` toggling, the legacy on/off feedback, and the C warning line when enabling are wired through the text-command path with focused server tests. Remaining lag-control gaps are full lost-connection automation behavior and exact excess-lag handling beyond the existing `/maxlag` persistence.
- C god-only `/dlight <value>` and `/showattack` admin command cores are now represented in the Rust runtime command path: `/dlight` preserves the full six-character command requirement, `atoi`-prefix parsing, global override mutation, and no-feedback behavior; `/showattack` preserves the six-character abbreviation minimum, toggle behavior, no-feedback behavior, and now toggles world-level attack debug output. `World::apply_legacy_hurt` queues the C-shaped `hurt by ...` and `dam after armor ...` debug lines for affected targets when attack debugging is enabled. Focused core/server tests cover god gates, abbreviation handling, runtime state mutation, and hurt debug text. Remaining command/admin gaps include wiring the `dlight_override` runtime field into daylight/light call sites, adding `showattack` output for the pure `act_attack` pre-hurt line, and exact audit side effects.
- C god-only `/dlight <value>` now applies the daylight override immediately to the world game-date state using the current game-time timestamp and area, and `/dlight 0` clears back to natural area light. Focused server coverage pins nonzero override application and zero reset behavior. Remaining command/admin gaps include adding `showattack` output for the pure `act_attack` pre-hurt line and exact audit side effects.
- C `/showattack` direct melee pre-hurt output is now represented in the Rust attack completion path: `AttackResolution` carries the legacy attack/parry/chance/armor operands from `sub_attack`, and `World::complete_attack_with_rolls` queues the attacker-facing `attack <name>, diff=...` system line before applying the existing target-facing `hurt by ...` / `dam after armor ...` debug output. Focused core coverage pins the C typo-compatible `chan` line. Remaining command/admin gaps include exact audit side effects.
- Player text commands now cover C `/description` from `src/system/command.c`: the Rust runtime recognizes the legacy minimum three-character prefix, requires non-empty text, replaces double quotes with single quotes and percent signs with spaces, truncates to the 159-byte `LENDESC - 1` payload, stores the character description, and emits the C-shaped confirmation/error feedback. Focused server tests cover sanitization, empty input, truncation, and rejected too-short/extra-suffix command forms.
- Player local speech commands now cover the basic C `/holler`, `/shout`, `/say`, `/murmur`, and `/whisper` command path: exact full-command recognition, `CF_SHUTUP` blocking text, underwater `Blub.` fallback without shout/holler endurance cost, default legacy talk distances/costs from `GameSettings`, shout/holler endurance spending plus regeneration-ticker stamp, quote rejection for the C-rejecting speech modes, and byte-preserving local connected-player fan-out through existing legacy text formatters are covered by focused server tests. Remaining speech/chat gaps are swear detection, audit/dlog side effects, dynamic admin-tuned talk settings instead of default settings, exact sector-hearing/shout-sector filtering in the command fan-out, and cross-area chat routing.
- Player local emote commands now cover the C `/emote`/`/me` and shortcut slice: legacy `cmdcmp` abbreviations for `/em`, `/me`, `/slap`, `/wa`, `/hugme`, `/bo`, and `/eg`, C-shaped emote text formatting through the byte-preserving speech fan-out, quote rejection through the existing `emote` formatter, `/slap <name>` trout text, and underwater `/emote`/`/me` fallback to `feels wet` are covered by focused server tests. Remaining speech/chat gaps are swear detection, audit/dlog side effects, dynamic admin-tuned talk/emote settings instead of default settings, exact sector-hearing filtering in the command fan-out, and cross-area chat routing.
- Player text commands now cover the C `/noexp` and `/nolevel` toggles from `src/system/command.c`: full-command recognition, `CF_NOEXP` / `CF_NOLEVEL` mutation, C-shaped NoExp and NoLevel feedback, client refresh signaling, and the legacy area-3 Gatekeeper-room coordinate block that prevents enabling either mode while still allowing disabling are covered by focused server tests.
- C god-only `/itemname` and `/itemdesc` cursor-item commands are now represented in the Rust runtime command path: full-command recognition, god-only gating, missing-cursor `Need citem.` feedback, 79-byte legacy C string truncation, cursor item name/description mutation, legacy look-item feedback reuse, and inventory refresh signaling are covered by focused server tests. Remaining item-admin command gaps are exact `look_item` color/ITEMDESC marker parity, audit/xlog side effects, and broader item-modification command coverage.
- C god-only `/itemmod <pos> <skill-or-nr> <value>` cursor-item command is now represented in the Rust runtime command path: full-command recognition, god-only gating, legacy `atoi`-style position/value parsing, C `lookup_skill` aliases for item modifier value names, missing-cursor and position/value-number/value bounds feedback, modifier slot mutation, legacy look-item feedback reuse, and the C-shaped `Item modified: ...` system line are covered by focused server tests. Remaining item-admin command gaps are exact `set_item_requirements` recomputation side effects beyond current look output, audit/xlog side effects, and broader item-modification command coverage.
- Area 17 lock-picking notification fan-out now mirrors the C `notify_area(..., NT_NPC, NTID_TWOCITY_PICK, cn, 0)` side effect for successful `IDR_PICKDOOR`, `IDR_PICKCHEST`, and `IDR_BURNDOWN` ignition paths. `World::notify_area` queues legacy driver messages to characters in the 16-tile square, pick-door and burn-barrel world mutations invoke it directly, and the server pick-chest reward path invokes it after successful cursor reward creation. Focused core tests cover nearby delivery and out-of-range suppression. Remaining Area 17 gaps are `DRD_TWOCITY_PPD` thief quest mutation, exact sector/light invalidation parity beyond current dirty-sector/light refresh hooks, dlog/audit side effects, and live area-data smoke coverage.
- Player text commands now cover the C god-only `/setexpmod <value>` global tuning helper from `src/system/command.c`: full-command recognition, god-only gating, `atof`-style prefix parsing, `0.1..=1000.0` validation, runtime EXP modifier storage with default `1.0`, and C-shaped success/error feedback are covered by focused server tests. Remaining global-tuning command gaps include wiring the runtime EXP modifier into every experience grant path, persistence/admin audit side effects, and the related hardcore bonus tuning commands.
- C god-only `/staffcode <name> <code>` now has a Rust runtime command slice: legacy six-character prefix recognition, alphabetic online-target parsing, two-letter uppercase staff-code parsing with `A` defaults for missing/non-letter positions, C-shaped not-found and success feedback, and runtime display integration for `/whostaff`, `/tell`, and local chat staff brackets are covered by focused server tests. Remaining staff-code gaps are persistence in the character snapshot/schema, exact audit/server-chat side effects in staff admin commands that consume the code, and cross-area/offline target lookup.
- Area 17 `IDR_COLORTILE = 86` now dispatches from the item-driver registry for the C `colortile` puzzle path: zero-character/non-player calls no-op, row/color are decoded from `drdata[0..1]`, the existing Two-City runtime color state is initialized with C-shaped `RANDOM(6)+1` rolls, matching tiles no-op successfully, wrong tiles randomize the five good colors, emit the legacy color-dancing feedback, and same-area teleport the player to `(5,250)`. Focused core coverage pins dispatch and the server runtime branch is compiled through workspace tests. Remaining Area 17 gaps include `IDR_BURNDOWN`, `IDR_SKELRAISE`, fixed-layout `DRD_TWOCITY_PPD` persistence beyond represented fields, quest/dialogue state, dlog/audit parity, and live area-data smoke coverage.
- C god-only `/setskill <name> <skill-or-nr> <value>` now has a Rust runtime command slice: full-command recognition, god-only gating, C alpha-only online target-name parsing, skill-name or numeric value lookup, position/value bounds feedback, bare-value mutation in `value[1]`, C `calc_exp`-style `exp_used` recalculation including supermax costs, `CF_UPDATE` marking, and the legacy result line with exp-used delta are covered by focused server tests. Remaining setskill/admin gaps are exact `update_char` recomputation side effects beyond current value/refresh flags, audit/xlog side effects, and cross-area/offline target lookup.
- C god-only `/create <template>` now has a Rust runtime command slice: legacy three-character prefix recognition, god-only gating, occupied-cursor and missing-template feedback, real `ZoneLoader` template instantiation, cursor placement, carried-character ownership, `CF_ITEMS` marking, and inventory refresh signaling are covered by focused server tests. Remaining create/admin gaps are exact `dlog` audit side effects, bond-take/bond-wear side effects, and broader item-admin command coverage.
- C `/col1`/`/col2`/`/col3` malformed color argument parsing now preserves the legacy pointer-walk around `atoi`: after each component Rust skips only digit bytes like C's `while (isdigit(*ptr))`, so negative values and mixed digit/non-digit tokens can intentionally leave the parser on the same byte for the next component. Focused server coverage pins negative and mixed-token edge cases.
- C god-only `/prof` now has a Rust admin command boundary: it preserves the legacy four-character `cmdcmp` recognition, non-god fall-through behavior, and emits the same profile header/footer used by `cmd_show_prof` for the currently represented no-counter runtime. Focused server coverage pins recognition, privilege gating, and output shape. Remaining profiling gaps are adding a real Rust profiling table if runtime profiling parity becomes useful.
- Player text commands now cover the C non-live-quest `/wimp` fallback from `src/system/command.c`: the Rust runtime recognizes the legacy four-character minimum prefix shape and emits the exact `You're not in the live quest area... RUN!` system feedback. Focused server coverage pins recognition and rejection of too-short/extra-suffix forms. Remaining Live Quest command gaps are the actual area-specific wimp-out behavior and related quest state side effects.
- Area 4 `IDR_PENT` activation/deactivation world mutation now has a first runtime slice matching C `activate_pentagram` / `deactivate_pentagram`: player activation marks `drdata[1]`, applies the color sprite offset, raises `V_LIGHT` modifier value to 100, refreshes item light, and queues legacy sound `42`; timer callbacks with nonzero status clear `drdata[1]`/`drdata[4]`, remove the color sprite offset, restore light modifier value `10`, and refresh item light. Focused core coverage pins activation, sound fan-out, and timer deactivation. Remaining pentagram gaps are quest solve counters/serials, player NPPD participation state, demon spawning, rewards/records, exact global RNG parity, and live area-4 data smoke coverage.
- Area 17 `IDR_PICKDOOR` lock-pick side effects now distinguish the actual C picked-lock branch from timer auto-close: `PickDoorToggle` carries whether a player picked the lock, timer closes no longer emit `You pick the lock.` and no longer fan out `NTID_TWOCITY_PICK`, while successful player opens still do both. Focused core/world tests cover player open and timer close parity.
- C god-only `/listitem <id>` now has a Rust runtime command slice: the legacy five-character `cmdcmp` prefix is accepted, non-god callers do not claim the command, invalid IDs emit the C-shaped error, and valid live items report item number/name, description, hex flags, driver/template ID/sprite, carried-by or map position, and nonzero modifier lines with legacy value names. Focused server tests cover privilege gating, invalid IDs, prefix recognition, carried/position output, and modifier formatting.
- C god-only `/setlevel <level>` now has a Rust runtime command slice: full-command recognition, god-only gating, C `atoi`-prefix level parsing, `level2exp(level)` fourth-power experience assignment, self level mutation, C arch flag transitions for below-30 and above-35 levels, mage-only `V_DURATION` and warrior-only `V_RAGE` initialization, carried spell-slot `12..29` destruction, attached character-effect cleanup, and silent success with client refresh signaling are covered by focused server tests. Remaining setlevel/admin gaps are exact `update_char` recomputation side effects and audit/xlog parity.
- Area 15 `IDR_SWAMPARM = 73` now dispatches behind the legacy swamp libload guard for C `swamparm`: zero-character timer callbacks scan the C horizontal and adjacent-row trigger offsets, preserve the 1-in-5 / 1-in-3 activation probability shape through the current deterministic runtime seam, advance `drdata[0]`/sprite animation frames, reset after frame 15, reschedule every tick, and apply C `hurt(co, 10 * POWERSCALE, 0, 1, 50, 90)` damage to horizontal targets on frame 12. Focused core/world tests cover timer-only behavior, area guard, animation, reset, and damage target selection. Remaining swamp item gaps are exact global RNG parity, Clara/monster character-driver behavior, remaining hardkill/quest side effects, and live area-15 data smoke coverage.
- Area 12 `IDR_MINEWALL = 60` now dispatches through the mine item-driver boundary for the first C `minewall` slice: the existing area-12 libload guard applies, zero-character timer calls initialize the legacy random-looking wall sprite from `(x + y) % 3`, preserve the one-time `drdata[4]` initialized flag, expose the opened-wall collapse timer boundary for `drdata[3] == 8`, player use blocks occupied cursors and exhausted diggers with typed outcomes, successful digs apply the C miner-profession endurance formula, increment `drdata[3]`, reset `drdata[5]`, advance the sprite, stage 3 removes temporary sight blocking, stage 8 opens the wall by clearing the map item/move blocker and setting `IF_VOID`, and collapse timers restore the wall when the tile is free or retry while blocked. Focused core/world tests cover initialization, collapse boundary, cursor/exhaustion gates, miner endurance reduction, stage mutation, open-map mutation, and collapse restoration. Remaining mine-wall gaps are mining result RNG/rewards/cave-ins/golems/artifacts, player dig animation toggles, exact dlog/audit side effects, and live area-12 data smoke coverage.
- C god-only `/setkarma <name> <value>` now has a Rust runtime command slice: `Character` carries a serde-defaulted legacy karma field, the command preserves `cmdcmp(..., minlen=5)` prefix recognition, god-only gating, online target lookup, C `atoi`-prefix value parsing, mutation of the target character's karma, and exact success/not-found feedback. Focused core/server tests cover snapshot defaults, command prefix behavior, privilege gating, target mutation, and missing-target feedback. Remaining karma/admin gaps are audit/log parity and broader karma-system behavior beyond the admin setter.
- C god-only `/listchars` now has a Rust runtime command slice: it preserves the legacy five-character prefix recognition, god-only gating, active character scan, player/NPC counters, ID-ordered C-shaped `Player:` / `NPC:` output with NPC listing limited by the same running-count guard, and total summary line. Focused server tests cover privilege gating, short-prefix rejection, and mixed player/NPC output formatting. Remaining admin command gaps include exact audit/log side effects and broader cross-area visibility once real area transfer is implemented.
- C god-only `/questlog <name>` now has a Rust runtime command slice: it preserves the legacy five-character prefix recognition, god-only gating, online target lookup, missing-target feedback, missing-runtime quest-data feedback, and C-shaped `Quest log for ...` / `Quest #n: Open|Closed, Done level: d` output for entries with nonzero flags using the retained `PlayerRuntime.quest_log`. Focused server tests cover open/closed quest formatting, privilege gating, short-prefix rejection, missing target, and missing runtime data. Remaining questlog/admin gaps include offline/repository lookup, exact audit/log side effects, and full quest initialization/area PPD side-effect parity.
- Area 36 `IDR_CALIGAR` skelly-door retry semantics now preserve the C `caligar_skelly_door` return-code edge: diagonal/invalid/no-progress world applications and busy targets surface the typed retry-shaped outcome instead of generic no-op, and `CaligarSkellyDoorBusy` maps to legacy item-driver return code `2` like locked skelly doors. Focused core tests cover player success, busy target, diagonal touch, and return-code classification.
- C god-only `/sethardcoreexpbonus`, `/sethardcoremilexpbonus`, and `/sethardcorekillexpbonus` tuning commands now have Rust runtime state and command handling alongside `/setexpmod`: full-command recognition, god-only fall-through, `atof`-prefix parsing, legacy value ranges (`0.1..=1000.0` for general/military hardcore EXP bonuses and `1.0..=3.0` for kill EXP), default values from `GameSettings`, and C-shaped success/error feedback are covered by focused server tests. Remaining global-tuning gaps include wiring these runtime values into all relevant EXP grant paths, persistence/admin audit side effects, and exact live settings reload parity.
- C god-only `/setdecaytime`, `/setplayerbodytime`, `/setnpcbodytime`, `/setnpcbodytimearea32`, `/setrespawntime`, `/setlagouttime`, and `/setregentime` tuning commands now have Rust runtime state initialized from `GameSettings`, god-only command handling, C `cmdcmp` minimum-length recognition, `atoi`-prefix parsing, legacy range gates, and C-shaped success/error feedback with focused server tests. Remaining global-tuning gaps include wiring these mutable runtime timer values into every decay/body/respawn/lagout/regeneration call site, persistence/admin audit side effects, and exact live settings reload parity.
- C god-only `/setsewerrespawntime <seconds>` now has Rust runtime state initialized from `GameSettings::sewer_item_respawn_time`, preserves the legacy full-command recognition, `atoi`-prefix parsing, 1-hour-to-7-day range gate, and C-shaped success/error feedback including seconds and hour conversions. Focused server tests cover valid changes, invalid bounds, god-only gating, and minimum-prefix rejection. Remaining sewer tuning gaps are wiring the mutable runtime value into all rat/sewer item respawn call sites, persistence/admin audit side effects, and exact live settings reload parity.
- C god-only communication tuning commands `/sethollerdist`, `/setshoutdist`, `/setsaydist`, `/setemotedist`, `/setquietsaydist`, `/setwhisperdist`, `/sethollercost`, and `/setshoutcost` now have Rust runtime state initialized from `GameSettings`, preserve the legacy `cmdcmp` minimum lengths, `atoi`-prefix parsing, C range gates, C-shaped success/error feedback, and god-only fall-through behavior. Local speech fan-out now uses the mutable runtime distances/costs instead of fresh defaults, and `/murmur` uses the separate quiet-say distance like C. Focused server tests cover command mutation, privilege/minimum-length gates, runtime shout cost/range, and quiet-say range. Remaining communication tuning gaps are persistence/live settings reload parity, audit/xlog side effects, and exact sector-hearing/shout-sector filtering in speech fan-out.

Chest gaps still to port:

- Live PostgreSQL migration/data smoke verification for `DRD_KEYRING_PPD` and `DRD_TREASURE_CHEST_PPD` persistence across logout/server restart.
- Achievement persistence/protocol sending beyond runtime marker updates.
- `IDR_RANDCHEST = 34` exact RNG parity and full live-data smoke coverage.

Recommended next chest steps:

1. Verify treasure chest cooldowns survive logout/server restart against PostgreSQL-backed character snapshots.
2. Persist/runtime-load chest achievement state and send achievement protocol updates.
3. Verify `IDR_RANDCHEST` daily access persistence against PostgreSQL-backed snapshots and full loot table behavior against live data.
4. Replace scaffold fallback with robust client-facing DB login rejection/transfer behavior once password verification and selection are complete.

### Other High-Value Next Steps

- Continue the per-session map cache toward exact `player.c` parity: effects, known-name packets, light/dark/LOS changes, sector skip optimization, and broader runtime invalidation coverage.
- Wire the ported positional `sound_area` primitive through remaining combat/effect/driver sound call sites and port exact legacy ambient-sound call cadence/RNG parity.
- Integrate PostgreSQL-backed login/character selection and logout save instead of the temporary `new_warrior_m` scaffold.
- Continue combat action execution: exact RNG parity, full `sub_attack` side effects, hurt/death integration, rage/surround hits, fightback state, clan/hate/group `can_attack` policy, and feeding hit notification messages into NPC driver queues from the full combat path.
- Continue spell queue execution beyond the primitive heal/magic-shield/pulse/bless bridge: projectile/effect-backed targeted fireball/ball/flash, moving-target fireball prediction, spell item expiry/effect lifecycle, notifications/sounds, and exact policy checks.
- Continue door family details: remaining multi-tile door edge cases and exact sound/light/LOS invalidation side effects.
- Continue account depot integration: `DRD_ACCOUNT_WIDE_DEPOT` raw item-blob decode/encode, loading/saving account-depot snapshots across logout/restart, exact `look_item` color/ITEMDESC marker/keyring-hover packet parity, immediate DB flush behavior, and generic container/depot/merchant GUI command handling.
- Finish runtime integration for `IDR_NIGHTLIGHT`, `IDR_TORCH`, and `IDR_TOYLIGHT`: character stat recomputation, exact feedback text, and full no-space drop/destroy parity for torch-extracted orbs.
- Finish enchant/orb family details: exact feedback/look output, requirement recomputation, live/configured orb-spawner verification, and full extracted-orb no-space/drop feedback parity.
- Implement actual cross-area transfer execution for `IDR_RECALL`, `IDR_CITY_RECALL`, and teleport outcomes; Rust currently returns typed handoff outcomes for callers.

## Pending Priority Order

1. Stabilize client-visible interaction loop: cached map diffs, action feedback text, chest cooldown/messages, look output.
2. Continue `src/system/player.c`, `src/system/player_driver.c`, `src/system/do.c`, `src/system/act.c` behavior beyond the primitive actions already ported.
3. Continue `src/system/map.c`, `src/system/los.c`, `src/system/path.c` parity for light, traps, notify callbacks, and exact LOS cache behavior.
4. Port `src/system/database/*.c` to PostgreSQL repositories and migrations, especially real login/load/save.
5. Port `src/system/game/*.c`, `src/system/skill.c`, `src/system/effect.c`, `src/system/death.c`, `src/system/respawn.c`.
6. Port `src/system/drvlib.c`, `src/system/libload.c`, all `src/module/**` drivers using the static Rust registry plan.
7. Port area modules under `src/area/**`.
8. Port chat, auction, anti-cheat, weather, event, command/admin systems.

### Iteration 193 Additional Progress

- `IDR_FLASK` shake item-state parity now mirrors C `flask_driver`: successful unfinished-flask mixes set the carried item to `Magical Potion` with size-specific magical sprites/descriptions, and failed mixes reset the item to an `Empty Potion` with size-specific empty bottle sprites/descriptions, cleared modifiers/drdata except size, value `10`, class requirements cleared, and the legacy stinking-liquid feedback. Focused core tests cover both final and ruined bottle states.

### Iteration 13 Additional Progress

- Area 36 `IDR_CALIGAR` now dispatches the C `caligar_training` branch (`drdata[0] == 1`): player-only uses of lesson IDs `1..3` return a typed training outcome, `PlayerRuntime` preserves the fixed-size legacy `DRD_CALIGAR_PPD` block, training observations update `watch_flag` bits with C's skeleton/vampire/zombie bit mapping, the outer PPD blob load/save replaces or appends the Caligar block, and the server emits the exact one-time training observation feedback while repeated observations stay silent. Focused core/server compile tests cover dispatch, fixed-layout PPD, outer blob integration, and runtime outcome handling. Remaining Caligar item gaps are weight/weight-door/gun/key/skelly-door/extinguish branches, quest NPC PPD state, exact dlog/logging side effects, and live area-data smoke coverage.

### Iteration 14 Additional Progress

- Area 36 `IDR_CALIGAR` now dispatches the C `caligar_weight` branch (`drdata[0] == 2 || 4`): character use pushes the weight one tile in facing direction only onto the legacy allowed floor sprite ranges, blocks occupied/movement-blocked/bad-floor targets with the C `It won't move.` feedback path, stores original coordinates and last-touch tick in `drdata[4..12]`, halts the actor action after movement, schedules existing weight timers on startup, and zero-character timer callbacks return untouched weights to their home tile after five minutes before rescheduling every five seconds. Focused core tests cover dispatch, successful movement, blocked movement, timer return, and startup timer registration. Remaining Caligar item gaps are weight-door/gun/key/skelly-door/extinguish branches, quest NPC PPD state, exact dlog/logging side effects, and live area-data smoke coverage.

### Iteration 18 Additional Progress

- Area 36 `IDR_CALIGAR` now dispatches the C `caligar_weight_door` branch (`drdata[0] == 3`): character use computes the opposite-side exact target from the actor/item relative position, preserves the southern lock check against the two legacy weight positions, halts on locked doors with `The door is locked.`, teleports only onto the exact target tile with busy-target feedback instead of nearby fallback placement, reverses cardinal facing after success, and preserves retry-style no-op behavior for automatic/diagonal/invalid calls. Focused core tests cover locked, successful, and busy-target outcomes. Remaining Caligar item gaps are gun/key/skelly-door/extinguish branches, quest NPC PPD state, exact log_area/dlog side effects, and live area-data smoke coverage.

### Iteration 21 Additional Progress

- Area 36 `IDR_CALIGAR` now dispatches the C `caligar_gun` branch (`drdata[0] == 5..=9`): subtypes map to the legacy east/south/west/north/all-direction shots, timer and character calls return a typed projectile outcome, world application creates retained `EF_EDEMONBALL` effects with C strength/base/start/target fields, and guns reschedule themselves after 12 ticks. Focused core/world tests cover subtype dispatch, four-way projectile placement, and timer rescheduling. Remaining Caligar item gaps are key assembly, skelly-door, extinguish, quest NPC PPD state, exact log_area/dlog side effects, and live area-data smoke coverage.

### Iteration 25 Additional Progress

- Area 36 `IDR_CALIGAR` now dispatches the C `caligar_key_assembly` branch (`drdata[0] == 10`): carried palace-key parts require a cursor item with legacy `IID_CALIGARPALACEKEYPART`, preserve the C sprite-combination matrix for partial key sprites `13420`/`13421`, destroy the cursor component on successful partial assembly, create the final `caligar_palace_chest_key` template on the cursor for the two final pairings, and emit the legacy `Nothing happens.` / `This does not seem to fit.` feedback for missing or mismatched cursor parts. Focused core/server compile tests cover dispatch, item-ID parity, partial/final combinations, and feedback/runtime application paths. Remaining Caligar item gaps are quest NPC PPD state, exact log_area/dlog side effects, and live area-data smoke coverage.

### Iteration 28 Additional Progress

- Area 36 `IDR_CALIGAR` now dispatches the C `caligar_skelly_door` branch (`drdata[0] == 12`): non-character timer calls preserve retry-style no-op behavior, character use exposes the legacy `drdata[1]` door index, runtime checks the fixed-layout `DRD_CALIGAR_PPD` `door_flag[4]` bytes for all three lock bits, locked doors emit the C-shaped `three seperate locks` feedback, unlocked doors teleport the player to the exact opposite side of the door, busy targets leave the player in place with retry feedback, and successful use reverses cardinal facing and stops current action. Focused core tests cover dispatch, PPD door-flag gates, exact teleport/facing, and busy-target handling. Remaining Caligar item gaps are quest NPC PPD state, exact log_area/dlog side effects, live area-data smoke coverage, and broader Caligar NPC/dialogue drivers.

### Iteration 32 Additional Progress

- Normal `IDR_POTION` successful use now emits the C `log_area(..., "%s drinks a potion.")` client-visible nearby text through the existing runtime area-feedback fan-out at legacy distance 10, while preserving the resource mutation/consumption outcome path. Focused server coverage pins the C-shaped message text and fallback name behavior. Remaining potion parity gaps are empty-bottle template creation through loaded data, exact audit/log side effects, and broader live-data smoke coverage.

### Iteration 39 Additional Progress

- Normal `IDR_POTION` empty-bottle replacement now follows the C `potion_driver` path when `drdata[0]` is set: the server instantiates `empty_potionN` from loaded templates, applies HP/mana/endurance restoration with legacy `POWERSCALE` caps, replaces the carried potion in the same cursor/inventory slot, frees the consumed live item, emits the existing nearby drink text, and keeps missing-template cases deferred. Focused server tests cover template replacement, resource capping, carried ownership, and normal-potion no-potion-area feedback. Remaining potion parity gaps are exact audit/log side effects and broader live-data smoke coverage.

### Iteration 41 Additional Progress

- Area 8 `IDR_FDEMONLIGHT = 44` now dispatches from the item-driver registry for the C `fdemon_light` timer path: the area-8 libload guard is enforced, nonzero-character calls no-op, runtime startup includes Fire Demon lights in existing light-timer registration, world context computes the C-style maximum power from the nearest `IDR_FDEMONLOADER` items for loader groups 1..3 using their little-endian `drdata[1..2]` power fields, and powered/off states switch between legacy sprites `14192`/`14189` with `V_LIGHT = 200` or `0` while rescheduling every second. Focused core tests cover powered/off transitions, area guard, and player no-op behavior. Remaining Fire Demon gaps are loader/cannon/gate/waypoint/farm/blood/lava item branches, module-global defense-station quest state, exact sound/log fan-out, and live area-data smoke coverage.

### Iteration 42 Additional Progress

- Area 8 `IDR_FDEMONLOADER = 45` now dispatches from the item-driver registry for the C `fdemon_loader` core path: the area-8 libload guard is enforced, valid red-crystal cursor insertion consumes the cursor item, sets animation `drdata[3] = 7`, stores next power in little-endian `drdata[4..5]`, marks inventory dirty, queues legacy sound `41`, and updates loader sprite/ground overlay sprites with the C formulas; timer callbacks decrement animation/power, promote queued power when animation ends, reschedule every second, update empty/powered sprites and overlays, and queue sound `43` when the loader becomes empty; Fire Demon characters can clear active loaders, and occupied/stuck/missing/wrong-crystal feedback reasons are exposed to the runtime with C-shaped text. Startup light-timer scheduling now includes existing Fire Demon loaders. Focused core/world tests cover insertion, timer countdown, blocking feedback reasons, cursor destruction, ground overlay mutation, sound fan-out, and timer rescheduling. Remaining Fire Demon gaps are loader Farmy/defense-station PPD quest progression, cannon/gate/waypoint/farm/blood/lava item branches, module-global defense-station quest state, exact dlog/notify fan-out, and live area-data smoke coverage.

### Iteration 45 Additional Progress

- Area 8 `IDR_FDEMONFARM = 49` now dispatches from the item-driver registry for the C `fdemon_farm` core path: the area-8 libload guard is enforced, timer callbacks grow `drdata[2]` by `drdata[0]` until `drdata[1]`, expose the size-based crystal foreground sprites `59020`/`59040`/`59041`/`59042`/`59043`, reschedule every two seconds, and startup timer registration includes existing Fire Demon farms. Player harvests require an empty cursor, report the legacy not-ready text with current/required growth, create the matching `fdemon_crystal1..5` template on the cursor, reset farm strength to zero, and clear the foreground crystal overlay. Focused core/world tests cover timer growth, overlay mutation, startup scheduling, harvest outcomes, cursor blocking, and not-ready feedback typing. Remaining Fire Demon gaps are cannon/gate/waypoint/blood/lava item branches, farm dlog/audit parity, module-global defense-station quest state, exact notify fan-out, and live area-data smoke coverage.

### Iteration 47 Additional Progress

- C `can_attack` player-vs-player guard ordering now returns immediately after area/PK/level/hate policy admits combat, before the later NPC/group/clan suppression checks. Focused core tests cover same-group players remaining attackable after the PvP branch admits them.

### Iteration 50 Additional Progress

- Area 17 `IDR_PICKDOOR = 79` now dispatches from the item-driver registry for the C `pick_door` path: the area-17 libload guard is enforced, player use requires the exact carried lockpick context while non-player use may open, already-open player use no-ops, zero-character timer callbacks only close open doors, opening stores and clears movement/sight/sound/door blockers, closing restores the stored blockers, the legacy 20-second auto-close timer and one-second blocked-doorway retry are represented, and runtime feedback emits the C locked/picked text. Focused core/world tests cover lockpick gating, timer dispatch, open/close mutation, and auto-close behavior. Remaining Area 17 gaps include `IDR_BURNDOWN`, `IDR_COLORTILE`, `IDR_SKELRAISE`, notification fan-out for lock picking, exact dlog/audit parity, and live area-data smoke coverage.

### Iteration 57 Additional Progress

- Area 17 `IDR_SKELRAISE = 87` now dispatches from the item-driver registry for the C `skelraise` chair path: the area-17 libload guard is enforced, active chairs emit the legacy touch feedback, missing/wrong cursor use emits the crumble feedback, `IID_AREA17_BLOODBOWL` cursor use maps `drdata[0]` kinds `0..5` to the legacy raised-skeleton character templates, consumes the blood bowl, spawns the raised skeleton at the chair tile, stores the raised character id in `drdata[4..7]`, marks active state in `drdata[2]`, increments the chair sprite, schedules the ten-second watchdog timer, and timer callbacks keep polling while the raised skeleton exists or reset the chair sprite/state after it disappears. Focused core and server compilation tests cover dispatch boundaries and runtime integration. Remaining Area 17 gaps include fixed-layout `DRD_TWOCITY_PPD` persistence beyond represented fields, notification fan-out for lock picking/skelraise quest state, exact dlog/audit parity, and live area-data smoke coverage.

### Iteration 58 Additional Progress

- Area 8 `IDR_FDEMONBLOOD = 50` now dispatches from the item-driver registry for the C `fdemon_blood` core player-use path: the area-8 libload guard is enforced, zero-character calls no-op, bare-hand/wrong-item/full-container gates return legacy-shaped feedback, cursor flasks are destroyed while the blood item switches to sprite `14348`, valid `IID_AREA8_BLOOD` containers increment their carried blood count/sprite/description, and the used blood map item is removed. Focused core/world tests cover dispatch gates, flask destruction, cursor/container mutation, map removal, and runtime feedback wiring. Remaining Fire Demon gaps are cannon/gate/waypoint/lava item branches, blood/lava Farmy PPD quest progression, exact dlog/notify/sound fan-out, and live area-data smoke coverage.

### Iteration 71 Additional Progress

- Area 8 `IDR_FDEMONLAVA = 51` now dispatches from the item-driver registry for the C `fdemon_lava` core path: the area-8 libload guard is enforced, bare-hand/wrong-item/empty-blood-container gates return legacy-shaped feedback, valid golem-blood containers decrement their blood count/sprite/description, lava activation opens movement blocking and shows the mist overlay, one-second timer decay transitions through mist/flame/cooling/final-blocked stages, map foreground and `MF_MOVEBLOCK`/`MF_FIRETHRU` state are updated, characters on the lava tile take the legacy staged `hurt` damage, and timer scheduling stops at the final stage. Focused core/world tests cover dispatch gates, activation mutation, timer stage output, tile mutation, scheduling, and damage. Remaining Fire Demon gaps are cannon/gate item branches, blood/lava Farmy PPD quest progression, exact dlog/notify/sound fan-out, and live area-data smoke coverage.

### Iteration 75 Additional Progress

- Area 6 `IDR_EDEMONTUBE = 43` now dispatches from the Rust item-driver registry with the C area-6 libload guard: character use performs the remembered same-area tube teleport, timer callbacks refresh sprite/light from the matching Earth Demon loader section power, discover the target loader with the C down-then-up passability rule, cache the target coordinates in `drdata[2..5]`, reschedule every second, and startup timer registration includes existing tubes. Focused core/world tests cover timer light state, target caching, character teleport outcome, startup scheduling, and runtime target discovery. Remaining Earth Demon tube gaps are the powered-section auto-teleport scan for nearby visible players, exact section-power module-global ordering when multiple loaders share a section, sound/log fan-out, and live area-6 data smoke coverage.

### Iteration 79 Additional Progress

- Area 14 `IDR_JUNKPILE = 71` now dispatches from the Rust item-driver registry with the existing C area-14 libload guard: zero-character/timer calls no-op, player use requires an empty cursor, valid searches roll the C junk table shape (`steelbar` on rolls 1/2/4/5/7/9, money on roll 3, otherwise nothing), create the reward on the cursor, destroy the used pile even on empty searches, and emit the legacy cursor/found feedback. Focused core/server tests cover dispatch gates, steelbar reward creation, money reward creation, cursor placement, and map item destruction. Remaining Area 14 random-module gaps are random shrine PPD/rewards, gastrap foreground animation/damage, exact global RNG parity, and audit/log side effects.

### Iteration 80 Additional Progress

- Area 14 `IDR_GASTRAP = 72` now dispatches from the Rust item-driver registry with the C area-14 libload guard: character-triggered inactive traps schedule the same one-tick first callback plus three-tick animation callback shape, active traps ignore repeated character use, zero-character timer callbacks advance only while active, `drdata[1]` cycles through the nine-frame gas animation and resets to zero, nearby gas foreground sprites in the C center/east/west/south/north search order are updated across the four legacy sprite ranges, dirty sectors are marked, and player-triggered gas applies legacy `hurt(cn, POWERSCALE * drdata[0], 0, 1, 50, 33)` damage. Focused core/world tests cover dispatch gating, animation reset, timer scheduling, foreground mutation, and damage. Remaining Area 14 random-module gaps are random shrine PPD/rewards, exact gas-trap global timer-cadence smoke parity, exact global RNG parity, and audit/log side effects.

### Iteration 84 Additional Progress

- Area 8 `IDR_FDEMONGATE = 47` now dispatches from the Rust item-driver registry with the C area-8 libload guard: zero-character timer callbacks read the legacy level/rate bytes, find the first stale slot in the three stored character-id/serial pairs, return a typed `fdemon<level>s` spawn request at the gate tile, reschedule by `drdata[1] * TICKS`, and startup timer registration includes existing Fire Demon gates. The server runtime instantiates the requested template, places the spawned character at the gate, initializes its rest tile and direction, stores the spawned character id back into the matching gate slot, and keeps one-spawn-per-tick behavior. Focused core tests cover timer dispatch, area guard, timer-only no-op behavior, and rescheduling; workspace tests pass. Remaining Fire Demon gaps are cannon item behavior, blood/lava Farmy PPD quest progression, exact serial tracking beyond live character-id staleness, exact `item_drop_char` around-gate placement, dlog/notify/sound fan-out, and live area-data smoke coverage.

### Iteration 97 Additional Progress

- Area 8 `IDR_FDEMONCANNON = 46` now dispatches from the Rust item-driver registry with the C area-8 libload guard: player use reports the legacy lifeless feedback when its associated loaders have no power and otherwise no-ops, zero-character timer callbacks resolve the nearest three Fire Demon loaders, use the maximum loader power, scan for a non-player/non-playerlike target in the cannon direction, create a retained `EF_EDEMONBALL` with C base sprite `2` and `power / 50 + 1` strength, drain loader power in loader order, toggle the cannon active sprite bit, and reschedule every second. Focused core tests cover dispatch, lifeless use, projectile creation, loader drain, active-bit set/clear, and timer rescheduling. Remaining Fire Demon gaps are blood/lava Farmy PPD quest progression, exact serial tracking beyond live character-id staleness, exact `item_drop_char` around-gate placement, dlog/notify/sound fan-out, cannon target-sector ordering parity, and live area-data smoke coverage.

### Iteration 98 Additional Progress

- Area 8 Fire Demon blood/lava quest progression now carries the C-compatible `DRD_FARMY_PPD` block for the represented `farmy_ppd.boss_stage` path: Rust decodes/encodes the fixed 340-byte structure under `MAKE_DRD(DEV_ID_DB, 77 | PERSISTENT_PLAYER_DATA)`, preserves unknown soldier/emote fields, advances successful golem-blood fills from stages `19..=20` to `21` with the legacy commander-report feedback, and advances successful lava activation from stages `22..=23` to `24` with the matching feedback. Focused core tests cover layout, outer PPD round-trip, and stage transitions; workspace tests and `cargo build -p ugaris-server` pass. Remaining Fire Demon gaps are broader Farmy soldier/NPC quest behavior, exact serial tracking beyond live character-id staleness, exact `item_drop_char` around-gate placement, dlog/notify/sound fan-out, cannon target-sector ordering parity, and live area-data smoke coverage.

### Iteration 103 Additional Progress

- Area 8 `IDR_FDEMONGATE` slot bookkeeping now stores the spawned character's actual legacy serial in `drdata[6 + slot * 4]` and treats a live character ID with a changed serial as stale, matching the C `ch[co].serial` guard. The server spawn path passes the instantiated character serial into the world slot update. Focused core coverage pins matching-serial occupancy, changed-serial respawn admission, and byte layout. Remaining Fire Demon gaps are broader Farmy soldier/NPC quest behavior, exact `item_drop_char` around-gate placement, dlog/notify/sound fan-out, cannon target-sector ordering parity, and live area-data smoke coverage.

### Iteration 106 Additional Progress

- Area 10/11 shared `src/common/ice_shared.c` drivers now dispatch from the Rust item-driver registry for `IDR_ITEMSPAWN`, `IDR_WARMFIRE`, `IDR_BACKTOFIRE`, and `IDR_MELTINGKEY`: item spawns map legacy `drdata[0]` kinds to melting keys, ice equipment, and palace bomb/cap templates; occupied cursor and bad-kind bug feedback outcomes are typed; warm fires create an `ice_scroll` with one-byte return coordinates in `drdata[0..1]`, remove carried `IDR_CURSE` spells/effects, and preserve hand-warming/cure feedback; back-to-fire scrolls teleport to stored same-area coordinates and consume themselves; melting keys advance age, update sprites with the C formula, reschedule ten-second timer callbacks, and destroy on expiry. Focused core/server compile tests cover dispatch, mapping, cursor gates, warm-fire curse detection, back-to-fire coordinates, and melting-key timer mutation. Remaining ice/palace shared gaps are exact `can_carry` no-feedback behavior for item spawns, initial scheduling for newly created melting keys if not supplied by templates/startup, exact dlog/audit side effects, and live area-10/11 data smoke coverage.

### Iteration 115 Additional Progress

- Area 10/11 shared `src/common/ice_shared.c` item-spawn melting keys now start their C ten-second zero-character timer immediately when granted to the cursor, instead of depending on template/startup scheduling. Focused server coverage verifies spawned `IDR_MELTINGKEY` items are queued, tick once after ten seconds, age their `drdata[1]`, and reschedule. Remaining ice/palace shared gaps are exact `can_carry` no-feedback behavior for item spawns, exact dlog/audit side effects, and live area-10/11 data smoke coverage.

### Iteration 118 Additional Progress

- Area 10/11 shared `src/common/ice_shared.c` `IDR_ITEMSPAWN` now applies the represented C `can_carry(cn, in2, 0)` gates after template creation and before cursor assignment: duplicate `IDR_ONECARRY` rewards such as palace bombs/caps are rejected with the legacy one-carry feedback, `IF_BONDTAKE` owner mismatches are silently rejected, failed grants no longer masquerade as template-creation bug feedback, and successful melting-key grants still schedule the ten-second timer. Focused server tests cover melting-key scheduling, duplicate one-carry rejection, and bonded-item silent rejection. Remaining ice/palace shared gaps are exact dlog/audit side effects and live area-10/11 data smoke coverage.

### Iteration 120 Additional Progress

- `src/system/command.c` weather admin commands are now represented in the Rust runtime around the existing `/weather` display path: `ServerRuntime` retains mutable weather state instead of always using clear defaults, god-only `/setweather <type> <intensity>` validates the C weather type/intensity ranges, starts the one-minute transition, updates current weather/intensity/effects, and emits the legacy `Weather changing to ...` feedback; `/clearweather` transitions to clear weather, resets affected areas, and emits `Weather clearing globally.`; `/setareaweather <area> <type>` validates area/type and represented area-allowed weather gates, adds/removes affected areas with the C set/clear shape, and emits the legacy area feedback. Focused server tests cover mutation, validation, non-god denial, disallowed underground weather, and existing `/weather` display behavior. Remaining weather gaps are the actual global/area weather update model, persistence/synchronization, client `SV_MOD2` weather packets, live movement/visibility/combat weather effects, and exact dlog/audit side effects.

### Iteration 126 Additional Progress

- `src/system/drvlib.c` simple-baddy fight-driver warcry task admission now preserves the C strict endurance gate: NPC fight-task selection requires endurance greater than `V_WARCRY * POWERSCALE / 3`, while the lower-level `do_warcry` primitive still mirrors C's direct spell cast allowance at exact cost. Focused core coverage verifies exact-cost fight tasks skip warcry and above-cost tasks admit it. Remaining fight-driver gaps include exact `fight_driver_attack_enemy` fallback ordering for failed tasks, broader area-driver reuse, exact global RNG/sound cadence, and exact NPC scheduling.

### Iteration 129 Additional Progress

- `src/system/command.c` `/autoturn` now has a Rust runtime command slice: the legacy `cmdcmp(..., minlen=5)` prefix shape is represented, `PlayerRuntime` persists the C `lostcon_ppd.autoturn` int at slot 16 in the fixed `DRD_LOSTCON_PPD` layout, the command toggles the flag and reprints the lag-control status like C `show_lostconppd`, and `/status` reflects the stored automatic-turning state. Focused core/server tests cover byte layout, outer PPD append behavior, command toggling, and status output. Remaining lost-connection work is actual autoturn driver behavior and the other lag-control automation toggles.

### Iteration 133 Additional Progress

- Area 16 `IDR_FORESTCHEST = 78` now dispatches from the forest item-driver path for C `forest.c` `chest`: it is guarded to area 16 like the legacy module load, preserves the occupied-cursor block, requires the exact robber/skelly key item IDs from carried items, maps chest `drdata[0]` to the C money amounts `9733` and `17587`, creates a cursor money item on success, and tracks the one-time reward state in the C-compatible `DRD_AREA3_PPD.imp_flags` slot. Runtime feedback now emits the legacy cursor/key/empty/success text, and focused core/player/server tests cover dispatch gates, PPD layout, money creation, and repeat-empty behavior. Remaining forest gaps are Forest Imp/William/Hermit character drivers, monster death quest side effects, exact dlog/audit behavior, and live area-16 data smoke coverage.

### Iteration 136 Additional Progress

- Player text commands now cover the C god-only `/exp` admin helper from `src/system/command.c`: the legacy three-character minimum command shape is represented, non-god callers do not claim the command, empty or numeric arguments target the caller, named arguments perform local online character lookup, zero amounts report the target's current experience, nonzero amounts mutate target experience with legacy-shaped feedback, and updated targets are marked for client refresh. Focused server tests cover self-report, self-grant, named target grant/report, not-found feedback, prefix rejection, and god-only gating. Remaining admin command gaps include `/milexp`, exact `give_exp` level-up/NOEXP/modifier side effects, audit/log side effects, and broader admin command coverage.

### Iteration 142 Additional Progress

- Player text commands now cover the C god-only `/milexp` admin helper from `src/system/command.c`: full-command recognition, non-god callers do not claim the command, empty or numeric arguments target the caller, named arguments perform local online character lookup, zero amounts preserve the C report typo by showing normal experience, nonzero amounts grant one normal experience point through the represented character state, mutate signed military points with the C default hardcore 10% bonus, update represented military normal-exp accounting, emit legacy-shaped feedback, and mark updated targets for client refresh. Focused server tests cover self-report, self-grant, named hardcore target grant/report, not-found feedback, full-command rejection, and god-only gating. Remaining military/admin gaps include fixed-layout `DRD_MILITARY_PPD` encode/decode, rank promotion/chat side effects, configurable hardcore bonus command wiring, exact `give_exp` level-up/NOEXP/modifier side effects, audit/log side effects, and broader admin command coverage.

### Iteration 144 Additional Progress

- Area 20 `IDR_LQ_TICKER = 103` now dispatches from the Rust item-driver registry for the C `lq_ticker` boundary: it is guarded to area 20 like the legacy module load, nonzero-character calls return a handled no-op, zero-character timer calls return a typed one-second reschedule request, and the server runtime applies that request through the existing item-driver timer queue. Focused core tests cover timer rescheduling, character-call no-op behavior, and area guard parity. Remaining live-quest gaps are initial door/NPC table discovery, NPC respawn spawning, `IDR_LQ_ENTRANCE`, LQ player/NPC PPD state, character drivers, and full quest progression side effects.

### Iteration 145 Additional Progress

- Area 22 Lab 1 `IDR_LABTORCH = 199` now dispatches from the Rust item-driver registry for C `labtorch`: it is guarded to area 22 like the legacy module load, zero-character calls store the current light modifier in `drdata[1]`, player use cannot light an unlit torch, NPC/non-player use lights unlit torches by incrementing the sprite/restoring `V_LIGHT`, and lit-torch use extinguishes by decrementing the sprite/clearing light. Focused core tests cover area guard, timer storage, player no-op, NPC lighting, and extinguishing. Remaining Lab 1 torch gap is the exact `notify_area(..., NT_NPC, NTID_LABGNOMETORCH, ...)` fan-out on extinguish.

### Iteration 146 Additional Progress

- `/iwilldie` confirmation now uses C `atoi`-style numeric-prefix parsing instead of strict Rust integer parsing, so inputs such as `/iwilldie 77abc` confirm character ID `77` like the legacy command path. Focused server coverage pins the accepted numeric-prefix behavior. Remaining PK command gaps are offline `lookup_name`/`lookup_ID` repository integration for names, reset-name/audit side effects, broader command/help parity, repository-backed clan/alliance policy, and exact realtime source persistence semantics.

### Iteration 147 Additional Progress

- Area 22 Lab 1 `IDR_LABTORCH` extinguish now mirrors the C `notify_area(it[in].x, it[in].y, NT_NPC, NTID_LABGNOMETORCH, in, cn)` side effect: when a character turns off a lit lab torch, the world queues the legacy NPC notification to nearby driver-message recipients with the torch item ID and actor ID while preserving timer storage, player-lighting no-op behavior, NPC lighting, and light mutation. Focused core coverage verifies the notification payload and range filtering. Remaining Lab 1 torch gaps are broader Lab 1 gnome/master character-driver behavior, exact dlog/audit side effects, and live area-22 data smoke coverage.

### Iteration 148 Additional Progress

- `src/system/drvlib.c` `fight_driver_attack_enemy` now preserves the C `!nomove` gate for simple-baddy movement-only weighted tasks: distance-3 spacing, distance-7 spacing, and attack-back positioning are no longer enqueued when a caller requests no movement, while the existing C exception for adjacent/distance-two direct attacks remains intact. Focused core coverage pins the nomove suppression for attack-back/spacing task kinds. Remaining fight-driver gaps include exact shared global RNG use, broader area-driver reuse, and exact NPC scheduling cadence.

### Iteration 151 Additional Progress

- C god-only `/staffcode` now persists through the Rust character snapshot path instead of living only in transient server runtime state: `Character` carries a serde-defaulted `staff_code`, the command mutates both the persisted character field and compatibility runtime map, and `/tell`, channel chat, and `/whostaff` display prefer the persisted field with runtime fallback. Focused core/server tests cover legacy snapshot defaulting, command mutation, and persisted-code tell/chat formatting. Remaining staff-code/admin gaps are exact audit/server-chat side effects and cross-area/offline target lookup.

### Iteration 155 Additional Progress

- C god-only `/resetgift <name> <area>` now has a Rust runtime command slice: full-command recognition, god-only gating, legacy alphabetic online-target parsing, `atoi`-prefix area parsing, `0..=63` area validation, fixed-layout `DRD_MISC_PPD.treedone` bit clearing through `PlayerRuntime`, and C-shaped missing-target/player-data/invalid-area/success feedback are covered by focused server tests. Remaining resetgift/admin gaps are exact audit/log side effects and cross-area/offline target lookup.

### Iteration 161 Additional Progress

- C god-only `/reset <name>` now has a Rust runtime command slice: full-command recognition, god-only gating, legacy alphabetic online-target parsing, local online target lookup, C value-clamping semantics for bare values `0..=V_IMMUNITY` (primary stats capped at 10, other represented skills capped at 1), `V_RAGE`/`V_DURATION` capping, `exp_used` clearing, update flagging, silent success, and missing-target feedback are covered by focused server tests. Remaining reset/admin gaps are exact `update_char` recomputation side effects beyond represented value/update state, audit/xlog side effects, and cross-area/offline target lookup.

### Iteration 164 Additional Progress

- Area 20 `IDR_LQ_ENTRANCE = 104` now dispatches from the Rust item-driver registry for C `lq_entrance`: it is guarded to area 20 like the legacy module load, zero-character calls no-op, live-quest open state, level range, missing entrance, and recent LQ-death penalty gates return typed outcomes with legacy-shaped server feedback, and successful entry returns the existing quiet same-area teleport outcome to the configured quest entrance. Focused core tests cover area guard, no-op, blocked gates, penalty timing, and teleport shape; server check coverage verifies runtime outcome handling. Remaining live-quest gaps are wiring real `lq_data`/`misc_ppd.last_lq_death` state into the context, initial door/NPC table discovery, NPC respawn spawning, LQ player/NPC PPD state, character drivers, and full quest progression side effects.

### Iteration 167 Additional Progress

- Area 20 `IDR_LQ_TICKER = 103` now also applies the C `lq_ticker` one-time door discovery slice: the world scans live normal `IDR_DOOR` items with `drdata[10]`, records LQ door slots in stable item-id order starting at slot 1, stores the door nickname from the item name, writes the legacy `MAKE_ITEMID(DEV_ID_LQ, keyID)` bytes into `drdata[1..4]` with initial key `0`, and does not rediscover or overwrite doors on later ticker callbacks. Focused core coverage verifies discovery filtering, byte layout, and one-time behavior. Remaining live-quest gaps are real LQ door admin command wiring, NPC table discovery/respawn spawning, LQ player/NPC PPD state, character drivers, and full quest progression side effects.

### Iteration 168 Additional Progress

- Area 20 `IDR_LQ_TICKER = 103` now has a Rust state boundary for the C `lq_respawn[]` scan: `World` stores fixed-slot LQ NPC definitions with legacy slot bounds `1..512`, records scheduled respawn ticks, timer callbacks queue due `LqNpcSpawnRequest` values in slot order, clear consumed respawn entries like successful C `spawn_npc`, and leave future respawns pending. Focused core tests cover due respawn queuing, schedule clearing, future retention, and slot-bound rejection. Remaining live-quest gaps are applying queued spawn requests through template-backed runtime character creation, LQ NPC command/admin table wiring, real LQ door admin command wiring, LQ player/NPC PPD state, character drivers, and full quest progression side effects.

### Iteration 170 Additional Progress

- Area 20 LQ queued NPC respawns now apply in the server runtime: due `LqNpcSpawnRequest` values instantiate `lq_<basename>` character templates, force the legacy `CDR_LQNPC = 74` driver boundary, apply configured name/description/mode/rest/level/resource initialization, place the NPC at the configured LQ slot coordinates, insert template inventory items, and record the live character id/serial back into `World::lq_npcs`. Focused core/server tests cover character-driver ID dispatch, slot identity recording, and runtime template-backed spawn application. Remaining live-quest gaps are C `lq_raise`/`lq_statboost`/`lq_equipment` parity, LQ NPC dialogue/combat driver state, LQ NPC command/admin table wiring, real LQ door admin command wiring, LQ player/NPC PPD state, and full quest progression side effects.

### Iteration 176 Additional Progress

- Area 20 LQ NPC spawning now ports the C `lq_statboost` item slice: runtime `lq_<basename>` spawns create up to three carried `lqx_spell` items in spell slots 12..29, choose the weapon-skill modifier with the legacy dagger/staff/sword/two-hand/hand priority, add warrior attack/parry/tactics/warcry modifiers, add mage bless/light/fireball/magic-shield/freeze modifiers only when represented by the NPC base template, always add the misc immunity/wisdom/intuition/agility/strength modifier item, and apply those deltas before final HP/mana/endurance resource initialization. Focused server coverage pins slot placement, item ownership, modifier layout, and effective stat changes. Remaining live-quest gaps are `lq_equipment` quality gear creation, LQ NPC dialogue/combat driver state, LQ NPC command/admin table wiring, real LQ door admin command wiring, LQ player/NPC PPD state, and full quest progression side effects.

### Iteration 180 Additional Progress

- Area 20 LQ NPC spawning now ports the C `lq_raise` skill-cost rescaling slice before statboost/equipment: spawned `lq_<basename>` templates distribute `level2exp(level + 2) - 1` across represented non-derived bare values by their original template weights, skip the same legacy values (`V_PROFESSION`, `V_COLD`, `V_DEMON`, `V_SPEED`, `V_LIGHT`, `V_WEAPON`, `V_ARMOR`), convert weighted costs back to skills with the C `cost2skill`/`raise_cost` loop including Seyan cost behavior, recalculate exp/exp_used/level through the represented `calc_exp`/`exp2level` equivalents, and initialize HP/endurance/mana from the raised effective values after existing LQ statboost items. Focused server coverage now pins raised bare values, effective statboost deltas, resources, exp, carried `lqx_spell` modifiers, and slot identity recording. Remaining live-quest gaps are `lq_equipment` quality gear creation, LQ NPC dialogue/combat driver state, LQ NPC command/admin table wiring, real LQ door admin command wiring, LQ player/NPC PPD state, and full quest progression side effects.

### Iteration 185 Additional Progress

- Area 20 LQ NPC spawning now ports the C `lq_equipment` quality gear slice after `lq_raise`/`lq_statboost`: spawned `lq_<basename>` NPCs choose the right-hand quality weapon template from the raised base Dagger/Staff/Sword/Two-Hand values using the legacy later-weapon overwrite order, compute quality tiers as `min(10, value / 10 + 1)`, create armor-skill based `helmetNq1`, `armorNq1`, `leggingsNq1`, and `sleevesNq1` templates into the C worn slots, and preserve carried ownership for inserted items. Focused server tests cover weapon tier selection, Two-Hand overriding Sword like C's sequential `if` blocks, armor slot placement, carried ownership, and full LQ spawn integration. Remaining live-quest gaps are LQ NPC dialogue/combat driver state, LQ NPC command/admin table wiring, real LQ door admin command wiring, LQ player/NPC PPD state, full quest progression side effects, and live LQ data smoke coverage.

### Iteration 191 Additional Progress

- Area 34 `IDR_TEUFELRATNEST = 140` now dispatches through the Teufelheim item-driver boundary for the C `teufelratnest` core state machine: zero-character timer calls require the timer context, honor the five-tick destroyed-nest cooldown before restoring sprite `15281`, decrement the little-endian wave counter, update the legacy visible nest description, classify rat spawn level/template tables for nest kinds `0/1/2`, and reschedule after 20 seconds. Player use now preserves the live-guard block outcome and successful destroy mutation (`wave = 0`, `drdata[2] = 5`, sprite `0`) with focused core tests. Remaining Teufel rat-nest gaps are runtime template-backed rat spawning/slot serial bookkeeping, random spawned-rat stat suffixes, exact `item_drop_char` placement behavior, rat death PPD score updates, dlog/audit side effects, and live area-34 data smoke coverage.

### Iteration 200 Additional Progress

- Player text commands now cover the C god-only `/create_orb` admin helper from `src/system/command.c`: full-command recognition, god-only gating, no-argument random orb creation using the existing legacy orb value table, skill-name creation through `lookup_skill` parity, `<value> <skill>` valued-orb creation with C `atoi`-prefix parsing, `empty_orb` template instantiation, legacy orb names/drdata layout, and inventory-first smart placement are covered by focused server tests. Remaining orb/admin gaps are exact global RNG stream parity, exact `give_char_item_smart` drop/no-space feedback and dlog/audit side effects.
- Area 34 `IDR_TEUFELRATNEST` timer outcomes now apply the first runtime rat-spawn slice: the server instantiates the classified rat template, initializes HP/endurance/mana/lifeshield from base values, sets right-down facing plus `CF_NONOTIFY`, stores the spawned character ID and serial in the legacy five-slot `drdata[10..30]` layout, removes over-level live guards before replacement like C, increments the wave by 10 only for empty/stale slots below the 50000 cap, and schedules the next 20-second nest timer. Focused server/core tests cover slot serial storage, wave growth, and the existing dispatch classification. Remaining Teufel rat-nest gaps are random spawned-rat stat suffixes, exact `item_drop_char` placement behavior/home coordinates after drop, rat death PPD score updates, dlog/audit side effects, and live area-34 data smoke coverage.

### Iteration 6 Additional Progress

- Area 34 `IDR_TEUFELRATNEST` spawned rats now apply the C random stat-suffix slice after successful template instantiation: a `RANDOM(20)` roll of `0..4` adds `RANDOM(10)+7` to Attack, Parry, Freeze, Flash, or Immunity respectively, appends the legacy ` *A`/` *P`/` *R`/` *F`/` *I` name suffix, appends the matching description sentence, and marks the character for update. Focused server tests cover all five suffixes and the default no-suffix branch. Remaining Teufel rat-nest gaps are exact `item_drop_char` placement behavior/home coordinates after drop, rat death PPD score updates, exact global RNG stream parity, dlog/audit side effects, and live area-34 data smoke coverage.

### Iteration 8 Additional Progress

- Area 34 `IDR_TEUFELRATNEST` spawned rats now use a C-shaped `item_drop_char` placement helper instead of generic 3x3 character dropping: placement tries the nest tile, front-side tiles, behind-side tiles when `IF_FRONTWALL` is not set, then the legacy two-tile front/behind fallback order including the duplicated C attempts. Rat `rest_x/rest_y` now store the actual placed tile after successful drop, matching the C `tmpx/tmpy = ch[co].x/y` side effect. Focused core/server tests cover normal and front-wall drop order plus blocked-center rat spawn home coordinates. Remaining Teufel rat-nest gaps are rat death PPD score updates, exact global RNG stream parity, dlog/audit side effects, and live area-34 data smoke coverage.

### Iteration 21 Additional Progress

- Area 34 `CDR_TEUFELRAT` death scoring now mirrors the C `teufelrat_dead` slice: the Teufel character-driver IDs are represented at the Rust registry edge, player killers receive fixed-layout `DRD_TEUFELRAT_PPD` `kills`/`score` updates, score uses `level * level / 100` with C lag/lost-connection reduction to `1`, the outer PPD blob load/save replaces or appends the rat block, and the server queues the legacy `#90 ... Rat Kills` / `#80 ... Rat Points` feedback after lethal hurt events. Focused core/server tests cover PPD layout, block framing, score calculation, and runtime hurt-event wiring. Remaining Teufel rat-nest gaps are exact global RNG stream parity, dlog/audit side effects, reward dialogue payout/special rewards, and live area-34 data smoke coverage.

### Iteration 27 Additional Progress

- C `give_exp` runtime modifiers are now applied to the god `/exp` command path: grants pass through a reusable server helper that applies hardcore EXP bonus, global `/setexpmod`, `/noexp` and area-21 suppression, and `/nolevel` next-level capping before updating the represented character EXP state. Focused server tests cover modifier stacking, hardcore bonus, no-exp blocking, and no-level capping. Remaining EXP-modifier gaps are wiring the helper into every non-admin EXP grant path, exact level-up/check-level side effects, macro-daemon tracking, audit/xlog side effects, and signed negative EXP parity beyond the current unsigned Rust character EXP storage.

### Iteration 35 Additional Progress

- Area 22 `CDR_LAB2UNDEAD = 198` now has a typed Rust driver-state foundation matching the C `lab2_undead_driver_data` create-message slice: `DRD_LAB2_UNDEAD` is represented, the character-driver registry handles Lab 2 undead tick/death/respawn calls with C-compatible return code `1`, `CharacterDriverState::Lab2Undead` carries aggressive/helper/undead/patrol/grave/regenerate/opened-by fields, legacy `nextnv`-style args parse `aggressive`, `helper`, `patrol`, and `undead`, `NT_CREATE` messages are consumed, and the C graveyard/crypt patrol coordinate tables set `patx`/`paty`/`patstep` while disabling helper mode. Focused core tests cover ID/DRD constants, dispatch return-code parity, parser behavior, create-message consumption, and both patrol tables. Remaining Lab 2 undead gaps include runtime invocation from spawned grave enemies, regenerate item creation wiring, holy-water give handling, standard-message/fight-driver reuse, patrol movement, death reward/PPD grave-bit updates, Arathas wake-all behavior, exact dlog/audit integration, and live area-data smoke coverage.

### Iteration 37 Additional Progress

- Area 22 `CDR_LAB2UNDEAD` now has the first runtime message-processing slice for C `lab2_undead_driver` `NT_GIVE`: completed `AC_GIVE` transfers queue `NT_GIVE` to the receiver, the server tick loop dispatches Lab 2 undead messages separately from simple-baddy messages, holy-water bowls (`IDR_LAB2_WATER`, `drdata[0] == 5`) are destroyed on receipt, normal gifts are destroyed like C, valid holy water queues the legacy giver feedback, no-magic/non-undead cases say `Mwahahahaha...`, true undead say `Arrgh!`, clear `CF_NODEATH`, create mist, push the regenerate spell start tick 20 seconds out, and take C-shaped `hurt(cn, 20 * POWERSCALE, co, 1, 0, 0)` damage through the reusable hurt primitive. Focused core tests cover give notification, damaging true undead, regenerate delay bytes, mist creation, and no-magic laugh-off. Remaining Lab 2 undead gaps include regenerate item creation wiring, standard-message/fight-driver reuse, patrol movement/door closing, cathedral self-destruction, death reward/PPD grave-bit updates, Arathas wake-all behavior, exact dlog/audit integration, and live area-data smoke coverage.

### Iteration 42 Additional Progress

- `src/module/simple_baddy.c` `NT_NPC` helper alert enemy tracking now preserves the C `fight_driver_add_enemy(cn, target, 1, 0)` visible-flag semantics: reported enemies keep their current last-known coordinates but are recorded as hidden even when currently visible to the helper. Focused world coverage prevents the Rust runtime from accidentally promoting help-id alerts into visible targets before the normal fight-driver refresh path. Remaining simple-baddy gaps include broader `standard_message_driver` reuse, full movement/pathing, respawn/body-drop behavior, exact global RNG parity, and exact NPC tick scheduling.

### Ralph Loop Iteration 50 Additional Progress

- Area 22 `CDR_LAB2UNDEAD` patrol movement now has a Rust runtime slice for the C `lab2_undead_driver` patrol branch: idle Lab 2 undead with patrol state move toward the current C waypoint using the shared movement/path fallback, advance `pat` when within the legacy three-tile threshold, preserve crypt-patrol two-second waits at waypoints `0`, `3`, and `4`, emit the C `A gust of wind?` / `Strange.` area text at the matching waypoints, and the server tick loop invokes the patrol pass after Lab 2 undead message processing. Focused core tests cover waypoint advancement/wait text and walking toward the active waypoint. Remaining Lab 2 undead gaps include regenerate item creation wiring, standard-message/fight-driver reuse, crypt door-closing/corridor enemy removal, cathedral self-destruction, death reward/PPD grave-bit updates, Arathas wake-all behavior, exact dlog/audit integration, and live area-data smoke coverage.

### Ralph Loop Iteration 54 Additional Progress

- Area 22 `CDR_LAB2UNDEAD` template creation now ports the C `NT_CREATE` regenerate-spell installation slice: undead Lab 2 character templates parse their driver args, create a carried `lab2_regenerate_spell` item when the template is available, place it into the last free legacy spell slot, write the target character ID into the C-shaped `lab2_regenerate_data.cn` bytes, set `CF_NODEATH`, and store the item ID in `Lab2UndeadDriverData.regenerate_item_id` for the existing holy-water delay path. Focused zone-loader coverage verifies slot placement, carried ownership, driver ID, target bytes, `NODEATH`, and patrol parsing. Remaining Lab 2 undead gaps include standard-message/fight-driver reuse, crypt door-closing/corridor enemy removal, cathedral self-destruction, death reward/PPD grave-bit updates, Arathas wake-all behavior, exact dlog/audit integration, and live area-data smoke coverage.

### Ralph Loop Iteration 57 Additional Progress

- Area 22 `CDR_LAB2UNDEAD` cathedral self-destruction now ports the C `lab2_undead_driver` ground-sprite branch: idle Lab 2 undead standing on legacy cathedral sprites `20456` or `17062` say `Arrgh!`, create a mist effect at their tile, clear `CF_NODEATH`, mark themselves dead/update with zero HP, and stop before patrol movement. The server tick loop invokes this pass before Lab 2 undead patrol processing. Focused core tests cover both cathedral sprites, the non-cathedral no-op, mist creation, area text, and death state. Remaining Lab 2 undead gaps include standard-message/fight-driver reuse, crypt door-closing/corridor enemy removal, death reward/PPD grave-bit updates, Arathas wake-all behavior, exact dlog/audit integration, and live area-data smoke coverage.

### Ralph Loop Iteration 59 Additional Progress

- Area 22 `CDR_LAB2UNDEAD` crypt patrol door-closing now mirrors the C `lab2_undead_driver` slice: idle patrol-2 undead standing left of the crypt door at `(168,156)` and within the legacy `< 3` coordinate window close an open normal `IDR_DOOR` through the shared door toggler before patrol movement. The server tick loop invokes this pass after cathedral self-destruction and before Lab 2 undead patrol movement. Focused core tests cover successful close and wrong-side rejection. Remaining Lab 2 undead gaps include standard-message/fight-driver reuse, second-corridor enemy removal, death reward/PPD grave-bit updates, Arathas wake-all behavior, exact dlog/audit integration, and live area-data smoke coverage.

### Ralph Loop Iteration 62 Additional Progress

- Area 22 `CDR_LAB2UNDEAD` crypt patrol second-corridor enemy removal now mirrors the C `lab2_undead_driver` `NT_CHAR` slice: Lab 2 undead driver state carries a serde-defaulted enemy table, patrol-2 undead consume `NT_CHAR` messages, visible non-self targets inside the C second-corridor rectangle `(169..=188,154..=158)` are removed from that table, and targets outside the corridor remain tracked. Focused core tests cover removal and rejection. Remaining Lab 2 undead gaps include standard-message/fight-driver reuse, death reward/PPD grave-bit updates, Arathas wake-all behavior, exact dlog/audit integration, and live area-data smoke coverage.

### Ralph Loop Iteration 78 Additional Progress

- Area 22 `CDR_LAB2UNDEAD` grave-spawn death bookkeeping now preserves the C opener/serial guard: template-backed grave spawns install/retain `Lab2UndeadDriverData`, store the source grave item, opener character id, and opener serial, and lethal hurt events for the matching player opener mark the matching Lab 2 grave bit in `PlayerRuntime`. Focused server tests cover spawn metadata and serial-gated grave-bit marking. Remaining Lab 2 undead death gaps include the C crypt/yard completion counters, gold reward thresholds, Arathas wake-all behavior, exact dlog/audit integration, and live area-data smoke coverage.

### Module Re-Architecture And Kill Loop Session

- The three monolithic Rust files were split into module directories with no behavior change: `crates/ugaris-core/src/item_driver.rs` (22.4K lines) became `item_driver/` with dispatch/types/ids plus per-C-source domain files and per-domain test files; `crates/ugaris-core/src/world.rs` (31.5K lines) became `world/` with the `World` struct in `mod.rs` and impl blocks split by legacy system; `crates/ugaris-server/src/main.rs` (30.5K lines) became ~30 concern modules plus `src/tests/`. Private items moved across module boundaries were re-scoped `pub(crate)`, glob re-exports keep the public crate surface identical, and the full workspace test count was preserved through the split before new work started.
- `src/system/death.c` is now substantially ported: lethal `hurt` runs the C `kill_char` follow-up (respawn timer registration, kill-score experience with the exact taper table and hardcore/LAG handling, timed `AC_DIE`), `AC_DIE` completion ports `die_char` (bodies, loot containers, gold money items, player exp loss/rest return, NPC destruction), bodies decay through a generic `expire_item` timer, and `respawn_callback` re-instantiates zone templates server-side with blocked-tile retries. Characters gained serde-defaulted `template_key`/`respawn_ticks` fields and zone placement stamps `rest_x/rest_y` like C `tmpx/tmpy`. Eleven focused tests cover the kill loop; see the Ported table entry for remaining gaps (loot tables, cross-area rest transfer, first-kill/military/achievement hooks).
- `src/module/merchants/store.c` and the core of `merchant.c` are now ported: `CDR_MERCHANT` driver state parses the C args, stores are created from carried stock, players open stores by saying `"<name> ... trade"` (plain say speech now fans out as NT_TEXT driver messages to nearby NPCs), C `salesprice`/`buyprice` formulas drive `con_type 2` store views with `SV_PRICE`/`SV_ITEMPRICE`/`SV_CPRICE`, and `CL_CONTAINER` routes merchant-first with `check_merchant` validation like `cl_container`. Eleven focused tests cover prices, store creation, trade activation, buy/sell, and greeting memory. Store persistence, special stores, the auction clerk, and day/night shop movement remain.
- The death-mode loot system (`src/system/loot/loot.c`) was intentionally deferred: only the pents JSON tables exist and no zone `.chr` template currently references `loot_table`/`loot_table_death`, so porting it now would have no live effect.

### Playtest Fix Session (spells, tick pacing, LoS, door pathing)

- C `run_queue` from `src/system/player_driver.c` is now ported into `World::run_player_spell_queue` (`crates/ugaris-core/src/world/actions.rs`): queued spells were previously never dequeued, so no spell ever cast from the client. The three C priority passes (bless/heal/magicshield, freeze/flash/warcry/pulse, fireball/ball incl. character-target variants) run before the persistent player action, started tasks consume their queue slot and end the pass (C return 1), permanently failed tasks are dropped and scanning continues (C return 2 via `error_state`), and mana-low bless stays queued (C `error_state_mana`). Focused tests cover queued self-spell execution, bless waiting for mana, and invalid-spell discard.
- The legacy lockstep tick pacing is now honored: the client advances its clock once per received tick frame (`prefetch_tick++` in the community client), but the Rust server sent every payload as its own frame, causing rubber-banding "lag" in walking and combat. `ServerRuntime` now buffers per-session payloads (`tick_out`) and `flush_tick_frames` sends exactly one greedily packed frame per session per tick (splitting only above `MAX_LEGACY_TICK_PAYLOAD`), with empty frames for idle logged-in sessions and no fake ticks from out-of-tick event flushes. Focused tests cover frame packing and empty-frame behavior.
- C `plr_map_update` visibility is now ported: `tile_visibility` (`crates/ugaris-server/src/map_sync.rs`) combines tile light with scaled daylight (`check_light`), applies infrared/infravision boosts, keeps the 3x3 center visible, ports the exact `trans_light` quantization table, and gates by LOS; dark tiles send all-zero cells (client renders black), items are gated by `char_see_item`, characters by `char_see_char`. The visible-map cache now stores C `cmap`-style field values (`CellTile`/`CellCharacter`) instead of packet bytes, full refreshes stomp every cell (the client never clears its map on `SV_SETORIGIN`), and one-step walks send only scroll/origin/char packets while `VisibleMapCache::shift` replicates the client's flat memmove so the per-tick diff pass fills fringe tiles and corner-reveal LOS changes. Dark/light profession night sight (prof >= 30) is not ported yet.
- C pathfinder door traversal is now ported: `normal_check_target`/`ignorechar_check_target` treat `MF_DOOR` tiles as pathable even while closed (and ignore-character mode only respects item-caused `MF_TMOVEBLOCK`), so clicking beyond a closed door paths through it and the existing `walk_or_use_driver` bump opens the door mid-route. Focused path and world tests cover door routing, item-blocker retention, and the end-to-end click-behind-door use action.
- Forward-looking work now lives in `PORTING_TODO.md`: a prioritized, checkbox-based task list (P0 playability blockers through P4 area content) with per-task C references, Rust destinations, and acceptance criteria, written so follow-up sessions can execute tasks mechanically. Known immediate gaps documented there: no regeneration tick, `CL_RAISE`/`CL_SPEED`/`CL_FIGHTMODE`/`CL_LOOK_CHAR`/`CL_LOOK_ITEM` unhandled, player death ignores saves, and the game clock never advances.

### Ralph Loop - Regeneration Tick

- P0 "Regeneration tick" is now ported: `World::regenerate_characters` (new `crates/ugaris-core/src/world/regen.rs`, called once per tick from `main.rs` right after `world.advance()`) mirrors C `regenerate()` (act.c:2101, skill-gated endurance + magic-shield lifeshield regen, self-throttled to once per real second via a new `Character.last_regen` field mirroring C `ch.last_regen`) and C `act_idle()`'s (act.c:99) HP/endurance/mana regen plus the warcry-without-magicshield lifeshield leak, gated by `regen_ticker + regen_time` and the `MF_NOREGEN`/`CF_PLAYER`/area-33 special cases. `Character.regen_ticker` is now also stamped on every non-idle/non-passive action completion in `World::tick_basic_actions_with_attack_policy` (`world/actions.rs`), mirroring C `act()`'s `switch (ch[cn].action) { case AC_IDLE/AC_MAGICSHIELD/AC_BLESS_SELF/AC_HEAL_SELF: break; default: ch[cn].regen_ticker = ticker; }` - this is what makes the idle regen gate correctly hold off while a character is actively fighting/walking.
  - Deliberate deviation from C, documented in `regen.rs`: Rust's tick loop treats `action == 0` (`action::IDLE`) as "nothing queued" and skips those characters entirely in `tick_basic_actions_with_attack_policy`, so there is no per-batch idle-completion event (C's `act1`-scaled batch) to hook into. The idle regen instead applies continuously once per real tick using the per-tick-equivalent amount (C's `act1 * val * 15` collapses to `val * 15` per tick). The steady-state rate matches C exactly; only the batching granularity differs.
  - Not ported (tracked separately, out of scope for this task): `reduce_rage`/`increase_rage` (no `rage` field exists on `Character` yet - the todo item's "if the field exists" clause), the `NT_CHAR` notify-area emission at the end of `act_idle` (owned by the separate P0 "NPC sighting messages" task), and `check_endurance`'s fast-mode revert (owned by the "Speed mode" P0 task).
  - 14 focused tests added: 13 in `world/tests/regen.rs` (idle HP/endurance/mana regen and caps, regen-time gate, NOREGEN tile player-vs-NPC distinction, area-33 HP skip, warcry/magicshield lifeshield leak, `regenerate()` endurance/lifeshield gating on bare skill and speed mode, per-second throttling via `last_regen`, area-33 lifeshield zeroing, out-of-bounds no-op) and 1 in `world/tests/actions.rs` (regen_ticker stamped on `AC_WALK` completion, not on `AC_MAGICSHIELD`). Full workspace suite (1011 ugaris-core tests + others), `cargo fmt --all`, `cargo build -p ugaris-server`, and a 10s boot smoke (`entering Rust game loop`, ticks advancing, no panics) all pass.

### Ralph Loop - Skill Raising (CL_RAISE)

- P0 "Skill raising (`CL_RAISE`)" is now ported. C `cl_raise` (`src/system/player.c`) reads a little-endian `u16` value index straight from the packet and calls `raise_value` (`src/system/skill.c`), discarding the return value - it is a *different* function from the already-ported `raise_value_exp` used by stat scrolls: `raise_value` only spends already-earned-but-unspent exp (`exp_used += cost`, gated by `exp_used + cost <= exp`) and never touches `exp` itself, and it also gates on `CF_NOEXP` internally (the scroll call site checks `NOEXP` itself before calling `raise_value_exp`, so that check was missing for the new function and had to be added).
  - New `raise_value` helper added to `crates/ugaris-core/src/item_driver/scrolls.rs` next to the existing `raise_value_exp`/`raise_cost`/`skillmax`/`bare_value`/`skill_raise_cost_factor` helpers, reusing all of them (no duplicated cost math).
  - New `World::raise_skill(character_id, value) -> RaiseSkillOutcome` in new module `crates/ugaris-core/src/world/skills.rs` wraps the helper, sets `CharacterFlags::UPDATE` on success, and returns the raised value's bare/effective values plus `exp`/`exp_used` (or `Blocked`).
  - `crates/ugaris-server/src/main.rs` now handles `ClientAction::Raise { value }`: on `Raised`, it builds a small packet with `SV_SETVAL0`/`SV_SETVAL1` for just the one changed value plus `exp`/`exp_used` (same fields `login.rs` sends for all 43 values, but scoped to the single raised value since nothing else changed) and sends it to the character's session(s); on `Blocked` it sends nothing, matching C's silent-failure behavior exactly (`cl_raise` discards `raise_value`'s return code with no client-visible side effect).
  - 9 focused tests added in new `world/tests/skills.rs`: successful raise (bare/effective bump by 1, `exp_used` increases by the exact C `raise_cost` cube-based formula, `exp` untouched, `UPDATE` flag set), effective value never lowered to match bare when it was already boosted above it by other means, and blocked cases - insufficient unspent exp, `CF_NOEXP`, skill not present (bare value 0), already at `skillmax`, unraisable skill (`cost == 0`, e.g. Armor), out-of-range client-supplied value index (no panic), and unknown character id.
  - Not ported (pre-existing, out of scope, same gap already noted for the scroll path): `update_char` modifier recompute, level-up recalculation (moot here since `raise_value` never grants `exp`), and achievement checks. No `ugaris-server`-crate test exists for the `main.rs` match arm itself, matching the existing precedent for other simple inline `ClientAction` handlers (`GetQuestLog`, `ReopenQuest`) which also rely solely on lower-level unit tests plus already-tested `PacketBuilder` methods for their packet bytes.
  - Full workspace suite (1020 ugaris-core tests + others, up from 1011), `cargo fmt --all`, `cargo build -p ugaris-server` (zero warnings), and a boot smoke (`legacy TCP listener ready`, `loaded area zone map`, `entering Rust game loop`, ticks advancing, no panics) all pass.

### Ralph Loop - Speed Mode (CL_SPEED) and Fight Mode (CL_FIGHTMODE)

- P0 "Speed mode (`CL_SPEED`) and fight mode (`CL_FIGHTMODE`)" is now ported. C `cl_speed` (`src/system/player.c`) validates the raw mode byte against `SM_NORMAL`(0)/`SM_FAST`(1)/`SM_STEALTH`(2) (else silently ignored), additionally gates `SM_FAST` on `ch[cn].endurance >= POWERSCALE` (else ignored), then sets `ch[cn].speed_mode` - no client feedback packet on any path. `cl_fightmode` was read in full and confirmed a genuine no-op stub (`return;` with no body); grepping the entire C tree for `fight_mode` shows `ch[cn].fight_mode` is declared in `player.h` but never read or written anywhere else, so there is no C behavior left to port for it.
  - New `SpeedMode::from_client_mode(u8) -> Option<SpeedMode>` (`crates/ugaris-core/src/entity.rs`) validates the raw byte.
  - New `World::set_speed_mode(character_id, mode) -> bool` (new module `crates/ugaris-core/src/world/speed.rs`) applies the C `cl_speed` gates and returns whether the mode changed (used only internally by the handler; C sends no feedback either way).
  - `crates/ugaris-server/src/main.rs` now handles `ClientAction::Speed { mode }` (calls `World::set_speed_mode`, ignores the result - matches C's silent success/failure) and `ClientAction::FightMode { .. }` as an explicit documented no-op match arm (previously both fell through the catch-all `_ => {}`).
  - Also ported the sibling `check_endurance()` (act.c:1838), called unconditionally from the same `tick_char()` loop immediately before `regenerate()` (no position gate, unlike `regenerate()`/`act_idle()`): reverts `SM_FAST` to `SM_NORMAL` and logs `"You're exhausted."` once endurance drops below `POWERSCALE`. Added to `World::regenerate_characters` (`crates/ugaris-core/src/world/regen.rs`) ahead of the existing position-gated logic, using the pre-existing `queue_system_text`/`drain_pending_system_texts` plumbing (already wired to clients via `send_pending_world_system_texts` in `ugaris-server/src/world_events.rs`, previously only fed by other systems). This closes the gap the regen-tick ledger entry above flagged as deferred to this task.
  - 10 focused tests added: `world/tests/speed.rs` (6 - normal/stealth always succeed regardless of endurance, fast requires endurance >= POWERSCALE exactly with a boundary test at exactly `POWERSCALE`, invalid mode byte ignored, unknown character ignored) and 4 new tests in `world/tests/regen.rs` for `check_endurance` (revert + exhausted message below `POWERSCALE`, no revert/no message at exactly `POWERSCALE`, non-fast speed modes untouched even at 0 endurance, runs even for characters outside map bounds where `regenerate()`/`act_idle()` early-return).
  - Fixed a pre-existing test, `regenerate_endurance_blocked_in_fast_speed_mode`: it asserted `regenerate()`'s fast-mode regen block using endurance=0, but endurance=0 also satisfies `check_endurance`'s revert condition, which now runs first and flips the character back to `SM_NORMAL` before `regenerate()` sees it - so the test was silently exercising a code path it didn't intend to (real C behavior: the character stops being throttled and endurance regenerates normally). The fixture now holds endurance at exactly `POWERSCALE` (the revert boundary) to isolate the regen-block assertion the test name describes, with an added assertion that `speed_mode` stays `Fast` throughout.
  - Not ported (genuinely out of scope - no C behavior exists): `cl_fightmode`'s body.
  - Full workspace suite (1030 ugaris-core tests, up from 1020, + others, 0 failed), `cargo fmt --all`, `cargo build -p ugaris-server` (zero warnings), and a 10s boot smoke (`entering Rust game loop`, ticks advancing every ~40ms, no panics) all pass.

### Ralph Loop - Player Death Saves

- P0 "Player death saves" is now ported. Reading the full C `hurt()` (`src/system/death.c:1085`) showed `god_save_char` (`death.c:851`) is called from *inside* `hurt()` itself at the fatal-damage decision point (`death.c:1262`), not from `die_char`/`kill_char` - it runs immediately, before `kill_char` ever schedules the `AC_DIE` death animation, and is priority-ordered strictly after the PK-death check (`cc && CF_PLAYER(cn) && CF_PLAYER(cc)`), so a PK kill never consults `saves` even if the victim has some. This meant the fix belonged in `World::apply_legacy_hurt` (`crates/ugaris-core/src/world/hurt.rs`), not `World::die_character` as originally scoped in the todo note - a saved character never gets `CF_DEAD`, `deaths++`, or the death-animation timer, so routing it through `die_character` (which always runs the body/item/exp-loss sequence) would have been wrong.
  - New `World::god_save_character` (`crates/ugaris-core/src/world/death.rs`) ports `god_save_char` digit-for-digit: decrement `saves` then cap at 10 (the literal, slightly odd C order), `got_saved++` (new serde-defaulted `Character.got_saved: u32` field mirroring C `ch.got_saved`, added everywhere `Character` is constructed), `hp = 1*POWERSCALE`, `remove_all_poison` (already existed), `extinguish` via the existing `remove_show_effect_type(id, EF_BURN)` helper, the two exact Ishtar log lines, and `transfer_to_restarea` (same-area case only, reusing the same rest-position/no-rest-set fallback pattern already used by `die_character`; cross-area transfer stays out of scope per the separate P3 todo item).
  - `apply_legacy_hurt` gained a `cause_is_player` precheck (mirrors the C PK condition) and a new `LegacyHurtOutcome::god_saved` bool; the death-threshold branch now checks `PLAYER && !cause_is_player && saves > 0` between the existing `NODEATH` and normal-kill branches (matching the exact C `if/else if/else` order) and calls `god_save_character` once the target's mutable borrow ends. While reading the full threshold-check block, also found and fixed a related sound-gating bug: C plays the "killed with" death sound (`death.c:1220-1229`) for *every* death-threshold hit - including `NODEATH` and the saves branch, not only an actual kill - so `god_saved` was added to the existing sound condition alongside `killed`/`nodeath_saved` (the "Killed with X.XX damage..." log line itself remains unported, tracked as a follow-up).
  - Moved `legacy_save_number` (the C `save_number` spelled-out-count helper) from being `pub(crate)`-only in `crates/ugaris-server/src/area_apply.rs` into `crates/ugaris-core/src/world/death.rs` as `pub fn legacy_save_number`, re-exported through `world::*`; `area_apply.rs`'s random-shrine-security path and the new save message now share the one implementation instead of duplicating it.
  - 5 focused tests added in `crates/ugaris-core/src/world/tests/hurt.rs`: full save (saves/got_saved counters, hp reset, rest-position teleport, exact Ishtar feedback text), poison+burn-effect removal, the saves>10-after-decrement cap quirk, PK death ignoring an available save, and saves=0 falling through to a normal kill.
  - Not ported (out of scope, left for future `hurt.rs`/`die_character` work): the sibling `hurt()` branches sharing the same C `else` chain - `shutdown_save_char`, `area_save_char` (areas 11/12/22/25/31/32/33/36), `arena_save_char`, the Teufelheim PK special case, and the LQ/area-21 death specials - plus the "Killed with X.XX damage by a lvl N NAME." log line.
  - Full workspace suite (1035 ugaris-core tests, up from 1030, + others, 0 failed), `cargo fmt --all`, `cargo build -p ugaris-server` (zero warnings), and a boot smoke (`legacy TCP listener ready`, `loaded area zone map`, `entering Rust game loop`, ticks advancing, no panics) all pass.

### Ralph Loop - Game Clock Advancement

- P0 "Game clock advancement" is now mostly ported. Reading the full C `tick_date()` (`src/system/date.c:267`) and its one call site (`src/server.c:616-618`) showed the game clock is *not* incremented at some fixed per-tick delta - `time_now = time(NULL)` is refreshed every real loop iteration and `tick_date()` recomputes every date/light global from scratch via `game_time = time_now - STARTTIME`, with `DAYLEN = 2 real hours` per in-game day. `crates/ugaris-core/src/game_time.rs` already had the full `GameDate::calculate` math ported and unit-tested (from an earlier iteration) - it was simply never called from anywhere except the `/dlight` admin command and tests, so `world.date` stayed at its zeroed `Default` forever in the running server.
  - New `World::advance_date(unix_time, area_id, dlight_override) -> bool` (new module `crates/ugaris-core/src/world/date.rs`) wraps `GameDate::calculate` and returns whether `daylight` changed since the previous call - kept as a separate method rather than folded into the existing no-arg `World::advance()` (which only bumps the tick counter and has ~30 call sites, mostly tests, that would otherwise need an `area_id` argument threaded through for no benefit).
  - Wired into `crates/ugaris-server/src/main.rs`: a new `current_unix_time()` helper (same `SystemTime::now().duration_since(UNIX_EPOCH)` idiom already used by `rng.rs`/`xmas.rs`/`stacks.rs`) feeds `world.advance_date(...)`, called once before the tick loop starts (so pre-first-tick logins already see a live clock, matching `tick_date()` running before `tick_login()` in the same C iteration) and once per tick immediately after `world.advance()` (matching `tick_date()`'s position immediately before `tick_char()` in the C loop). `runtime.dlight_override` is forwarded exactly like the existing `/dlight` admin command (`commands_admin.rs`) does today, so an active override survives every tick's recompute instead of being clobbered back to the natural value.
  - Also found and fixed a real, previously load-bearing-but-invisible bug while verifying the "nightlight timers already fire on a daylight threshold" gotcha: `World::execute_item_driver_timer_request` (`crates/ugaris-core/src/world/item_outcomes.rs`), the sole dispatch path for *every* timer-driven item-driver callback (`process_due_timers`, C `call_item(..., 0, ...)` equivalent), built its `ItemDriverContext` without ever setting `daylight`/`hour`/`fullmoon`/`newmoon`/`solstice`/`equinox` - they stayed at the struct's `Default` (`0`/`false`) for every timer call, forever. This is exactly the C `dlight`/`hour`/moon-phase globals that `nightlight_driver` (`src/module/base.c:1812`, ported to `crates/ugaris-core/src/item_driver/lights.rs`) and `swampwhisp_driver` (`crates/ugaris-core/src/item_driver/area15_swamp.rs`) read directly. Before this fix the bug was masked because `world.date` was *also* always frozen at zero, so `context.daylight` (0) always matched `world.date.daylight` (0) - nightlights would light up once and then never turn back off since `context.daylight > 80` could never be true. Now `execute_item_driver_timer_request` copies these six fields from `self.date` unconditionally at the top of its context-augmentation block, so every timer-driven driver sees the live game clock like C's globals always did.
  - Tests: new `crates/ugaris-core/src/world/tests/date.rs` (6 tests) - `advance_date` delegates exactly to `GameDate::calculate`, reports no change while `daylight` stays at its starting value (midnight, new moon, still-frozen default), reports a change across a real sunrise boundary (just-before-sunrise stays 0, one hour past sunrise reaches the full `255`, and calling again with the same time reports no further change), forwards a `/dlight`-style numeric override through regardless of the natural computation, respects the per-area light override table (area 23 underground), and advances exactly one `yday` per `DAY_LEN` (7200) real seconds.
  - Not ported (deliberately, investigated and documented in the todo note): a map-wide "mark all light-dirty sectors when daylight changes" sweep. C's only "periodic refresh on `dlight` change" is `player.c:2357` (`if (dlight != player[nr]->dlight) redo = 1;`), which forces one player's *next* visible-map send to be a full resend instead of the incremental scan - it does not touch `tile.dlight` (the static indoor-light geometry from `compute_dlight`/`reset_dlight`, which the C source only recomputes on door/structure mutations, confirmed by grepping every call site in `create.c`). Grepping `crates/ugaris-server/src/map_sync.rs` found zero uses of `world.dirty_sectors`/`skip_x_sector` anywhere - the current per-tick sync path (`map_diff_payloads`) already recomputes every visible tile's effective light fresh from `world.date.daylight` each tick and diffs it against the cached view, so daylight changes already reach clients without any dirty-sector plumbing. Revisit once `map_sync.rs` starts consuming dirty sectors as a real network-traffic optimization - at that point drive a full dirty-mark off `advance_date`'s existing boolean return value.
  - Full workspace suite (1041 ugaris-core tests, up from 1035, + others, 0 failed), `cargo fmt --all`, `cargo build -p ugaris-server` (zero warnings), and a 10s boot smoke (`entering Rust game loop`, ticks advancing, no panics) all pass.

### Ralph Loop - Look At Character (CL_LOOK_CHAR)

- P0 "Look at character (`CL_LOOK_CHAR`)" is now ported for the core text/paperdoll behavior. Reading the full C `cl_look_char` (`src/system/player.c:803-819`) and `look_char` (`src/system/tool.c:1928-2009`) showed: `cl_look_char` only bounds-checks the target and gates on `char_see_char` (already ported in `ugaris_core::see`) before calling `look_char`, which sends exactly two `SV_TEXT` packets (`"#1"` header: title/name/level, and `"#2"` body: description + player-only stat lines) plus one `SV_LOOKINV` paperdoll packet in between, all gated by the *looker* being `CF_PLAYER` (always true in practice - only players can send `CL_LOOK_CHAR`).
  - New `World::look_character_text` and `World::look_character_paperdoll` (new module functions in `crates/ugaris-core/src/world/text.rs`, alongside the pre-existing `tabunga_lines`/`notify_area` text helpers) port the header/body string construction and the `plr_send_inv` paperdoll fields (target `sprite`/`c1`/`c2`/`c3`/12 worn-slot item sprites, looked up through `world.items`). Also added a dedicated `PROF_TABLE: [(&str, u8); 20]` matching C `prof[P_MAX]` (`src/system/prof.c`) exactly - this is *not* the same as the pre-existing `entity::PROFESSION_NAMES`, which mirrors an unrelated JSON-export table in `src/system/game/character.c` that spells profession 7 "Master Trader" instead of prof.c's gameplay-visible "Trader" (confirmed by reading both C tables in full; using the wrong one would have produced wrong player-visible text).
  - `crates/ugaris-server/src/main.rs` wires `ClientAction::LookCharacter { character }`: converts the client's `u16` target id to `CharacterId(u32::from(character))` (matching the existing `CharacterId(u32::from(u16))` convention used elsewhere, e.g. `world/spawn.rs`'s gate-slot decode), looks up the target's `PlayerRuntime` via `runtime.player_for_character` for the two session-only facts C stores directly on `ch[]` but this codebase keeps on `PlayerRuntime` - `has_used_random_shrine(51)` (C `DEATH_SHRINE`, for the "the Brave" header variant) and `current_mirror_id` (C `ch.mirror`, defaulting to 0 if the target has no runtime) - then sends the header/body through the existing `command_feedback` queue and the paperdoll through a direct `PacketBuilder::look_inventory` send (the builder already existed, byte-exact, but had zero callers anywhere in the server crate until now).
  - 12 focused tests added in `crates/ugaris-core/src/world/tests/text.rs`: saves/deaths/mirror/karma line for a normal player target, singular "1 save" wording, hardcore death-count variant, "the Brave" header variant, `CF_WON` title prefix (`Sir `/`Lady `), player-only lines omitted for non-player (NPC) targets while profession lines still show (matches C: `show_prof_info` runs unconditionally, not gated on target `CF_PLAYER`), profession title-prefix percent math, `None` when the looker isn't `CF_PLAYER`, `None` when the target is invisible (LOS/see gate), `None` for unknown looker/target ids, and paperdoll sprite/color/worn-slot mapping including the unknown-target `None` case.
  - Not ported (documented gaps, matching the C `look_char` branches that need systems this codebase doesn't have yet): labyrinth-solved count (`count_solved_labs`), first-kill Hell flavor text (`check_first_kill`), army rank (`DRD_RANK_PPD`/`rankname[]` - P3 "Military ranks" todo item), PK info (`show_pk_info`/`DRD_PK_PPD` kill/death counts), clan info (`show_clan_info` - P3 "Clan system" todo item), club info (`show_club_info`), and the looker-`CF_GOD` debug branch (dumping the target's carried non-worn items via `look_item` plus active effect-slot types - admin-only, deferred since `CL_LOOK_ITEM`'s own text builder is the next unported P0 task). No dedicated `ugaris-server`-crate test exists for the `main.rs` match arm itself, matching the established precedent for other simple inline `ClientAction` handlers (`Raise`, `Speed`, `GetQuestLog`) - the wiring only reuses already-tested core functions and `PacketBuilder` methods.
  - Full workspace suite (1053 ugaris-core tests, up from 1041, + others, 0 failed), `cargo fmt --all`, `cargo build -p ugaris-server` (zero warnings), and a 10s boot smoke (`legacy TCP listener ready`, `loaded area zone map`, `entering Rust game loop`, ticks advancing, no panics) all pass.
  - Closed out (2026-07-03, no code changes): re-read C `player.c:2357-2380` end to end to settle whether the deferred "light-dirty sector" sweep was a real remaining gap. Confirmed `redo = 1` only ever guards the `skipx_sector(...)` early-`continue` inside `plr_map_update`'s per-tile double loop (`player.c:2374-2380`) - i.e. its entire purpose in C is to defeat a performance shortcut that lets the loop skip recomputing tiles in already-synced sectors. Rust's `tile_visibility`/`map_diff_payloads` (`crates/ugaris-server/src/map_sync.rs`) has no equivalent skip-sector shortcut at all - every visible tile's light is recomputed from `world.date.daylight` unconditionally each tick and diffed against the cached `VisibleMapCell` - so there is nothing for a "daylight changed" signal to defeat; the behavior is already unconditionally correct. Marked the P0 task `[x]` in `PORTING_TODO.md`. `world.dirty_sectors`/`advance_date`'s bool return remain unused by `map_sync.rs` on purpose and should only be wired up if a future task adds a real skip-sector fast path to `map_diff_payloads` for network-traffic reduction.

### Ralph Loop - Look At Map Item (CL_LOOK_ITEM)

- P0 "Look at map item (`CL_LOOK_ITEM`)" is now ported. Reading the full C `cl_look_item` (`src/system/player.c:764-787`) showed it is structurally identical to the already-ported `cl_take`: bounds-check the target tile (`x<1||x>=MAXMAP-1||y<1||y>=MAXMAP-1`), resolve `map[m].it`, no-op if empty, gate on `char_see_item(cn, in)` (already ported in `ugaris_core::see`), then call the same `look_item(cn, it+in, -1)` that inventory-look uses (slot `-1` just means "not an inventory slot" for the text builder - `legacy_item_look_text` already ports this and doesn't need the slot value at all).
  - New `look_map_item_text` in `crates/ugaris-server/src/inventory.rs`: bounds-checks via the existing `MapGrid::legacy_inner_bounds`, reads the item id off `MapTile::item` via `world.map.tile(x, y)`, looks up the looker character and the item, gates visibility with `ugaris_core::see::char_see_item`, and reuses `legacy_item_look_text` (the same function `inventory_look_slot` already calls) for the response text.
  - Wired `ClientAction::LookItem { x, y }` into `apply_inventory_client_action` (alongside `Swap`/`LookInventory`/`UseInventory`) and added it to the `main.rs` match arm that routes `InventoryCommandResult` to `command_feedback` - no new dispatch plumbing was needed since the packet parsing (`CL_LOOK_ITEM = 24`, 5-byte length, `ClientAction::LookItem` variant) already existed end-to-end from prior work, only the consuming handler was missing.
  - 5 focused tests added in `crates/ugaris-server/src/tests/inventory.rs`: visible map item returns the expected `legacy_item_look_text` string, out-of-bounds coordinates no-op, an empty tile no-ops, a carried (not-on-map) item no-ops, and an item outside `DIST_MAX` (40 tiles) line-of-sight range no-ops.
  - Full workspace suite (332 ugaris-server tests, up from 327, + others, 0 failed), `cargo fmt --all`, `cargo build -p ugaris-server` (zero warnings), and a 10s boot smoke (`entering Rust game loop`, ticks advancing, no panics) all pass.

### Ralph Loop - Junk Item (CL_JUNK_ITEM)

- P0 "Junk item (`CL_JUNK_ITEM`)" is now ported. Reading the full C `cl_junk_item` (`src/system/player.c:1325-1337`) showed it destroys `ch[cn].citem` (the cursor item) unless it carries `IF_NOJUNK` (`server.h:177`, `1ull << 35`) - the todo note's mention of an "`IF_QUEST` guard" was incorrect (`IF_QUEST` gates drop/give elsewhere, not junk); corrected the todo text. `cl_junk_item` logs via `dlog` (debug-only, no gameplay effect, not ported), clears `ch[cn].citem = 0`, calls `destroy_item(in)` (`drvlib.c:2427` - frees the item slot, recursing into `destroy_item_container` for held containers), and sets `CF_ITEMS` so the next per-tick sync sends `SV_SETCITEM` clearing the client's cursor sprite. No packet is sent directly by the handler itself.
  - Protocol layer needed zero changes: `CL_JUNK_ITEM = 30` (1-byte packet, no args) and `ClientAction::JunkItem` were already fully wired in `crates/ugaris-protocol/src/client.rs`/`command.rs` from prior work - only the consuming server-side handler was missing.
  - New `apply_junk_item_client_action` in `crates/ugaris-server/src/item_apply.rs`: reads the character's `cursor_item`, no-ops if empty or if the item is missing, no-ops if the item has `ItemFlags::NOJUNK`, otherwise calls the pre-existing `World::destroy_item` (`crates/ugaris-core/src/world/items.rs:107`), which already clears `cursor_item` and inserts `CharacterFlags::ITEMS` as part of its generic item-removal side effects - no new `World` method needed.
  - Wired `ClientAction::JunkItem` in `crates/ugaris-server/src/main.rs`'s command dispatch, alongside the `TakeGold`/`DropGold` arm: on success, pushes the character onto `command_inventory_refresh` (the existing per-tick `CF_ITEMS`-equivalent snapshot queue that builds `SV_SETCITEM`/`SV_SETITEM`/`SV_GOLD`).
  - 3 focused tests added in `crates/ugaris-server/src/tests/item_apply.rs`: a plain cursor item is destroyed and the cursor cleared with `CharacterFlags::ITEMS` set, an `ItemFlags::NOJUNK` item is left untouched (cursor unchanged, item still present), and an empty cursor is a no-op.
  - Rust's `destroy_item` does not yet recurse into contained items the way C's `destroy_item_container` does for containers-on-cursor; this matches the pre-existing gap noted for other item-destruction paths in this codebase (no dedicated container-content-loss test added here - out of scope for this slice, flagged for whichever future task ports container-aware destruction).

### Ralph Loop - Ping (CL_PING)

- P0 "Ping (`CL_PING`)" is now ported. Reading the full C `cl_ping` (`src/system/player.c:1352-1358`) showed it is a pure transport echo: it reads the raw 4-byte value from the client's payload as an `unsigned int` and writes it straight back, prefixed with `SV_PING` (49), as a 5-byte packet - no timestamp math, no state read/write on `ch[cn]` at all. Cross-checking the community client (`astonia_community_client/src/client/protocol.c`) confirmed the value is the client's own `SDL_GetTicks()` at send time (`cmd_ping`), and that `sv_ping`/`svl_ping` are just the client's two-pass (length-then-process) naming convention applied to the single `SV_PING` packet type - there is no distinct `SV_LPING` type as the stale todo note assumed; corrected during implementation.
  - Protocol layer needed almost no changes: `CL_PING = 39`, its 5-byte length-table entry, and `ClientAction::Ping { value: u32 }` parsing were already fully wired in `crates/ugaris-protocol/src/client.rs`/`command.rs` from prior work; `SV_PING = 49` also already existed in `packet.rs`. Only the response builder and the server-side handler were missing.
  - New `PacketBuilder::ping(value: u32)` in `crates/ugaris-protocol/src/packet.rs`, mirroring the existing `ticker`/`mirror` methods exactly (`u8` type byte + `put_u32_le` value, matching C's native/little-endian raw pointer cast).
  - Wired `ClientAction::Ping { value }` in `crates/ugaris-server/src/main.rs`'s command dispatch: builds the packet and sends it directly back to the originating session via `runtime.send_to_session` - no `World`/character-state interaction at all, matching C's handler exactly (it doesn't even receive a `co`/target character, only the raw buffer).
  - 2 focused tests added: `crates/ugaris-protocol/src/command.rs` (`parses_ping_opaque_value_little_endian` - little-endian decode of an arbitrary 4-byte value) and `crates/ugaris-protocol/src/packet.rs` (`ping_echoes_opaque_value_unmodified_like_c_cl_ping` - builder produces the exact 5-byte `[SV_PING, ..value_le]` layout). No dedicated `ugaris-server`-crate test was added for the `main.rs` match arm itself, matching the established precedent for other simple inline `ClientAction` handlers (`Raise`, `GetQuestLog`) that rely on already-tested lower-level parse/builder unit tests.
  - Full workspace suite (ugaris-protocol tests up from 31 to 33, ugaris-core/ugaris-server unaffected, 0 failed), `cargo fmt --all`, `cargo build -p ugaris-server` (zero warnings), and a 10s boot smoke (`entering Rust game loop`, ticks advancing, no panics) all pass.
  - Full workspace suite (335 ugaris-server tests, up from 332, + others, 0 failed), `cargo fmt --all`, `cargo build -p ugaris-server` (zero warnings), and a 10s boot smoke (`entering Rust game loop`, ticks advancing, no panics) all pass.

### Ralph Loop - Fast Sell (CL_FASTSELL)

- P0 "Fast sell (`CL_FASTSELL`)" is now ported for the merchant branch. Reading the full C `cl_fastsell` (`src/system/player.c:877-922`) showed the flow is: bounds-check the slot (identical to `cl_use_inv`'s `pos>=0 && pos<INVENTORYSIZE && !(12<=pos<=29)` guard), call `swap(cn, pos)` (`src/system/do.c:1216`) which picks the slot item up onto the cursor and swaps back whatever was already held (so an empty slot with a held cursor item just puts the held item into the slot and the resulting `citem` is 0 - a no-op sell attempt, matching C's `if (!(in = ch[cn].citem)) return;`), re-validate with `check_merchant(cn)`/`check_container_item(cn)` if those windows were open, then either sell to the merchant (`player_store(cn, 0, 1, 0)`, blocking `IF_QUEST` items first with the exact hold-SHIFT message) or - if no merchant is open - store into whatever container/depot/account-depot is open via `con_in`.
  - New `apply_fast_sell` in `crates/ugaris-server/src/merchants.rs` reuses `inventory_swap_slot` (the existing simplified C `swap`) for the pickup, `World::check_merchant` for re-validation, and `World::merchant_store_sell` (already-ported `player_store`/`buy` sell path) for the trade; returns a `FastSellResult` distinguishing "inventory changed" (the swap ran - true even when nothing sold, matching C leaving the item on the cursor) from "sold" (a real trade happened, so the merchant store view needs repainting too).
  - Wired `ClientAction::FastSell { slot }` in `crates/ugaris-server/src/main.rs`: pushes `command_inventory_refresh` whenever the swap ran, and `command_container_refresh` only on an actual sale (merchant store prices/wares only change when something sold, but the client's cursor/inventory always needs the swap reflected).
  - REMAINING (documented in `PORTING_TODO.md`): only the merchant branch is wired. C's `con_in` branch (`check_container_item` + `player_depot`/`account_depot_store`/`container`) is not implemented from an inventory slot - the per-character legacy depot (`DRD_DEPOT_PPD`/`MAXDEPOT`, `src/system/depot.c`) isn't ported at all yet (only the account-wide depot exists), so fast-selling into an open item container or account depot from an inventory slot is out of scope for this slice.
  - Tests: new `crates/ugaris-server/src/tests/merchants.rs` (5 tests) - sells to an open merchant using the exact C `buyprice` formula and stocks the sold item for resale, swaps a held cursor item back into an empty slot as a no-op sale when nothing was in the slot, blocks quest items with the verbatim C message while leaving the item on the cursor, rejects the equip/spell slot range like C's bounds check, and no-ops (but still swaps) without an active merchant.
  - Full workspace suite (340 ugaris-server tests, up from 335, + others, 0 failed), `cargo fmt --all`, `cargo build -p ugaris-server` (zero warnings), and a 10s boot smoke (`legacy TCP listener ready`, `loaded area zone map`, `entering Rust game loop`, ticks advancing, no panics) all pass.

### Ralph Loop - NPC Sighting Messages (`NT_CHAR` Emission), Partial

- P0 "NPC sighting messages (`NT_CHAR` emission)" - producer side wired for
  the walk-completion call site; other `act_*` call sites deferred. Reading
  the full C `notify_area` (`src/system/notify.c:146-168`) corrected two
  wrong premises in the original todo note: (1) `notify_area` itself has
  **no** `char_see_char`/visibility gate - it is an unconditional `NOTIFY_SIZE`
  (32-tile) bounding-box broadcast to every character in range regardless of
  invisibility/LOS; the visibility gate is applied *downstream*, inside each
  driver's own message consumer (confirmed in `merchant.c:354-388`'s explicit
  `char_see_char(cn, co)` check and `simple_baddy.c:198-204`'s helper-bless
  gate) - so no gate belongs inside Rust's `World::notify_area`. (2)
  `src/system/create.c` (spawn) never calls `notify_area` at all - the only
  spawn-time notify is the self-targeted `notify_char(n, NT_CREATE, ...)`,
  which Rust's zone-loading path already sends; there is no second `NT_CHAR`
  call site to add to `World::spawn_character`.
  - Also fixed a real, independent bug found while reading `notify_area`:
    Rust's implementation used a `+-16` tile radius; C's `NOTIFY_SIZE` is 32
    (a 65x65 box, not 33x33). Fixed in
    `crates/ugaris-core/src/world/text.rs` for all `notify_area` callers
    (`NT_NPC`/`NT_SPELL` sites unaffected in behavior beyond the wider box).
  - Wired the highest-value producer call site: `World::complete_walk`
    (`crates/ugaris-core/src/world/actions.rs`) now calls
    `self.notify_area(after.x, after.y, NT_CHAR, character_id.0 as i32, 0, 0)`
    after a successful move, gated on `!CharacterFlags::NONOTIFY`, mirroring
    C `act_walk` (`act.c:227-229`) exactly (including that the mover itself
    ends up inside its own notify box, same as C).
  - This makes the previously dead-at-runtime `NT_CHAR` consumers actually
    fire during normal gameplay: simple-baddy aggro/helper-bless
    (`character_driver.rs::process_simple_baddy_messages`, driven by
    `World::process_simple_baddy_message_actions_with_random`) and the
    Lab2-undead patrol-removal check (`world/lab2_undead.rs`) both already
    existed and were unit-tested in isolation, but had no live producer
    before this change.
  - Tests: `world/tests/actions.rs` - a completed walk queues exactly one
    `NT_CHAR` message (with `dat1` = the mover's id) to every character
    inside the 32-tile box including the mover itself, a character outside
    the box gets nothing, `CharacterFlags::NONOTIFY` suppresses the
    broadcast entirely, and a failed walk (no movement) sends nothing. Fixed
    three pre-existing tests whose fixtures placed a "should not be
    notified" character just outside the old (incorrect) 16-tile radius but
    inside the correct 32-tile one - moved those fixtures further away to
    keep testing the real boundary:
    `world/tests/doors.rs::world_executes_area17_pick_door_with_legacy_timer`,
    `world/tests/effect_tick.rs::world_fireball_machine_timer_creates_retained_projectile_and_reschedules`,
    `world/tests/item_outcomes.rs::labtorch_extinguish_notifies_nearby_npcs`.
  - REMAINING: C fires `notify_area(.., NT_CHAR, ..)` from nearly every
    `act_*` completion, not just walk - `act_idle`, `act_take`, `act_use`,
    `act_drop`, `act_attack`, `act_give`, and every spell-cast completion
    (`act_firering`/`act_fireball`/`act_flash`/`act_magicshield`/`act_bless`/
    `act_warcry`/`act_freeze`/`act_pulse`/`act_heal`). Only `complete_walk`
    is wired here; `complete_take`/`complete_use`/`complete_drop`/
    `complete_give` (`world/actions.rs`) and the spell-outcome handlers in
    `world/item_outcomes.rs` still don't emit `NT_CHAR`. The `act_idle`
    equivalent (`world/regen.rs`) is deliberately *not* wired in this slice:
    Rust's idle regen runs continuously every real tick (documented
    pre-existing gap in that module's doc comment) rather than once per C's
    `act1`-sized idle batch, so naively adding a per-tick `notify_area` call
    there would flood every idle character's neighbors with an `NT_CHAR`
    message every single tick - a much higher rate than C's batched
    emission - and needs the idle-batching gap closed first to be faithful.
    The todo's suggestion to "simplify the merchant greeting scan to consume
    `NT_CHAR` like C `merchant_driver`" is also not done - the existing
    `greet_nearby_players` per-tick brute-force scan
    (`world/merchant.rs`) still does the job independently of the message
    queue; migrating it to a `process_merchant_messages` `NT_CHAR` arm is
    left for a future slice since it is optional per the todo's own wording
    ("keep the scan fallback if you must").
  - Full workspace suite (1053 ugaris-core tests via `--lib`, plus
    ugaris-server/protocol/db/net, 0 failed), `cargo fmt --all`,
    `cargo build -p ugaris-server` (zero warnings), and a 10s boot smoke
    (`legacy TCP listener ready`, `loaded area zone map`,
    `entering Rust game loop`, ticks advancing, no panics) all pass.

### Ralph Loop Iteration 13 Additional Progress

- Continued the P0 "NPC sighting messages (`NT_CHAR` emission)" task by
  wiring the remaining inventory/social `act_*` completions:
  `World::complete_take`/`complete_drop`/`complete_use`/`complete_give`
  (`crates/ugaris-core/src/world/actions.rs`) now each emit
  `notify_area(.., NT_CHAR, ..)` matching their exact C call site -
  `act_take` (act.c:333-335) and `act_drop` (act.c:440-441) gate on
  `!CF_NONOTIFY` right after the item moves; `act_drop` additionally fires
  an *unconditional* `NT_ITEM` (act.c:443) for the dropped item regardless
  of `CF_NONOTIFY`; `act_use` (act.c:376-379) fires `NT_CHAR` once
  target/item validation passes, before the deeper item-driver outcome is
  known (so it still fires even if the eventual `use_item` driver later
  declines); `act_give` (act.c:871-875) fires `NT_CHAR` from the giver's
  position after `notify_char(co, NT_GIVE, ...)` (already wired via
  `transfer_cursor_item`). Added `NT_ITEM` to the `world/mod.rs`
  `character_driver::` re-export list (only `NT_CHAR`/`NT_GIVE`/etc. were
  imported before).
- Tests: `world/tests/actions.rs` gained one notify + one no-notify test per
  call site (`complete_take_notifies_nearby_characters_with_nt_char`,
  `complete_take_skips_notify_when_cf_nonotify_set`,
  `complete_drop_notifies_nt_char_and_unconditional_nt_item`,
  `complete_use_notifies_nt_char_once_validation_passes`,
  `complete_use_skips_notify_when_validation_fails`,
  `complete_give_notifies_nt_char_after_nt_give`,
  `complete_give_skips_nt_char_when_cf_nonotify_set`). Updated the
  pre-existing
  `world/tests/lab2_undead.rs::give_completion_notifies_lab2_undead_receiver`
  test, which now correctly observes 2 driver messages (`NT_GIVE` then
  `NT_CHAR`) since the receiver sits inside its own notify box - this is
  correct C behavior, not a regression.
- REMAINING (unchanged scope for a future slice): `act_attack`
  (act.c:792-794) still doesn't emit `NT_CHAR` in Rust
  (`World::complete_attack_with_rolls_and_clash_roll` in
  `world/combat.rs`). Every spell-cast `act_*` completion
  (fireball/ball/earthrain/earthmud/flash/magicshield/bless/warcry/freeze/
  pulse/heal) still doesn't emit `NT_CHAR`/`NT_SPELL` in Rust
  (`world/item_outcomes.rs`'s spell-outcome handlers). `act_idle`
  (`world/regen.rs`) remains intentionally deferred pending the idle-batch
  granularity fix. The merchant greeting scan migration to consume
  `NT_CHAR` via `process_merchant_messages` remains optional/undone.
- Full workspace suite (1063 tests total across all crates, 0 failed),
  `cargo fmt --all`, `cargo build -p ugaris-server` (zero warnings), and a
  10s boot smoke (ticks advancing, NPC driver messages processed, no
  panics) all pass.

### Ralph Loop Iteration 14 Additional Progress

- Continued the P0 "NPC sighting messages (`NT_CHAR` emission)" task by
  wiring `act_attack` (act.c:763-793): `World::complete_attack_with_rolls_and_clash_roll`
  (`crates/ugaris-core/src/world/combat.rs`) now emits
  `notify_area(.., NT_CHAR, ..)` from the attacker's position after
  `apply_legacy_hurt`, gated on `!CF_NONOTIFY`, firing on both hit and miss
  rolls (C calls `sub_attack` unconditionally, then runs the
  surround/rage/notify tail regardless of the roll outcome). Added a
  defensive "attacker still alive" check (`!CharacterFlags::DEAD`) mirroring
  C's `if (!ch[cn].flags) return 0` guard against `sub_attack` having killed
  the attacker mid-call - currently unreachable since nothing damages the
  attacker during its own attack, but kept for parity and future
  reflect-damage effects.
- Tests: `world/tests/combat.rs` gained
  `completed_attack_notifies_nearby_characters_with_nt_char_on_hit_and_miss`
  (verifies `NT_CHAR` fires on both a hit and a miss roll, filtering out the
  unrelated `NT_SEEHIT` that `apply_legacy_hurt` also queues to the same
  bystander on a hit since bystanders inside the 16-tile `hurt()` radius get
  both messages) and `completed_attack_skips_notify_when_cf_nonotify_set`
  (uses a miss roll to isolate the `CF_NONOTIFY` gate from
  `apply_legacy_hurt`'s own unconditional `NT_SEEHIT` broadcast).
- REMAINING (unchanged scope for a future slice): every spell-cast `act_*`
  completion (fireball/ball/earthrain/earthmud/flash/magicshield/bless/
  warcry/freeze/pulse/heal) still doesn't emit `NT_CHAR`/`NT_SPELL` in Rust
  (`world/item_outcomes.rs`'s spell-outcome handlers) - this is the natural
  next slice. `sub_surround`/`V_SURROUND` (act.c:697-705) and
  `increase_rage` remain unported (no `rage`/`V_SURROUND` fields on
  `Character`), independent of the `NT_CHAR` wiring done here. `act_idle`
  (`world/regen.rs`) remains intentionally deferred pending the idle-batch
  granularity fix. The merchant greeting scan migration to consume
  `NT_CHAR` via `process_merchant_messages` remains optional/undone.
- Full workspace suite (1065 tests total across all crates, 0 failed),
  `cargo fmt --all`, `cargo build -p ugaris-server` (zero warnings), and a
  10s boot smoke (ticks advancing, NPC driver messages processed, no
  panics) all pass.

### Ralph Loop Iteration 15 Additional Progress

- Completed the P0 "NPC sighting messages (`NT_CHAR` emission)" task
  (marked `[x]` in `PORTING_TODO.md`) by wiring the last remaining call
  sites: the 12 spell-cast `act_*` completions in
  `crates/ugaris-core/src/world/spells.rs`. `complete_bless`,
  `complete_flash`, `complete_fireball`, `complete_ball`,
  `complete_firering`, `complete_magicshield`, `complete_pulse`,
  `complete_freeze`, `complete_warcry`, and `complete_heal` now each emit
  `notify_area(.., NT_CHAR, ..)` gated on `!CharacterFlags::NONOTIFY`
  followed by an unconditional `notify_area(.., NT_SPELL, .., value, fn)`,
  matching every corresponding C call site exactly: `act.c:936-940`
  (fireball), `1057-1061` (ball, which intentionally carries
  `CharacterValue::Flash` as the payload - not a "Ball" value, which
  doesn't exist in C either - copied digit-for-digit from
  `notify_area(ch[cn].x, ch[cn].y, NT_SPELL, cn, V_FLASH, fn)`),
  `929-933`/`935-941` (firering, including the "did `hurt` kill the caster"
  `if (ch[cn].flags)` guard, ported as `!CharacterFlags::DEAD` mirroring
  `complete_attack`'s existing equivalent guard), `1041-1044` (flash),
  `1090-1093` (magicshield), `1237-1241` (bless, plus `sound_area`),
  `1399-1402` (warcry), `1556-1560` (freeze, plus `sound_area`),
  `1637-1640` (pulse), `1671-1674` (heal, broadcast from the caster's
  position, not the healed target's). `complete_earthrain`/
  `complete_earthmud` were intentionally left unchanged: C's own
  `act_earthrain`/`act_earthmud` have their `notify_area` calls commented
  out (dead code), so there is no C behavior to port there - confirmed by
  reading `act.c:969-1001` directly.
- Tests: added
  `world/tests/spells.rs::completed_firering_notifies_nearby_characters_with_nt_char_and_nt_spell`
  (the only spell with no existing player-facing completion test) and
  added `NT_CHAR`/`NT_SPELL` assertions to the existing
  `player_magicshield_spell_sets_up_and_completes_lifeshield_gain`,
  `player_heal_spell_restores_target_hp_on_completion`,
  `player_bless_spell_installs_carried_spell_item_on_completion`,
  `player_flash_spell_installs_timed_speed_spell_on_self`,
  `player_freeze_spell_installs_negative_speed_spell_on_nearby_target`
  (`world/tests/spells.rs`),
  `player_pulse_damages_low_health_target_and_creates_visible_effects`
  (`world/tests/effects.rs`), `targeted_fireball_sets_up_projectile_action`
  and `targeted_ball_sets_up_projectile_action`
  (`world/tests/effect_tick.rs`), and both warcry tests
  (`player_warcry_sets_up_and_debuffs_sound_reachable_targets`,
  `player_warcry_does_not_pass_soundblocking_tiles` in
  `world/tests/text.rs` - the second proves the broadcast is unconditional
  even when a soundblock wall stops the warcry effect itself from reaching
  the target). Updated one pre-existing test whose fixture locked in the
  old "no messages" behavior:
  `world/tests/spells.rs::action_tick_attack_policy_can_block_area_spell_targets`
  now asserts the attack-policy-blocked pulse target still observes
  `NT_CHAR`/`NT_SPELL` (the area broadcast from the caster's position is
  unconditional, independent of whether any individual target's damage was
  blocked by the attack policy), matching C exactly - this is a corrected
  test, not a weakened one.
- REMAINING (all intentional/deferred, documented in the P0 task note):
  `sub_surround`/`V_SURROUND` (act.c:697-705) and `increase_rage` remain
  unported (no `rage`/`V_SURROUND` fields on `Character` yet), independent
  of all `NT_CHAR`/`NT_SPELL` wiring done across iterations 13-15.
  `act_idle` (`world/regen.rs`) remains intentionally deferred pending the
  idle-batch granularity fix. The merchant greeting scan migration to
  consume `NT_CHAR` via `process_merchant_messages` remains
  optional/undone (explicitly optional per the task's own wording).
- Full workspace suite (1066 tests total across all crates, 0 failed),
  `cargo fmt --all`, `cargo build -p ugaris-server` (zero warnings), and a
  10s boot smoke (ticks advancing, NPC driver messages processed, no
  panics) all pass.

## Ralph Loop - `update_char` Stat Recomputation (Iteration 16)

- Ported C `update_char(cn)` (`src/system/create.c:1710`, plus its
  `armor_skill_req`/`armor_skill_bonus` helpers at `create.c:1661-1708`)
  as `World::update_character(cn)` /
  `recompute_character_values` in
  `crates/ugaris-core/src/world/character_values.rs`. This closes the
  first (and largest) of the four slices the P1 todo note called for, and
  actually covers all four: (1) worn/spell item modifier sum from
  `character.inventory[0..30]` with the seyan (`WARRIOR|MAGE`, 72.5%) vs.
  single-class (50%) cap on powers/attributes/skills, the separate
  non-warrior bless-item cap (50%), and `IF_BEYONDMAXMOD` items bypassing
  the cap entirely via a `beyond[]` accumulator; (2) driver-spell flags
  reused via the existing `refresh_driver_spell_flags` (called first, so
  `CF_NONOMAGIC` is up to date for the "no-magic creatures get no item
  bonus" branch, matching C's evaluation order); (3) Armor/Weapon bonuses
  from Body Control (`*5`/`/4` plus the bare-handed-player Weapon bonus)
  or, when Body Control is unraised, `get_spell_average(cn) * 17.5`, plus
  `armor_skill_bonus`'s body(50)/head(20)/legs(15)/arms(15)-weighted
  requirement-vs-raised comparison; (4) HP/endurance/mana current-value
  clamp to the recomputed max, exactly like C's three `if (hp > value*POWERSCALE)` guards.
- Also ported: the base-attribute averaging from C's `skill[]` table
  (`skill.c:27`), hardcoded as `skill_base_attributes` since Rust has no
  existing skill-table representation (only `raise_cost`-style logic in
  `world/skills.rs`, which doesn't carry the C `base1/base2/base3`
  triples) - all 43 entries transcribed from the C source, including the
  `-1,-1,-1` (no base) entries for powers/attributes/Armor/Weapon/Light/
  Cold/Profession; the `value[1][n]==0 && n>=V_PULSE -> 0` skip so
  unraised skills get no item bonus; the `V_DEMON`
  (`min(value[0],value[1])`) and `V_COLD` (`value[0]=value[1]=mod[n]`)
  special cases; Speed Skill (`+= value[0][SpeedSkill]/2`), Athlete
  (`+= prof*3`), Thief (Stealth `+= prof*2` in thief mode else `prof`,
  Percept `+= prof/2`), and Demon-profession (`+= prof` to Hand/Dagger/
  Staff/Sword/TwoHand/Attack/Parry/Tactics/Immunity/Flash/Fireball/Freeze,
  gated on each skill being raised and `CF_DEMON`) bonuses; and the day
  (`P_LIGHT`, hour 6-18)/night (`P_DARK`, hour outside 6-18)/clan
  (`P_CLAN`) attribute (Wis..Str) profession bonuses.
- `World::update_character` wraps the pure `recompute_character_values`
  with the map-touching bits it can't do standalone: reads
  `self.date.hour` and the character's tile `MapFlags::CLAN` for the two
  context inputs the pure function needs, then calls the pre-existing
  `refresh_character_light_after_value_change` when Light changed (same
  helper `world/light.rs` already exposed for other call sites) so map
  light re-emission matches C's `remove_char_light`/`add_char_light`
  dance around the value recompute.
- Wired the very first real call site: `inventory_swap_slot`
  (`crates/ugaris-server/src/inventory.rs`) now calls
  `world.update_character(character_id)` after a worn-slot swap
  (`slot < 12`), matching C `swap()`'s `if (pos < 12) update_char(cn);`
  (`src/system/do.c:1294`). Before this iteration, equipping/unequipping
  gear via `CL_SWAP` had **zero** effect on character stats in Rust - this
  was a real, previously-undocumented gameplay gap, not just a missing
  formula port.
- 11 focused tests added in
  `crates/ugaris-core/src/world/tests/character_values.rs`: wearing/
  removing an item's modifier, the 50%/72.5% single-class/seyan caps,
  `IF_BEYONDMAXMOD` bypass, the unraised-skill-gets-no-bonus rule, Speed
  Skill + Athlete profession stacking, the spell-average Armor bonus
  (Body Control unraised), Body Control's Armor/Weapon bonuses for a
  bare-handed player, the HP current-value clamp, and the unconditional
  `CF_UPDATE` flag set. All matched hand-computed expected values on the
  first run (no formula fixes needed after transcription).
- Documented, deliberate gaps (not silently dropped - noted in both the
  function's doc comment and the todo/ledger entries): `ch.ef[]`
  area-effect light contributions have no Rust equivalent (Rust effects
  aren't attached to characters as a 4-slot list the way C's `ch.ef[]`
  is); the `P_CLAN` night-in-catacombs bonus only checks the `MF_CLAN` map
  tile flag because `World` has no current-area id to replicate C's
  `areaID == 13` half of the OR; sprite reselection (demon suit sets,
  weapon-in-hand sprite offsets) and the `player_reset_map_cache` call on
  infravision toggle are display-only side effects intentionally left for
  a future client-sync-focused pass.
- REMAINING (iteration 16 snapshot, superseded below): `World::update_character`
  was not yet called from spell install/expiry, level up, login, or death
  respawn.
- Full workspace suite (1077 tests total across all crates, 0 failed),
  `cargo fmt --all`, `cargo build -p ugaris-server` (zero warnings), and a
  10s boot smoke (`entering Rust game loop`, ticks advancing, NPC driver
  messages processed, no panics) all pass.

### Iteration 17 follow-up: spell install/expiry, `raise_skill`, login, death respawn

- Migrated every remaining `world/spells.rs` install/expire call site from
  the old `apply_item_modifier_deltas`/`refresh_driver_spell_flags` pair to
  `World::update_character`, each matched against its exact C call site
  (function + line, all confirmed by reading the C source, not guessed):
  - `install_bless_spell` <- `bless_someone`/`bless_self`
    (`act.c:1117`/`act.c:1158`, `update_char` called twice: once after
    destroying a pre-existing bless item, once after installing the new
    one - both the old-item-removal and new-item-install deltas replaced).
  - `install_bonus_spell` (armor/weapon/hp/mana spells) <- `add_bonus_spell`
    (`drvlib.c:2646`).
  - `install_beyond_potion_spell` <- `add_potion_spell`
    (`module/alchemy.c:1007`).
  - `install_speed_spell` (shared by warcry/freeze) <- `warcry_someone`
    (`act.c:1324`) / `freeze_someone` (`act.c:1522`).
  - `install_curse_spell` <- `ice_curse` (`act.c:1470`): C calls
    `update_char(co)` **unconditionally after both branches** (existing-item
    stack-and-cap and new-item-create), not just the new-item path the old
    Rust code touched; restructured both branches to fall through to one
    `update_character` call at the end, removing the manual
    `add_character_value_delta` loop from the existing-item branch entirely
    (the item's already-updated `modifier_value` is picked up by the
    recompute automatically).
  - `install_firering_spell` <- `act_firering` (`act.c:882`): C does **not**
    call `update_char` here (the firering item carries no modifiers) - kept
    the call anyway since `update_character` is a strict superset of the
    `refresh_driver_spell_flags` call it replaces and is a no-op for
    firering specifically (documented in a code comment), consolidating to
    one recompute function everywhere in the file.
  - `install_timed_identity_spell` (infravision/oxygen/underwater-talk) <-
    `add_spell` (`tool.c:1683`).
  - `remove_driver_spells` (curse-cleanse/oxygen-end helpers) <- mirrors
    the `remove_poison`/`remove_all_poison` "only recompute if something was
    actually removed" pattern (no single matching C function name; call
    sites are area-specific item drivers).
  - `poison_character` <- `poison_someone` (`system/poison.c:61`).
  - `remove_poison_by_driver` (backs `remove_poison`/`remove_all_poison`) <-
    `poison.c:128`/`poison.c:148`. **Bug fix**: this previously only set
    `CF_ITEMS|CF_UPDATE` and never actually recomputed `value[0]`, so curing
    poison never restored the HP the poison's permanent modifier had eaten.
  - `poison_callback_from_timer`'s `tick == 0` branch <- `poison_callback`
    (`poison.c:102`). **Bug fix**: the per-10-ticks HP-modifier decrement
    (`it[in].mod_value[0]--`) only set a flag before, never recomputing
    `value[0][V_HP]`, so poison's escalating permanent HP loss was inert.
  - `schedule_existing_spell_timers` (bulk startup/load recompute, not yet
    wired to any live call site - reserved for a future
    save/load-on-login pass): switched its per-character
    `refresh_driver_spell_flags` loop to a full `update_character` per
    character, so a character loaded with spell items already in inventory
    gets correct `value[0]` totals immediately, matching what a live
    `update_char` call would already have applied.
  - `remove_spell_from_timer` <- `remove_spell` (`tool.c:1591`): moved the
    `update_char(cn)` call to the exact same position as C - right after
    the item is removed from inventory, *before* the freeze-duration
    rescale, which reads the character's now-recomputed Speed value (this
    requires re-fetching the character reference after the recompute, since
    it needs `&mut World` while the freeze-rescale code needs the character
    mutably too).
  - Deleted the now fully-superseded `apply_item_modifier_deltas`/
    `add_character_value_delta` helpers from `character_values.rs`
    entirely (zero remaining callers after the migration).
- Wired `World::raise_skill` (`world/skills.rs`) to call
  `update_character` after `raise_value` bumps `value[1]`, matching C
  `raise_value` (`system/skill.c:256`) - so raising e.g. Body Control now
  immediately re-applies its derived Armor/Weapon bonus, not just the raw
  raised number.
- Wired player-death respawn: `World::die_character` (`world/death.rs`)
  now calls `update_character` right after `place_character_on_map`
  succeeds, matching C `die_char` (`death.c:807`, the `update_char(cn)`
  call that runs only when `transfer_to_restarea` returns 0/success -
  Rust's `place_character_on_map` return value plays the same role as the
  cross-area-handoff-vs-same-area-success branch C guards on).
- Wired login: `ugaris-server/src/snapshots.rs::apply_character_snapshot`
  (DB-loaded existing characters) and the template-instantiation/
  hard-coded-scaffold path in `main.rs`'s login handler both now call
  `world.update_character(character_id)` once the character and its
  items are fully in the world, matching C `login_ok`
  (`database_character/database_character.c:1512`, confirmed by reading
  the function: `update_char(cn)` runs once equipment/profession/karma
  validation is done, right before the "newbie -> set hp/end/mana to max"
  branch that follows it - not ported here since Rust's two login paths
  already set starting resources explicitly for new characters).
- Test fallout (all fixed by adding realistic `values[1]` (raised base)
  baselines to fixtures, never by weakening an assertion): the previous
  ad-hoc delta helpers never enforced C's floor clamp
  (`n <= V_STR && value[0][n] < 0 -> 0`, `create.c:1863-1865`) or the
  50%-of-raised-base cap on item/bless modifiers (`create.c:1815-1819`),
  so several fixtures that poked `value[0]` directly without a matching
  `value[1]` baseline produced different (and, per a from-scratch C
  source re-read, *incorrect*) numbers than before once the real
  recompute ran. Fixed in `world/tests/spells.rs` (curse-stack and
  ice-demon-freeze tests now assert the correct 0-floor-clamped values and
  the ice-demon test gained an actual Cold-modifier item on the target
  since `V_COLD` is entirely item-driven per `create.c:1795-1798` and was
  being silently reset to 0 by the newly-live recompute; the bless test
  gained a raised `values[1]` baseline so the bless cap has something to
  apply against; two poison tests gained a large `values[1][V_HP]`
  baseline so poison's tiny HP modifier doesn't itself clamp `hp` down via
  the new max-HP guard), `world/tests/text.rs` (warcry test, same HP-clamp
  fix), `world/tests/hurt.rs` (poison-lifeshield test, same HP-clamp fix),
  `world/tests/death.rs` (player-death-respawn test needed `values[1]`
  Hp/Endurance/Mana baselines matching its pre-existing `values[0]`
  pokes), and `world/tests/skills.rs` (the "effective stays above bare"
  raise test now uses a real worn item with a Sword modifier instead of a
  bare `value[0]` poke, since only real items survive a full recompute).
- STILL REMAINING: level-up (the "Experience/level-up side effects" P1
  todo item is still unported, so there is no level-up call site to wire
  yet); item-driver-level raise/scroll/potion/enchant paths
  (`item_driver/scrolls.rs::raise_value_exp`, `item_driver/potions.rs`)
  still don't call `update_character` since item drivers operate on
  `&mut Character` only, with no `&mut World` access - wiring those needs
  either threading `&mut World` through the item-driver dispatch or having
  the `World`-level caller recompute after applying the driver's outcome;
  left as a distinct follow-up slice. The documented gaps in the
  recompute algorithm itself (`ch.ef[]` area-effect light, `P_CLAN`/
  `areaID == 13`, sprite reselection) are unchanged.
- Full workspace suite (1462 tests total across all crates, 0 failed),
  `cargo fmt --all`, `cargo build -p ugaris-server` (zero warnings), and a
  10s boot smoke (`entering Rust game loop`, ticks advancing, NPC driver
  messages processed, no panics) all pass.

## Ralph Loop - `update_char` Stat Recomputation (Iteration 18)

- Closed the "item-driver-level raise" sub-gap left open at the end of
  iteration 17, choosing the second option that note described (the
  `World`-level caller recomputes after applying the driver's outcome)
  rather than threading `&mut World` through the entire item-driver
  dispatch, since the latter would be a much larger, riskier refactor
  touching dozens of driver files across `item_driver/*.rs`.
- C `raise_value_exp` (`src/system/skill.c:315-377`) calls
  `update_char(cn)` immediately after bumping `value[1][v]` for every
  successful raise. The stat scroll driver (`base.c:6031` `IDR_STATSCROLL`,
  Rust `item_driver/scrolls.rs::stat_scroll_driver`) loops calling
  `raise_value_exp` per scroll charge purely on `&mut Character` (no
  `&mut World` access) and returns `ItemDriverOutcome::StatScrollUsed`.
  `World::apply_item_driver_outcome` (`world/item_outcomes.rs`) now
  matches that outcome and calls `self.update_character(character_id)`
  once after the loop completes - equivalent to C's per-raise calls since
  `update_char` is idempotent on the final `value[1]` state (verified by
  reading the full function body: it always recomputes from the current
  `value[1]`/equipment/spell state, never accumulates deltas).
- Test: `world/tests/item_outcomes.rs::stat_scroll_use_triggers_update_character_recompute`
  raises Body Control (skill index 23) by 1 via a stat scroll through
  `World::execute_item_driver_request` and asserts the derived Armor bonus
  (`body_control * 5`, `create.c:1710`) immediately reflects the raised
  value (55, not the stale pre-raise 50), proving the recompute actually
  fires from this call site rather than only mutating `values[1]`.
- Verified the other two drivers the iteration-17 note grouped with
  scrolls are non-issues after reading their C sources directly (not
  oversights, no wiring needed): `enchant_item`/`anti_enchant_item`
  (`src/module/base.c:3543`/`5781`, backing `item_driver/orbs.rs`'s
  `enchant_driver`/`anti_enchant_driver`) mutate only the *target item's*
  `mod_index`/`mod_value` array and never call `update_char` in C either -
  the recompute only happens later when the enhanced item is worn/re-worn,
  which is already wired via `inventory_swap_slot`. `item_driver/potions.rs`'s
  drivers (`potion_driver`/`special_potion_driver`/`beyond_potion_driver`)
  only heal/restore current `hp`/`mana`/`endurance` directly or install
  timed spells through `install_beyond_potion_spell` (already calling
  `update_character` since iteration 17); none of them touch `values[]`
  directly, so no additional wiring is needed there.
- STILL REMAINING: level-up recompute (the "Experience/level-up side
  effects" P1 todo item is still unported, so there is no level-up call
  site to wire yet). `raise_value_exp` also calls C `check_levelup(cn)`
  before bumping the value, which stays unported until that task lands.
  The `src/area/18/bones.c:317-431` and `src/area/37/arkhata.c:800-801`
  call sites of `raise_value_exp` (used for area-specific skill trainers,
  distinct from the generic stat scroll item) are not yet ported to Rust
  at all - `raise_value_exp` has no usage outside `item_driver/scrolls.rs`
  in the Rust tree today, so those area drivers remain fully out of scope
  until someone ports `area18_bones.rs`/`area37_arkhata.rs`'s trainer NPC
  interactions. The documented gaps in the recompute algorithm itself
  (`ch.ef[]` area-effect light, `P_CLAN`/`areaID == 13`, sprite
  reselection) are unchanged from previous iterations.
- Full workspace suite (1078 + 9 + 3 + 33 + 340 = 1463 tests across all
  crates, 0 failed), `cargo fmt --all`, `cargo build -p ugaris-server`
  (zero warnings), and a 10s boot smoke (`entering Rust game loop`, ticks
  advancing, NPC driver messages processed, no panics) all pass.

## Ralph Loop - Experience/Level-Up Side Effects (Iteration 19)

- Consolidated `exp2level`/`level2exp`/`level_value` (`src/system/
  tool.c:1272-1283`) into the new canonical `crates/ugaris-core/src/
  world/exp.rs`. These three pure formulas had independently accreted in
  three separate spots: `ugaris-server/src/spawns.rs`
  (`legacy_level2exp`/`legacy_exp_to_level`, used by LQ raise/reset),
  `ugaris-server/src/area_apply.rs` (`legacy_level_value`/
  `legacy_level_exp`, used by random shrines and quest rewards), and
  `ugaris-core/src/item_driver/helpers.rs` (`legacy_level_value`, used by
  `food.rs`/`area17_two.rs`/`area29_brannington.rs`). Deleted the two
  server-crate duplicates outright and repointed every call site at the
  core functions (re-exported through `main.rs`'s `ugaris_core::world`
  import so `use super::*` picks them up in `spawns.rs`/`area_apply.rs`
  without new per-file imports); turned the `item_driver/helpers.rs` copy
  into a one-line delegate to `crate::world::level_value` since dozens of
  `item_driver/*.rs` files already call it by that name.
- Ported `World::check_levelup(character_id)` (`tool.c:1318-1356`) as a
  new `impl World` block in `world/exp.rs`. Read the full C function
  before writing anything: it loops `while
  exp2level(max(exp,exp_used)) > level`, and per iteration does level
  increment, `set_sector`, a "Thou gained a level!" `log_char`, hardcore
  save-reset vs. non-hardcore save-grant (capped at 10) with two feedback
  lines, a level-20 profession unlock, a level-10-multiple server chat
  broadcast, an achievement check, `reset_name`, and a debug `dlog` call.
  Ported the first five (level, save handling, profession unlock, dirty
  sector, level-up text) exactly; the last four have no Rust primitive to
  hook into yet (no all-sessions broadcast helper, no level-keyed
  achievement system, no name-color-by-level refresh, no debug log sink)
  and are called out explicitly in the function's doc comment as
  documented gaps rather than silently dropped.
- Notably, `check_levelup` does **not** refill HP/endurance/mana on
  level-up and does not call `update_char` itself in C (verified by
  reading the function body directly) - the existing `PORTING_TODO.md`
  task description's mention of "HP/end/mana refill on level" and
  "`update_char`" turned out not to correspond to anything in the actual
  C source for this function, so no such gap exists to close here.
- Wired `check_levelup` into the two `give_exp` call sites that reach a
  live player today: killer experience (`world/death.rs`'s
  `KillExpAward` queue, drained in `main.rs`'s tick loop) and the
  `/god exp` admin command. Both route through
  `ugaris-server/src/commands_admin.rs::give_exp_with_runtime_modifiers`,
  which changed signature from `(&mut Character, ...)` to
  `(&mut World, CharacterId, ...)` so it can call
  `world.check_levelup(character_id)` after updating `exp`, gated on
  `!character.flags.contains(NOLEVEL)` exactly like C's
  `if (!(ch[cn].flags & CF_NOLEVEL)) check_levelup(cn);` tail call. This
  function stays in the server crate (not `ugaris-core`) because its two
  multipliers, `exp_modifier`/`hardcore_exp_bonus`, are live-tunable
  `ServerRuntime` fields adjustable via `/setexpmod`/
  `/sethardcoreexpbonus`, not `ugaris-core`'s static `GameSettings`
  defaults.
- Tests: 13 new focused tests. `world/tests/exp.rs` (11 tests) covers the
  `exp2level`/`level2exp`/`level_value` formulas directly, single- and
  multi-level-up-in-one-call, hardcore save reset vs. non-hardcore save
  grant/cap, the level-20 profession unlock (both granting it and not
  overwriting an already-chosen one), the `max(exp, exp_used)` source,
  the noop/false-return case, and the unknown-character case. Extended
  `ugaris-server/src/tests/commands_admin.rs::
  god_exp_command_uses_runtime_exp_modifiers_and_legacy_gates` with two
  new assertions: the hardcore target actually levels from 1 to 3 (130
  exp with the test's 2x/1.5x runtime modifiers crosses `level2exp(3) ==
  81`), and the `NOLEVEL`-flagged character does not level up despite its
  capped exp sitting one point below the next threshold.
- STILL REMAINING (documented in the todo note, not silently dropped):
  stat-scroll `raise_value_exp` (`item_driver/scrolls.rs`) still doesn't
  call `check_levelup` before its raise, even though C does - the driver
  only has `&mut Character`, so this needs the same outcome-based pattern
  iteration 18 used for wiring `update_character` into
  `world/item_outcomes.rs`. The two area-specific `raise_value_exp` call
  sites (`src/area/18/bones.c:317-431`, `src/area/37/arkhata.c:800-801`)
  remain fully unported (no Rust usage of `raise_value_exp` exists
  outside `scrolls.rs`). Roughly seven other direct-mutation exp grant
  call sites still bypass `give_exp`/`check_levelup` entirely: four
  random-shrine reward sites in `area_apply.rs`, ~4 inline quest/area
  reward grants in `main.rs`, `item_driver/food.rs`'s food exp bonus,
  `player.rs:2921`, and the `/milexp` admin command. Porting those
  properly needs a `World`-level `give_exp` entry point reachable from
  `ugaris-core` item drivers too (today the exp/hardcore multipliers live
  only in the server-crate wrapper) - a larger follow-up slice, not
  attempted this iteration to keep the change self-contained and fully
  tested.
- Full workspace suite (1089 + 9 + 3 + 33 + 340 = 1474 tests across all
  crates, 0 failed), `cargo fmt --all`, `cargo build -p ugaris-server`
  (zero warnings), and a 10s boot smoke (`entering Rust game loop`, ticks
  advancing, NPC driver messages processed including live kill-exp
  `check_levelup` calls, no panics) all pass.

## Ralph Loop - Experience/Level-Up Side Effects (Iteration 20)

- Closed the "STILL REMAINING" gap from iteration 19 above: C
  `raise_value_exp` (`src/system/skill.c:315-361`) calls
  `check_levelup(cn)` right after adding the raise cost to
  `exp`/`exp_used` (before bumping `value[1][v]`), once per successful
  raise. The stat scroll driver (`item_driver/scrolls.rs`, `base.c:6031`
  `IDR_STATSCROLL`) loops calling `raise_value_exp` per scroll charge on
  `&mut Character` only (no `&mut World` access), so - following the same
  outcome-based pattern iteration 18 used to wire `update_character` into
  this same call site - `World`'s `ItemDriverOutcome::StatScrollUsed`
  handler (`world/item_outcomes.rs::apply_item_driver_outcome`) now calls
  `self.check_levelup(character_id)` immediately before
  `self.update_character(character_id)`, matching C's per-charge
  `check_levelup`/`update_char` ordering. A single batched call after the
  loop completes is equivalent to per-charge calls because both
  `check_levelup` (loops until `exp2level(exp) <= level`) and
  `update_character` are idempotent/monotonic on the final `exp`/
  `value[1]` state; documented on the `ItemDriverOutcome::StatScrollUsed`
  doc comment (`item_driver/types.rs`) including why the `V_PROFESSION`
  level-20-unlock edge case cannot diverge from per-charge ordering -
  raising `V_PROFESSION` itself requires `value[1][V_PROFESSION]` already
  non-zero (checked earlier in `raise_value_exp`), which is exactly the
  condition under which `check_levelup`'s unlock (`if
  value[1][V_PROFESSION] == 0`) is already a no-op.
- Test: new
  `world/tests/item_outcomes.rs::stat_scroll_use_triggers_check_levelup`
  raises a cheap raisable value (Pulse, index 11) from a low bare value
  via a stat scroll, computes the expected exp grant by hand from
  `raise_cost`, and asserts the character's `level` field actually
  increments (1 -> 2), not just `exp` - the existing
  `stat_scroll_use_triggers_update_character_recompute` test only ever
  exercised the `update_character` half of this outcome.
- Updated both `PORTING_TODO.md` P1 entries that referenced this gap
  ("`update_char` stat recomputation"'s STILL REMAINING note and
  "Experience/level-up side effects"'s REMAINING note) to reflect it is
  now closed; the remaining gaps in both tasks are unchanged (unported
  `bones.c`/`arkhata.c` `raise_value_exp` call sites, the level-10 "Grats"
  broadcast, `achievement_check_level`, `reset_name`, and the ~7
  direct-mutation exp-grant call sites that bypass `give_exp`/
  `check_levelup` entirely, which all still need a `World`-level
  `give_exp` entry point as a larger follow-up slice).
- Full workspace suite (1090 + 9 + 3 + 33 + 340 = 1475 tests across all
  crates, 0 failed), `cargo fmt --all`, and `cargo build -p ugaris-server`
  (zero warnings) all pass. This change does not touch the runtime loop,
  login, map sync, or protocol, so no boot smoke was required.

## Ralph Loop - Iteration 21: `update_char` Sprite Reselection

- Resumed the `update_char` stat recomputation P1 task (`- [~]`) and
  closed one of its three remaining documented gaps: sprite reselection.
  Ported C `create.c:1969-2120` as `recompute_character_sprite`
  (`crates/ugaris-core/src/world/character_values.rs`): the
  `CF_PLAYER && (!CF_GOD || sprite-in-admin-exempt-range)` gate; the
  weapon-in-hand `off` selection (0 nothing / 1 one-hand weapon / 2
  two-handed weapon / 3 torch only / 4 torch + one-hand weapon, from
  `WN_RHAND`/`WN_LHAND` and the raw left-hand `drdata[0]` "lit" byte); the
  full 12-arm `CF_WARRIOR|CF_MAGE|CF_MALE|CF_FEMALE|CF_ARCH` `sbase`
  switch (60/65/75/70/85/80/95/90/105/100/115/110); and the six-slot
  (`head`/`arms`/`legs`/`body`/`cloak`/`feet`) `IID_DEMONSKIN1/2/3`
  full-suit override to sprite 27/157/39 with `off` forced to 0. Added
  the three `IID_DEMONSKIN*` constants (`MAKE_ITEMID(DEV_ID_DB, 0xA8..0xAA)`),
  which did not exist anywhere in the Rust tree before. `World::update_character`
  now calls this right after the value recompute and, on an actual sprite
  change, calls `mark_dirty_sector` on the character's tile - the Rust
  equivalent of C's `set_sector(ch[cn].x, ch[cn].y)` call in the same
  branch. `reset_name(cn)` (clearing a cached colored-name buffer on a
  demon-sprite transition) is documented as an intentional no-op: Rust
  has no server-side name-color cache to invalidate, confirmed by
  grepping for one and finding none.
- Added 5 focused tests to `world/tests/character_values.rs`: unarmed
  class/gender base sprite selection, the two-handed-weapon `off`
  offset, the full demon-skin-1-suit override (regardless of class or
  weapon), the god-admin-sprite exemption (a `CF_GOD` character keeps an
  out-of-range custom sprite untouched), and dirty-sector marking on an
  actual sprite change (using `world.skip_x_sector` the same way the
  existing light dirty-sector tests do).
- While writing the two-handed-weapon test, found and fixed a real
  latent bug in the already-shipped (iteration 16) Body Control
  bare-handed Weapon-bonus check in the same file:
  `item.flags.contains(ItemFlags::WEAPON)` is wrong, because `IF_WEAPON`
  in C is a composite of several *single-bit* weapon-class flags
  (`IF_AXE|IF_DAGGER|IF_HAND|IF_STAFF|IF_SWORD|IF_TWOHAND`) and C's own
  check is `flags & IF_WEAPON` (true if *any* one bit is set) - but
  bitflags' `.contains()` requires *every* bit in the argument to be set
  simultaneously, which no real single-category weapon item ever has, so
  the bare-handed Weapon bonus was silently never suppressed by an
  actually-equipped weapon. Fixed both this call site and the new
  sprite-recompute one to use `.intersects()` instead, matching the
  already-correct pattern in `world/npc_fight.rs:2381`. The existing
  `body_control_boosts_armor_and_weapon_for_bare_handed_player` test never
  caught this because it only exercised the empty-right-hand path; added
  `body_control_bare_handed_bonus_is_suppressed_by_a_real_weapon_in_hand`
  to lock in the fix.
- `PORTING_TODO.md`'s `update_char` task entry updated with an iteration
  21 progress note; the task stays `- [~]` since the `ch.ef[]`
  area-effect light and `P_CLAN`/`areaID == 13` gaps remain (the latter
  is mechanically simple - `area_id` is a real, already-threaded
  per-server-instance value - but touches roughly 32 existing
  `update_character` call sites, deferred as its own slice rather than
  batched into this one).
- Full workspace suite (1096 + 9 + 3 + 33 + 340 = 1481 tests across all
  crates, 0 failed), `cargo fmt --all`, and `cargo build -p ugaris-server`
  (zero warnings) all pass. Boot-smoked
  `target/debug/ugaris-server --bind-addr 127.0.0.1:5556` for 10+ seconds
  (this change touches character recompute, which runs on login/equip):
  "entering Rust game loop" appeared, ticked cleanly to tick 31, no
  panics.

## Ralph Loop - Experience/Level-Up Side Effects (Iteration 22)

- Resumed the P1 "Experience/level-up side effects" task (`- [~]`) and
  closed the infrastructure gap its iteration-19/20 notes called out: C
  `give_exp(cn, val)` (`src/system/tool.c:1371-1423`) itself had no Rust
  equivalent - only the server-crate wrapper
  `commands_admin.rs::give_exp_with_runtime_modifiers` duplicated its
  logic, unusable from `ugaris-core` item drivers (which only ever get
  `&mut World`, never `&mut ServerRuntime`).
- Ported the full C `give_exp` algorithm as `World::give_exp` in the new
  canonical spot, `crates/ugaris-core/src/world/exp.rs`: the
  `CF_HARDCORE`/global `exp_modifier` multiplier chain, the
  `CF_NOEXP`/area-21 no-op gate, the `CF_NOLEVEL` exp-band clamp (floors
  at the current level's `level2exp` and ceils at `next_level2exp - 1`),
  the i32-range clamp (C's `long long` -> `int` cast, INT_MAX/INT_MIN),
  the "prevent an unexpected decrease from a positive grant" guard, and
  the `check_levelup` tail call gated on `!CF_NOLEVEL`. The trailing C
  `macro_track_exp_gain(cn)` anti-macro-daemon hook is documented as an
  intentional gap on the function doc comment - no macro-daemon system
  exists anywhere in the Rust tree to track activity for.
- **Single source of truth for the exp multipliers.** `World` already
  carried a `settings: GameSettings` field with `exp_modifier`/
  `hardcore_exp_bonus` (defaulting to 1.0, matching C's defaults), but the
  server crate never read or wrote it - the live-tunable `/setexpmod`/
  `/sethardcoreexpbonus` admin commands instead mutated two duplicate
  fields on `ServerRuntime`. For `World::give_exp` to see the live-tunable
  values without silently diverging from the server-crate call sites,
  removed the `ServerRuntime::exp_modifier`/`hardcore_exp_bonus` fields
  entirely and repointed both admin commands
  (`crates/ugaris-server/src/commands_admin.rs`) to mutate
  `world.settings.exp_modifier`/`hardcore_exp_bonus` directly. Both
  commands already had `world: &mut World` in scope
  (`apply_admin_character_command`'s signature), so this was a
  same-function field-path swap, not a signature change.
  `commands_admin.rs::give_exp_with_runtime_modifiers` is now a two-line
  wrapper around `world.give_exp` (kept only so call sites still read like
  their C `give_exp(cn, val)` counterparts); its two existing callers
  (killer-exp in `main.rs`'s tick loop, `/god exp`) needed only their now-
  unused `&runtime` argument dropped.
- **Wired two more direct-mutation exp sites named in the iteration 19/20
  "STILL REMAINING" list through `give_exp`:**
  - `/milexp` (`commands_admin.rs`, C `cmd_milexp`/
    `give_military_pts_no_npc` `command.c:3014`/`tool.c:3281-3299`): C's
    `give_military_pts_no_npc(co, val, 1)` grants a **fixed** `1` exp via
    `give_exp` (independent of the typed `val` amount, which instead goes
    to `military_points`) - the Rust command previously did a raw
    `target.exp = target.exp.saturating_add(1)`, bypassing every
    multiplier/gate/level-up. Now calls `world.give_exp(target_id, 1,
    area_id)`. While reading the C source line-by-line for this fix, also
    found and fixed a real latent bug: the hardcore `military_points`
    multiplier was hardcoded to `1.10` in Rust instead of reading the
    already-live-tunable `runtime.hardcore_military_exp_bonus` field
    (default 1.10 too, so the bug was invisible unless an admin actually
    ran `/sethardcoremilexpbonus`).
  - The demon-shrine book driver (`ugaris-core/src/player.rs::
    touch_demonshrine`, C `demonshrine_driver` `base.c:3189-3235`, the
    `player.rs:2921` site the iteration-19/20 notes named): C calls
    `update_char(cn)` (for the Demon value bump) then `give_exp(cn, ...)`;
    the previous Rust port did neither, mutating `character.exp` raw and
    never recomputing derived stats. Since `touch_demonshrine` only has
    `&mut Character` (it is a `PlayerData` method, not a `World` one), it
    now returns `exp_added` unapplied (via the existing
    `DemonShrineResult::Learned { exp_added }` variant) and its caller -
    the `ItemDriverOutcome::DemonShrine` arm in `ugaris-server/src/
    main.rs`'s tick loop - calls `World::update_character` then
    `World::give_exp` in that order, matching C exactly.
  - `item_driver/food.rs`'s lollipop exp bonus (C `lollipop`
    `base.c:3242-3261`, calling `give_exp`) had the same shape of bug: the
    bare-`&mut Character` driver mutated `character.exp` raw. It now
    leaves `character.exp` untouched and only returns the base amount via
    the existing `ItemDriverOutcome::LollipopLicked.exp_added` field; a
    new arm added to `World::apply_item_driver_outcome`
    (`world/item_outcomes.rs`) calls `self.give_exp` with it, following
    the exact same outcome-based pattern iteration 18/20 established for
    `ItemDriverOutcome::StatScrollUsed`.
- Tests: 8 new cases in `world/tests/exp.rs`
  (`give_exp_applies_global_modifier_and_marks_update`,
  `give_exp_applies_hardcore_bonus_before_global_modifier`,
  `give_exp_is_a_noop_for_noexp_characters`,
  `give_exp_is_a_noop_in_area_21`,
  `give_exp_caps_nolevel_characters_at_the_next_level_threshold`,
  `give_exp_floors_nolevel_characters_back_to_their_level_band_on_negative_grants`,
  `give_exp_prevents_unexpected_decrease_from_a_positive_grant`,
  `give_exp_triggers_check_levelup_unless_nolevel`); new
  `world/tests/item_outcomes.rs::lollipop_lick_grants_exp_through_give_exp_not_a_raw_mutation`;
  new
  `tests/commands_admin.rs::milexp_routes_its_fixed_one_exp_through_give_exp_and_honors_runtime_military_bonus`;
  updated the two `god_setexpmod_updates_runtime_with_legacy_feedback`-
  style tests and `god_exp_command_uses_runtime_exp_modifiers_and_legacy_gates`
  to assert against `world.settings.*` instead of the now-removed
  `runtime.*` fields; renamed and updated
  `player::tests::demonshrine_touch_updates_value_and_blocks_repeats` to
  assert the new caller-applies-the-exp contract; updated
  `item_driver/tests/food.rs`'s lollipop assertion the same way.
- **STILL REMAINING** (unchanged in shape from the iteration 19/20 notes,
  four sites left instead of seven): `area_apply.rs`'s four random-shrine
  reward sites and `main.rs`'s ~4 inline quest/area reward grants still do
  raw `character.exp +=` instead of `world.give_exp(...)`. Every one of
  them already has `&mut World` in scope (they are server-crate,
  non-item-driver code), so closing this task fully is now a mechanical
  swap-in at each site plus a test per site - no further infrastructure
  work is needed, unlike the item-driver sites this iteration closed. The
  level-10 "Grats" broadcast, `achievement_check_level`, and `reset_name`
  gaps inside `check_levelup` itself (documented since iteration 19)
  remain unchanged.
- Full workspace suite (1105 + 9 + 3 + 33 + 341 = 1491 tests across all
  crates, 0 failed), `cargo fmt --all`, and `cargo build -p ugaris-server`
  (zero warnings) all pass. Boot-smoked
  `target/debug/ugaris-server --bind-addr 127.0.0.1:5556` for 10 seconds
  (this change touches item-driver dispatch and an admin-command-reachable
  runtime field): "entering Rust game loop" appeared, ticked cleanly with
  no panics.

### Iteration 23: closed the `area_apply.rs` half of the exp-side-effects "STILL REMAINING" note

- C `shrine_edge`/`shrine_vitality`/`shrine_braveness`/`shrine_continuity`
  (`src/area/14/random.c:2028/2078/2176/2126`) and the area-2 zombie
  shrine's experience roll (`src/area/2/area2.c:259/325/390`) all grant
  exp via `give_exp(cn, ...)`, not a raw `ch[cn].exp += ...`; Rust's
  `area_apply.rs` counterparts bypassed `World::give_exp` entirely (no
  hardcore/`exp_modifier` multiplier, no `CF_NOEXP`/`CF_NOLEVEL` gates, no
  `check_levelup`).
- The four `apply_random_shrine_*` functions only ever had `&mut
  Character` in scope (not `&mut World`), so - following the
  outcome-based pattern established in earlier iterations for
  `ItemDriverOutcome::StatScrollUsed`/`LollipopLicked` - they now stop
  mutating `character.exp` themselves and just report the amount through
  their existing `Used { exp, .. }` result variants; the four
  `RandomShrineKind` match arms in `main.rs` call `world.give_exp(...)`
  (and, for vitality, also `world.update_character(...)` matching C's
  trailing `update_char(cn)`) once the `&mut Character` borrow returned
  by `apply_random_shrine_*` has ended.
- `apply_zombie_shrine` already took `&mut World` directly, so its fix was
  a straight inline swap: `character.exp = character.exp.saturating_add(exp_added)`
  became `world.give_exp(character_id, i64::from(exp_added), area_id)`,
  requiring a new `area_id: u32` parameter threaded from `main.rs`'s
  `args.area_id`.
- Updated the pre-existing unit tests for the four `apply_random_shrine_*`
  functions (`crates/ugaris-server/src/tests/area_apply.rs`,
  `tests/item_apply.rs`, `tests/commands_admin.rs`) to stop asserting
  `character.exp` mutation (documented with inline comments explaining the
  new caller-applies-the-grant contract) and added a new test,
  `apply_zombie_shrine_experience_routes_through_give_exp_and_honors_noexp_and_modifier`,
  proving the `CF_NOEXP` gate now blocks the zombie-shrine exp grant and
  the runtime `exp_modifier` multiplier scales it - both silently ignored
  by the old raw-mutation code.
- **STILL REMAINING**: `main.rs` has 4 confirmed raw-mutation exp-grant
  sites left (`grep 'character.exp = character.exp.saturating_add'
  crates/ugaris-server/src/main.rs`): two in the "warp"/reward-sphere
  block (~line 3376/3432, `level_value(reward_level)/7` and `/70` grants,
  C source not yet cross-referenced this iteration - likely
  `src/area/23_24/strategy.c` or a lab/warp module) and two more further
  down (~line 3826/3876, C source not yet identified). Each already has
  `&mut World` in scope per the previous notes, so wiring is mechanical
  once the exact C call site (and hence the correct `area_id`/ordering)
  is confirmed - a good next slice.
- Full workspace suite (1105 + 9 + 3 + 33 + 342 = 1492 tests across all
  crates, 0 failed), `cargo fmt --all`, and `cargo build -p ugaris-server`
  (zero warnings) all pass. Boot-smoked
  `target/debug/ugaris-server --bind-addr 127.0.0.1:5556` for 10 seconds
   (this change touches the world-action processing loop): "completed
   world actions" ticked cleanly every frame with no panics.

### Iteration 24: closed the last `main.rs` raw-mutation exp-grant sites

- Cross-referenced the 4 remaining `character.exp = character.exp
  .saturating_add(...)` sites in `main.rs` (named but not yet identified
  in iteration 23's note) against C:
  - `ItemDriverOutcome::WarpBonus` handler's two grants (formerly ~line
    3376/3432) are C `warpbonus_driver` (`src/area/25/warped.c:423` the
    sphere-kind-1 `give_exp(cn, level_value(level) / 7)` full-cycle
    reward, `:453` the `give_exp(cn, level_value(level) / 70)` per-step
    trickle exp when `!ppd->nostepexp`).
  - `ItemDriverOutcome::BookcaseText`'s library-solved-once grant is C
    `bookcase` (`src/area/17/two.c:2622`,
    `give_exp(cn, min(level_value(ch[cn].level) / 5, 80000))` - matches
    the pre-existing `bookcase_library_exp` helper's formula exactly, so
    only the call site needed fixing, not the formula).
  - `ItemDriverOutcome::StafferAnimationBook`'s one-time grant is C
    `staffer_animation_book` (`src/area/29/brannington.c:521`,
    `give_exp(cn, min(level_value(60) / 5, level_value(ch[cn].level) / 4))`
    - matches the pre-existing driver-side `exp_added` computation in
    `area29_brannington.rs::staffer2_driver` exactly).
- All four now call `world.give_exp(character_id, ..., u32::from(args
  .area_id))` (the same `args.area_id` the sibling `RandomShrineKind`/
  `Chest` dispatch arms already use a few hundred lines up) instead of
  mutating `character.exp` directly, so the hardcore/`exp_modifier`
  multipliers, `CF_NOEXP`/area-21 gate, `CF_NOLEVEL` clamp, and
  `check_levelup` tail call now apply uniformly to these four grants for
  the first time.
- The warp-bonus reward-sphere match needed restructuring: the exp arm
  (`Some(1)`) can no longer live inside the
  `world.characters.get_mut(&character_id)` borrow the save/military/
  gold/lollipop arms use, since `give_exp` needs `&mut World` itself, not
  `&mut Character`. Hoisted the match to the top level on
  `reward_sphere_kind` so only the non-exp arms individually re-borrow
  `world.characters`; verified behaviorally identical against
  `warped.c:397-441` line by line, including preserving the `Some(2)`
  "only grant a save if `saves < 10 && !CF_HARDCORE`" guard (now an `if`
  inside the arm instead of a match guard, since match guards can't see
  the re-borrowed `character` from inside the arm body).
- No new dedicated tests: these are inline dispatch-loop match arms with
  no testable pure-function boundary of their own (the actual formulas
  they call - `bookcase_library_exp`, `warpbonus_driver`'s
  `reward_sphere_kind`/`reward_level` computation, `staffer2_driver`'s
  `exp_added` computation - already have direct unit tests in
  `ugaris-core/src/item_driver/tests/`). This matches the established
  precedent for the sibling `RandomShrineKind::Edge` arm from iteration
  23, which is likewise untested at the `main.rs` wiring level. Full
  workspace suite stayed at the same pass counts as iteration 23 (1105 +
  9 + 3 + 33 + 342 tests, 0 failed), confirming no behavior regression.
- Grepped the whole workspace for any other raw `character.exp` grant
  mutation (excluding `exp_used`/`exp_cost`/`exp_added` counters and the
  intentional `saturating_sub` refund/loss sites in `potions.rs`/
  `death.rs`, and `scrolls.rs::raise_value_exp`'s raw `+=`, which matches
  C `skill.c:353-354` exactly - that function adds to `exp` directly in C
  too, not via `give_exp`): none remain. This closes the "main.rs's
  inline quest/area reward grants" sub-gap completely.
- `cargo fmt --all`, `cargo test --workspace` (0 failed), and
  `cargo build -p ugaris-server` (zero warnings) all pass. Boot-smoked
  `target/debug/ugaris-server --bind-addr 127.0.0.1:5556` for 10 seconds
  (this change touches the item-driver outcome dispatch in the tick
  loop): ticked cleanly with "completed world actions"/"processed NPC
  driver messages" every frame, no panics.
- **STILL REMAINING** on the "Experience/level-up side effects" task
  overall (unchanged from earlier iterations, and outside this specific
  give_exp-routing sub-scope): the level-10-multiple "Grats" server-wide
  broadcast (`server_chat(6, ...)`), `achievement_check_level`, and
  `reset_name(cn)` inside `check_levelup` itself have no Rust
  equivalents anywhere in the tree.

## Ralph Loop - Iteration 25: `update_char` `P_CLAN`/`areaID == 13` Gap

- Resumed the `update_char` stat recomputation P1 task (`- [~]`) and
  closed the `P_CLAN`/`areaID == 13` catacombs-bonus gap that iteration
  21 deferred as "mechanically simple but touches roughly 32 existing
  `update_character` call sites". Re-examined that assumption: `World`
  has no `area_id` field, but every `ugaris-server` process only ever
  loads and runs a single area for its entire lifetime
  (`ServerConfig::area_id`, set once from CLI args at startup and never
  mutated afterward - confirmed by grepping every `config.area_id`/
  `args.area_id` use in `main.rs`). So instead of threading a new
  `area_id: u16` parameter through `update_character` and its ~17
  non-test call sites (the refactor iteration 21 was avoiding), added a
  single `pub area_id: u16` field directly to `World`
  (`crates/ugaris-core/src/world/mod.rs`, defaults to `0` for free via
  the struct's existing `#[derive(Default)]`) and set it exactly once in
  `crates/ugaris-server/src/main.rs`, immediately after
  `World::default()` and before the zone map load
  (`world.area_id = config.area_id;`). Zero call sites needed touching
  beyond that one line plus the recompute function itself.
- `World::update_character` (`crates/ugaris-core/src/world/
  character_values.rs`) now computes `in_clan_area` as `self.area_id ==
  13 || tile.flags.contains(MapFlags::CLAN)`, matching C `create.c:1856`
  (`ch[cn].prof[P_CLAN] && n >= V_WIS && n <= V_STR && (areaID == 13 ||
  (mmf & MF_CLAN))`) exactly. The `P_CLAN` bonus arithmetic itself
  (`character_values.rs:511-514`) was already correct from an earlier
  iteration and only needed the real `areaID == 13` input wired in -
  confirmed by reading that block again before changing anything.
- 2 new tests in `world/tests/character_values.rs`:
  `clan_profession_bonus_applies_in_area_13_catacombs_without_clan_tile_flag`
  (sets `world.area_id = 13`, spawns on a tile with no `MF_CLAN` flag,
  asserts the bonus applies anyway) and
  `clan_profession_bonus_does_not_apply_outside_area_13_or_clan_tile`
  (area `1`, no clan tile, asserts no bonus) - both directly exercise
  `World::update_character`, not just the pure-data helper, since the
  new field lives on `World`.
- `PORTING_TODO.md`'s `update_char` task entry and `PORTING_LEDGER.md`'s
  `create.c` `update_char` Ported-table row updated; the task stays
  `- [~]` since one gap remains: `ch.ef[]` area-effect light
  contributions to `V_LIGHT` (a separate, larger effects-attachment
  system gap, not something this field addressed).
- Full workspace suite (1107 + 9 + 3 + 33 + 342 = 1494 tests across all
  crates, 0 failed - core count includes the 2 new tests), `cargo fmt
  --all`, and
  `cargo build -p ugaris-server` (zero warnings) all pass. Boot-smoked
  `target/debug/ugaris-server --bind-addr 127.0.0.1:5556` (this change
  touches `World` initialization and the character recompute that runs
  on login/equip): "entering Rust game loop area_id=1" appeared, ticks
  advanced cleanly with "completed world actions"/"processed NPC driver
  messages" every frame, no panics.

## Ralph Loop - Iteration 26: "Grats" Level-Up Channel Broadcast

- Resumed the "Experience/level-up side effects" P1 task (`- [~]`) and
  closed the "Grats" broadcast third of its remaining note (C
  `check_levelup`'s `if (ch[cn].level % 10 == 0) server_chat(6, ...)`,
  `src/system/tool.c:1347-1350`, sending `"0000000000" COL_MAUVE "Grats:
  %s is level %d now!"`). `World::check_levelup` (`ugaris-core`) has no
  session/runtime access, so this needed the same queue-then-drain
  pattern already used for `pending_system_texts`/`pending_area_texts`:
  added `WorldChannelBroadcast { channel: u8, message_bytes: Vec<u8> }`
  plus `World::queue_channel_broadcast`/`drain_pending_channel_broadcasts`
  (`crates/ugaris-core/src/world/text.rs`), and a new
  `pending_channel_broadcasts: Vec<WorldChannelBroadcast>` field on
  `World` (`world/mod.rs`).
- `check_levelup` (`crates/ugaris-core/src/world/exp.rs`) now queues one
  broadcast per level-up crossing a multiple of ten, building the exact
  C byte sequence: `b"0000000000"` + `COL_CHAT_GRATS` (`ugaris-core`'s
  existing alias for `COL_MAUVE`, `crates/ugaris-core/src/text.rs:36`) +
  `format!("Grats: {name} is level {level} now!")`. Channel 6 is already
  the "Grats" joinable chat channel in `commands_chat.rs`
  (`LEGACY_CHAT_CHANNELS`), so no new channel metadata was needed.
- Added `send_pending_world_channel_broadcasts` (`crates/ugaris-server/
  src/world_events.rs`), wired into the tick loop right after the
  existing `send_pending_world_system_texts` call
  (`crates/ugaris-server/src/main.rs`). It drains the queue each tick
  and, for each event, collects the `CharacterId`s of every connected
  player whose `PlayerRuntime::chat_channels` bitmask has the target
  channel's bit set (`1 << (channel - 1)`, the same rule
  `apply_chat_command` uses for player-authored channel messages - no
  clan/mirror/area/ignore filters apply to channel 6), then sends the
  payload via `ugaris_protocol::packet::system_text_bytes` to every
  session for each recipient character. Collecting the recipient IDs
  into a `Vec` first avoids a `runtime.players` immutable-borrow
  conflict with the later `&mut runtime` session-send calls.
- 3 new tests in `crates/ugaris-core/src/world/tests/exp.rs`:
  `check_levelup_queues_a_grats_channel_broadcast_at_level_ten` (asserts
  the exact byte-for-byte payload), `check_levelup_does_not_queue_a_
  grats_broadcast_for_non_multiple_of_ten_levels`, and
  `check_levelup_queues_one_grats_broadcast_per_multiple_of_ten_when_
  gaining_several_levels` (a single `check_levelup` call that crosses
  both level 10 and level 20 in its loop queues exactly two broadcasts,
  in order).
- `PORTING_TODO.md`'s "Experience/level-up side effects" task entry and
  `PORTING_LEDGER.md`'s `tool.c` `check_levelup` Ported-table row
  updated; the task stays `- [~]` since two gaps remain:
  `achievement_check_level` (needs a general achievement engine - no
  Rust achievement system exists beyond a handful of hardcoded
  chest/transport milestone flags, out of this task's scope) and
  `reset_name(cn)` (an intentional no-op - Rust has no server-side
  colored-name cache to invalidate, already documented in
  `character_values.rs`).
- Full workspace suite: `cargo test --workspace` passed (1110 ugaris-core
  + 9 ugaris-db + 3 ugaris-net + 33 ugaris-protocol + 342 ugaris-server =
  1497 tests, 0 failed - core count includes the 3 new tests). `cargo fmt
  --all` made no changes to the touched files. `cargo build -p
  ugaris-server` passed with zero warnings. Boot-smoked (this change
  touches the tick loop): `target/debug/ugaris-server --bind-addr
  127.0.0.1:5556` logged "entering Rust game loop area_id=1
  mirror_id=1" and advanced ticks cleanly with "completed world
  actions"/"processed NPC driver messages" every frame, no panics.

## Ralph Loop - Iteration 27: Closing "Experience/Level-Up Side Effects"

- Resumed the P1 "Experience/level-up side effects" task (`- [~]`) for a
  final verification pass rather than new feature work: re-audited the
  whole workspace (`grep -rn "\.exp = \|\.exp +=\|\.exp\.saturating_add"`
  across every crate's `src/`, excluding `tests/`) for any remaining raw
  `character.exp` grant mutation that should route through
  `World::give_exp`/`check_levelup`. Found six hits total; three were
  already the canonical `give_exp` internals (`world/exp.rs`), and the
  other three were each checked line-by-line against their exact C
  originals:
  - `item_driver/scrolls.rs::raise_value_exp`'s `character.exp =
    character.exp.saturating_add(cost)` matches C `raise_value_exp`
    (`src/system/skill.c:358-359`, `ch[cn].exp += cost;`), which is a
    bare addition followed by its own direct `check_levelup(cn)` call -
    not a `give_exp` call - so no multiplier/`NOEXP`/area-21 gating ever
    applied there in C either. Correct as-is.
  - `item_driver/potions.rs`'s and `world/death.rs`'s `saturating_sub`
    sites are exp *losses* (a potion side-effect refund-in-reverse and
    the death exp-loss penalty), matching C's own bare `ch[cn].exp -=
    loss;` subtractions (`death.c:790/916/968`) - `give_exp` is a
    grant-only function in C and is never called for losses.
  - `commands_admin.rs`'s `/setlevel` debug command's `character.exp =
    level2exp(level)` matches C `cmd_setlevel`'s own direct assignment
    (a GM debug override, not a gameplay grant).
- Cross-checked `World::check_levelup` (`world/exp.rs:132-203`) against
  C `check_levelup` (`src/system/tool.c:1318-1363`) one more time, line
  by line: level-increment loop condition, "Thou gained a level!" text,
  hardcore-vs-normal saves handling (reset to 0 vs `+1` capped at 10)
  with both feedback lines, the `level >= 20 &&
  !value[1][V_PROFESSION]` unlock guard, the level-10-multiple "Grats"
  channel-6 broadcast byte sequence, and the `set_sector` dirty-tile
  refresh all match exactly. The only unmatched C lines are
  `achievement_check_level` (no Rust achievement engine exists at all -
  tracked under its own P4 "Achievements" task in `PORTING_TODO.md`),
  `reset_name(cn)` (a genuine no-op - Rust has no server-side
  colored-name cache to invalidate), and `dlog(cn, 0, "gained a
  level")` (no Rust debug-log sink exists anywhere in the tree, a
  pre-existing documented gap shared with several other ported
  functions).
- Marked the task `- [x]` in `PORTING_TODO.md` with a closing note
  explaining the audit and why the two remaining C lines are correctly
  out of scope. No production code changes this iteration - this was a
  verification-only closure pass confirming a task that prior
  iterations had substantially completed but left `- [~]` out of an
  abundance of caution.
- Verification: `cargo fmt --all` made no changes. `cargo test
  --workspace` passed at the same counts as the prior iteration (no
  regressions, no new tests needed since no behavior changed). `cargo
  build -p ugaris-server` passed with zero warnings. No boot-smoke
  required (no runtime/protocol/login/map-sync code touched).
- Iteration 28: closed `update_char` stat recomputation's (P1) final
  documented gap and marked the task `- [x]` in `PORTING_TODO.md`. C
  `update_char` (`create.c:1785-1797`) sums `mod[V_LIGHT] +=
  ef[fn].light` across a character's up to four attached effects
  (`ch[cn].ef[0..4]`); Rust's `World::update_character` now computes this
  via a new `World::character_attached_effect_light` helper
  (`crates/ugaris-core/src/world/character_values.rs`) that sums `.light`
  across effects with `Effect::target_character == Some(character_id)`
  (infrastructure that already existed from prior iterations' show-effect
  work for magicshield/firering/pulseback/burn/bless/warcry/freeze/
  potion/curse/cap/lag/strike/flash) and passes the total into
  `recompute_character_values` as a new `effect_light: i32` parameter,
  added directly into `mod_arr[V_LIGHT]` before the shared cap/formula
  loop - matching C exactly, since `V_LIGHT` (index 9) sits outside the
  `n <= V_STR || n >= V_PULSE` mod-percentage-cap range and is explicitly
  exempted from the `CF_NOMAGIC` gate alongside `V_WEAPON`/`V_ARMOR`, so
  no extra capping logic was needed. Documented, intentional deviation:
  C's `ch[cn].ef[]` is a fixed four-slot array and `add_effect_char`
  (`effect.c:209`) silently refuses a character's fifth simultaneous
  attached effect (dropping its light contribution and its visibility to
  the owning client); Rust's `Effect::target_character` has no such cap,
  so `character_attached_effect_light` approximates the C array by
  summing only the four lowest-effect-id (earliest-attached) matching
  effects, which is exact for the common 0-4-effect case and only
  deviates from C when a character has 5+ character-attached effects
  simultaneously (a rare combat/spell-stacking edge case). 2 new focused
  tests in `crates/ugaris-core/src/world/tests/character_values.rs`:
  `character_attached_effect_light_contributes_to_v_light` (single
  magicshield effect's light reaches V_LIGHT) and
  `character_attached_effect_light_caps_at_four_effects_by_creation_order`
  (a fifth attached effect's light is excluded, matching the four-slot
  cap). `cargo fmt --all` / `cargo test --workspace` (1112/9/3/33/342
  passed, 0 failed across all five workspace test binaries) / `cargo
  build -p ugaris-server` all clean with zero warnings; boot-smoked past
  tick 233 with no panics. This closes out `update_char` as fully ported:
  all four recompute slices (equipment/spell modifier sum with caps,
  driver-spell flags, armor/weapon values, HP/endurance/mana clamps) plus
  every C call site (equip/unequip, spell install/expiry, skill raising,
  level-up, death respawn, login) plus every sub-detail gap (sprite
  reselection, `P_CLAN`/area-13 catacombs bonus, and now the
  character-attached-effect light contribution) are ported, with only
  the trivial `player_reset_map_cache` display-cache no-op (Rust has no
  client-side scroll-diff cache to invalidate) and the above four-slot
  approximation remaining as intentional, documented deviations from C.

## Ralph Loop - Iteration 29: Equipment Slot Rules on Swap (`CL_SWAP`)

- C `can_wear` (`src/system/tool.c:994-1098`) and `check_requirements`
  (`tool.c:943-991`) gate C `swap()`'s worn-slot placement
  (`do.c:1216-1299`, invoked only when `pos < 12` and the cursor holds an
  item) but had no Rust equivalent - `inventory_swap_slot`
  (`crates/ugaris-server/src/inventory.rs`) previously let any item into
  any worn slot unconditionally. Ported both as `World::can_wear`
  (new `pub fn` on `impl World` in `crates/ugaris-core/src/world/items.rs`)
  and a `pub(crate) fn check_requirements(character, item)` helper it
  calls: all 12 `WN_*` slot-flag matches (`worn_slot::HEAD` needs
  `IF_WNHEAD`, etc, using the crate's existing `legacy::worn_slot`
  constants which already match the C `WN_*` `#define` values 1:1), the
  `WN_LHAND`/`WN_RHAND` two-handed hand-conflict rules (equipping into
  `WN_LHAND` is blocked outright if `WN_RHAND` holds an `IF_WNTWOHANDED`
  item; equipping an `IF_WNTWOHANDED` item into `WN_RHAND` is blocked if
  `WN_LHAND` is occupied by anything, regardless of its flags -
  transcribed exactly from C's asymmetric `inr`-flag-check vs.
  `inl`-truthiness-check), `min_level`/`max_level`, all four
  `needs_class` bits (1=reject Mage/"Warrior-only", 2=reject Warrior/
  "Mage-only", 4="Seyan'Du-only" requires both `CharacterFlags::MAGE` and
  `CharacterFlags::WARRIOR` via bitflags `contains` on the OR'd mask,
  8="Arch-only" requires `CharacterFlags::ARCH`), negative-`modifier_index`
  stat requirements (read against `character.values[1]`, the base/raised
  array, not the equipment-modified `values[0]` effective total - matches
  C's `ch[cn].value[1][-v1]`), and `IF_BONDWEAR` ownership
  (`item.owner_id != character.id`). Ported C's out-of-range-index guard
  (`v1 <= -V_MAX`) as `mod_index <= -(CHARACTER_VALUE_COUNT as i16)`
  *before* negating, since a naive `-mod_index` on `i16::MIN` panics with
  an overflow (a bug caught by a dedicated test).
- Wired into `inventory_swap_slot`: when the cursor holds an item and the
  target `slot < 12`, `world.can_wear(character_id, item_id, slot)` must
  return true or the swap is silently rejected (`InventoryCommandResult::
  Ignored`) - matching C's `cl_swap`, which calls `swap()` and discards
  its return value/`error` entirely, so a failed wear attempt has always
  been silent to the legacy client too. Unequipping (empty cursor,
  `pos < 12`) is unaffected, since C only calls `can_wear` inside
  `if ((in = ch[cn].citem))`.
- 15 new focused tests: 6 core-level in
  `crates/ugaris-core/src/world/tests/items.rs`
  (`can_wear_rejects_positions_outside_the_worn_slot_range`,
  `check_requirements_rejects_above_maximum_level`,
  `check_requirements_seyanddu_gate_needs_both_mage_and_warrior_flags`,
  `check_requirements_arch_gate_rejects_non_arch_characters`,
  `check_requirements_bondwear_restricts_to_the_bonded_owner`,
  `check_requirements_ignores_out_of_range_modifier_index_without_panicking`)
  and 9 server-level end-to-end in
  `crates/ugaris-server/src/tests/inventory.rs` covering slot-flag match/
  mismatch, min-level rejection, needs_class rejection, stat-requirement
  rejection, both two-handed hand-conflict directions plus the
  non-conflict success case, and confirming unequip bypasses `can_wear`.
- Explicitly out of scope for this slice (documented, not silently
  dropped): (1) C `swap()`'s `store_item`-based auto-unequip cross-hand
  cleanup (`do.c:1260-1271`, e.g. "equip a torch into `WN_LHAND` while a
  two-handed weapon occupies `WN_RHAND`, freeing `WN_RHAND` into the
  backpack") is unreachable dead code in the normal flow - tracing both
  trigger conditions shows `can_wear` already rejects the placement
  before that code can execute (the `WN_LHAND` case needs `WN_RHAND` to
  hold `IF_WNTWOHANDED`, which `can_wear`'s own `WN_LHAND` branch already
  vetoes; the `WN_RHAND` case needs the incoming item to be
  `IF_WNTWOHANDED` while `WN_LHAND` is occupied, which `can_wear`'s own
  `WN_RHAND` branch already vetoes) - so it was intentionally not ported;
  (2) the "no switching equipment in Teufel PK arena" gate (`do.c:1230-
  1233`, `areaID == 34 && (tile.flags & MF_ARENA) && pos != WN_LHAND &&
  pos < 12`) needs an `area_id` parameter threaded through
  `inventory_swap_slot` and its `crates/ugaris-server/src/merchants.rs`
  `apply_fast_sell` call site (which currently has no area_id available),
  deferred to keep this slice focused on the task's named "worn slot flag
  match, min level, class gates, two-handed vs left hand" scope; (3) the
  `IF_MONEY`-item-swapped-into-a-slot-converts-to-gold branch
  (`do.c:1276-1287`) is a distinct money-handling concern unrelated to
  equipment-slot rules, left for a future task.
- `cargo fmt --all` / `cargo test --workspace` (1118 core + 9 + 3 + 351
  server + 0 doc-tests, all green, no failures) / `cargo build
  -p ugaris-server` clean with zero warnings; boot-smoked past tick 232
  with no panics.

## Ralph Loop - Ground Item Decay (Iteration 30)

- C `act_drop` (`src/system/act.c:386-448`) itself does not arm any decay
  timer; the timer is actually armed one layer down, inside
  `set_item_map` (`src/system/map.c:36-85`), which every ground-item
  placement path (player drop, container overflow spread, etc.) funnels
  through: `if (it[in].flags & IF_TAKE) { set_expire(in, item_decay_time); }`.
  `set_expire` (`src/system/expire.c`, full file read) itself no-ops for
  `IF_NODECAY` items (`if (it[in].flags & IF_NODECAY) return 1;`) before
  scheduling a single-shot `expire_timer` callback that removes the item
  (and destroys any container contents) from the map at
  `ticker + item_decay_time`. `item_decay_time` is a runtime-mutable
  global defaulting to `5 * 60 * TICKS` (`src/config/game_settings.c`),
  already ported unchanged to `GameSettings::item_decay_time` in
  `crates/ugaris-core/src/game_settings.rs`. (Note: the `PORTING_TODO.md`
  task text referenced `item.c`/`tool.c` and a function named
  `expire_item`; neither exists in the C tree - the real home is
  `expire.c` + `map.c`, corrected here for future reference.)
- Rust's `World::set_item_expire` (pre-existing, `world/death.rs`, used
  for player/NPC body decay via `set_expire_body`'s simplified
  single-shot equivalent) is now also wired into `World::complete_drop`
  (`crates/ugaris-core/src/world/actions.rs`): after `act_drop` succeeds
  and the item is confirmed on the map, `item.flags.contains(TAKE) &&
  !item.flags.contains(NODECAY)` arms `set_item_expire(item_id,
  settings.item_decay_time)`. Since Rust's `set_item_expire` (unlike C's
  `set_expire`) has no built-in `IF_NODECAY` check, the gate combining
  both flags lives at the `complete_drop` call site instead - functionally
  equivalent to C's two-layer check (`set_item_map`'s `IF_TAKE` gate +
  `set_expire`'s internal `IF_NODECAY` early return).
- `map.rs`'s lower-level `drop_item`/`set_item_map` primitives were left
  untouched (no timer-queue access exists at that layer, matching the
  existing architecture where `World` owns `self.timers`); `drop_item` is
  currently dead code with no call sites outside its own unit test, so no
  further wiring was needed this iteration.
- 2 new focused tests in `crates/ugaris-core/src/world/tests/items.rs`:
  `complete_drop_arms_decay_timer_for_take_items_and_expires_after_item_decay_time`
  (item survives at `item_decay_time - 1` ticks, is gone at exactly
  `item_decay_time` ticks, matching C's `ticker + duration` due-time
  semantics) and
  `complete_drop_does_not_arm_decay_timer_for_nodecay_take_items` (an
  `IF_TAKE | IF_NODECAY` item, e.g. a dropped lit torch, survives well
  past `item_decay_time`).
- `cargo fmt --all` / `cargo test --workspace` (1120 core + 9 + 3 + 351
  server + 0 doc-tests, all green) / `cargo build -p ugaris-server` clean
  with zero warnings; boot-smoked (`entering Rust game loop`, ticking with
  no panics for 10+ seconds).

## Ralph Loop - `SV_SETVAL`/Resource Streaming On Change (Iteration 31)

- The function the `PORTING_TODO.md` task called `plr_update` does not
  exist under that name; the real C home is `player_stats()`
  (`src/system/player.c:2944-3398`), called once per tick per `ST_NORMAL`
  player from the `player_map`/`player_stats`/`player_act` triple
  (`player.c:3648-3662`). It gates the 43-slot value-table diff loop
  (`SV_SETVAL0`/`SV_SETVAL1`) behind `CF_UPDATE`, clearing the flag right
  after; gates the item/citem/cprice/gold diff behind `CF_ITEMS`, likewise
  clearing after; and sends HP/endurance/mana/lifeshield/exp/exp_used
  unconditionally whenever they differ from a per-session shadow (no flag
  gate at all for those fields - C keeps a full per-player shadow of every
  field it last sent and diffs field-by-field). Confirmed `CF_UPDATE`
  (`1<<8`) / `CF_ITEMS` (`1<<12`) numeric values already matched the
  existing Rust `CharacterFlags::UPDATE`/`::ITEMS` bit positions - that
  part of the port (dozens of set-sites across `world/*.rs`,
  `item_driver/*.rs`) was already faithfully done; nothing ever consumed
  or cleared the flags.
- New `crates/ugaris-server/src/resource_sync.rs`
  (`queue_resource_sync_frames`), wired into the tick loop in `main.rs`
  immediately before `queue_periodic_player_frames` (mirrors C's
  `player_map` then `player_stats` call ordering). Since Rust has no
  per-session shadow-value cache (unlike C's `player[nr]->value[][]`/
  `hp`/`gold`/`item[]`), this sends a full snapshot of whichever
  category's flag is set - same packet shapes as `login_payload`'s value
  loop (`SV_SETVAL0/1` for all 43 slots, `SV_SETHP`/`SV_ENDURANCE`/
  `SV_SETMANA`/`SV_LIFESHIELD`, `SV_EXP`/`SV_EXP_USED`) for `UPDATE`, and
  `inventory_snapshot_payload`'s shape (`SV_SETCITEM`, `SV_SETITEM` per
  slot, `SV_GOLD`) for `ITEMS` - instead of C's per-field diff, and clears
  exactly the flag(s) that were acted on. This preserves C's flag-gating
  semantics (nothing sent when neither flag is set) and coexists
  harmlessly with the existing ad-hoc `command_inventory_refresh`/
  `command_container_refresh`/per-action pushes in `main.rs`, which were
  intentionally left in place this iteration per the task's own
  "migrate call sites gradually, do not break existing tests" note.
- 5 new focused tests in `crates/ugaris-server/src/tests/resource_sync.rs`:
  no packet sent when neither flag is set, `UPDATE` sends values/HP/exp
  and clears only `UPDATE` (leaving `ITEMS` untouched), `ITEMS` sends
  cursor/inventory/gold and clears only `ITEMS`, both flags set in the
  same tick produce one combined frame and clear both, and sessions not
  in `PlayerConnectionState::Normal` are skipped without touching the
  flag.
- REMAINING: no per-session shadow diff cache exists yet, so this sends
  full category snapshots rather than only the fields that actually
  changed (functionally correct, strictly more bytes on the wire than C
  - a future task could add a shadow cache parallel to
  `VisibleMapCache`/`ClientEffectCache` for exact diff parity);
  `command_inventory_refresh`/`command_container_refresh` call sites in
  `main.rs` were not migrated away, so a handful of actions will now
  (harmlessly) double-send an inventory snapshot within the same tick.
- `cargo fmt --all` / `cargo test --workspace` (1120 core + 9 + 3 + 356
  server + 0 doc-tests, all green) / `cargo build -p ugaris-server` clean
  with zero warnings; boot-smoked (`entering Rust game loop`, ticking with
  no panics for 10+ seconds).

## Ralph Loop - Serial Validation Everywhere (Iteration 32)

- Read `src/system/player_driver.c` in full (`cl_kill`/`cl_give`/
  `player_driver_kill`/`player_driver_give`/`player_driver_charspell`
  setters, `run_queue`/`check_high_prio_task`/`check_med_prio_task`/
  `check_low_prio_task`, the pre-switch `switch (player[nr]->action)`
  staleness block, and the post-`run_queue` dispatch switch) plus
  `src/system/drvlib.c`'s `give_driver`/`take_driver`/`use_driver`/
  `drop_driver`/`fireball_driver`/`ball_driver`. Contrary to the todo
  note's "C guards every queued action" phrasing, C only actually
  *validates* a captured serial in two places: the `PAC_KILL` pre-switch
  block (`if (ch[player[nr]->act1].serial != player[nr]->act2)
  player[nr]->action = PAC_IDLE;`, `player_driver.c:1055-1058`) and
  `fireball_driver`/`ball_driver` (`if (!ch[co].flags || ch[co].serial !=
  serial) { error = ERR_DEAD; return 0; }`, `drvlib.c:1118-1156`), reached
  via the queued `PAC_FIREBALL2`/`PAC_BALL2` character-target variants.
  `take_driver`/`drop_driver`/`give_driver`/`check_high_prio_task`'s
  bless/heal all receive the captured serial as an unused parameter -
  `player_driver_take`/`use`/`kill`/`give`/`charspell` capture
  `it[in].serial`/`ch[co].serial` into `act2` every time, but only the two
  call sites above ever compare it. This is dead data capture in C, not a
  missing check, so it was not ported as a check for take/drop/give/
  bless/heal (matches the "port observable behavior, not C oddities as
  new features" hard rule).
- `crates/ugaris-core/src/world/spells.rs::setup_fireball_character`/
  `setup_ball_character` already had the `fireball_driver`/`ball_driver`
  serial guard (`target_serial != 0 && target.id.0 != target_serial`);
  added the missing `PAC_KILL` guard to the `PlayerActionCode::Kill` arm
  of `World::apply_player_action_setup`
  (`crates/ugaris-core/src/world/actions.rs`), checked before the
  existing attack-policy/PK-hate logic to match C's pre-switch ordering,
  using the same `0`-is-no-check sentinel convention as the fireball/ball
  guards (real kills always carry a real, non-zero serial once captured
  correctly - see below).
- Found the actual live-gameplay gap while wiring this up: `crates/
  ugaris-server/src/player_actions.rs::apply_player_action` - the
  function that turns a parsed `ClientAction` into a `PlayerRuntime`
  action/queue entry - hardcoded serial `0` for `ClientAction::
  CharacterSpell` and the character-targeted (`x == 0`) branch of
  `ClientAction::MapSpell`, and routed `ClientAction::Kill`/`Give`
  through the generic `action_to_queued` helper, which also always
  produces `arg2 = 0`. This meant the world-layer fireball/ball
  character-serial checks were always defeated by the `0` sentinel in
  real gameplay (never actually validated), and `PAC_KILL` had no serial
  to check at all. Added a `character_serial` lookup helper and explicit
  `ClientAction::Kill`/`Give` match arms to `apply_player_action`,
  captured the live target serial for `CharacterSpell`/character-targeted
  `MapSpell` the same way C's `cl_kill`/`cl_give`/
  `player_driver_charspell` do (synchronous lookup at packet-receive
  time, before the action is queued/dispatched); threaded
  `&World::characters` through `ServerRuntime::queue_action`
  (`crates/ugaris-server/src/main.rs`) from its one call site, which
  already had `world` in scope.
- Fixed a pre-existing test bug this exposed:
  `tests::world_events::setup_world_actions_promotes_deferred_legacy_
  player_fightback` (`crates/ugaris-server/src/tests/world_events.rs`)
  set `player.next_fightback_serial = 99` without giving the mock
  attacker character a matching `serial = 99` (it defaulted to the
  attacker's `character_id.0 = 2`); the new `PAC_KILL` guard now
  correctly rejects that mismatch, exactly like the sibling test
  `hurt_events_start_legacy_player_fightback_for_nearby_attacker` right
  above it already does it correctly. Set `attacker.serial = 99` to match
  (same fix pattern as the working sibling test), not a weakened
  assertion.
- 7 new focused tests: `world_kill_setup_aborts_to_idle_when_target_
  serial_is_stale` / `world_kill_setup_proceeds_when_target_serial_
  matches` (`crates/ugaris-core/src/world/tests/combat.rs`), and
  `apply_player_action_kill_captures_live_target_serial` /
  `apply_player_action_kill_of_unknown_character_captures_zero_serial` /
  `apply_player_action_give_captures_live_target_serial` /
  `apply_player_action_character_spell_captures_live_target_serial` /
  `apply_player_action_map_spell_character_target_captures_live_serial`
  (`crates/ugaris-server/src/tests/commands_player.rs`).
- `cargo fmt --all` / `cargo test --workspace` (1122 core + 9 + 3 + 361
  server + 0 doc-tests, all green) / `cargo build -p ugaris-server` clean
  with zero warnings; boot-smoked (`entering Rust game loop`, ticking with
  no panics for 10+ seconds).

## Ralph Loop - Logout/Exit Flow: `CDR_LOSTCON` Linger (Iteration 33)

- Read `src/system/player.c`'s `kick_player`/`exit_player`/`exit_char`/
  `player_client_exit`/`read_login`, `src/module/lostcon.c`'s full
  `lostcon_driver`/`lostcon_dead`, and `src/system/database/
  database_character.c`'s `tick_login()` reclaim branch. The todo's
  `cl_exit`/`take_over_char` names don't exist verbatim in the current C
  tree; the real functions are `kick_player` (disconnect entry point:
  `ac_player_disconnect`, then `ch[cn].driver = CDR_LOSTCON` +
  `char_driver(driver, CDT_DEAD, cn, 0, 0)` to arm `dat->timeout = ticker +
  lagout_time` - the character is *not* despawned on disconnect),
  `player_client_exit` (sends `SV_EXIT`, the real `cmd_exit`), and the
  in-place reclaim spread across `tick_login()`
  (`ch[n].driver = 0; login_ok(n, 1);`) and `read_login`
  (`ch[cn].player = nr; ch[cn].driver = 0;`). `lagout_time` defaults to
  `5 * 60 * TICKS` = 7200 ticks (`game_settings.c:171`) and was already a
  live `ServerRuntime`/`GameSettings` field with a `/setlagouttime` admin
  command wired up (from an earlier iteration) but nothing read it yet.
- Rust `World`-side state (`crates/ugaris-core/src/world/lostcon.rs`, new):
  reused the existing `Character.driver_state: Option<CharacterDriverState>`
  slot instead of adding a new `Character` field (which would have required
  editing ~15+ existing `Character { ... }` struct-literal test/helper call
  sites across the tree, since `Character` has no `Default` impl) - added a
  `CharacterDriverState::Lostcon(LostconDriverData { deadline: u64 })`
  variant (`character_driver.rs`) alongside the existing `Merchant`/
  `SimpleBaddy`/etc. variants, and fixed the four now-non-exhaustive
  `match`es this opened up (`character_driver.rs::apply_simple_baddy_
  create_message`, `world/npc_fight.rs::simple_baddy_lastfight`, `world/
  npc_idle.rs::setup_pending_simple_baddy_friend_bless`, `world/
  npc_messages.rs::simple_baddy_recorded_enemy_ids`). `World::
  enter_lostcon`/`reclaim_lostcon`/`is_lostcon`/`expired_lostcon_characters`
  are the C `kick_player`/`tick_login`-reclaim/`lostcon_driver`-timeout
  equivalents.
- Rust session-side glue (`crates/ugaris-server/src/lostcon.rs`, new):
  `enter_lostcon_on_disconnect` stashes the disconnecting session's
  `PlayerRuntime` (which carries the PPD-backed persistent state - ppd_blob,
  keyring, chest history, achievements, etc. - that C keeps alive in `ch[]`-
  adjacent structures regardless of socket state, but Rust's architecture
  ties to the session-owned `PlayerRuntime`) into a new `ServerRuntime.
  lostcon_players: HashMap<CharacterId, PlayerRuntime>` map instead of
  dropping it, and keeps the account depot in `account_depots` rather than
  removing it. `reclaim_lostcon_on_login` restores the stashed
  `PlayerRuntime` onto the new session via a new `PlayerRuntime::
  reclaim_for_session` (resets only session-transient fields - socket id,
  command queue, scrollback, fightback timers - leaving all PPD-backed
  state untouched) and clears the world driver. `take_expired_lostcon_
  characters` polls `World::expired_lostcon_characters` each tick and hands
  back the stashed player+depot for saving.
- Wired into `main.rs`: `SessionEvent::Disconnected` now calls
  `enter_lostcon_on_disconnect` instead of saving+removing immediately (the
  old immediate save+remove is kept as the fallback for the case where
  there's no live world character to linger, matching C's
  `if (player[nr]->state == ST_NORMAL)` guard). A new per-tick block right
  after `world.regenerate_characters` collects expired lingerers, saves
  each through a DB repository if configured (same `character_save_request`
  used by the old immediate-disconnect path), and calls
  `world.remove_character` - the C `exit_char`/`kick_char` tail.
  `SessionEvent::Login` calls `reclaim_lostcon_on_login` in three spots: the
  DB-repository `Ready` arm (before the stale `load_character_snapshot`
  call, which would otherwise overwrite the live in-memory lingering
  character with pre-disconnect DB data - skipped entirely on a successful
  reclaim) and the no-DB-repository scaffold fallback (so `DATABASE_URL`-
  less runs also honor the reclaim instead of falling through to a fresh
  template spawn).
- Tests: 6 in `crates/ugaris-core/src/world/tests/lostcon.rs` (deadline
  arming, missing-character no-op, still-on-map-and-attackable while
  lingering, reclaim clears driver/state, reclaim is a no-op when not
  lingering, expiry set matches deadline+driver and excludes reclaimed
  characters); 1 in `crates/ugaris-core/src/player.rs`
  (`reclaim_for_session_keeps_ppd_state_and_resets_session_bookkeeping`); 5
  in `crates/ugaris-server/src/tests/lostcon.rs` (enter/deadline+stash,
  enter-falls-back-when-missing, reclaim restores stashed player, reclaim
  no-op when not lingering, expiry collection only takes matured entries
  and leaves others in place).
- `cargo fmt --all` / `cargo test --workspace` (1130 core + 9 + 3 + 33 +
  366 server + 0 doc-tests, all green, zero warnings) / `cargo build -p
  ugaris-server` clean with zero warnings; boot-smoked (`entering Rust game
  loop`, 279+ ticks with no panics).
- REMAINING (documented in the `PORTING_TODO.md` task note, not silently
  dropped): the `lostcon_driver` self-defense AI cascade (auto-heal/
  potion/magicshield, `fight_driver_attack_visible`/
  `fight_driver_follow_invisible`) - a lingering character is attackable
  and takes/deals damage normally today but will not proactively fight
  back; the instant-leave-at-restarea/arena special cases and
  `karma <= -12`/`-5` early-exit branches in `lostcon_driver`; the
  `CDR_LOSTCON` exp-loss cap on death (already tracked in the `death.rs`
  ledger row); and duplicate-login kick of a still-connected (non-lostcon)
  old session (`read_login`'s `ch[cn].player != nr` guard).

## Ralph Loop - PostgreSQL Login Hardening (Iteration 34, partial)

- Read `src/system/player.c`'s `read_login` (lines 300-500, full function)
  and `player_client_exit` (lines 260-276) plus
  `src/system/database/database_character.h`'s `LS_*` constants. The task
  note's `begin_login`/`cmd_exit` names don't exist in the C oracle - the
  real functions are `find_login`/`load_char` (login lookup,
  `database_character.c`) and `player_client_exit` (`SV_EXIT` sender,
  `player.c:260`); `cmd_exit` is an unrelated area-20 `/usurp` chat command
  (`src/area/20/lq.c:2139`), a red herring in the original task text.
- Confirmed via `load_char` (`database_character.c:767-1159`) that an
  unknown character name and a wrong password both resolve to the same
  `find_login` code (`-3`, "Username or password wrong.") -
  anti-enumeration by design, no distinction leaked to the client.
  Character creation on unknown name does not happen in the login path at
  all; it happens in `tick_login()` (`database_character.c:1216-1264`)
  only for an *existing* `chars` row with `CF_USED` unset (a
  website/registration-provisioned slot), which is out of scope for this
  task and not something `ugaris-db`'s `begin_login_tx` needs to add - it
  already uniformly rejects absent rows exactly like C.
- Found the actual bug: `crates/ugaris-server/src/main.rs`'s
  `SessionEvent::Login` handler called `repository.begin_login(...)` and,
  for every outcome other than `LoginOutcome::Ready` (wrong password,
  locked, IP-locked, not paid, shutdown, account-not-fixed,
  too-many-bad-passwords, duplicate, new-area) *and* for a hard `Err` from
  the DB call, only logged a `warn!` and then fell straight through to the
  unconditional scaffold character-spawn + login-bootstrap-payload block
  below - so a wrong password (or a DB outage) silently logged the client
  into a brand-new scaffold character instead of being rejected.
- Fix: added `login_reject_message(outcome: &LoginOutcome) -> Option<&'static
  str>` (`crates/ugaris-server/src/login.rs`) mapping every reject variant
  to the exact C `read_login` switch text (`player.c:396-444`, copied
  digit-for-digit) via nine new message constants in
  `crates/ugaris-server/src/constants.rs`
  (`LOGIN_REJECT_INTERNAL_ERROR`/`_LOCKED`/`_WRONG_PASSWORD`/`_DUPLICATE`/
  `_NOT_PAID`/`_SHUTDOWN`/`_IP_LOCKED`/`_ACCOUNT_NOT_FIXED`/
  `_TOO_MANY_BAD_PASSWORDS`). `LoginOutcome::NewArea` also rejects (cross-
  area transfer isn't implemented; reuses C's target-area-server-down
  message rather than spawning a wrong-area scaffold) since the "Cross-
  area transfer" P3 todo item owns that feature. In `main.rs`, a non-`None`
  reject now builds a `PacketBuilder::exit(reason)` `SV_EXIT` payload
  (opcode 19, existing `crates/ugaris-protocol/src/packet.rs` builder,
  unchanged), queues + immediately flushes it to the session (so the
  client receives it before the socket closes, since `SessionCommand` is
  an ordered per-session `mpsc` channel), sends
  `SessionCommand::Disconnect`, logs, and `continue`s the outer select
  loop instead of falling into the scaffold-spawn/bootstrap code.
- 2 new focused tests in `crates/ugaris-server/src/tests/login.rs`:
  `login_reject_message_matches_legacy_find_login_switch` (all ten
  `LoginOutcome` variants including `Ready`/`Waiting` returning `None`)
  and `runtime_login_rejects_wrong_password_with_sv_exit_and_disconnects`
  (verifies the `SV_EXIT` byte layout and that the session command channel
  receives the `Send` frame containing the reject text followed by
  `Disconnect`, in that order).
- `cargo fmt --all` / `cargo test --workspace` (1130 core + 9 + 3 + 33 +
  368 server + 0 doc-tests, all green, zero warnings) / `cargo build -p
  ugaris-server` clean with zero warnings; boot-smoked (`entering Rust game
  loop`, no panics for 10+ seconds, no DB configured so the reject path
  itself wasn't exercised live - covered by the unit tests instead).
- REMAINING (task left `[~]`, not closed): no mocked-pool or
  `DATABASE_URL`-gated live test exercises `begin_login_tx`'s row-decision
  branching (unknown name / wrong password / locked / IP-locked / not-
  fixed / not-paid / allowed-area mismatch) against a real Postgres -
  `ugaris-db` has no async test harness or Postgres mocking dependency
  today, and adding one is out of scope under the "do not update
  dependencies" rule; `LoginOutcome::Duplicate`/`TooManyBadPasswords` are
  defined and now correctly rejected if ever returned, but
  `begin_login_tx` never constructs them (duplicate-session kick and bad-
  password rate limiting are unported, already tracked elsewhere in this
  row and in the lostcon ledger note); and the `NewArea` cross-server
  redirect (`player_to_server`) remains a reject stub pending the
  dedicated "Cross-area transfer" task.

## Ralph Loop - PostgreSQL Login Hardening: Duplicate/TooManyBadPasswords (iteration 35)

- Closed the "`LoginOutcome::Duplicate`/`TooManyBadPasswords` are defined
  but never constructed" gap left by the previous iteration.
- Added `migrations/0004_bad_passwords.sql`: a `bad_passwords` table
  (`id`, `ip`, `created_at`) mirroring C's `badip` table, whose schema is
  documented as a trailing SQL comment in `src/system/badip.c`.
- Ported C `is_badpass_ip` (`badip.c:56-72`) as `is_ip_rate_limited`
  (`crates/ugaris-db/src/character.rs`): counts `bad_passwords` rows for
  the login IP in three sliding windows (60s/3600s/86400s) via one SQL
  query with `count(*) filter (where ...)`, then applies the exact C
  thresholds (`>3`, `>8`, `>25` respectively, strict greater-than not
  greater-or-equal) through a small pure helper,
  `is_badpass_counts_rate_limited`, extracted specifically so the
  threshold logic is unit-testable without a live database connection.
  Called from `CharacterRepository::begin_login` before the row-lookup
  transaction even opens, matching C `load_char`'s `is_badpass_ip` guard
  which runs before `START TRANSACTION`.
- Ported C `add_badpass_ip` (`badip.c:78-85`) as
  `record_bad_password_attempt`: inserts one `bad_passwords` row for the
  IP. Wired into `begin_login_tx` only in the branch where an existing
  character row was found but the password comparison fails - matching
  C `load_char_pwd` returning `tmp==1` (`database_character.c:876-877`,
  `if (tmp == 1) { login_passwd(); add_badpass_ip(login.ip); }`) - and
  deliberately *not* wired into the "no such character name" branch
  (`row` is `None`), preserving the existing anti-enumeration behavior:
  probing random usernames does not arm the rate limiter, only repeated
  wrong-password attempts against a real account do, exactly like C.
- Ported C `load_char_dup` (`database_character.c:731-753`) inline in
  `begin_login_tx`: a `select count(*) from characters where account_id =
  $1 and id != $2 and current_area != 0` query (constant
  `BEGIN_LOGIN_TX_DUPLICATE_SQL`) run after the password/locked/paid
  checks and before the area-resolution branch (matching C's call-site
  order in `load_char`), returning `LoginOutcome::Duplicate` if any other
  character on the same account is currently online. Carried over C's
  `if (sID == 1) return 1; // hack for easier testing` exemption as
  `account_id != 1` (Postgres bigserial `accounts.id` starts at 1, same
  as C's subscriber-ID convention).
- Read `clean_badpass_ips` (`badip.c:88-93`) and grepped the full C tree
  for its call sites: zero results outside its own declaration/definition
  - it is genuine dead code in the legacy server, so it was intentionally
  left unported rather than adding an unreachable Rust equivalent.
- 5 new tests in `crates/ugaris-db/src/character.rs`:
  `badpass_ip_rate_limit_matches_legacy_thresholds` (all three window
  boundaries as strict `>`, and that any single window tripping is
  sufficient independent of the others),
  `badpass_ip_sql_scopes_to_the_three_legacy_windows_for_one_ip`,
  `duplicate_login_query_excludes_self_and_scopes_to_online_characters`.
- `cargo fmt --all` / `cargo test --workspace` (1130 core + 12 db + 3 net
  + 33 protocol + 368 server tests, all green, zero warnings) / `cargo
  build -p ugaris-server` clean with zero warnings; boot-smoked past tick
  230 with no panics.
- REMAINING (task stays `[~]`): mocked-pool/`DATABASE_URL`-gated tests
  exercising `begin_login_tx`'s full row-decision branching (including
  the new duplicate/rate-limit branches) against a real Postgres, and a
  live end-to-end TCP reject test, both remain blocked on a Postgres
  instance not available in this environment (out of scope to add a
  mocking dependency per the "do not update dependencies" rule). The
  `NewArea` cross-server redirect stays a separate deferred task.

## Ralph Loop - PostgreSQL Login Hardening: Live `begin_login_tx` DB Tests (iteration 36)

- Closed the last remaining gap on the "PostgreSQL login hardening" task:
  `ugaris-db/src/character.rs` had no test exercising `begin_login_tx`'s
  row-decision branching against a real database - only pure helper
  functions were unit-tested. Discovered a local Docker daemon with a
  cached `postgres:16-alpine` image was available in this environment
  (previous iterations had recorded "no local Postgres" as a hard
  blocker, which was true for a native `psql`/`pg_ctl` install but not
  for Docker), so used it to both build and verify a real
  `DATABASE_URL`-gated test suite instead of leaving the gap open.
- Added `crates/ugaris-db/src/character.rs::tests::live_login`, a
  12-test module covering every `begin_login_tx` branch: unknown
  character name -> `WrongPassword`; wrong password against a real
  account -> `WrongPassword` + asserts exactly one `bad_passwords` row
  was recorded (C `add_badpass_ip`); locked character -> `Locked`;
  locked account -> `Locked`; ip-locked account -> `IpLocked`; unfixed
  account -> `AccountNotFixed`; not-paid account -> `NotPaid`;
  `allowed_area <= 0` -> `InternalError`; another online character on the
  same account -> `Duplicate`; the same scenario with `account_id == 1`
  -> NOT `Duplicate` (C's `sID == 1` "hack for easier testing" exemption,
  `database_character.c:731-753`); `allowed_area != request.area_id` ->
  `NewArea` with the correct `area_id`/`mirror`; and the success path ->
  `Ready` with the `characters.current_area` row updated and exactly one
  `login_sessions` row inserted.
- Test isolation: each test opens its own `Transaction`, takes a
  transaction-scoped `pg_advisory_xact_lock` (auto-released on
  commit/rollback) to serialize against sibling live tests, resets
  `accounts_id_seq` to a test-specific offset via `setval` (safe because
  Postgres sequences are not rolled back, so this is race-free even
  though the tests never commit), inserts fixture `accounts`/`characters`
  rows, calls `begin_login_tx` directly, asserts, and always rolls back -
  zero manual cleanup, fully idempotent, safe to re-run against the same
  database indefinitely. The `account_id == 1` exemption test resets the
  sequence to land exactly on id 1 deterministically instead of relying
  on database insertion order.
- Added `tokio` under `[dev-dependencies]` in `crates/ugaris-db/Cargo.toml`
  (`tokio.workspace = true`) to get `#[tokio::test]` - `tokio` is already
  a workspace dependency used by other crates, so this only wires an
  existing workspace dependency into `ugaris-db`'s test target; no new
  crate or version was introduced, consistent with the "do not update
  dependencies" rule.
- Verification: `docker run -d postgres:16-alpine`, applied all four
  `migrations/*.sql` files by hand with `psql` against the fresh
  database, then ran `DATABASE_URL=postgres://... cargo test -p
  ugaris-db` - all 24 tests (12 pre-existing + 12 new live tests) passed,
  repeated 3x with default parallel test threads with no flakiness, then
  destroyed the container. Without `DATABASE_URL` set (this repo's
  default/CI state), the same 12 live tests compile and pass by skipping
  early (`connect()` returns `None` when the env var is unset), so
  `cargo test --workspace` stays fully green with no live Postgres
  present: 1130 core + 24 db + 3 net + 33 protocol + 368 server tests,
  zero warnings. `cargo fmt --all` clean. `cargo build -p ugaris-server`
  clean with zero warnings. Boot-smoked past tick 228 with no panics.
- Task marked `[x]` in `PORTING_TODO.md`: every literal requirement in
  the task description (legacy `SV_EXIT` reject text, anti-enumeration
  character-creation behavior, and "extend `character.rs` tests ...
  otherwise gate live tests behind `DATABASE_URL`") is now satisfied.
  Two minor items remain genuinely out of scope and are noted in the
  ledger row rather than blocking completion: automated (vs. this
  iteration's manual) migration-application verification, and a true
  end-to-end reject test over a real TCP socket (the `SV_EXIT`
  payload/dispatch wiring itself is already covered by
  `login_reject_message` unit tests in `crates/ugaris-server/src/
  login.rs`). The `NewArea` cross-server redirect remains a separate
  deferred "Cross-area transfer" task.

## Ralph Loop - Merchant Store DB Persistence (Iteration 37)

- Ported C `src/system/database/database_merchant.c`
  (`load_merchant_inventory`/`save_merchant_inventory`; the incremental
  `merchant_tasks.c` task queue was intentionally not ported - see below)
  so merchant stores survive a server restart instead of always
  regenerating from the zone-file `always` stock.
- Added `migrations/0005_merchant_stores.sql`: a single `merchant_stores`
  table keyed by `(merchant_name, merchant_x, merchant_y)` like C's
  `merchant_items`/`merchant_gold` pair, but storing the whole ware list
  (item + count + always flag) as one `jsonb` column per merchant instead
  of one row per ware slot - C hand-rolls `drdata_to_json`/
  `modifiers_to_json` string builders because MySQL had no native JSON
  binding convenience for it in that codebase; Postgres/`sqlx::types::Json`
  make that unnecessary since `ugaris_core::entity::Item` already derives
  `Serialize`/`Deserialize` (same trick `character.rs` uses for
  `character_json`/`item_json`).
- Added `crates/ugaris-db/src/merchant.rs`: `MerchantWareSnapshot`,
  `MerchantStoreSnapshot`, `MerchantRepository` trait +
  `PgMerchantRepository` (`save_store`/`load_store`), mirroring `area.rs`'s
  minimal repository shape (no transactions needed - each call is a single
  upsert/select). Registered as `Database::merchants()` in `lib.rs`.
  `Item` has no `PartialEq`, so neither does `MerchantWareSnapshot`/
  `MerchantStoreSnapshot`; tests compare via `serde_json` serialization
  instead of `assert_eq!` on the structs directly.
- Wired into `crates/ugaris-server/src/merchants.rs`:
  `merchant_store_snapshot` (world -> DB snapshot, C
  `save_merchant_inventory`'s field mapping) and
  `apply_merchant_store_snapshot` (DB snapshot -> world, C
  `load_merchant_inventory`'s full gold/pricemulti/ware overwrite) as pure
  conversion helpers, plus `save_merchant_store_if_configured` (a no-op
  when `--database-url`/`DATABASE_URL` wasn't set).
- Wired into `crates/ugaris-server/src/main.rs`: added a
  `merchant_repository: Option<PgMerchantRepository>` built alongside the
  existing `character_repository` from the same `ugaris_db::Database`
  connection. C `create_store`'s "try `load_merchant_inventory`, else
  `queue_merchant_full_save`" is ported by diffing
  `world.merchant_stores.keys()` before/after each tick's
  `world.process_merchant_actions()` call (since `ensure_merchant_store`
  only actually creates a store once per merchant lifetime, this diff
  reliably finds only newly-created stores without needing a dirty flag);
  for each, `load_store` is awaited and applied on a hit, or
  `save_store` is awaited with the just-built initial snapshot on a miss.
  The `Container` (buy) and `FastSell` (fast-sell) command handlers both
  call `save_merchant_store_if_configured` after a successful trade -
  Rust has no equivalent of C's `merchant_tasks.c` background task queue
  (`queue_merchant_item_add`/`_remove`/`_update`/`_gold_update`,
  processed later by `process_pending_merchant_updates`), so this instead
  follows C's *own* `add_item_to_merchant`/`remove_item_from_merchant`/
  `update_merchant_item` helpers, which are themselves "simple
  implementation - just save the entire inventory" full-store saves; the
  behavior is equivalent, just with more I/O per trade than C's targeted
  incremental row updates.
- Tests: `crates/ugaris-db/src/merchant.rs` - a pure JSON round-trip test
  (no database needed) plus a `mod live` (following `character.rs`'s
  `live_login` convention exactly: `DATABASE_URL`-gated, skips instead of
  failing when unset/unreachable) with two tests - save-then-load round
  trips gold/pricemulti/wares, and loading an unknown merchant returns
  `None`. `crates/ugaris-server/src/tests/merchants.rs` - 4 new tests for
  the conversion helpers: snapshot captures name/position/gold/wares
  correctly, snapshot is `None` without a store, applying a snapshot
  overwrites gold/pricemulti/wares, and applying a snapshot with
  out-of-range ware slots doesn't panic and leaves in-range slots alone.
- Verification: `cargo fmt --all` clean. `cargo test --workspace`: 1130
  core + 27 db (24 pre-existing + 3 new merchant tests) + 3 net + 33
  protocol + 372 server (368 pre-existing + 4 new merchant tests), zero
  warnings, zero failures. `cargo build -p ugaris-server` clean. Because
  this change touches the tick loop and DB wiring, did a full live
  end-to-end check beyond the required boot-smoke: spun up a throwaway
  local `postgres:16-alpine` Docker container, applied all five
  `migrations/*.sql` files, ran `DATABASE_URL=... cargo test -p
  ugaris-db` (all 27 tests green for real, including the 2 live merchant
  tests actually round-tripping through Postgres, confirmed the test row
  is deleted afterward so re-runs don't accumulate rows), then ran the
  actual `target/debug/ugaris-server` binary against that same database
  twice: first run logged `saved initial merchant store to database` for
  all three zone-1 merchants (Egbert/Fred/Dolf) with 108-slot ware arrays
  persisted; second run (unmodified DB) logged `loaded merchant store
  from database` for all three instead of re-saving, confirming the
  load-else-save branch actually round-trips through a live database, not
  just through the pure-Rust unit tests. Also ran the plain
  boot-smoke without `DATABASE_URL` (matching the required recipe): past
  tick 230 with `DATABASE_URL not set; starting without persistence`
  logged once and no panics, confirming the feature is fully optional.
  Destroyed the Docker container afterward.
- Task marked `[x]` in `PORTING_TODO.md`. REMAINING (noted in both the
  todo entry and the ledger table row above): (1) store position is keyed
  off `character.x/y` at store-creation time, not C's `tmpx`/`tmpy` -
  day/night shop relocation (`MerchantDriverData.dayx/nightx`/etc.) is
  still unported per `world/merchant.rs`'s existing module doc, so a
  future day/night-move port needs to re-key or move the persisted row
  too; (2) `add_special_store` (still unported - see the "Special stores"
  task) doesn't trigger a save; (3) C's periodic `save_all_merchants`
  full-DB sweep and the admin `#saveall` command aren't wired to the new
  repository (every trade already self-saves, so this is a smaller gap
  than in C, mainly relevant for merchants that were never traded with
  after a restock); (4) the incremental per-item task queue
  (`merchant_tasks.c`) itself is intentionally not ported, as explained
  above.

## Ralph Loop - Client Command Audit Completion (Iteration 38)

- Closed the P1 "Client command audit completion" task: `ClientAction::
  Nop`, `ClientInfo`, `Log`, and `ModPacket` were parsed from the wire
  (`crates/ugaris-protocol/src/command.rs`) but never explicitly matched
  anywhere downstream - they silently fell through a catch-all `_ => {}`
  in both `crates/ugaris-server/src/main.rs`'s per-tick dispatch and
  `player_actions.rs::apply_player_action`'s immediate dispatch, with
  only a generic raw-opcode `"action queued for gameplay port"` log line
  ever touching them.
- Read the C oracle in full: `cl_nop` (`src/system/player.c:681-683`) is a
  literal `;` no-op used as a keep-alive filler packet, unlogged in C.
  `cl_clientinfo` (`player.c:1339-1350`) has its *entire* body commented
  out - the `client_info` payload (skip/idle counters, sysmem/vidmem,
  display surfaces) is parsed and discarded, also unlogged in C. Both
  therefore got explicit non-logging match arms (matching the existing
  `FightMode` no-op precedent already in the tick-loop match) instead of
  a log line, since C itself has no log call for either.
- `cl_log` (`player.c:1218-1226`) forwards the client-supplied message to
  `charlog` (`src/system/logging/log.c:122-146`), which formats
  `"<name> (<cn>): <message> [ID=<charID><,IP=a.b.c.d>]"` and writes it
  via `xlog`. Ported as a new pure helper,
  `player_actions::format_client_log_message(name, id, message)`,
  reproducing the `name (id): message [ID=id]` shape and called from a
  `debug!` trace line in the tick-loop `Log` arm; the optional `,IP=...`
  suffix is intentionally omitted since `ServerRuntime` doesn't currently
  track each session's peer address alongside its character (noted as a
  documented simplification, not silently dropped).
- `cl_mod1`/`cl_mod3` (`player.c:1421-1504`, `mod_packet.c`) route
  handshake subtypes `0x01-0x0F` (pong/mod-version/mod-ready) through a
  blind acknowledge - the C comment literally reads "For now, just
  acknowledge we received them / Future: track mod version, handle pong
  responses, etc." - and route anti-cheat subtypes `0x10-0x2F` to
  `ac_handle_packet`, not yet ported. A `debug!`-logged no-op for
  `ClientAction::ModPacket { packet_type, subtype, .. }` is therefore a
  faithful port of the C oracle's own present-day stub, not a Rust-side
  gap. Checked both the community client and the fuller client repo:
  neither currently sends any `CL_MOD1` traffic, confirming no live
  client exercises this path yet.
- Also updated `player_actions.rs::apply_player_action` (the immediate,
  synchronous dispatch called from `ServerRuntime::queue_action`) to give
  all four variants one explicit arm (`Nop | ClientInfo(_) | Log(_) |
  ModPacket { .. } => {}`) instead of relying on the generic
  `action_to_queued` fallthrough - behaviorally identical (none of the
  four ever produced a queued driver action), but no longer silent by
  omission.
- Tests: `crates/ugaris-server/src/tests/player_actions.rs` - added
  `format_client_log_message_matches_legacy_charlog_shape` (pins the
  exact `charlog`-derived string shape) and
  `apply_player_action_ignores_nop_client_info_log_and_mod_packet` (all
  four variants round-trip `apply_player_action` and `action_to_queued`
  without mutating `PlayerRuntime` or producing a queued action).
- Verification: `cargo fmt --all` clean. `cargo test --workspace`: 1130
  core + 27 db + 3 net + 33 protocol + 374 server (372 pre-existing + 2
  new tests), zero warnings, zero failures. `cargo build -p ugaris-server`
  clean, zero warnings. Boot-smoke: `target/debug/ugaris-server
  --bind-addr 127.0.0.1:5557` logged `legacy TCP listener ready`,
  `loaded area zone map ...`, then `entering Rust game loop` with no
  panics.
- Task marked `[x]` in `PORTING_TODO.md`. REMAINING (noted in both the
  todo entry and the ledger table row above): `CL_MOD2`/`CL_MOD4`/
  `CL_MOD5` and unknown `CL_MOD1`/`CL_MOD3` subtypes still hard-disconnect
  the session at the decoder layer (`crates/ugaris-protocol/src/
  client.rs`) instead of C's "trash the input bytes, keep the connection
  alive" behavior, and several `CL_MOD1` handshake packet sizes in
  `mod_packet_size()` don't match the current C `mod_system.h`/
  `mod_anticheat.h` struct sizes. Neither is observed in practice today
  (no current client sends these packet types), but both should be fixed
   as a separate framing-layer task before the mod/anti-cheat protocol is
   actually driven end to end.

## Ralph Loop - Special Stores (`add_special_store`/`create_special_item`)

- Ported the P1 "Special stores" task in full. C `create_special_item`
  (`src/system/tool.c:2620-2789` - the task's `create.c` reference was
  stale) builds one randomly-enchanted item: an optional potion branch,
  the 21-entry `ITEM_TYPES[]` base-item roll (armor/helmet/sleeves/
  leggings/sword/twohanded/dagger/staff at a remapped quality tier, or one
  of 13 fixed rings/hats/capes/accessories), a non-gaussian `lowhi_random`
  strength roll, a weighted roll over the 76-entry `special_item[]` table
  (transcribed verbatim - counted directly from the C source with `awk`;
  the task description's "72 entries" estimate was off by 4), and
  `set_item_requirements_sub` (level/Arch-class gating from the item's
  highest modifier value). `add_special_store`
  (`src/module/merchants/store.c:229-323`) rolls strength 1-22 (reused
  directly as `create_special_item`'s strength argument) and a derived
  `base` tier, calls `create_special_item(str, base, 1, 1000)` (never a
  potion, "no junk" tier), and adds the item to the merchant's store.
  `merchant_driver` (`merchant.c:337-347,546-548`) seeds five special
  wares the first time a `special`-flagged merchant's store is created,
  then adds one more every 12 real-time hours via `dat->lastadd`.
- Rust: new `crates/ugaris-core/src/world/special_item.rs` -
  `World::create_special_item`/`World::add_special_store`/
  `World::refresh_special_stores`, reusing the existing
  `legacy_random_below_from_seed` C-style LCG (already used by
  `add_item_to_merchant_store`'s random-overwrite fallback) and the
  previously-parsed-but-unused `MerchantDriverData::special`/
  `last_special_add` fields. Added `pub const IID_GENERIC_SPECIAL` to
  `item_driver/ids.rs` (`0x0100002C`, C `IID_GENERIC_SPECIAL`). Threaded a
  new `&mut ZoneLoader` parameter only through the three new methods, not
  through `ensure_merchant_store`/`process_merchant_actions` (which have
  many pre-existing call sites/tests that don't need item-template
  instantiation) - `World::refresh_special_stores` is a new top-level
  per-tick entry point called from `main.rs` right after
  `process_merchant_actions()`, using the `last_special_add == 0` sentinel
  to detect "never seeded yet" (mirroring the existing
  `clear_expired_merchant_memory` 12h-tick-comparison idiom already in
  `world/merchant.rs`) and returning the set of merchants whose store
  changed so `main.rs` can persist them through the existing
  `save_merchant_store_if_configured` helper (C: each successful
  `add_special_store` call ends with its own `queue_merchant_full_save`).
- This is also the first `ugaris-core` `World` method to take a
  `&mut ZoneLoader` parameter directly (every prior item-template
  instantiation call site lived in `ugaris-server`, e.g. `chests.rs`) -
  worth knowing if a future task extends this pattern further, since
  `create_special_item` is explicitly meant for chest/loot reuse too per
  its own C doc comment (not wired to that system yet).
- Tests: `crates/ugaris-core/src/world/tests/special_item.rs` (8 tests).
  The core equipment-roll test locks in the exact deterministic output
  (item name, description text, value, modifier slot/value, `min_level`,
  `template_id`) for a fixed RNG seed, cross-checked against a standalone
  Python replica of the LCG and weighted-table-roll algorithm before
  writing the assertions - it matched on the first `cargo test` run, with
  no discrepancies to debug. Also covers the potion branch returning an
  unmodified template, a missing-template `None` result,
  `add_special_store` requiring an existing store, a single successful
  add, the full seed-five/no-op-same-tick/refresh-after-12h sequence, and
  a `special == 0` no-op. Verified real `ugaris_data` templates exist for
  every one of the 21 `ITEM_TYPES` entries and all three potion families,
  and that two real zone files (`zones/12/mine.chr`, `zones/31/
  mineshop.chr`) use `special=1` in production data, confirming this
  wasn't dead configuration.
- Verification: `cargo fmt --all` clean. `cargo test --workspace`: 1137
  core (8 new) + 27 db + 3 net + 33 protocol + 374 server, zero warnings,
  zero failures. `cargo build -p ugaris-server` clean, zero warnings.
  Boot-smoke: `target/debug/ugaris-server --bind-addr 127.0.0.1:5557`
  logged `entering Rust game loop area_id=1` with no panics past tick 350
  (area 1 has no `special=1` merchants, so `refresh_special_stores`
  correctly no-ops there every tick).
- Task marked `[x]` in `PORTING_TODO.md`. REMAINING (noted in both the
  todo entry and the ledger table row): `create_special_item` is not yet
  wired to chest/loot generation (`create.c:1102`'s `special_prob`/
  `special_str`/`special_base` template fields, parsed but discarded by
  the zone loader today); the aclerk auction NPC's duplicate
  special-store logic is unported since aclerk itself has no driver yet;
  and on the very first tick a brand-new special store is created, both
  the new explicit save and the pre-existing `newly_created_stores`-diff
  DB-load/save loop may both touch the same merchant in the same tick
  (harmless, but worth knowing if the merchant-persistence flow is
  touched again).

- P2 "Generic NPC text analysis (`analyse_text_driver`)" - ported a
  reusable keyword-matcher into `character_driver.rs` shared by every
  `analyse_text_driver` copy in the C tree (`merchant.c`, `gwendylon.c`,
  `bank.c`, `base.c`, `military.c`, `forest.c`, `area3.c`, `arkhata.c`,
  `orb_bank_npc.c`): `TextQaEntry`/`TextAnalysisOutcome`/
  `tokenize_text_words`/`analyse_text_qa`/`format_qa_answer`. The
  tokenizer matches C's delimiter set (`' ' ',' ':' '?' '!' '"' '.'`),
  own-name filtering (`strcasecmp` against the NPC's name), 20-word cap,
  and 250-byte-per-word bailout; matching requires exact word-count
  parity with a qa entry (C's `n == w && !qa[q].word[n]` check), not a
  prefix match. C's guard clauses (system/info log filter, self-talk,
  player flag, distance, visibility) need `World` state this module
  doesn't have, so they're the caller's responsibility - documented in
  the function doc comment. Transcribed `merchant.c`'s 13-row `qa[]`
  table verbatim as `MERCHANT_QA` and wired it into
  `world::merchant::process_merchant_messages` via new
  `merchant_qa_reply`/`merchant_quiet_say` helpers, applying the C guard
  clauses (`CF_PLAYER`, `char_dist <= 12`, `char_see_char`) before
  matching. Also fixed the new `quiet_say` distance to use
  `settings.quietsay_dist` (C: `quietsay_dist = SAYDIST/3`) instead of
  reusing `SAY_DIST`; left the pre-existing greet-on-sight message's use
  of `SAY_DIST` alone since fixing it is out of this task's scope.
  Skipped the C prefix-skip step (name+verb stripped from a fully
  formatted `"Name says: \"text\""` log line before tokenizing) because
  Rust's `push_driver_text_message` already stores only the bare spoken
  text - replicating the literal C skip logic against unprefixed text
  would corrupt the first word of one-word messages, so tokenization
  starts directly on the raw text instead, and the final word is always
  flushed (C only flushes on a following delimiter, which the always-quoted
  C log line guarantees but our unquoted text does not).
  Added 7 focused unit tests in `character_driver.rs` (keyword hit, case
  insensitivity, no-match incl. empty word list, own-name gating, exact
  word-count requirement, `answer_code`-only entries, oversized-word
  bailout) and 3 world-level tests in `world/tests/merchant.rs`
  (qa reply fires and is visible in `pending_area_texts`, non-player
  speakers are ignored, speakers beyond distance 12 are ignored).
  Verification: `cargo fmt --all` clean; `cargo test --workspace`: 1147
  core + 27 db + 3 net + 33 protocol + 374 server, zero warnings, zero
  failures. `cargo build -p ugaris-server` clean. Boot-smoked
  `target/debug/ugaris-server --bind-addr 127.0.0.1:5556` for 10s, no
  panics.
  Task marked `[~]` in `PORTING_TODO.md`: only `merchant.c`'s table is
  wired to a live driver; the other seven `qa[]` tables (and the
  `mem_*` driver-memory system from the next P2 task) still need
  porting.

- P2 "Driver memory (`mem_*`)" - ported C `mem_add_driver`/
  `mem_check_driver`/`mem_erase_driver` (`src/system/drvlib.c`, declared
  in `drvlib.h` - the task description's `src/system/mem.c` reference was
  stale; that file is an unrelated `xmalloc`/`xfree` allocator-tracking
  module). C's `struct char_mem_data` is a per-character, 8-slot (`nr`
  0..=7) list of remembered character identifiers addressed via
  `set_data(cn, DRD_CHARMEM + nr, ...)`; ported as `character_driver::
  DriverMemory` (`slots: [Vec<u32>; 8]`, `Default` via
  `std::array::from_fn`) plus free functions mirroring C exactly:
  out-of-range slots are a no-op (`false`/nothing), duplicate adds don't
  create a second entry (still return `true`), and erase only clears the
  targeted slot. C dedupes membership by a stable identity (`ch[co].ID |
  0x80000000` for players else `ch[co].serial & 0x7fffffff`) that
  survives character-table slot reuse; kept the existing merchant-greet
  port's simplification of using the raw runtime `CharacterId` instead
  (documented inline) rather than widen scope to thread persistent player
  IDs through. Added `driver_memory: DriverMemory` directly on
  `Character` (`entity.rs`) - not nested under the per-driver-kind
  `CharacterDriverState` enum the task description suggested, since C
  addresses memory slots per-character regardless of which module owns
  the character, matching how `driver_state`/`driver_messages` already
  sit directly on `Character`. Rewired `world/merchant.rs`'s greet-once
  tracking off the old `MerchantDriverData::greeted: Vec<u32>` field onto
  `mem_add_driver`/`mem_check_driver`/`mem_erase_driver` at slot 7 (C's
  literal `mem_add_driver(cn, co, 7)` call sites in `merchant.c`),
  keeping `MerchantDriverData::memory_clear_tick` as the driver's own
  timeout bookkeeping (C's `dat->memcleartimer`, which is caller-side,
  not part of `mem_*` itself). Adding the new `Character` field required
  updating every other test/production `Character { .. }` struct literal
  across both crates (`ugaris-core`, `ugaris-db`, `ugaris-server`) to
  initialize it.
  Tests: 6 new focused unit tests in `character_driver.rs` (check before
  add, add-then-check with unrelated-slot/unrelated-target isolation,
  duplicate-add idempotency via a slot-length assertion, out-of-range
  slot rejection for both add and check, erase-only-clears-requested-
  slot, and erase-on-out-of-range-slot silent no-op), plus updated the
  existing merchant greet/small-talk tests' `merchant_npc_already_greeted`
  helper to seed slot 7 via `mem_add_driver`.
  Verification: `cargo fmt --all` clean; `cargo test --workspace`: 1153
  core (6 new) + 27 db + 3 net + 33 protocol + 374 server, zero warnings,
  zero failures. `cargo build -p ugaris-server` clean. Boot-smoked
  `target/debug/ugaris-server --bind-addr 127.0.0.1:5556` for 10s,
  "entering Rust game loop" logged, no panics.
  Task marked `[x]` in `PORTING_TODO.md`. The "Generic NPC text analysis"
  task above is also now marked `[x]`: its actual deliverable (the
  reusable `analyse_text_qa` matcher) was already complete, the `mem_*`
  dependency it was waiting on is done, and its remaining per-NPC `qa[]`
  tables are each properly owned by their own already-tracked NPC/area
  porting tasks (`CDR_BANK`, `CDR_TRADER`, Military ranks, Areas 1/3/16/
  37) rather than needing separate tracking here.

### Ralph Loop - NPC Speech Helpers (`quiet_say`/`say`/`emote`/`murmur`) (Iteration 42)

- Ported the P2 "`quiet_say`/`say`/`emote` NPC speech helpers in core"
  task. C `src/system/talk.c`: `say()` (`"%s says: \"%s\""` at
  `say_dist`, and notably its `strchr(buf, '"')` quote-rejection check is
  commented out - unlike the other three), `quiet_say()` (identical text,
  `quietsay_dist`, quote-rejected), `emote()` (`"%s %s."` at
  `emote_dist`, quote-rejected), and `murmur()` (`"%s murmurs:
  \"%s\""` - reuses `whisper_dist`, it has no distance constant of its
  own - quote-rejected).
- Rust: added `World::npc_say`/`npc_quiet_say`/`npc_emote`/`npc_murmur`
  to `crates/ugaris-core/src/world/text.rs`. Each looks up the
  character's name/position, formats the message via a `log_text.rs`
  helper, and pushes a `WorldAreaText` onto the pre-existing
  `pending_area_texts` queue (already drained every tick by
  `crates/ugaris-server/src/world_events.rs::send_pending_world_area_texts`)
  at the C-matching `GameSettings` distance field. Added
  `murmur_message`/`quiet_say_message` to `crates/ugaris-core/src/
  log_text.rs`, joining the pre-existing `say_message`/`emote_message`/
  `whisper_message`/`shout_message`/`holler_message`.
- Migrated the three existing ad-hoc `pending_area_texts.push` call
  sites the task pointed at onto the new helpers, uncovering two latent
  bugs along the way (both fixed, not just migrated):
  - `world/lab2_undead.rs`'s `queue_lab2_undead_say` wrapper pushed the
    raw message text with no `"<name> says: \"...\""` wrapper at all,
    even though every one of its 4 call sites corresponds to C
    `say(cn, "Arrgh!")`/`say(cn, "Mwahahahaha...")`/etc., which always
    wraps. Removed the wrapper; call sites now call `self.npc_say(...)`
    directly. Fixed the 4 now-more-correct-failing unit tests in
    `world/tests/lab2_undead.rs` to assert the wrapped text (matches the
    Hard Rules: fix the test, don't weaken the port, when C proves the
    test wrong).
  - `world/merchant.rs`'s greeting message ("Hello %s! If you'd like to
    trade...") was built with `say_message` at `SAY_DIST`, but C's
    `merchant.c` greeting is actually `quiet_say(cn, "Hello %s! ...")`
    (confirmed by reading the C source directly) - same wire text either
    way (no quotes in the message), but the wrong (too large) broadcast
    distance. Switched to `self.npc_quiet_say(...)`. The small-talk qa
    reply (already correctly `quiet_say` per the previous iteration's
    ledger entry) now goes through the same shared helper instead of its
    own bespoke `merchant_quiet_say` method, which was deleted.
  - `world/npc_idle.rs`'s potion-drink message (`emote(cn, "drinks a
    potion")` per `simple_baddy.c`/`arkhata.c`) now calls
    `self.npc_emote(...)` instead of manually building the
    `emote_message`/`SAY_DIST/2` pair inline.
- Tests: 4 new unit tests in `world/tests/text.rs` (`npc_say` never
  rejects a `"` and uses `say_dist`; `npc_quiet_say`/`npc_emote`/
  `npc_murmur` each reject a `"` - dropping the queued message - and use
  their respective distance field), plus fixed the 4 `lab2_undead.rs`
  tests above.
- Verification: `cargo fmt --all` clean. `cargo test --workspace`: 1158
  core (5 net new after removing/fixing existing ones) + 27 db + 3 net +
  33 protocol + 374 server, zero warnings, zero failures. `cargo build
  -p ugaris-server` clean, zero warnings. Boot-smoked
  `target/debug/ugaris-server --bind-addr 127.0.0.1:5556` for 10s,
  "entering Rust game loop" logged, no panics.
 - Task marked `[x]` in `PORTING_TODO.md`. REMAINING: `holler`/`shout`/
   `whisper` NPC-side helpers not added (nothing in the NPC driver tree
   calls them yet - only player local speech in
   `crates/ugaris-server/src/commands_chat.rs` does); add them the same
   way if a future NPC driver needs to holler/shout/whisper. The merchant
   greeting is also still missing C's `COL_LIGHT_BLUE`/`COL_RESET` color
   codes around the trade phrase - a separate, pre-existing content gap
   left out of this task's scope (distance/helper-choice only).

## Ralph Loop - Idle NPC Chatter (Iteration 43)

- Ported C `merchant_driver`'s idle-murmur block
  (`src/module/merchants/merchant.c` lines ~463-540, `qa[]`'s neighbor):
  once per in-game minute (`ticker > dat->last_talk + TICKS * 60`), a
  1-in-25 `RANDOM(25)` roll picks a random flavor line via
  `RANDOM(max_case + 1)`. Merchants named "Lori" (case-insensitively, per
  C's `strcasecmp`) get 4 extra mine-only cases (`max_case = 20` instead
  of 16). Transcribed all 21 cases digit-for-digit/letter-for-letter,
  including the literal capitalization quirk in case 20's
  `"Flips %s coins"` emote and the indoor-ceiling-vs-outdoor-sky branch
  (`map[...].flags & MF_INDOORS`).
- Added the previously-missing `World::npc_whisper` speech helper
  (`crates/ugaris-core/src/world/text.rs`, C `whisper()` in
  `src/system/talk.c:296` - `"<name> whispers: \"<text>\""` at
  `whisper_dist`, quote-reject guard) since case 1 of the murmur table
  needed it; the only other NPC speech helpers before this
  (`npc_say`/`npc_quiet_say`/`npc_emote`/`npc_murmur`) didn't cover it.
  Also added `hisname` (C `src/system/tool.c:1488`: lowercase
  his/her/its possessive pronoun by `CF_MALE`/`CF_FEMALE`/neuter flags,
  the lowercase sibling of the pre-existing `look_character_hename`
  which mirrors `Hename`) since several cases need `hisname(cn)`.
- New `world/merchant.rs::merchant_idle_chatter` (private, wired into
  `World::process_merchant_actions` after `greet_nearby_players` and
  before the memory-clear timer, matching C's block order) reuses the
  existing `MerchantDriverData::last_talk` field (already present from
  an earlier iteration, previously unused) and the same
  `legacy_random_below_from_seed(&mut self.legacy_random_seed, n)`
  pattern every other RNG-driven world driver in this codebase uses.
  Added `pub(crate) const MERCHANT_TALK_INTERVAL_TICKS` (C's
  `TICKS * 60`) alongside the pre-existing `MERCHANT_MEMORY_CLEAR_TICKS`.
- Scope note: the todo item also mentions "citizen equivalents" - C has
  at least 8 more `RANDOM(25)` idle-murmur blocks (`bank.c::bank_driver`,
  `orb_bank_npc.c`, `base.c::trader_driver`, a *second* distinct one in
  `merchant.c::aclerk_driver` at line ~800, `area3.c`, `clanmaster.c`,
  `tunnel.c`, `gwendylon.c`, `sidestory.c`) but every one of those is a
  whole unported NPC driver in its own right with its own P2/P3/P4 todo
  item (bank -> "`CDR_BANK`"; trader -> "`CDR_TRADER`"; aclerk ->
  "Aclerk / auction NPC"; area3/gwendylon -> P4 area tasks; clanmaster ->
  P4 Area 30; tunnel -> P4 Area 33), so porting each one's table is
  properly scoped as part of that driver's own port - the same call the
  "Generic NPC text analysis" task made for its per-driver `qa[]` tables.
  Left `PORTING_TODO.md`'s "Idle NPC chatter" checkbox `[~]` with this
  note rather than `[x]`.
- Tests: 5 new unit tests in `world/tests/merchant.rs`
  (`merchant_idle_chatter_murmurs_on_lucky_roll`,
  `merchant_idle_chatter_stays_quiet_below_talk_interval`,
  `merchant_idle_chatter_skips_unlucky_roll`,
  `merchant_idle_chatter_grants_lori_the_extended_case_range`,
  `merchant_idle_chatter_emote_reflects_indoor_ceiling_vs_outdoor_sky`).
  Since the block is randomness-driven, each test seeds
  `world.legacy_random_seed` to a value pre-computed (via a scratch
  program running the exact same `legacy_random_below_from_seed`
  algorithm) to land on a known `(RANDOM(25), RANDOM(max_case + 1))`
  pair, matching how other legacy-RNG-driven tests in this codebase pin
  the seed instead of asserting on randomness.
- Verification: `cargo fmt --all` clean. `cargo test --workspace`: 1163
  core (10 net new) + 27 db + 3 net + 33 protocol + 374 server, zero
  warnings, zero failures. `cargo build -p ugaris-server` clean, zero
  warnings. Boot-smoked `target/debug/ugaris-server --bind-addr
  127.0.0.1:5556` for 10s, "entering Rust game loop" logged, no panics.

## Ralph Loop - `CDR_BANK` Banker NPC (Iteration 44)

- Closed out the previous iteration's "Idle NPC chatter" `[~]` task as
  `[x]`: its own note already explained the remaining "citizen
  equivalents" are properly scoped as part of each of their own drivers'
  future P2/P4 ports (bank, trader, aclerk, area3/gwendylon/clanmaster/
  tunnel), so there was no standalone follow-up left for that task
  itself.
- Ported C `src/module/bank.c` (`bank_driver`, `bank_dead`, `bank_ppd`)
  end to end as the next topmost P2 task:
  - `character_driver.rs`: `CDR_BANK = 22` (matches
    `src/system/drvlib.h:70`), `BankDriverData` (day/night positions,
    door, store bounds, open/close hours, greet-throttle/memory-clear
    ticks - same shape as `MerchantDriverData`), `parse_bank_driver_args`
    (C `bank_driver_parse`, defaults `open=6`/`close=23` before parsing),
    and `BANK_QA` (C's 15-row `qa[]`, including the literal "Sorry, I'm
    just a merchant" line copy-pasted from `merchant.c` - preserved, not
    "fixed"). `zone.rs` wires `CDR_BANK` spawn-time arg parsing the same
    way `CDR_MERCHANT` already does.
  - `player.rs`: `DRD_BANK_PPD = MAKE_DRD(DEV_ID_DB, 38 | ...)` (matches
    `src/system/drdata.h:100`), `PlayerRuntime::bank_gold` (C `struct
    bank_ppd { int imperial_gold; }`), `encode_legacy_bank_ppd`/
    `decode_legacy_bank_ppd` following the `teufelrat` single-`i32`-field
    codec pattern exactly (offset const, `decode_legacy_ppd_blob` match
    arm, `encode_legacy_ppd_blob` mid-loop arm plus `had_bank`-guarded
    append-if-nonzero tail block).
  - `world/bank.rs` (new, ~430 lines): `World::process_bank_actions`
    ports the full per-tick `bank_driver` body - message loop (`NT_TEXT`
    small talk via the shared `analyse_text_qa` matcher plus raw
    `strcasestr`-style deposit/withdraw/balance substring detection,
    `NT_GIVE` cursor destruction), a periodic-scan greeting (same
    simplification `world/merchant.rs` already established for `NT_CHAR`
    instead of reacting to `notify_area` broadcasts), the 16-line
    `bank_mutterings[]` idle-chatter table with `RANDOM(25)`/`RANDOM(16)`
    throttling (byte-for-byte C text), the 12h greet-memory-clear timer,
    and the day/night shop-position/door movement block. The last part
    needed three primitives with **no prior Rust equivalent anywhere in
    the codebase** (confirmed via targeted research before writing code):
    `is_closed(x,y)` (checks a door item's `drdata[0]` via the existing
    `door_open_state` helper), `is_room_empty(xs,ys,xe,ye)` (linear scan
    over `CF_PLAYER` characters in a bounding box - C's sector-stepped
    loop has no equivalent index in this codebase, same observable
    result), and `opening_time(from,to)` (wrap-around-midnight hour
    gate); `move_driver`/`use_item_at` map onto the pre-existing
    `World::setup_walk_toward`/`World::toggle_door`.
  - Cross-boundary design note (this is the first driver to need it):
    `World` cannot see `PlayerRuntime` (owned by the `ugaris-server`
    session layer), so the persistent `ppd->imperial_gold` balance
    cannot be read or written from inside `World::process_bank_actions`.
    Deposit's "enough carried gold?" check and `Character.gold`
    debit/credit happen synchronously in `World` (mirroring
    `world/merchant.rs::merchant_store_sell`'s existing direct-mutation
    pattern), but the actual persistent-balance mutation and the
    withdraw/balance reply text (which need the *current* balance,
    unknown to `World`) are queued as a new `BankEvent`
    (`Deposit`/`Withdraw`/`Balance`) via `pending_bank_events`/
    `drain_pending_bank_events`, following the exact
    `pending_kill_exp`-style convention already used throughout `World`.
    `crates/ugaris-server/src/world_events.rs::apply_bank_events`
    (mirroring `apply_teufel_rat_death_from_hurt_event`'s `runtime`+
    `world` shape) drains the queue and applies each event to the
    correct `PlayerRuntime`, called from `main.rs`'s tick loop right
    after `process_merchant_actions`.
  - Three documented (not silent) deviations from C, each with an inline
    code comment: (1) the bank's own door toggles directly via
    `toggle_door` rather than the full keyed-door `use_driver` dispatch
    (`item_driver::door_driver`'s key-requirement gate) - no existing
    zone data is expected to key a bank door; (2) the "account"/"explain
    deposit/withdraw/balance" qa answers drop their C
    `COL_LIGHT_BLUE`/`COL_RESET` color styling (the shared `&str`-based
    `analyse_text_qa` pipeline cannot carry the raw non-UTF8 legacy color
    marker byte - a real Rust-string-literal constraint, not a
    convenience shortcut) while keeping the wording byte-for-byte
    identical; (3) `NT_GIVE` unconditionally destroys the received item
    rather than first attempting `give_driver`'s hand-it-back behavior,
    following the same simplification `world/merchant.rs` already
    established (no generic "give item back" helper exists yet).
  - Tests: 17 new focused tests in `world/tests/bank.rs` (arg parsing,
    greet-once memory, qa small talk, the "explain deposit" double-reply
    quirk, deposit success/insufficient-funds/no-amount, withdraw
    queued/no-amount/negative-amount, balance queued, `NT_GIVE`
    destruction, idle-chatter lucky/unlucky-roll seeded RNG, no-day-
    position spawn-tile return+turn, and `opening_time` wrap-around) plus
    2 new PPD round-trip tests in `player.rs` (`bank_ppd_codec_matches_...`,
    `bank_ppd_blob_round_trips_...`).
  - Verification: `cargo fmt --all` clean. `cargo test --workspace`:
    1182 core (19 net new) + 27 db + 3 net + 33 protocol + 374 server,
    zero warnings, zero failures. `cargo build -p ugaris-server` clean,
    zero warnings. Boot-smoked `target/debug/ugaris-server --bind-addr
    127.0.0.1:5556` for 12s, "entering Rust game loop" logged, no
    panics.
  - `PORTING_TODO.md`'s `CDR_BANK` checkbox marked `[x]`.
- Task marked `[~]` in `PORTING_TODO.md` with the scope note above.
- 2026-07-04 (iteration 45): **`CDR_TRADER` player-to-player trade
  middleman NPC** (P2, `[~]` - `CDR_TRADER` itself is fully done,
  `CDR_JANITOR` remains) - ported C `src/module/base.c::trader_driver`
  in full.
  - Added `TraderDriverData` (`character_driver.rs`, `Default`-derived
    since C never parses zone-file args into `struct trader_data`) and
    wired spawn-time initialization for `CDR_TRADER` in `zone.rs`. Added
    `TRADER_QA` transcribing `base.c`'s shared `qa[]` table (also used by
    the unported `CDR_JANITOR`/`CDR_MACRO`) through the existing
    `analyse_text_qa` matcher.
  - New `crates/ugaris-core/src/world/trader.rs`
    (`World::process_trader_actions`) ports: the "trade with
    <name>"/"stop trade"/"accept trade"/"show trade" text-command state
    machine with exact C string matching (including the case-sensitive
    `strstr` quirk - `text` is *not* lowercased for these checks, only
    the qa small talk lowercases internally - and the "accept trade"
    exact-phrase requirement with its own "say it by itself" scold);
    `NT_GIVE` item collection capped at 10 items per side
    (`MAX_TRADER_ITEMS`) with `IF_VOID` marking and cross-partner
    notification; the three-minute timeout (`TRADER_TIMEOUT_TICKS`) with
    automatic item return; `return_items`'s swap-on-deal semantics
    (`switched` flag); the greeting, ported as the same periodic
    nearby-player scan `world/bank.rs`/`world/merchant.rs` already
    established instead of reacting to `NT_CHAR` broadcasts, but
    additionally turning to face the greeted/replied-to player since
    C's `talkdir` mechanic (`offset2dx` + `turn`, called once per tick
    with whichever direction was last set) is an observable part of
    *this* driver's behavior, unlike bank/merchant which never turn; the
    12-line idle-murmur table (`RANDOM(25)`/`RANDOM(12)`); and the 12h
    driver-memory clear timer (slot 7, shared `DriverMemory`/
    `mem_add_driver`/`mem_check_driver`/`mem_erase_driver`).
  - Added `drvlib::offset2dx` (C `tool.c:309-349`, the 8-way
    direction-toward helper) since this is the first ported driver that
    needs the turn-to-face-the-speaker mechanic; existing bank/merchant
    greetings never call `turn`.
  - Two things need `legacy_item_look_text` (lives in the
    `ugaris-server` crate, not `ugaris-core`) and are deferred via a new
    `TraderEvent`/`pending_trader_events` queue (mirroring the existing
    `BankEvent`/`pending_bank_events` convention) applied by
    `crates/ugaris-server/src/world_events.rs::apply_trader_events`: the
    "show trade" item dump (`Trading:`/`For:` headers + per-item look
    text) and the `NT_GIVE` success branches' "`<name>` gave me:"
    cross-notification to the other trading partner. `apply_trader_events`
    needs no `ServerRuntime`, only `&mut World`, since neither event
    touches `PlayerRuntime`.
  - Documented (not silent) deviations from C: (1) `dat->c1ID`/`c2ID`
    (`ch[co].ID`, a player's persistent ID) is the raw runtime
    `CharacterId` instead - the same simplification already established
    for driver-memory membership and the merchant/bank greet-tracking
    ports; (2) `find_char_byname`'s C slot-order iteration becomes a
    `CharacterId`-sorted scan for determinism (`World::characters` is a
    `HashMap`); (3) `return_items`'s `is_gk_room(c2)` gatekeeper-room
    guard is not replicated (gatekeeper NPC/lab room concept not ported
    yet); (4) the successful "Deal." branch's `achievement_award(...,
    ACHIEVEMENT_TRUST_BUT_VERIFY, 1)` calls are not replicated
    (achievements not ported yet); (5) `give_char_item`'s `dlog(cn, in,
    "was given %s from NPC", ...)` audit line is skipped (no generic
    "was given from NPC" audit path exists); (6) COL_LIGHT_BLUE/
    COL_LIGHT_GREEN/COL_RESET color markers around "help"/"accept
    trade"/"stop trade"/"show trade"/the greeting's "help"/the "gave me:"
    notice are dropped (same simplification `BANK_QA` already
    established - the legacy color marker is a raw non-UTF8 byte that
    cannot round-trip through a plain Rust `&str` literal) - wording
    stays byte-for-byte identical otherwise.
  - Tests: 20 new focused tests in `world/tests/trader.rs` (greet-once,
    qa small talk, help-text qa, trade-start success/busy/unknown-
    player/full-inventory, give-item routes to the correct side and
    notifies the other partner, give-item from a non-trading player is
    returned not destroyed, give-item at the 10-item cap is returned,
    stop-trade returns items and resets state, stop-trade from an
    outsider is rejected, accept-trade requires both sides before
    swapping items, accept-trade-in-a-longer-sentence scold, show-trade
    queues the event with current items, timeout cancels and returns
    items, timeout does not fire early, idle-chatter lucky/unlucky-roll
    seeded RNG, and 12h memory clear) plus 1 new test in `drvlib.rs` for
    `offset2dx`'s 8-way snapping.
  - Verification: `cargo fmt --all` clean. `cargo test --workspace`:
    1203 core (20 net new) + 27 db + 3 net + 33 protocol + 374 server,
    zero warnings, zero failures. `cargo build -p ugaris-server` clean,
    zero warnings. Boot-smoked `target/debug/ugaris-server --bind-addr
    127.0.0.1:5556` for 10s, "entering Rust game loop" logged, no
    panics.
  - `PORTING_TODO.md`'s `CDR_TRADER`/`CDR_JANITOR` checkbox marked `[~]`
    with a note that `CDR_TRADER` itself is done and only `CDR_JANITOR`
    remains as a follow-up (a materially different, self-contained
    lamp-lighting/item-tidying AI loop).
- 2026-07-04 (iteration 46): **`CDR_JANITOR` lamp-lighting/item-tidying
  NPC** (P2, completes the `CDR_TRADER`/`CDR_JANITOR` task from
  iteration 45) - ported C `src/module/base.c::janitor_driver` in full.
  - Added `JanitorDriverData` (`character_driver.rs`, new
    `CharacterDriverState::Janitor` variant) and wired spawn-time
    initialization for `CDR_JANITOR` in `zone.rs` (C never parses
    zone-file args into `struct janitor_data` either, matching
    `CDR_TRADER`'s precedent). Unlike C's `struct janitor_data` (which
    also carries `light[MAXLIGHT]`/`take[MAXTAKE]`, a cache of item IDs
    discovered via `NT_ITEM` notify messages as the janitor patrols via
    `scan_item_driver`), only `cnt` (the "N lights I turned on" murmur
    counter) is kept as genuinely persistent state - the new
    `crates/ugaris-core/src/world/janitor.rs` recomputes the nearest
    matching light/take-item candidate directly from `World::items`
    every tick instead, the same class of simplification already
    established for the merchant/bank/trader greeting scans.
  - `World::process_janitor_actions` ports the full `janitor_driver`
    tick body: absorb any held cursor item into the deep-inventory "bag"
    range (`item[30..INVENTORY_SIZE]`, C's own comment on `struct
    char.item[]`, `"30-(INVENTORYSIZE-1) inventory"`) if there is room;
    if not currently holding an item, take the nearest visible `IF_TAKE`
    junk item on the janitor's town half (C's `NT_ITEM` handler's
    `y == 192` divide filter, `base.c:5107-5111`) that is not already
    resting on one of the nine fixed home-area tiles (`161..=162,
    178..=183`), gated by `char_see_item` like C's `take_driver`; toggle
    the nearest known `IDR_TOYLIGHT` (same town-half filter) whose
    on/off state doesn't match the current day/night `ls` target
    (`dlight > 200` -> off, else on) via the existing
    `setup_walk_toward_use_item` helper (no visibility gate, matching
    C's commented-out `char_see_item` check in `use_driver`'s light
    branch); otherwise pop the highest-occupied bag slot (or the already-
    held cursor item) and try to drop it at each of the nine home tiles
    in C's exact order (`janitor_drop`'s hardcoded coordinate list),
    restoring it to the bag on total failure, then walk toward the home
    tile or idle. The idle-murmur table (18 lines, including the dynamic
    "N lights I turned on in my life" counter case seeded to 25598) is
    rolled only right after a successful light-toggle action (1-in-50),
    unlike the other NPC drivers' per-minute throttle - matching C's
    `janitor_driver` exactly (the murmur roll is nested inside the
    `use_driver` success branch, not gated by a separate timer).
  - No new pathfinding/action machinery was needed: the janitor's
    take/use/drop movement is built entirely on the existing
    `setup_walk_toward`/`setup_walk_toward_use_item`/`do_take`/`do_drop`/
    `do_use`/`adjacent_direction`/`adjacent_use_direction` primitives
    already used by the player action pipeline (`world/actions.rs`).
  - Documented (not silent) deviation: C's bag-unstash loop reads
    `ch[cn].item[INVENTORYSIZE]` first (`base.c:5093`, an off-by-one
    out-of-bounds read past the valid `0..INVENTORYSIZE` array range)
    before falling back to `INVENTORYSIZE-1`; this port starts at the
    last valid index instead of replicating undefined behavior.
  - Tests: 12 new focused tests in `world/tests/janitor.rs` (adjacent
    vs. distant light-toggle-needed dispatch, leaving an
    already-correct-state light alone and taking a junk item instead,
    town-half exclusion, home-drop-zone exclusion, nearest-of-two
    take-item selection, bag-absorb-then-restore-on-blocked-drop, first-
    open-home-spot drop, first-spot-blocked-falls-to-second-spot drop,
    fixed-line and dynamic-counter idle murmur on seeded RNG rolls, and
    dead/unused characters are skipped by the dispatch loop).
  - Verification: `cargo fmt --all` clean. `cargo test --workspace`:
    1215 core (12 net new) + 27 db + 3 net + 33 protocol + 374 server,
    zero warnings, zero failures. `cargo build -p ugaris-server` clean,
    zero warnings. Boot-smoked `target/debug/ugaris-server --bind-addr
    127.0.0.1:5556` for 12s, "entering Rust game loop" logged, no
    panics.
  - `PORTING_TODO.md`'s `CDR_TRADER`/`CDR_JANITOR` checkbox marked `[x]`
    (both halves of the task are now complete).

- Ported P2 "Aclerk / auction NPC", slice (1) - `merchant.c::aclerk_driver`
  (`CDR_ACLERK = 4`, the Cameron arena clerk NPC):
  - New `AclerkDriverData`/`parse_aclerk_driver_args` in
    `character_driver.rs` (field-for-field identical to
    `MerchantDriverData` - C literally copy-pastes the struct shape - kept
    as its own type so `CharacterDriverState` stays a plain enum) plus zone
    spawn wiring in `zone.rs` mirroring `CDR_MERCHANT`'s.
  - New `crates/ugaris-core/src/world/aclerk.rs`: `World::process_aclerk_actions`
    creates the store (`World::ensure_merchant_store` generalized to accept
    either `CharacterDriverState::Merchant` or `::Aclerk`, since C's
    `create_store` call is identical between the two drivers), greets each
    visible player once within 5 tiles with "Welcome to the Cameron
    Arena! ..." (memory slot 7, 12h clear timer) - matching only the
    *first* of C's three back-to-back `NT_CHAR` `quiet_say` blocks, since
    the first ends with an unconditional `{ remove_message(...); continue;
    }` that makes the second ("arena is safe") and third (a merchant-style
    trade greeting) blocks unreachable dead code - reacts to the hardcoded
    `abuser()` persistent-player-ID list saying "<name> ... trade" with a
    `RANDOM(3)` murmur/emote/murmur (checked against the raw runtime
    `CharacterId`, the same simplification already established for
    `TraderDriverData::c1_id`/`c2_id`), destroys given items, and rolls an
    11-line idle-murmur table on the same `RANDOM(25)`-then-`RANDOM(n)`
    throttle as `merchant_driver`. `World::refresh_special_stores`
    (`world/special_item.rs`) is likewise generalized to cover `CDR_ACLERK`
    for the `special`-flagged seed-five/12h-refresh timer.
  - Deviation confirmed digit-for-digit against C: unlike `merchant_driver`,
    `aclerk_driver`'s "`<name> ... trade`" handler never sets
    `ch[co].merchant = cn` - only the `abuser()` reaction runs - so saying
    "<clerk>, trade" never actually opens the arena clerk's store in C
    either; this port matches that exactly (the store still gets created
    and stocked via `create_store`/`add_special_store`, it is just never
    reachable through the trade-request path).
  - Two idle-chatter cases have an embedded period in the C format string
    itself (`"eyeballs deep within the forest."`, `"...to wake himself
    up."`) that doubles up with `emote()`'s own `"%s %s."` wrapper -
    reproduced exactly (`"...forest.."`, `"...himself up.."`), documented
    inline with `// C: ...` comments.
  - Checked the community client (`astonia_community_client/src/game/
    render.c`) for any auction UI before committing to this slice: the
    only "auction" hit is a chat-palette color name, confirming there is
    no `CL_*` auction client protocol to port at all right now - slice (3)
    from the todo item can likely become N/A once slice (2) (`src/system/
    auction/*.c` + `database_merchant.c`'s separate auction-house DB
    tables, which the arena clerk driver itself never touches) is
    actually audited by a future iteration.
  - Tests: 13 new focused tests in `world/tests/aclerk.rs` (arg parsing,
    shared store creation, greet-once-within-5-tiles plus the >5-tile
    miss, no-merchant-assignment-on-trade-text, abuser vs. non-abuser
    trade-text reactions, given-item vanishing, idle-chatter murmur/skip/
    below-interval/doubled-period-emote on seeded `legacy_random_seed`
    rolls, and the 12h greet-memory clear).
  - Verification: `cargo fmt --all` clean. `cargo test --workspace`: 1228
    core (13 net new) + 27 db + 3 net + 33 protocol + 374 server, zero
    warnings, zero failures. `cargo build -p ugaris-server` clean, zero
    warnings. Boot-smoked `target/debug/ugaris-server --bind-addr
    127.0.0.1:5556` for 12s, "entering Rust game loop" logged, no panics.
  - `PORTING_TODO.md`'s "Aclerk / auction NPC" checkbox marked `[~]` with a
    REMAINING note for slices (2) and (3).

- Continuation (iteration 48): ported P2 "Aclerk / auction NPC" slice (2) -
  the database layer of `src/system/auction/auction_db.c` (all 12 of its
  public functions): `init_auction_database`, `db_create_auction`,
  `db_update_auction`, `db_get_auction`, `db_delete_auction`,
  `db_search_auctions`, `db_get_player_auctions`,
  `db_count_active_auctions`, `db_create_delivery`,
  `db_get_pending_deliveries`, `db_mark_delivery_claimed`,
  `db_get_delivery_summary`, `db_cleanup_expired_auctions`.
  - New `crates/ugaris-db/src/auction.rs`: `AuctionRepository` trait +
    `PgAuctionRepository` impl, following the `merchant.rs` repository
    convention (typed request/record structs, a `Pg*` impl, unit tests
    plus `DATABASE_URL`-gated `live` integration tests).
  - New `migrations/0006_auction_house.sql`: `auctions` and
    `auction_deliveries` tables. C stores the auctioned item as a raw
    `struct item` BLOB and filters/sorts on byte offsets inside it via
    `CAST(SUBSTRING(a.item_data, offsetof(struct item, name), 40) AS
    CHAR)`-style queries; Rust stores the item as `jsonb` instead (same
    convention as `merchant_stores.wares_json`) and filters/sorts on its
    `name`/`min_level`/`max_level` keys with normal `jsonb` operators -
    simpler and immune to C struct-layout drift. `item_template` is kept
    as its own column like C, purely for future template-based
    indexing/browsing. C's MySQL `ENUM` status/reason columns become
    `text` + `check` constraints (`AuctionStatus`/`DeliveryReason` enums on
    the Rust side, string values copied byte-for-byte:
    `active`/`sold`/expired`/`cancelled`, `won`/`expired`/`cancelled`/
    `sold`/`outbid`). `created_at`/`ends_at` are `timestamptz` read/written
    as unix-epoch `i64` via `extract(epoch from ...)`/`to_timestamp(...)`,
    the same convention `character.rs` already uses for `login_time`.
  - `db_get_character_name` (`auction_db.h`) is deliberately **not**
    ported: grepping the full C tree (`grep -rn db_get_character_name`)
    shows it is declared and defined but never called anywhere - dead
    code. Every real caller needing a seller name instead relies on the
    `LEFT JOIN chars c ON a.seller_id = c.ID` C already inlines into
    `db_get_auction`/`db_search_auctions`/`db_get_player_auctions`, which
    this port replicates by joining Postgres's `characters` table in the
    equivalent queries (`SELECT_AUCTION_COLUMNS`'s
    `coalesce(c.name, 'Unknown')`).
  - `create_auction`/`create_delivery` return the new row's id via
    Postgres `RETURNING id`, even though C's `db_create_auction`/
    `db_create_delivery` are `bool`-only and grepping shows no C caller
    ever retrieves the created auction's id via `LAST_INSERT_ID()` - the
    id is free with `RETURNING` and useful for a future business-logic
    slice, so it's exposed rather than discarded.
  - `db_search_auctions`'s `filter.limit`/`filter.offset` are **not**
    clamped inside `search_auctions` (C doesn't clamp there either -
    callers in `auction_house.c`/`auction_client.c` always pass a fixed
    page size); `get_player_auctions` clamps `limit` to
    `MAX_SEARCH_RESULTS` (50), matching `db_get_player_auctions` exactly.
  - `cleanup_expired_auctions` returns the number of auctions processed
    (C's `db_cleanup_expired_auctions` is `void`) purely so tests/future
    callers can assert something happened; behavior (winner gets the item
    + seller gets a `sold` gold delivery on a bid, or the seller gets the
    item back via an `expired` delivery with no bid) matches C's
    `WHERE status = 'active' AND ends_at <= NOW()` sweep exactly, run
    inside one query pass per expired row (C wraps each row in its own
    MySQL `START TRANSACTION`/`COMMIT`; this port relies on Postgres's
    per-statement auto-commit instead of an explicit transaction per row -
    documented as a minor deviation, not a correctness gap, since each
    delivery/status update is already a single atomic statement).
  - Tests: 5 unit tests (`AuctionStatus`/`DeliveryReason` string round
    trips including the C default-active fallback, `MAX_SEARCH_RESULTS`
    constant, item JSON round trip) + 4 `DATABASE_URL`-gated live tests
    (create/get/update/delete round trip, name-substring + level-range
    search with price-ascending sort order, delivery create/pending-list/
    claim/summary, and expired-auction cleanup covering both the
    winner-delivery and no-bid-return branches). These were not just
    compiled: spun up an ephemeral Postgres 16 Docker container, applied
    all six `migrations/*.sql` files by hand via `docker exec -i ... psql
    < file.sql`, and ran `DATABASE_URL=postgres://postgres:test@127.0.0.1:
    <port>/ugaris_test cargo test -p ugaris-db auction -- --test-threads=1`
    to actually exercise the SQL against a real server - this caught two
    real bugs the DATABASE_URL-less default run couldn't (missing column
    aliases on the `extract(epoch from ...)` expressions so
    `row.try_get("created_at_unix")` failed, and Postgres returning
    `NUMERIC` instead of `BIGINT` from `SUM(gold_amount)`, needing an
    explicit `::bigint` cast) - both fixed and reverified green before
    tearing the container down.
  - Verification: `cargo fmt --all` clean. `cargo test --workspace`: 1228
    core + 36 db (9 net new) + 3 net + 33 protocol + 374 server, zero
    warnings, zero failures. `cargo build -p ugaris-server` clean, zero
    warnings. No boot-smoke: this change only adds a new, currently-unused
    DB repository/migration - it touches neither the runtime loop, login,
    map sync, nor protocol.
  - `PORTING_TODO.md`'s "Aclerk / auction NPC" REMAINING note narrowed:
    slice (2) is now scoped to just the CRUD/search DB repository (done);
    the `auction_house.c` business logic and `/ah` command state machine
    in `auction_cmd.c` still need a future slice to actually call this
    repository; slice (3) stays N/A per the prior client audit.

### Ralph Loop Iteration 49: Aclerk / auction NPC - business logic + `/ah` command

- Ported the remaining pieces of P2 "Aclerk / auction NPC":
  `src/system/auction/auction_house.c` (fee/bid math, buy/bid/buyout/
  cancel/claim/search orchestration, `init_auction_house`/
  `update_auction_house`/`shutdown_auction_house`) and
  `src/system/auction/auction_cmd.c` (the `/ah` text command state
  machine), on top of the DB layer from iteration 48
  (`ugaris_db::auction`).
  - New `crates/ugaris-server/src/auction.rs` (~1,300 lines):
    - `AuctionError` mirrors `auction_house.h`'s error codes (1-10;
      `AUCTION_SUCCESS` maps to Rust `Ok(())`/`Ok(value)` instead of a
      variant).
    - Pure helpers: `format_money` (unifies C's two byte-identical
      `format_money_string`/`format_money`), `validate_auction_item`,
      `calculate_auction_fee` (5%, floored at 100 = 1 gold),
      `calculate_min_bid` (5% increment, 1-copper floor,
      `saturating_add` standing in for C's manual `ULLONG_MAX` overflow
      check), `format_time_left`/`format_price`/`format_item_details`/
      `format_item_modifiers` (colored, using the existing
      `ugaris_core::text::COL_*` legacy color-marker bytes), and a local
      `AUCTION_VALUE_ABBREV` table reproducing `get_value_name`'s short
      lowercase abbreviations (`"hp"`, `"m-shield"`, `"armor skill"`,
      etc.) - deliberately separate from `entity::CHARACTER_VALUE_NAMES`
      (unrelated Title-Case convention used by `legacy_item_look_text`,
      which is reused as-is for `/ah info`'s item lookup instead of
      reimplementing C's `look_item`).
    - Async orchestration functions (`auction_create`/`auction_bid`/
      `auction_buyout`/`auction_cancel`/`auction_claim_deliveries`/
      `auction_search`) take `&PgAuctionRepository` plus `&mut World`
      (concrete-type dependency injection, matching the existing
      `merchants.rs`/`world_events.rs` convention rather than a generic
      trait bound + mock, since nothing else in `ugaris-server` does
      that). `auction_create` validates the cursor item against C's six
      `IF_*` flag checks, then (DB insert succeeds first, matching C's
      "commit before consuming the item" ordering) deducts the fee and
      calls `World::destroy_item` (C's `consume_item`). `auction_bid`
      creates an `Outbid` delivery for the previous bidder before
      updating the auction row and returns their id so the caller can
      notify them if online. `auction_claim_deliveries` credits gold via
      `saturating_add` unconditionally (matching C's `ch[cn].gold +=`
      happening outside the DB transaction) and only marks an item
      delivery claimed if `give_item_to_character` actually placed it
      (leaving a full-inventory delivery pending for retry, like C's
      `GIVE_ITEM_FULL` case).
    - `apply_auction_command` is the `/ah`/`/auctionhouse` dispatcher
      (`auction_process_command` + `command.c`'s `cmdcmp(ptr, "ah", 2)`/
      `cmdcmp(ptr, "auctionhouse", 10)`), reusing the existing
      `legacy_cmd_prefix` helper for both the outer verb and each
      subcommand's abbreviation-length floor from C's `commands[]`
      table. `/ah help`/bare `/ah` work even without a repository;
      every other subcommand replies "The auction house is currently
      unavailable." when `--database-url` wasn't given, since (unlike
      merchant/bank/trader) auctions have zero in-memory `World` state -
      they're DB-only by the iteration-48 design decision.
  - Wired into `crates/ugaris-server/src/main.rs`: a new
    `auction_repository: Option<PgAuctionRepository>` alongside
    `merchant_repository`; the `/ah` dispatch call inside
    `ClientAction::Text`'s command chain; a startup
    `cleanup_expired_auctions` sweep (C `init_auction_house`); a
    60-real-second periodic sweep gated on `world.tick.0 %
    (TICKS_PER_SECOND * 60) == 0` (C's `maintenance_60s_task` calling
    `update_auction_house` - note this is wall-clock-ish via the tick
    counter, not a real timer, so it drifts with tick-rate changes, same
    class of simplification as other tick-gated periodic code in this
    file); and a shutdown sweep after the `ctrl_c` branch breaks the main
    loop (C `shutdown_auction_house`).
  - Deviation documented in the module's doc comment: C's
    `auction_bid`/`auction_buyout`/`auction_cancel` call `log_char`
    directly for most error cases, and `auction_cmd.c`'s command
    wrappers *also* log a second, usually near-duplicate message from
    their own `switch` on the status code - e.g. self-bidding shows two
    "you cannot bid on your own auction"-style lines back to back in C.
    This port keeps exactly one message per error, picking whichever of
    the two C messages is more specific: `cmd_auction_buy`/
    `cmd_auction_cancel` gained explicit match arms for
    `AUCTION_ERROR_INVALID_PRICE`/`AUCTION_ERROR_CANT_BID_OWN`
    respectively (using the low-level function's specific text - C's own
    `switch` in those two command wrappers has no case for them and
    would otherwise fall through to a generic "Failed to ..." message,
    silently dropping the more useful text a player actually sees in
    C); `cmd_auction_bid`'s `AUCTION_ERROR_BID_TOO_LOW` re-fetches the
    auction to recompute and show the exact minimum bid amount (C's
    `auction_bid` message) instead of only the generic "5% increment"
    text (C's `cmd_auction_bid` message).
  - Remaining gap: `auction_check_deliveries_login` (a login-time "you
    have N auction deliveries waiting" notice, `auction_house.c:1212-
    1272`) is not wired to the existing-but-unused
    `PlayerRuntime::deferred_init`/`DEFERRED_AUCTION` bit
    (`ugaris-core/src/player.rs:239-241,417`) - login is a large,
    heavily-tested, high-risk code path and this was judged out of scope
    for this slice. Players can still discover and claim pending
    deliveries any time via `/ah claim`.
  - Tests: 18 new tests in `crates/ugaris-server/src/tests/auction.rs`
    covering money formatting, fee/min-bid math (including the overflow
    saturation and the sub-20-copper 1-copper floor), item validation
    against all six disqualifying flags, the `get_value_name`
    abbreviation table (including an out-of-range fallback), modifier/
    requirement splitting, item-detail color tiers, time-left color
    tiers (including "Ended"), price color/buyout-suffix formatting,
    full help text, and `apply_auction_command`'s verb
    routing/repository-unavailable/bare-`/ah`-defaults-to-help/long-form-
    alias behavior. DB-touching command bodies (sell/buy/bid/cancel/
    search/info/claim's actual repository calls) are exercised only by
    type-checking plus the DB layer's own iteration-48 live tests against
    a real Postgres container - `ugaris-server` has no
    `DATABASE_URL`-gated test convention yet, matching every other
    repository-backed command in this crate (merchant, bank, trader).
  - Verification: `cargo fmt --all` clean. `cargo test --workspace`:
    1228 core + 36 db + 3 net + 33 protocol + 392 server (18 net new),
    zero warnings, zero failures. `cargo build -p ugaris-server` clean,
    zero warnings. A 12s boot-smoke showed "entering Rust game loop" with
    no panics.
  - `PORTING_TODO.md`'s "Aclerk / auction NPC" REMAINING note narrowed to
    just the login-notification gap above; task stays `[~]` for that one
    remaining item.

### Ralph Loop Iteration 50: Aclerk / auction NPC - login delivery notice (closes the task)

- Closed the last gap on P2 "Aclerk / auction NPC": wired
  `auction_check_deliveries_login` (`auction_house.c:1206-1270`) to the
  existing-but-unused `PlayerRuntime::deferred_init`/`DEFERRED_AUCTION`
  bit (`ugaris-core/src/player.rs:239-241,417`), matching C
  `tick_player`'s deferred-init sweep (`player.c:3660-3685`).
  - `ServerRuntime::login` (`crates/ugaris-server/src/main.rs`) now sets
    `player.deferred_init |= DEFERRED_AUCTION` on every login. C only
    does this in the `!(ch[cn].flags & CF_AREACHANGE)` branch
    (`player.c:618-629`), but that branch always holds in the current
    Rust login path since cross-area transfer isn't implemented yet
    (`login.rs`'s `LoginOutcome::NewArea` comment already documents
    this). C's `DEFERRED_ACHIEVEMENTS`/`DEFERRED_MOTD` bits are
    deliberately left unset - achievements and MOTD aren't ported yet
    (achievements has its own dedicated P4 task in `PORTING_TODO.md`;
    MOTD has no task at all and remains untouched).
  - The game loop gained a new per-tick sweep (next to the existing
    60-second `cleanup_expired_auctions` maintenance block): for every
    player with `deferred_init & DEFERRED_AUCTION != 0` and
    `world.tick.0 - login_tick >= 6` (literal port of C's `ticks >= 6`
    gate, reusing the tick unit `login_tick` already stores), the bit is
    cleared and a new `auction::auction_login_notice` is awaited, which
    calls `AuctionRepository::get_delivery_summary` (already ported in
    iteration 48) and formats the result via a new
    `format_auction_login_notice`.
  - `format_auction_login_notice` (`crates/ugaris-server/src/auction.rs`)
    reproduces C's four count/items/gold text combinations exactly
    (items+gold / items-only / gold-only / none), reusing the existing
    `format_money` helper for the gold/silver split - C's
    `total_gold >= 100` gold-vs-silver-only branch is exactly
    `format_money`'s own `gold > 0` check, so no duplicate logic was
    needed. The `COL_YELLOW`...`COL_RESET`-wrapped result is sent with
    `ugaris_protocol::packet::system_text_bytes` through the existing
    `sessions_for_character`/`send_to_session` pattern (the same one
    already used for ordinary command feedback).
  - Deviation documented in code comments: C's `count > 0` branch with
    neither pending items nor gold is unreachable dead code that reads
    an uninitialized `buf` before calling `log_char`; this port simply
    returns `None` (no notice) for that combination instead of
    replicating the undefined behavior.
  - Tests: 6 new tests in `crates/ugaris-server/src/tests/auction.rs`
    covering the no-pending-deliveries no-op, all three formatted-text
    branches, the above/below-a-gold silver-split boundary, and the
    unreachable-combination no-op.
  - Verification: `cargo fmt --all` clean. `cargo test --workspace`: 398
    `ugaris-server` tests (6 net new), zero warnings, zero failures.
    `cargo build -p ugaris-server` clean, zero warnings. A 12s boot-smoke
    showed "entering Rust game loop" with no panics.
  - `PORTING_TODO.md`'s "Aclerk / auction NPC" checkbox flipped to `[x]`:
    all three slices plus the login-notice gap are now done, and slice
    (3) stays confirmed N/A per the prior client audit.

## Ralph Loop - Gatekeeper NPC Welcome Dialogue (Iteration 51, partial)

`PORTING_TODO.md`'s P2 "Gatekeeper NPC" task (`src/system/gatekeeper.c`)
is the character standing in front of the already-ported `IDR_LABENTRANCE`
lab door. Read the full 830-line C file this iteration; ported the pure,
fully-tested logic slice that doesn't require `World`/tick-loop access,
following the same "pure state machine, not yet wired" pattern already
established by `clara_dialogue_step` (Area 15).

- `crates/ugaris-core/src/character_driver.rs`:
  - `CDR_GATE_WELCOME: u16 = 39` / `CDR_GATE_FIGHT: u16 = 40` driver-id
    constants (C `src/system/drvlib.h`).
  - `GATEKEEPER_QA: &[TextQaEntry]` - the verbatim 27-row `qa[]` table
    from `gatekeeper.c:83-112`, reusing the existing
    `analyse_text_qa`/`TextQaEntry` engine (no new tokenizer needed).
    Every accepted spelling variant for the four class choices
    (Arch-Warrior/Arch-Mage/Arch-Seyan'Du/Seyan'Du, including the
    hyphenated and apostrophe'd single-token forms - C's tokenizer only
    splits on `' ' ',' ':' '?' '!' '"' '.'`, not `-`/`'`) is covered.
  - `gate_welcome_dialogue_step` - a pure port of `gate_welcome_driver`'s
    `switch (ppd->welcome_state)` (`gatekeeper.c:475-542`), states
    `0..=6`. This required literally reproducing C's `case 2:`/`case 3:`/
    `case 4:` fallthrough (no `break` after case 2, conditional
    fallthrough after case 3) via explicit helper functions
    (`gate_case3_stops`, `gate_case4`) rather than a `match` arm per
    state, since Rust has no fallthrough. Along the way this surfaced (and
    intentionally preserved, not "fixed") a genuine C quirk: the "fast
    path" (state 2 entered with the labyrinth requirement already
    satisfied) falls through cases 3 and 4 in one call and lands on state
    `6` directly, skipping the `case 5` "name the class, 100 gold" message
    entirely; the "slow path" (state 3 entered on a later call, after the
    labyrinth got solved in between) only reaches state `5` in that call,
    so the player sees the `case 5` message on the *next* call that the
    fast path never shows. Both paths are covered by dedicated tests.
  - `gate_welcome_state_after_repeat` - C's `analyse_text_driver` result
    `2` (`"repeat"`/`"restart"`) resets `welcome_state` to `0`, but only
    when `welcome_state <= 6` (`gatekeeper.c:566-570`).
  - `gate_enter_test_precheck`/`gate_class_choice_is_valid` - the
    `enter_test` preconditions (`gatekeeper.c:316-390`): `CF_PAID` gate,
    `teleport_next_lab`-vs-`CF_GOD` gate, `CF_NOEXP` gate, then (skipped
    entirely for `CF_GOD`) the four-class flag-combination validation and
    the carried-item-count rules (zero items for Arch classes, up to three
    for Seyan'Du). Deliberately excludes the side-effecting tail
    (`take_money`, the `enter_room` 9-slot room search/spawn) since that
    needs `World`/zone-loader access this pure module doesn't have.
- `crates/ugaris-core/src/player.rs`: `DRD_GATE_PPD` (`MAKE_DRD(DEV_ID_DB,
  65 | PERSISTENT_PLAYER_DATA)`) now round-trips through the legacy PPD
  blob, modeled on the existing `DRD_WARP_PPD` fixed-12-byte-layout
  pattern: `gate_welcome_state`/`gate_target_class`/`gate_step` fields
  (mirroring C `struct gate_ppd`) plus `encode_legacy_gate_ppd`/
  `decode_legacy_gate_ppd`, wired into both the decode-block match and the
  encode-block match-plus-append-if-nonzero tail.
- Tests: 9 new tests in `character_driver.rs` (QA table coverage across
  every word/code combination, fast-path and slow-path dialogue-state
  assertions including the exact terminal-state discrepancy, the
  repeat-reset boundary, the state-6 silent wait, and the full
  `enter_test` precondition/class-validation matrix including the
  `CF_GOD` bypass) plus 3 new tests in `player.rs` (fixed-layout
  round-trip, outer PPD blob block framing, append-without-existing-block).
- Verification: `cargo fmt --all` clean. `cargo test --workspace`: 1240
  `ugaris-core` (62 net new across the two files) + 36 db + 3 net + 33
  protocol + 398 server, all green, zero failures. `cargo build -p
  ugaris-server` clean, zero warnings. A 10s boot-smoke showed "entering
  Rust game loop" with no panics (expected - nothing calls the new
  functions yet, so this doesn't touch the runtime tick loop).
- Remaining (left `[~]` in `PORTING_TODO.md`, precise notes there): (1)
  `enter_room`/`enter_test`'s side effects - `take_money`, spawning
  `gatekeeper_w`/`_m`/`_s` via `loader.instantiate_character_template`
  (template data already exists in `ugaris_data`, confirmed at
  `zones/3/above3_generic.chr:3538-3789`), the 9-room busy/refund search,
  stripping the player's spells/items and teleporting them in; (2) a
  `World::process_gate_welcome_actions()` tick-loop entry point (modeled
  on `World::process_trader_actions`) to actually invoke
  `gate_welcome_dialogue_step`/`GATEKEEPER_QA` and `say()` the result -
  without this, no gatekeeper NPC talks in-game yet; (3) `gate_fight_driver`
  (reuse `world/npc_fight.rs` combat helpers) and `gate_fight_dead`'s
  class-grant rewards, including `turn_seyan` (`src/system/tool.c:
  4278-4353`), which is not ported anywhere in the tree yet and is a
  substantial full character re-roll (temp-template stat copy, gear
  unequip, several unrelated PPD deletes); (4) the `NTID_GATEKEEPER`
  cross-NPC message hookup connecting the welcome NPC to its spawned
  opponent.

## Ralph Loop - Gatekeeper NPC Welcome Dialogue Tick-Loop Wiring (Iteration 52, partial)

Continued `PORTING_TODO.md`'s P2 "Gatekeeper NPC" task: wired iteration
51's pure dialogue/QA logic into `World`'s message loop and the server
tick loop, so the welcome NPC now actually greets and small-talks players
in-game (remaining gaps: `enter_test`'s class-choice spawn, the fight
driver, `turn_seyan`, and the idle "return to post" safety net - see
`PORTING_TODO.md`'s updated notes).

- `crates/ugaris-core/src/character_driver.rs`:
  - `GateWelcomeDriverData` (`last_talk`/`current_victim`/`amgivingback`,
    C `struct gate_welcome_driver_data`, `gatekeeper.c:411-415`) and a new
    `CharacterDriverState::GateWelcome` variant, wired into every
    exhaustive match (`npc_messages.rs`, `npc_fight.rs`, `npc_idle.rs`,
    and `apply_simple_baddy_create_message`'s driver-data-reuse match) and
    into `zone.rs`'s per-template driver-state initialization (default,
    like `CDR_TRADER` - C never parses zone-file args into this struct).
  - `needs_next_lab(lab_solved_bits: u64) -> bool` - a new pure helper
    that avoids porting `teleport_next_lab` (`src/system/lab.c:94-104`)
    at all for the welcome dialogue's `needs_lab` input: with
    `do_teleport = 0`, `teleport_lab`'s `!do_teleport || change_area(...)`
    always short-circuits true without touching the map, so the C
    function's *truthiness* (not its exact return code, which the
    dialogue doesn't need) reduces to "at least one of the five known lab
    checkpoint bits (10/15/20/25/30) is unsolved" - reusing the existing
    `item_driver::legacy_lab_destination` table (already ported for
    `IDR_LABENTRANCE`) instead of duplicating it. A player's `level` only
    changes *which* nonzero value `teleport_next_lab` would return, never
    whether it returns nonzero, so it's provably irrelevant here.
- `crates/ugaris-core/src/world/gatekeeper.rs` (new file):
  `World::process_gate_welcome_actions`, modeled directly on
  `world/trader.rs::process_trader_messages`: drains the welcome NPC's
  `driver_messages` and handles `NT_CHAR` (the greeting - calls
  `gate_welcome_dialogue_step`), `NT_TEXT` (`GATEKEEPER_QA` via
  `analyse_text_qa`; answer code `2` "repeat"/"restart" via
  `gate_welcome_state_after_repeat`; code `9` "reset" via a god-flag check
  - `Character::flags` is directly visible to `World`, unlike
  `PlayerRuntime`; codes `3`/`4`/`5`-`8` are bookkept as `didsay` like C
  but produce no reply yet, see remaining gaps), and `NT_GIVE`
  (give-back-or-destroy, reusing `world/trader.rs::trader_give_char_item`'s
  shape rather than porting `give_driver`'s pathfinding-retry, matching
  that module's own documented simplification). Faithfully reproduces
  several exact-tick throttle rules (`last_talk + TICKS*5`,
  `last_talk + TICKS*10` combined with a "current victim" lock that both
  the `NT_CHAR` and `NT_TEXT` branches read/write) and the C oddity that
  `dat->amgivingback` resets to `0` unconditionally every tick
  (`gatekeeper.c:621`), not just after a successful give-back, so the
  give-back flavor text can repeat across separate ticks.
  - Because the dialogue needs two facts that live in
    `crate::player::PlayerRuntime` (owned by `ugaris-server`, not
    `World`) - `gate_welcome_state` and `needs_next_lab`'s input - and
    because writing the result back also touches `PlayerRuntime`, added a
    snapshot-in/events-out split mirroring `world/bank.rs`'s `BankEvent`
    pattern exactly: `GateWelcomePlayerFacts` (caller-supplied, per
    player) in, `Vec<GateWelcomeOutcomeEvent>` (`UpdateWelcomeState`,
    `ResetLabPpd`) out.
- `crates/ugaris-server/src/world_events.rs`: `gate_welcome_player_facts`
  (snapshots every online player's two facts from `ServerRuntime.players`,
  mirroring `PkRelationSnapshot::from_runtime`'s shape) and
  `apply_gate_welcome_events` (writes `gate_welcome_state` back, or clears
  `lab_solved_bits`/`lab_ppd` for the god-only "reset" - the C
  `del_data(co, DRD_LAB_PPD)` equivalent, since there's no generic
  `del_data` - mirroring `apply_bank_events`'s shape).
- `crates/ugaris-server/src/main.rs`: calls
  `gate_welcome_player_facts` → `world.process_gate_welcome_actions` →
  `apply_gate_welcome_events` once per tick, right before
  `process_janitor_actions`.
- Tests: 1 new test for `needs_next_lab` in `character_driver.rs`
  (checkpoint-bit boundary cases, including that non-checkpoint bits never
  matter); 12 new tests in the new `crates/ugaris-core/src/world/tests/
  gatekeeper.rs` (greeting distance/visibility/throttle including the
  "different victim within the 10-tick window" C quirk, the
  labyrinth-still-needed wait and its state transition, QA small talk,
  the "repeat" reset and god-only "reset" text codes plus a non-god
  negative case, and both give-back and destroy-on-full-inventory `NT_GIVE`
  paths).
- Verification: `cargo fmt --all` clean. `cargo test --workspace`: 1252
  `ugaris-core` (12 net new) + 36 db + 3 net + 33 protocol + 398 server,
  all green, zero failures. `cargo build -p ugaris-server` clean, zero
  warnings. A 12s boot-smoke showed "entering Rust game loop" with no
  panics.
- Remaining (left `[~]` in `PORTING_TODO.md`, precise notes there): (1)
  `enter_test`'s class-choice codes `5`-`8` still need `enter_room`'s
  spawn side effects (`take_money`, `create_char`/`drop_char` for the
  `gatekeeper_w`/`_m`/`_s` opponent, the 9-room busy/refund search,
  stripping items, teleporting the player); (2) `gate_fight_driver`/
  `gate_fight_dead` (reuse `world/npc_fight.rs`) including `turn_seyan`
  (`src/system/tool.c:4278-4389` - confirmed this iteration to also need
  the still-unported per-character `DRD_DEPOT_PPD` plus roughly 11 other
  unmodeled `DRD_*` ids it clears); (3) the `NTID_GATEKEEPER` cross-NPC
  message hookup; (4) the idle "return to post" `secure_move_driver`
  safety net (needs a `tmpx`/`tmpy`-equivalent post position on
  `Character`, not modeled yet).

## Ralph Loop - Gatekeeper NPC `enter_test` Failure-Reply Wiring (Iteration 53, partial)

Continued `PORTING_TODO.md`'s P2 "Gatekeeper NPC" task: wired the
already-ported-but-unused `character_driver::gate_enter_test_precheck`
pure helper into `World`'s class-choice message handling, so the welcome
NPC's answer codes `5`-`8` (Arch-Warrior/Arch-Mage/Arch-Seyan'Du/Seyan'Du)
now produce C's exact validation-failure feedback instead of being
silently bookkept.

- `crates/ugaris-core/src/world/gatekeeper.rs`:
  - New free function `gate_carried_item_count(character: &Character) ->
    u32`, C's `enter_test` `cnt` loop (`gatekeeper.c:368-375`): inventory
    slots `INVENTORY_START_INVENTORY..INVENTORYSIZE` (`30..110`) plus
    `ch[cn].citem` (`Character::cursor_item`).
  - `gate_welcome_handle_text_message`'s `TextAnalysisOutcome::Matched`
    arm now has a dedicated `5..=8` case: builds a `GateEnterTestPrecheck`
    from the speaker's `Character::flags` (`PAID`/`GOD`/`NOEXP` all live
    directly on `Character`, so no new `PlayerRuntime` fact was needed
    beyond the already-snapshotted `needs_lab`) and the new carried-item
    count, calls `gate_enter_test_precheck`, then matches every variant:
    `NotPaid`/`LabNotSolved`/`NoExpMode`/`CarryingItems`/
    `CarryingTooManyItems` each call `World::queue_system_text` with C's
    verbatim `log_char(cn, LOG_SYSTEM, ...)` message text (private,
    addressed to the player only - *not* spoken by the NPC, matching C's
    distinction between `log_char` and `say`); `InvalidClass` calls
    `World::npc_say` with C's caller-side "That is not a possible
    choice." (the one branch where the *NPC* speaks); `Ready` (the
    success path) is intentionally left a no-op, since `enter_room`'s
    opponent-spawn side effect has no `World` counterpart yet (documented
    in the module doc comment and the `PORTING_TODO.md` REMAINING note).
    `didsay` still fires unconditionally for all of codes `5`-`8`,
    matching C (`enter_test` always returns `1` except on the
    class-validation `default`/mismatch case, which is exactly
    `InvalidClass`).
- Tests: 6 new tests in `crates/ugaris-core/src/world/tests/
  gatekeeper.rs` - one per failure message (`NotPaid`, `LabNotSolved`,
  `NoExpMode`, `CarryingItems`), the `InvalidClass` NPC reply (via
  `drain_pending_area_texts`, since `npc_say` is audible), and the
  `Ready` case asserting today's no-op (no system text, no area text, no
  event) while still confirming `didsay`'s `current_victim` bookkeeping
  fires.
- Verification: `cargo fmt --all` clean. `cargo test --workspace`: 1258
  `ugaris-core` (6 net new) + 36 db + 3 net + 33 protocol + 398 server,
  all green, zero failures. `cargo build -p ugaris-server` clean, zero
  warnings. A 12s boot-smoke showed "entering Rust game loop" with no
  panics.
- Remaining (left `[~]` in `PORTING_TODO.md`, precise notes there): (1)
  `enter_test`'s *success* path - `enter_room`'s spawn side effects
  (`take_money`, `create_char`/`drop_char` for the `gatekeeper_w`/`_m`/
  `_s` opponent, the 9-room busy/refund search, stripping items,
  teleporting the player); (2) `gate_fight_driver`/`gate_fight_dead`
  (reuse `world/npc_fight.rs`) including `turn_seyan`; (3) the
  `NTID_GATEKEEPER` cross-NPC message hookup; (4) the idle "return to
  post" `secure_move_driver` safety net.

## Ralph Loop - Gatekeeper NPC `enter_room` Opponent Spawn (Iteration 54, partial)

Continued `PORTING_TODO.md`'s P2 "Gatekeeper NPC" task: ported item (1)
from iteration 53's REMAINING list - `enter_test`'s success tail /
`enter_room` (`gatekeeper.c:227-407`), so answering "arch warrior"/"arch
mage"/"arch seyan'du"/"seyan'du" with a valid, precondition-satisfying
class choice now actually starts the private-room test instead of being a
silent no-op.

- `crates/ugaris-core/src/world/gatekeeper.rs` (all `pub` `World`
  methods, since `ugaris-server` needs to call them):
  - `gate_room_is_clear(xs, ys)`: C `enter_room`'s 9x17 room-clear scan
    (`gatekeeper.c:233-240`) - no character on any tile, and any item
    present must not carry `IF_TAKE` (fixed furniture is fine, pick-up-able
    clutter blocks the room).
  - `gate_take_money`/`gate_give_money_silent`: C `take_money`/
    `give_money_silent` (`src/system/tool.c:3820-3826,1441-1449`). Unlike
    bank gold, `Character.gold` is a plain field here, so no `PlayerRuntime`
    PPD indirection was needed (the `dlog`/Macro-Daemon activity-tracking
    side effects are omitted, matching every other `give_money_silent`
    call site already in this codebase).
  - `gate_finish_enter_room(player_id, xs, ys)`: the player-side tail of
    `enter_room`'s success path once the opponent already exists at
    `(xs + 4, ys + 13)` - `teleport_char_driver(cn, xs + 4, ys + 4)`
    including its "already within Manhattan distance 1 of the target"
    failure short-circuit (`drvlib.c:2652-2654`), stripping spell slots
    `INVENTORY_START_SPELLS..=INVENTORY_LAST_SPELLS` (`12..=29`) via the
    existing `destroy_item`, the two `log_char` notices ("All your spells
    have been removed." and the ten-minute door-direction message, digit-
    for-digit), and resetting HP/mana/endurance to `POWERSCALE * 1` (mana
    only if it was already nonzero) plus `regen_ticker = ticker`.
    `destroy_chareffects(cn)` is a documented no-op: `Character` has no
    active-spell-effect list modeled yet.
  - `GateEnterTestOutcome::Ready`'s handling in
    `gate_welcome_handle_text_message` now pushes a new
    `GateWelcomeOutcomeEvent::EnterTestReady { player_id, class }` instead
    of doing nothing, since the opponent's `create_char` needs
    `ZoneLoader::instantiate_character_template`, which `World` cannot
    call (mirrors why `spawns.rs` exists at all for every other
    template-instantiating spawn).
- `crates/ugaris-server/src/spawns.rs` (new
  `gate_enter_test_spawn_room`, modeled directly on
  `spawn_swampspawn_character`): the `GATE_TEST_ROOM_STARTS` constant is
  C's `room_start[]` (`gatekeeper.c:317`) transcribed digit-for-digit as 7
  `(xs, ys)` pairs; `gate_test_opponent_template` is `enter_room`'s
  `switch (class)` template pick (classes `7`/`8` both map to
  `gatekeeper_s`). The function: calls `gate_take_money` once up front
  (refusing with "Thou canst pay the price of 100G." on failure, exactly
  like C, before any room search); for each candidate room, skips busy
  ones via `gate_room_is_clear`, then instantiates the template, sets the
  opponent's `hp`/`endurance`/`mana` from `values[0]` (matching every
  other `spawn_*` function's post-`update_char` stat scaling),
  `Direction::RightDown` (C's `DX_RIGHTDOWN`), stores the opponent's
  "return to post" coordinates in `rest_x`/`rest_y` (documented
  substitution for C's `tmpx`/`tmpy` - `Character` has no dedicated field
  for that yet, same substitution `respawn_npc_character` already uses
  for other NPCs), and pushes the `NT_NPC`/`NTID_GATEKEEPER` driver
  message (`notify_char(co, NT_NPC, NTID_GATEKEEPER, cn, 0)`) before
  calling `World::spawn_character`; on a successful spawn, calls
  `gate_finish_enter_room` for the player-side effects, and on success
  there sets `PlayerRuntime::gate_target_class`/`gate_step` (C's
  `ppd->target_class = class; ppd->step = 1;`) and returns `true` -
  otherwise destroys the just-spawned opponent (`World::remove_character`,
  C's `remove_destroy_char`) and tries the next room. If every room stays
  busy, refunds the fee via `gate_give_money_silent` and sends the "the
  gatekeeper is busy at the moment" notice, matching C exactly.
  `apply_gate_welcome_events` (`world_events.rs`) now also takes `&mut
  World`/`&mut ZoneLoader` (previously only `&mut ServerRuntime`) so it
  can call this on the new `EnterTestReady` event; the `main.rs` call site
  was updated accordingly.
- Deviations documented in code comments (both gaps already tracked in
  `PORTING_TODO.md`'s REMAINING note): `destroy_chareffects` no-op, and
  the opponent's post-position `tmpx`/`tmpy` -> `rest_x`/`rest_y`
  substitution (only consumed once `gate_fight_driver`, still unported,
  reads it).
- Tests:
  - 6 new tests in `crates/ugaris-core/src/world/tests/gatekeeper.rs`:
    `gate_room_is_clear_rejects_occupied_and_takeable_item_tiles` (blocked
    by a character, blocked then unblocked by a takeable item, unblocked
    by non-takeable furniture), `gate_take_money_and_give_money_silent_
    match_c`, `gate_finish_enter_room_teleports_strips_spells_and_resets_
    resources` (teleport target, slot `12`/`29` stripped vs. slot `30`
    untouched, HP/mana/endurance/`regen_ticker`, both `log_char` texts,
    and item destruction), `gate_finish_enter_room_leaves_zero_mana_
    untouched`, `gate_finish_enter_room_fails_when_already_at_target`
    (the Manhattan-distance-`1` guard), and the pre-existing `Ready`-path
    test rewritten from asserting a no-op to asserting the new
    `EnterTestReady` event fires with the correct class code.
  - 3 new tests in `crates/ugaris-server/src/tests/spawns.rs` (following
    that file's existing `ZoneLoader::load_character_templates_str`
    inline-template pattern): `gate_enter_test_spawn_room_success_spawns_
    opponent_and_resets_player` (full happy path against a real inline
    `gatekeeper_w` template - opponent identity/position/stats/driver
    message, player teleport/resources/stripped inventory/destroyed
    items/notice text, and the `PlayerRuntime` state write),
    `gate_enter_test_spawn_room_refunds_when_every_room_is_busy` (all 7
    `GATE_TEST_ROOM_STARTS` blocked by dummy characters), and
    `gate_enter_test_spawn_room_rejects_when_underfunded`.
- Verification: `cargo fmt --all` clean. `cargo test --workspace`: 1263
  `ugaris-core` (6 net new) + 36 db + 3 net + 33 protocol + 401 server (3
  net new), all green, zero failures. `cargo build -p ugaris-server`
  clean, zero warnings. A 12s boot-smoke showed "entering Rust game loop"
  with no panics.
- Remaining (left `[~]` in `PORTING_TODO.md`, precise notes there,
  unchanged from iteration 53 minus item (1)): (1) `gate_fight_driver`/
  `gate_fight_dead` (reuse `world/npc_fight.rs`) including `turn_seyan`;
  (2) the `NTID_GATEKEEPER` cross-NPC message hookup (the message is now
  queued onto the opponent, but nothing consumes it until (1) exists);
  (3) the idle "return to post" `secure_move_driver` safety net for the
  *welcome* NPC (the opponent's own post position is now stored in
  `rest_x`/`rest_y`, ready for (1) to consume).

## Ralph Loop - Gatekeeper Welcome NPC Idle Return-To-Post (Iteration 55)

Closed remaining item (3) from iteration 54's gatekeeper slice: the
welcome NPC's own idle "return to post" safety net
(`gatekeeper.c:627-631`, `if (dat->last_talk + TICKS*30 < ticker) { if
(secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_UP, ret, lastact))
return; }`).

- `crates/ugaris-core/src/world/gatekeeper.rs`:
  - New `GATE_WELCOME_RETURN_TO_POST_TICKS` (`TICKS * 30`).
  - `process_gate_welcome_actions`/`process_gate_welcome_messages` now
    take an `area_id: u16` parameter (needed by `secure_move_driver` ->
    `setup_walk_toward`), threaded from `ugaris-server`'s
    `config.area_id` the same way `process_bank_actions`/
    `process_janitor_actions` already do.
  - After the per-message loop and the `talkdir` turn, added the C tail:
    when `data.last_talk + GATE_WELCOME_RETURN_TO_POST_TICKS < tick`, call
    the existing (unchanged) `secure_move_driver` (`world/npc_idle.rs`)
    toward `(gate.rest_x, gate.rest_y)` with `Direction::Up as u8` (C's
    `DX_UP`) and `ret = 0, lastact = 0`.
  - Confirmed no new `Character` field was needed: every zone-spawned
    character (including this NPC) already has its spawn tile captured in
    `rest_x`/`rest_y` by `zone.rs`'s `MapDirective::Character` handler
    (the existing "C `pop_create_char` stores the spawn tile in
    `ch.tmpx/tmpy`" substitution used elsewhere for NPC post positions,
    e.g. `world::bank`, `respawn_npc_character`). `ret`/`lastact` are
    always `0`: like `world::trader`/`world::bank`, this driver class
    doesn't thread the C dispatcher's own per-character last-action/return
    code through the tick loop - a simplification already accepted
    elsewhere in this codebase, since that pair only changes behavior
    right after a same-tick door-use action, which this stationary
    greeter NPC never performs.
  - Updated the module doc comment's gap list and the function doc
    comment to drop the now-closed "not ported" note.
- `crates/ugaris-server/src/main.rs`: updated the
  `process_gate_welcome_actions` call site to pass `config.area_id`.
- `crates/ugaris-core/src/world/tests/gatekeeper.rs`: added a local
  `RETURN_TO_POST` tick constant and updated all 17 existing
  `process_gate_welcome_actions` call sites for the new `area_id`
  parameter (passing `0`, matching this file's single-area test world
  convention). New tests:
  `gate_welcome_returns_to_post_after_thirty_seconds_idle` (spawns the
  gate NPC away from its `rest_x`/`rest_y`, advances the tick past the
  30s threshold, and asserts `secure_move_driver`'s walk-toward branch
  fired: `action::WALK` and `tox`/`toy` stepped one tile toward the
  post), `gate_welcome_stays_put_shortly_after_talking` (same setup but
  with `last_talk` set to the current tick, asserting no movement/action
  change).
- Verification: `cargo fmt --all` clean. `cargo test --workspace`: 1265
  `ugaris-core` (2 net new) + 36 db + 3 net + 33 protocol + 401 server,
  all green, zero failures. `cargo build -p ugaris-server` clean, zero
  warnings. A 12s boot-smoke showed "entering Rust game loop" with no
  panics.
- Remaining (left `[~]` in `PORTING_TODO.md`, updated notes there): (1)
  `gate_fight_driver`/`gate_fight_dead` (reuse `world/npc_fight.rs`)
  including `turn_seyan`; (2) the `NTID_GATEKEEPER` cross-NPC message
  hookup (queued onto the opponent, but nothing consumes it until (1)
  exists); the opponent side of the idle "return to post" net (also
  inside `gate_fight_driver`) is bundled with (1) since that driver
  doesn't exist yet.

## Ralph Loop - Gatekeeper Fight Opponent Driver + Death Reward (Iteration 56)

Closed remaining items (1)/(2) from iteration 55's gatekeeper slice:
`gate_fight_driver` (`gatekeeper.c:641-696`) and `gate_fight_dead`
(`gatekeeper.c:705-763`), the private-room duel opponent NPC spawned by
`gate_enter_test_spawn_room` (iteration 54).

- New `crates/ugaris-core/src/world/gate_fight.rs`:
  - `World::process_gate_fight_actions(area_id) -> usize`: collects every
    `CDR_GATE_FIGHT` character and runs `process_gate_fight_tick` on each,
    mirroring `process_gate_welcome_actions`'s shape.
  - `process_gate_fight_tick`: message loop (`NT_CREATE` seeds
    `creation_time` from the live tick, exactly like C reading the global
    `ticker`; `NT_NPC`/`NTID_GATEKEEPER` sets the tracked `victim`),
    `TICKS*60*10` self-destruct (`npc_say` "Thats all folks!" +
    `remove_character`), a narrowed `fight_driver_update` (refreshes
    `victim_visible`/last-known position via `char_see_char`, or clears
    the victim if its character no longer exists - the same end state as
    C trashing a stale/deleted enemy slot), "attack visible" (reuses the
    already-generic `World::attack_driver_direct` from `world/
    npc_fight.rs` - adjacent-attack-or-pathfind-toward, needing no new
    code), "follow invisible" (walks toward the last-known position via
    `secure_move_driver`, giving up once within 2 tiles without finding
    the victim there), return-to-post via `rest_x`/`rest_y` (C's
    `tmpx`/`tmpy`, same substitution `gate_enter_test_spawn_room` already
    made when spawning this opponent), and the `regenerate_simple_baddy`/
    `spell_self_simple_baddy`/`idle_simple_baddy` tail (all three already
    fully generic despite their names - no `SimpleBaddy`-specific state
    touched). Deliberately does **not** port C's generic 10-slot `struct
    fight_driver_data`/`DRD_FIGHTDRIVER` enemy-list machinery
    (`fight_driver_update`/`_attack_visible`/`_follow_invisible`,
    `drvlib.c:2170-2345`, shared by many other NPC types): C's own
    `gate_fight_driver` never calls `fight_driver_add_enemy` itself
    (`standard_message_driver(cn, msg, 1, 0)` only exists to catch
    incidental attacks from third parties, impossible in this private,
    single-opponent duel room), so tracking just the one `victim` is
    behaviorally exact for this NPC while avoiding a much larger,
    unrelated generic-combat porting task.
  - `World::apply_gate_fight_reward(killer_id, target_class) -> bool`:
    the killer's `gate_ppd.target_class` is supplied by the caller since
    `World` cannot read `PlayerRuntime` itself (see below). Always queues
    "Well done." (C's unconditional `log_char` before the `switch`);
    class `5`/`6`/`7` (Arch-Warrior/-Mage/-Seyan'Du) grant `CF_ARCH` (+
    `V_RAGE`/`V_DURATION` = 1 for classes 5/6) and the channel-6 "Grats:
    ... now!" broadcast (`queue_channel_broadcast`, `COL_MAUVE`, exact
    per-class article/title text) unless the guard fails (already has a
    conflicting class flag, or class 7's missing prerequisite), in which
    case - matching C's early `return` - the function skips the final
    `teleport_char_driver(co, 181, 198)` entirely, leaving the killer in
    place. Class `8` (plain Seyan'Du) is a **documented gap**: C's
    `turn_seyan` (`src/system/tool.c:4278-4389`) is a full character
    re-roll to a new template, needing the still-unported per-character
    `DRD_DEPOT_PPD` and ~11 other unmodeled `DRD_*` ids it clears (see
    iteration 52's research notes). This port skips the flag mutation and
    substitutes an honest placeholder system-text message instead of C's
    "You are a Seyan'Du now."/grats broadcast (which would otherwise lie
    about the character's actual state), but still performs the
    unconditional teleport since C's `case 8` has no early `return` -
    the player is never left stuck in the private room.
  - `CharacterDriverState::GateFight(GateFightDriverData)` (new variant,
    `character_driver.rs`; `GateFightDriverData { creation_time, victim,
    victim_last_x, victim_last_y, victim_visible }` mirrors C's `struct
    gate_fight_driver_data { creation_time; victim; }` plus the narrowed
    visibility-tracking fields folded in from the generic enemy-list
    struct). All 4 existing exhaustive matches over `CharacterDriverState`
    (`character_driver.rs`, `world/npc_fight.rs`, `world/npc_idle.rs`,
    `world/npc_messages.rs`) updated with the new arm.
  - `crates/ugaris-core/src/zone.rs`'s `instantiate_character_template`:
    new `CDR_GATE_FIGHT` branch initializing
    `CharacterDriverState::GateFight` and pushing an `NT_CREATE` bootstrap
    message - the same substitution `CDR_LAB2UNDEAD` already uses, since
    Rust's `World::spawn_character` doesn't auto-notify creation the way
    C's `create_char` unconditionally does (`notify_char(n, NT_CREATE,
    ticker, 0, 0)`, `create.c:1128`). This fires for the
    `gatekeeper_w`/`_m`/`_s` opponent templates the same way it fires for
    zone-file NPCs, since `gate_enter_test_spawn_room` (iteration 54) also
    calls `instantiate_character_template`.
- Death dispatch: unlike `CDR_SIMPLEBADDY` (routed through the generic
  `CharacterDriverOutcome`/`execute_character_died_driver` dispatch),
  `CDR_GATE_FIGHT`'s death reward follows the same direct-from-hurt-event
  pattern already established for `CDR_SWAMPMONSTER`/`CDR_TEUFELRAT`/
  `CDR_CALIGARSKELLY`: a new `crates/ugaris-server/src/world_events.rs`
  function, `apply_gate_fight_death_from_hurt_event(runtime, world,
  event)`, checks `event.outcome.killed` + `target.driver ==
  CDR_GATE_FIGHT` + `killer.flags.contains(PLAYER)`, reads
  `PlayerRuntime::gate_target_class` (the one fact `World` cannot see
  itself), and calls `World::apply_gate_fight_reward`. Wired into the
  existing per-event loop inside `apply_pk_hate_from_hurt_events`
  alongside its siblings - no changes needed to `hurt.rs`'s generic
  `apply_character_death_driver` dispatch (confirmed only
  `CDR_SIMPLEBADDY` actually uses that path for deaths; every other
  monster-specific death behavior in this codebase already hooks the
  `LegacyHurtEvent` stream directly).
- `crates/ugaris-server/src/main.rs`: `world.process_gate_fight_actions(
  config.area_id)` called each tick right after
  `apply_gate_welcome_events`, with an `info!` log when any opponent
  acted (mirroring the `gate_welcome_events_applied` logging precedent).
  Added `CDR_GATE_FIGHT` to the existing `character_driver::{...}`
  `ugaris_core` import list.
- Tests: 15 new tests in `crates/ugaris-core/src/world/tests/gate_fight.rs`
  (`NT_CREATE` bootstrap sets `creation_time`; `NT_NPC`/`NTID_GATEKEEPER`
  sets `victim`; self-destructs after exactly `TICKS*60*10` but not
  before; attacks an adjacent visible victim; walks one step toward a
  visible-but-distant victim; returns to post when no victim is set; gives
  up chasing once arrived at an invisible victim's last known position
  whose character no longer exists; and 8 `apply_gate_fight_reward`
  cases covering classes 5/6/7 success + their guard-failure paths, class
  8's documented-gap placeholder + still-teleports behavior, and an
  unmatched class still teleporting). 2 new tests in `crates/ugaris-
  server/src/tests/world_events.rs` (`lethal_gate_fight_hurt_grants_arch_
  warrior_and_teleports_killer`: the full `apply_legacy_hurt` ->
  `apply_pk_hate_from_hurt_events` -> `ARCH` flag + teleport + system-text
  pipeline; `lethal_gate_fight_hurt_by_non_player_does_not_grant_reward`:
  a non-player killer is a no-op).
- Verification: `cargo fmt --all` clean. `cargo test --workspace`: 1280
  `ugaris-core` (15 net new) + 36 db + 3 net + 33 protocol + 403 server (2
  net new), all green, zero failures. `cargo build -p ugaris-server`
  clean, zero warnings. A 12s boot-smoke showed "entering Rust game loop"
  with no panics.
- Remaining (left `[~]` in `PORTING_TODO.md`, updated notes there): only
  `turn_seyan` (`src/system/tool.c:4278-4389`) and the `gate_fight_dead`
  class-8 (plain Seyan'Du) flag/reroll grant that depends on it - the
  documented gap described above. Every other part of the Gatekeeper NPC
  task (welcome dialogue, `enter_test` preconditions, private-room
  opponent spawn, the fight driver, and the class 5/6/7 death rewards) is
  now fully wired and tested end-to-end.

## Ralph Loop - `turn_seyan` + Gate-Fight Class-8 Reroll (Iteration 57)

Closed the last documented gap from iteration 56's gatekeeper slice: C
`turn_seyan` (`src/system/tool.c:4278-4389`), the full character re-roll
to a plain Seyan'Du that `gate_fight_dead`'s class-8 case calls
unconditionally (both of C's own guard checks for that case are commented
out, unlike classes 5-7).

- New `crates/ugaris-core/src/world/turn_seyan.rs`,
  `World::apply_turn_seyan(cn, seyan_base_values: &[i16]) -> bool`
  (character-only half; `World` has no `ZoneLoader` reference, so the
  caller supplies `"seyan_m"`'s template `value[1][]`, matching C's own
  `create_char("seyan_m", 0)` + `destroy_char(co)` without ever
  registering a throwaway character in `World`): copies the base values,
  resets `exp`/`exp_used`/`level`/`lifeshield`, zeroes every profession,
  un-equips worn items (slots `0..12`) into the first free inventory slot
  at/past 30 (persistent forward-scanning cursor, matching C's exact
  `m` variable behavior - destroys the item instead if inventory is
  completely full), destroys spell-slot items `12..30`,
  `destroy_chareffects` (documented no-op - no active-effect list modeled
  yet, same precedent as `world/gatekeeper.rs`/`world/death.rs`), sets
  `CF_MAGE|CF_WARRIOR|CF_ITEMS`, recomputes hp/endurance/mana from the
  deliberately *stale* `value[0]` (matching C's exact ordering - it reads
  `value[0]` before the `update_char` call that actually recomputes it,
  so this line's real effect is superseded by `update_char`'s own
  hp/endurance/mana-exceeds-max clamp in every practical case, but is kept
  for exact parity), then strips `IF_QUEST`-flagged items from the
  remaining inventory (C's full `0..INVENTORYSIZE` sweep, though only
  spell - already emptied - and regular inventory slots can still hold
  anything by this point).
- New `PlayerRuntime::clear_turn_seyan_ppd` (`crates/ugaris-core/src/
  player.rs`), the `PlayerRuntime` half of `turn_seyan`'s ~22 `del_data`
  calls (`World` cannot touch `PlayerRuntime`): resets the 14 ids that
  have dedicated typed fields (`DRD_TREASURE_CHEST_PPD`, `DRD_AREA3_PPD`,
  `DRD_FLOWER_PPD`, `DRD_RANDCHEST_PPD`, `DRD_DEMONSHRINE_PPD`,
  `DRD_FARMY_PPD`, `DRD_RANDOMSHRINE_PPD`, `DRD_TWOCITY_PPD`,
  `DRD_ORBSPAWN_PPD`, `DRD_RUNE_PPD`, `DRD_LAB_PPD`, `DRD_RATCHEST_PPD`,
  `DRD_STAFFER_PPD`, `DRD_ARKHATA_PPD`) to their empty/default state (so
  `encode_legacy_ppd_blob` naturally omits the block on next save), and
  strips the other 10 non-depot ids that have zero Rust representation at
  all (`DRD_FIRSTKILL_PPD`, `DRD_AREA1_PPD`, `DRD_RANK_PPD`,
  `DRD_MILITARY_PPD`, `DRD_ARENA_PPD`, `DRD_NOMAD_PPD`,
  `DRD_SIDESTORY_PPD`, `DRD_TUNNEL_PPD`, `DRD_STRATEGY_PPD`,
  `DRD_QUESTLOG_PPD` - all newly added to `player.rs` as delete-only
  constants transcribed from `src/system/drdata.h`, with no decode/encode
  logic since nothing else reads or writes them yet) straight out of the
  raw `ppd_blob` bytes via a new generic `strip_ppd_blocks(bytes,
  remove_ids)` helper, which mirrors the existing `DRD_JUNK_PPD`-skip
  precedent already inside `encode_legacy_ppd_blob` (parse blocks, drop
  matching ids, re-emit survivors byte-for-byte). `DRD_DEPOT_PPD`'s
  "clear `IF_QUEST` flags from the 80 depot item slots" remains a
  **documented gap**: no per-character legacy depot (`struct depot_ppd`)
  exists in Rust at all - `ugaris-server::depot`'s `AccountDepotState` is
  a distinct, newer, account-wide system - so nothing can put quest items
  into a system that doesn't exist yet, meaning this gap has no observable
  effect until that depot is ported.
- Wiring: `World::apply_gate_fight_reward`'s signature grew a
  `seyan_base_values: Option<&[i16]>` parameter; its class-8 arm now calls
  `apply_turn_seyan` when `Some`, sending the real "You are a Seyan'Du
  now." system text + channel-6 "Grats: ... is a Seyan'Du now!" broadcast
  on success, falling back to the old honest placeholder message
  otherwise (template unresolved, or the reroll itself failed). On the
  `ugaris-server` side, `apply_gate_fight_death_from_hurt_event` grew a
  `loader: &ZoneLoader` parameter (looks up `"seyan_m"` only when
  `target_class == 8`) and now also calls `PlayerRuntime::
  clear_turn_seyan_ppd` once `apply_gate_fight_reward` confirms the reroll
  happened. `apply_pk_hate_from_hurt_events` grew the same `loader`
  parameter to thread it through from `main.rs`'s tick loop (which already
  owns a `zone_loader: ZoneLoader`); every existing call site (1 in
  `main.rs`, 11 across `tests/area_apply.rs`/`tests/world_events.rs`)
  updated to pass it (`&zone_loader` / `&ZoneLoader::new()`).
- Tests: 9 new tests in `crates/ugaris-core/src/world/tests/turn_seyan.rs`
  (stat/exp/level/profession reset, `CF_MAGE|CF_WARRIOR|CF_ITEMS` flag
  set, worn-item move-into-free-slot vs. destroy-when-inventory-full,
  spell-slot-item destruction, quest-item stripping from remaining
  inventory, hp/endurance/mana clamped down to the new recomputed max via
  `update_character`, and the two defensive-guard failures: missing
  character, mismatched base-value array length). 2 new tests in
  `crates/ugaris-core/src/player.rs` (`clear_turn_seyan_ppd` resets every
  typed field it touches; strips only the unmapped ids from a raw
  `ppd_blob`, leaving `DRD_DEPOT_PPD` and other unrelated ids untouched).
  1 test in `crates/ugaris-core/src/world/tests/gate_fight.rs` rewritten
  (the old class-8 "still teleports without reroll" test, renamed to
  clarify it's the no-template fallback path) plus 1 new test (class-8
  success: a leveled/`ARCH`-flagged/worn-weapon-equipped killer actually
  rerolls to level 1, `MAGE|WARRIOR|ITEMS`, base HP 10, cleared
  professions, moved (not destroyed) weapon, and the real Seyan'Du
  messages). 1 new test in `crates/ugaris-server/src/tests/
  world_events.rs` (`lethal_gate_fight_hurt_class_eight_turns_killer_
  seyan_and_clears_turn_seyan_ppd`: the full `apply_legacy_hurt` ->
  `apply_pk_hate_from_hurt_events` -> reward -> reroll -> PPD-clear
  pipeline, with a real `ZoneLoader` loaded from an inline `"seyan_m"`
  template string, confirming both the character mutation and the
  `PlayerRuntime::demonshrines` PPD field get cleared).
- Verification: `cargo fmt --all` clean. `cargo test --workspace`: 1292
  `ugaris-core` (12 net new) + 36 db + 3 net + 33 protocol + 404 server (1
  net new), all green, zero failures. `cargo build -p ugaris-server`
  clean, zero warnings. A 12s boot-smoke showed "entering Rust game loop"
  with no panics.
- Remaining (left `[~]` in `PORTING_TODO.md`): the Gatekeeper NPC task is
  now believed fully ported end-to-end (welcome dialogue, `enter_test`
  preconditions, private-room opponent spawn, the fight driver, and all
  four class 5/6/7/8 death rewards including `turn_seyan`), modulo the two
  small documented gaps inside `turn_seyan` itself (`destroy_chareffects`
  no-op, `DRD_DEPOT_PPD` quest-flag strip). Recommended next step before
  marking the task `[x]`: a full line-by-line re-read of `gatekeeper.c`
  against the Rust port to confirm nothing else was missed.

## Ralph Loop - Gatekeeper NPC Full Re-Read + `immortal_dead` (Iteration 58, complete)

Did the recommended full line-by-line re-read of `gatekeeper.c` (830
lines) against the Rust port, function by function:
`analyse_text_driver`/`qa[]` (all 26 entries present,
`character_driver.rs:831-967`), `gate_welcome_driver`'s whole message loop
(NT_CHAR welcome_state 0-6 including the exact case-2/3/4 fallthrough
quirk, NT_TEXT dispatch codes 2/5-9, NT_GIVE giveback logic, turn-to-
speaker, idle return-to-post), `enter_test` (every precondition in C
order: `CF_PAID`, `teleport_next_lab`/`CF_GOD`, `CF_NOEXP`, per-class
flag validation, item-count `!=0`/`>3` gates, `take_money`, the 14-entry
`room_start[]` busy-refund loop), `enter_room` (9x17 empty scan,
opponent creation/hp-mana-endurance init, `notify_char`, `drop_char`,
`teleport_char_driver`, spell-slot strip 12-29, hp/mana/endurance reset to
`POWERSCALE*1`), `gate_fight_driver` (creation-time self-destruct,
victim tracking, attack/follow/return-to-post/regen/spell_self chain),
and `gate_fight_dead` (all 4 class rewards + `turn_seyan` + final
teleport) all confirmed fully and faithfully ported (see the dedicated
sections above, iterations 51-57).
- Found and fixed one real gap: `immortal_dead` (`gatekeeper.c:701-703`),
  the `ch_died_driver`/`CDR_GATE_WELCOME` death handler for the welcome
  NPC, had never been ported at all (no `CDR_GATE_WELCOME` handling
  anywhere in the death-dispatch chain). Ported as
  `apply_gate_welcome_death_from_hurt_event`
  (`crates/ugaris-server/src/world_events.rs`), following the same
  driver-filter idiom as the other `apply_*_death_from_hurt_event`
  handlers (`target.driver == CDR_GATE_WELCOME`, no killer-flags check -
  C's dispatch is unconditional here, unlike `gate_fight_dead`'s own
  player-only filter which lives in the *caller*), wired into
  `apply_pk_hate_from_hurt_events`'s per-event handler list. C's
  `immortal_dead` calls `charlog` (a server-log-only write, never
  client-visible - confirmed against `log.c`), so the port reuses the
  existing `debug!(target: "client_log", ...)` + `format_client_log_message`
  precedent already established for `ClientAction::Log`/`cl_log` in
  `main.rs`, rather than `queue_system_text` (which would incorrectly
  send it to the client). In practice this path is unreachable through
  normal combat since the welcome NPC template carries `CF_IMMORTAL`
  (`hurt()` already suppresses lethal damage to `CF_IMMORTAL`
  characters) - ported anyway for strict fidelity and to close the gap
  cleanly.
- Confirmed as an intentional, documented non-gap (not fixed, left as
  architecture note only): `labentrance`'s C `ret == -1` branch ("the
  area containing the next labyrinth part is down") has no Rust
  equivalent. The Rust `needs_next_lab` helper only reproduces
  `teleport_next_lab(cn, 0)`'s truthiness (`do_teleport = 0` always
  short-circuits `change_area` in C, so `-1` can never actually be
  returned in that call mode); actual cross-area lab-entry execution
  (`IDR_LABENTRANCE`'s own dispatch in
  `crates/ugaris-core/src/item_driver/area22_lab.rs`) only distinguishes
  "solved all" vs. "level too low" outcomes. Since this is a monolithic
  single-process area server (no separate per-area server processes that
  could independently be "down"), the C `-1` sentinel has no reachable
  Rust equivalent condition to model; left unported as architecturally
  moot, same precedent as other cross-area-server-process concepts
  elsewhere in the codebase.
- Tests: 2 new tests in `crates/ugaris-server/src/tests/world_events.rs`
  (`gate_welcome_death_is_handled_but_sends_no_client_message`: confirms
  the handler fires on a lethal hit to a `CDR_GATE_WELCOME` character
  without queuing any client-visible system text;
  `gate_welcome_death_handler_ignores_non_matching_driver_and_non_lethal_hits`:
  confirms the driver-mismatch and non-lethal-event guards both return
  `false`).
- Verification: `cargo fmt --all` clean. `cargo test --workspace`: 1292
  ugaris-core + 36 db + 3 net + 33 protocol + 406 server (2 net new), all
  green, zero failures. `cargo build -p ugaris-server` clean, zero
  warnings. A 10s boot-smoke showed "entering Rust game loop" with no
  panics.
- Conclusion: `src/system/gatekeeper.c` is now fully ported end-to-end
  with no remaining unaddressed gaps (the two pre-existing documented
  no-ops inside `turn_seyan` - `destroy_chareffects`, `DRD_DEPOT_PPD` -
  and the architecturally-moot `labentrance` `-1` branch above are the
  only known deviations, all intentional and documented). Marking the
  `PORTING_TODO.md` task `[x]`.

- Ralph Loop iteration 59 - `src/system/questlog.c` (P3 "Questlog
  initialization & quest state machine"): ported the 85-entry `struct
  questlog questlog[]` metadata table digit-for-digit (including the two
  trailing-space quest names, `QLF_XREPEAT`-only entries 25/26/27/28, and
  every name/level-range/giver/area/exp field) into `QUEST_TABLE`/
  `quest_meta()` (`crates/ugaris-core/src/quest.rs`). Ported
  `questlog_scale`'s repeat-completion exp decay curve as `scale_exp` and
  `questlog_done`'s level-based taper (`> 44`/`> 19`/`> 4`/else bands) as
  `taper_exp_by_level`, both pure functions independent of `World` so they
  stay testable without a live game world. Added `QuestLog::complete_legacy`,
  the full `questlog_done` port: increments `done` (saturating at the C
  6-bit bitfield's max 63), sets `flags = QF_DONE`, and returns a
  `QuestCompletion { times_done, granted_exp, nominal_exp }` for the caller
  to route through `World::give_exp`/`dlog`/`sendquestlog` (this leaf
  module has no access to `World`/`PlayerRuntime`, which live in different
  structures - `World` owns `Character`, while `QuestLog` lives on
  `PlayerRuntime`). While porting, found and fixed two pre-existing bugs in
  `QuestLog::open`/`close` (previously untested against real C semantics):
  `open` used `flags |= QF_OPEN`, which could leave a stale `QF_DONE` bit
  set after reopening a done quest - C `questlog_open` assigns
  `flags = QF_OPEN` outright; `close` used an unconditional `flags &=
  !QF_OPEN`, whereas C `questlog_close` only transitions when `flags` is
  *exactly* `QF_OPEN` (`if (quest[qnr].flags == QF_OPEN) quest[qnr].flags =
  QF_DONE;`), leaving other states (closed, already done) untouched. Added
  10 new tests in `crates/ugaris-core/src/quest.rs` covering the table's
  length/contents/trailing-space names/`QLF_XREPEAT` entries, the
  repeatability-flag/table sync (guards against the hand-maintained
  `QUESTLOG_FLAGS` table silently drifting from `QUEST_TABLE`), the full
  `scale_exp` curve (`cnt` 0 through 10+), all four `taper_exp_by_level`
  bands, `complete_legacy`'s first/repeat-completion/out-of-range
  behavior, and the corrected `open`/`close` semantics.
  REMAINING (documented in `PORTING_TODO.md`, task left `[~]`):
  `questlog_init`'s derivation of quest flags from `area1_ppd`/
  `area3_ppd`/`staffer_ppd`/`twocity_ppd`/`nomad_ppd` NPC-dialogue state
  (blocked on `area1_ppd`/`nomad_ppd` becoming real decoded structs -
  currently delete-only stubs), the per-area `questlog_reopen_qN` reset
  side effects, `quest_exp.h`'s per-encounter exp/money constants, and any
  actual wiring from NPC dialogue drivers (none of which call
  `QuestLog::open`/`complete_legacy` yet, since the area NPC drivers
  themselves are separate unported P4 tasks).
  Verification: `cargo fmt --all` clean. `cargo test --workspace`: 1302
  ugaris-core (+10 new) + 36 db + 3 net + 33 protocol + 406 server, all
  green, zero failures. `cargo build -p ugaris-server` and `cargo build
  --workspace` both clean with zero warnings. 10s boot-smoke showed the
  tick loop running with no panics.

- Ralph Loop iteration 60 - `src/system/questlog.c` continued (P3
  "Questlog initialization & quest state machine", resuming the `[~]`
  task): unblocked the previous iteration's "need `area1_ppd`/`nomad_ppd`
  to become real decoded structs" note. Promoted both from delete-only
  stubs (raw bytes stripped wholesale via `strip_ppd_blocks` inside
  `clear_turn_seyan_ppd`, no dedicated field on `PlayerRuntime`) to real
  fixed-layout codecs in `crates/ugaris-core/src/player.rs`:
  `struct area1_ppd` (`src/area/1/area1.h:24-75`, 39 `int` fields = 156
  bytes) and `struct nomad_ppd` (`src/common/nomad_ppd.h:9-13`,
  `nomad_state[10]`/`nomad_win[10]`/4 open-roll/bet ints/`tribe_member` =
  25 `int` fields = 100 bytes), each with `encode_legacy_*`/
  `decode_legacy_*` fixed-size round-trip functions and named
  get/set accessors for the fields `questlog_init_area1`/
  `questlog_init_nomad` need (yoakin/gwendy/nook/lydia/guiwynn/logain/
  reskin/brithildie/camhermit/jessica states for area1;
  `nomad_state[]`/`nomad_win[]` element accessors for nomad), wired into
  the full `decode_legacy_ppd_blob`/`encode_legacy_ppd_blob` match-arm
  dispatch (decode arm, encode arm, `had_area1`/`had_nomad`
  append-if-missing) exactly like the pre-existing `area3_ppd`/
  `staffer_ppd`/`twocity_ppd` pattern. `DRD_AREA1_PPD`/`DRD_NOMAD_PPD`
  moved out of the "11 unmodeled ids" doc comment group (now 9: dropped
  to 8 remaining ids plus depot) and `clear_turn_seyan_ppd` now clears
  the two typed fields directly instead of stripping their ids from the
  raw blob. Ported `questlog_init_area1` (`src/system/questlog.c:828-
  1039`) and `questlog_init_nomad` (`src/system/questlog.c:1571-1607`)
  as pure functions in `crates/ugaris-core/src/quest.rs`
  (`init_area1_quests`/`init_nomad_quests`), taking a plain
  `Area1QuestState`/`NomadQuestState` snapshot struct (this leaf module
  has no access to `PlayerRuntime`; `PlayerRuntime::area1_quest_state`/
  `nomad_quest_state` build the snapshot) - including the
  `GWENDYLON_STATE_*`/`JESSICA_STATE_*`/`BRITHILDIE_STATE_*`/
  `CAMHERMIT_STATE_*` NPC-dialogue-state constants copied from
  `src/common/npc_states.h`, and `mark_init_done`/`set_flags` helpers
  matching C's repeated `if (!quest[qnr].done) quest[qnr].done = 1;
  quest[qnr].flags = QF_DONE;` idiom (seeds `done` to 1 once, distinct
  from `complete_legacy`'s `done++` semantics - re-running `questlog_init`
  must never bump the completion counter). While adding tests, found that
  14 pre-existing `player.rs` tests reused
  `make_drd(DEV_ID_DB, 22 | PERSISTENT_PLAYER_DATA)` (now `DRD_AREA1_PPD`)
  as a placeholder "genuinely unmodeled id" for blob round-trip framing
  assertions; repointed them all at `DRD_RANK_PPD` (still unmodeled) to
  avoid the new area1 decode/encode logic hijacking those blocks.
  Added 6 new tests in `player.rs` (area1/nomad fixed-layout round-trip
  incl. the `LEGACY_*_PPD_SIZE` byte-count assertions, outer-blob
  replace/append framing, snapshot-builder correctness, out-of-range
  `nomad_state`/`nomad_win` index safety) and 6 new tests in `quest.rs`
  (every `init_area1_quests` branch ladder including the 4-quest
  Gwendylon skull chain, `init_nomad_quests`'s three threshold ladders,
  and the "re-init never bumps an already-seeded `done` past 1"
  invariant).
  REMAINING (documented in `PORTING_TODO.md`, task still `[~]`):
  `questlog_init_area3`/`questlog_init_staff`/`questlog_init_twocity`
  (`src/system/questlog.c:1040-1470`) - these need several more named
  accessors on the existing `area3_ppd`/`staffer_ppd`/`twocity_ppd` raw
  byte stores (e.g. `seymour_state`, `astro2_state`, `crypt_state`,
  `william_state`, `hermit_state`, `carlos_state`, `smugglecom_state`,
  `aristocrat_state`, `yoatin_state`, `countbran_bits`/`countbran_state`,
  `brennethbran_state`, `spiritbran_state`, `broklin_state`,
  `dwarfchief_state`, `dwarfshaman_state` - none exist yet); the
  top-level `questlog_init` dispatcher (the `quest[MAXQUEST-1].done == 55`
  "already initialized" sentinel gate + calling all 5 sub-functions); the
  per-area `questlog_reopen_qN` reset side effects; `quest_exp.h`'s
  per-encounter exp/money constants; and any wiring from NPC dialogue
  drivers (none of which call `QuestLog::open`/`complete_legacy`/
  `init_area1_quests`/`init_nomad_quests` yet, since the area NPC drivers
  themselves are separate unported P4 tasks).
  Verification: `cargo fmt --all` clean. `cargo test --workspace`: 1311
  ugaris-core (+12 new: 6 player.rs + 6 quest.rs) + 36 db + 3 net + 33
  protocol + 406 server, all green, zero failures. `cargo build -p
  ugaris-server` clean with zero warnings. 10s boot-smoke showed the tick
  loop running with no panics (this change doesn't wire the new pure
  functions into any live caller yet - no NPC driver advances
  `area1_ppd`/`nomad_ppd` state, so nothing calls them at runtime).

- Ralph Loop iteration 61 - `src/system/questlog.c` continued (P3
  "Questlog initialization & quest state machine", resuming the `[~]`
  task): ported the remaining three `questlog_init_*` sub-functions -
  `questlog_init_area3` (`src/system/questlog.c:1040-1203`),
  `questlog_init_staff` (`:1203-1394`), `questlog_init_twocity`
  (`:1470-1546`) - as `init_area3_quests`/`init_staff_quests`/
  `init_twocity_quests` in `crates/ugaris-core/src/quest.rs`, mirroring
  the previous iteration's `Area1QuestState`/`NomadQuestState` snapshot
  pattern with new `Area3QuestState`/`StaffQuestState`/
  `TwocityQuestState` structs built by
  `PlayerRuntime::area3_quest_state`/`staff_quest_state`/
  `twocity_quest_state` (`crates/ugaris-core/src/player.rs`). Unlike
  area1/nomad, `area3_ppd`/`staffer_ppd`/`twocity_ppd` already existed
  as real fixed-layout raw-byte blocks with a handful of named
  accessors (`kelly_state`/`clara_state`/`imp_flags` for area3;
  `shanra_state`/`forestbran_done` for staffer; `thief_state`/
  `thief_killed[]`/`goodtile[]`/`solved_library` for twocity) - this
  iteration only needed to add the remaining fields these three
  functions read: `seymour_state`/`astro2_state`/`crypt_state`/
  `william_state`/`hermit_state` for area3; `carlos_state`/
  `smugglecom_state`/`aristocrat_state`/`yoatin_state`/
  `countbran_state`/`countbran_bits`/`brennethbran_state`/
  `spiritbran_state`/`broklin_state`/`dwarfchief_state`/
  `dwarfshaman_state` for staffer; `sanwyn_state`/`skelly_state`/
  `alchemist_state` for twocity - all computed as byte offsets from the
  C struct field declaration order (`src/system/game/ppd_structs.h`),
  same technique as the pre-existing offsets. While computing the
  area3 offsets, found and fixed a real pre-existing size bug:
  `LEGACY_AREA3_PPD_SIZE` was `17 * 4` (68 bytes) but C `struct
  area3_ppd` (`src/area/3/area3.h:18-35` / `src/system/game/
  ppd_structs.h:109-127`) has 18 `int` fields (`int imp_kills,
  imp_flags;` declares two fields on one line, easy to undercount) =
  72 bytes; corrected to `18 * 4`. This was safe because every use of
  the constant went through the symbolic name (no hardcoded `68`
  anywhere) - the missing 4 bytes only mattered for the never-before-
  accessed tail field `kassim_item_wait_starttime`, so no existing
  behavior changed, only newly-added out-of-bounds access became
  possible-but-safe. Faithfully reproduced two legacy C quirks instead
  of "fixing" them, per the porting rule to treat the C source as
  authority: `questlog_init_area3`'s `william_state` ladder
  (`:1177-1191`) has no final `else` branch, so quests 22/23 are left
  with whatever flags they already had when `william_state <= 0`,
  unlike every other ladder in the function (which all reset to `0`
  in that case); and `questlog_init_staff`'s `yoatin_state` ladder
  (`:1284-1290`) has a copy-paste bug where the "open" `else if` branch
  tests `ppd->aristocrat_state > 0` instead of `ppd->yoatin_state >
  0` - both are covered by dedicated regression tests that document
  the quirk in the test name/comment rather than silently encoding it.
  Added 8 new tests in `quest.rs` (every branch ladder for all three
  new functions: area3's 7 NPC states across 2 tests, staff's 11 NPC
  states across 4 tests including the two quirk-regression tests,
  twocity's 4 NPC states across 2 tests) and 3 new tests in `player.rs`
  (fixed-layout round-trip + `*_quest_state()` snapshot-builder
  coverage for the newly-added area3/staffer/twocity fields).
  REMAINING (documented in `PORTING_TODO.md`, task still `[~]`): the
  top-level `questlog_init` dispatcher (the `quest[MAXQUEST-1].done ==
  55` "already initialized" sentinel gate + calling all 5
  sub-functions) is still not ported - it needs a Rust representation
  of `DRD_QUESTLOG_PPD` (`struct quest[MAXQUEST]`, distinct from the
  in-memory `QuestLog` type), and nothing calls any of the five
  `init_*_quests` functions yet since the area NPC dialogue drivers
  that would advance `area1_ppd`/`area3_ppd`/`staffer_ppd`/
  `twocity_ppd`/`nomad_ppd` state are themselves separate unported P4
  tasks. The per-area `questlog_reopen_qN` reset side effects and
  `quest_exp.h`'s per-encounter exp/money constants also remain
  unported.
  Verification: `cargo fmt --all` clean. `cargo test --workspace`: 1322
  ugaris-core (+11 new: 8 quest.rs + 3 player.rs) + 36 db + 3 net + 33
  protocol + 406 server, all green, zero failures. `cargo build -p
  ugaris-server` clean with zero warnings. 10s boot-smoke showed the
  tick loop running with no panics (this change doesn't wire the three
  new pure functions into any live caller yet, same caveat as the
  area1/nomad ones from the previous iteration).
