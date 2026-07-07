# Progress Archive

Verbose per-iteration progress notes moved out of `PORTING_TODO.md`
to keep the working task list cheap to read. Newest entries are
appended by hand only when a note is too detailed for the ledger.


Add one line per completed task: date, task, ledger section touched.

- (start)
- 2026-07-03: Regeneration tick (P0) - ported `regenerate()`/`act_idle()`
  regen to `crates/ugaris-core/src/world/regen.rs`; ledger section
  "Ralph Loop - Regeneration Tick" and the `do.h`/`do.c`/`act.c` primitive
  actions row in the Ported table.
- 2026-07-03: Skill raising (`CL_RAISE`) (P0) - ported `raise_value` to
  `crates/ugaris-core/src/item_driver/scrolls.rs` and `World::raise_skill`
  to `crates/ugaris-core/src/world/skills.rs`; wired
  `ClientAction::Raise` in `crates/ugaris-server/src/main.rs`; ledger
  section "Ralph Loop - Skill Raising (CL_RAISE)".
- 2026-07-03: Speed mode (`CL_SPEED`)/fight mode (`CL_FIGHTMODE`) (P0) -
  ported `World::set_speed_mode` to
  `crates/ugaris-core/src/world/speed.rs` and `check_endurance` fast-mode
  revert to `crates/ugaris-core/src/world/regen.rs`; wired
  `ClientAction::Speed`/`FightMode` in `crates/ugaris-server/src/main.rs`
  (fight mode confirmed a no-op in C); ledger section "Ralph Loop - Speed
  Mode (CL_SPEED) and Fight Mode (CL_FIGHTMODE)".
- 2026-07-03: Game clock advancement (P0, partial) - ported
  `World::advance_date` to `crates/ugaris-core/src/world/date.rs`, wired
  into `crates/ugaris-server/src/main.rs`'s tick loop and startup, and
  fixed timer-driven `ItemDriverContext` date fields in
  `crates/ugaris-core/src/world/item_outcomes.rs`; ledger section "Ralph
  Loop - Game Clock Advancement". REMAINING: map-wide light-dirty sector
  marking on daylight change deferred (see todo note/ledger for why).
- 2026-07-03: Game clock advancement (P0) - re-reviewed the deferred
  "light-dirty sector" remainder and closed it: confirmed by reading C
  `player.c:2357-2380` that `redo = 1` on `dlight` change only defeats a
  `skipx_sector` recompute-skip optimization that Rust's
  `map_diff_payloads`/`tile_visibility` never has in the first place (it
  always recomputes every visible tile's light from `world.date.daylight`
  and diffs), so there is no remaining correctness gap; marked `[x]`.
  No code changes, ledger section "Ralph Loop - Game Clock Advancement"
  updated with the closing note.
- 2026-07-03: Look at character (`CL_LOOK_CHAR`) (P0) - ported
  `World::look_character_text`/`World::look_character_paperdoll` to
  `crates/ugaris-core/src/world/text.rs` and wired
  `ClientAction::LookCharacter` in `crates/ugaris-server/src/main.rs`
  (reuses the pre-existing but previously-uncalled
  `PacketBuilder::look_inventory` builder for the `SV_LOOKINV` paperdoll);
  ledger section "Ralph Loop - Look At Character (CL_LOOK_CHAR)".
  REMAINING: labyrinth/first-kill/army-rank/PK/clan/club info lines and
  the looker-`CF_GOD` debug branch are documented gaps pending their own
  P2/P3 systems.
- 2026-07-03: Look at map item (`CL_LOOK_ITEM`) (P0) - added
  `look_map_item_text` to `crates/ugaris-server/src/inventory.rs`, reusing
  the existing `legacy_item_look_text`/`char_see_item` ports; wired
  `ClientAction::LookItem` into `apply_inventory_client_action` and the
  `main.rs` command-feedback dispatch; ledger section "Ralph Loop - Look
  At Map Item (CL_LOOK_ITEM)".
- 2026-07-03: Junk item (`CL_JUNK_ITEM`) (P0) - added
  `apply_junk_item_client_action` to
  `crates/ugaris-server/src/item_apply.rs` (checks `ItemFlags::NOJUNK` and
  calls the existing `World::destroy_item`, which already clears
  `cursor_item`/sets `CharacterFlags::ITEMS`); wired
  `ClientAction::JunkItem` in `crates/ugaris-server/src/main.rs`'s command
  dispatch alongside the gold arm; ledger section "Ralph Loop - Junk Item
  (CL_JUNK_ITEM)". Corrected the todo note: C's real gate is `IF_NOJUNK`,
  not `IF_QUEST` (confirmed by reading `player.c:1325-1337`).
- 2026-07-03: Ping (`CL_PING`) (P0) - added `PacketBuilder::ping` to
  `crates/ugaris-protocol/src/packet.rs` and wired the
  `ClientAction::Ping` match arm in `crates/ugaris-server/src/main.rs`
  (opaque 4-byte echo, no state change); ledger section "Ralph Loop -
  Ping (CL_PING)".
- 2026-07-03: NPC sighting messages (`NT_CHAR` emission) (P0, partial) -
  wired `notify_area(.., NT_CHAR, ..)` into `World::complete_walk`
  (`crates/ugaris-core/src/world/actions.rs`), gated on `CF_NONOTIFY`, and
  fixed the `notify_area` radius bug (16 -> C's `NOTIFY_SIZE = 32`) in
  `crates/ugaris-core/src/world/text.rs`; ledger section "Ralph Loop - NPC
  Sighting Messages (NT_CHAR Emission), Partial". REMAINING: other
  `act_*` completion call sites (take/use/drop/give/attack/spells/idle)
  and the merchant-scan-to-message-consumer migration are deferred - see
  todo note and ledger for details.
- 2026-07-03: `update_char` stat recomputation (P1, partial) - ported the
  full `create.c:1710` recompute algorithm as
  `World::update_character(cn)`/`recompute_character_values` in
  `crates/ugaris-core/src/world/character_values.rs` (equipment/spell
  modifier sum with seyan/single-class caps and `IF_BEYONDMAXMOD` bypass,
  skill base-attribute averaging, Body Control/Armor Skill/spell-average
  armor bonuses, Speed Skill/Athlete/Thief/Demon profession bonuses, day/
  night/clan attribute bonuses, HP/endurance/mana clamp, light re-emission
  via the existing `refresh_character_light_after_value_change`); wired
  into worn-slot equip/unequip in
  `crates/ugaris-server/src/inventory.rs::inventory_swap_slot` (`pos < 12`
  only, matching C); added 11 tests in
  `crates/ugaris-core/src/world/tests/character_values.rs`; ledger section
  "Ralph Loop - `update_char` Stat Recomputation". REMAINING: not yet
  wired into spell install/expiry, level up, login, or death respawn -
  see the todo note for the precise next slice.
- 2026-07-03: `update_char` stat recomputation (P1, partial, iteration 17) -
  switched every `world/spells.rs` install/expire call site from the old
  `apply_item_modifier_deltas`/`refresh_driver_spell_flags` helpers (now
  deleted) to `World::update_character`, matching each C call site 1:1;
  also wired `World::raise_skill` (`world/skills.rs`), player-death
  respawn (`World::die_character`, `world/death.rs`), and login (both the
  DB-snapshot path in `ugaris-server/src/snapshots.rs` and the
  template/scaffold path in `main.rs`); fixed a real pre-existing bug
  where poison's HP-modifier decrement/removal never actually recomputed
  effective HP; updated ~10 test fixtures in `world/tests/{spells,text,
  hurt,death,skills}.rs` to add realistic `values[1]` baselines now that
  the recompute genuinely enforces C's floor-clamp and modifier caps;
  ledger section "Ralph Loop - `update_char` Stat Recomputation" extended.
  STILL REMAINING: level-up (no level-up system ported yet) and
  item-driver-level raise/scroll/potion paths (need `&mut World` access
  threaded through the item-driver dispatch) - see the todo note for
  details.
- 2026-07-03: Experience/level-up side effects (P1, partial, iteration 19) -
  ported `exp2level`/`level2exp`/`level_value`
  (`src/system/tool.c:1272-1283`) into the new canonical
  `crates/ugaris-core/src/world/exp.rs`, consolidating and deleting the
  three duplicate copies that had accreted in `ugaris-server/src/
  spawns.rs`, `ugaris-server/src/area_apply.rs`, and `ugaris-core/src/
  item_driver/helpers.rs`; ported `World::check_levelup`
  (`tool.c:1318-1356`, level loop, save grant/reset, level-20 profession
  unlock, dirty-sector refresh) with 11 tests in the new
  `crates/ugaris-core/src/world/tests/exp.rs`; wired it into the killer-exp
  and `/god exp` paths via `commands_admin.rs::give_exp_with_runtime_
  modifiers` (now takes `&mut World` instead of `&mut Character`); ledger
  section "Ralph Loop - Experience/Level-Up Side Effects" (new). REMAINING:
  stat-scroll `raise_value_exp`'s `check_levelup` call, the "Grats"
  broadcast, achievements, `reset_name`, and ~7 other direct-mutation exp
  grant sites still bypass level-up entirely - see the todo note.
- 2026-07-03: Experience/level-up side effects (P1, partial, iteration 22) -
  ported the canonical `World::give_exp` (C `give_exp` `tool.c:1371-1423`)
  into `crates/ugaris-core/src/world/exp.rs`, making `world.settings.
  exp_modifier`/`hardcore_exp_bonus` the single source of truth (removed
  the duplicate `ServerRuntime` copies); `commands_admin.rs::
  give_exp_with_runtime_modifiers` is now a thin wrapper. Wired two more
  direct-mutation exp sites through it: `/milexp`
  (`commands_admin.rs`, also fixed a hardcoded `1.10` hardcore-military
  multiplier that should have read the live-tunable
  `hardcore_military_exp_bonus`) and the demon-shrine book
  (`ugaris-core/src/player.rs::touch_demonshrine` + its
  `ItemDriverOutcome::DemonShrine` caller in `main.rs`, which now also
  calls the previously-missing `update_char` for the Demon value bump);
  also wired `item_driver/food.rs`'s lollipop exp bonus through the new
  `ItemDriverOutcome::LollipopLicked` arm in `world/item_outcomes.rs`. 11
  new/updated tests across `world/tests/exp.rs`,
  `world/tests/item_outcomes.rs`, `tests/commands_admin.rs`, and
  `player.rs`; ledger section "Ralph Loop - Experience/Level-Up Side
  Effects" extended. REMAINING: `area_apply.rs`'s four random-shrine
  reward sites and `main.rs`'s ~4 inline quest/area reward grants still
  bypass `give_exp` - each just needs a mechanical `world.give_exp(...)`
  swap-in now that the infrastructure exists.
- 2026-07-03: `update_char` stat recomputation (P1, partial, iteration 25) -
  closed the `P_CLAN`/`areaID == 13` catacombs-bonus gap by adding a real
  `pub area_id: u16` field to `World` (`world/mod.rs`), set once from
  `ServerConfig::area_id` at startup (`main.rs`, right after
  `World::default()`) since this server process is single-area for its
  whole lifetime - avoided threading `area_id` as a parameter through
  `update_character`'s ~17 non-test call sites. `World::update_character`
  now computes `in_clan_area` as `self.area_id == 13 ||
  tile.flags.contains(MapFlags::CLAN)`, matching C `create.c:1856`
  exactly. 2 new tests in `world/tests/character_values.rs`; ledger
  section "Ralph Loop - `update_char` Stat Recomputation" and the
  `create.c` `update_char` row in the Ported table extended. Boot-smoked
  (`entering Rust game loop area_id=1`). REMAINING for this task: only
  `ch.ef[]` area-effect light contributions to `V_LIGHT` are unported -
  a larger, separate effects-attachment gap.
- 2026-07-03: `update_char` stat recomputation (P1, iteration 28) - closed
  the task's final documented gap by porting the `mod[V_LIGHT] +=
  ef[fn].light` character-attached-effect contribution
  (`World::character_attached_effect_light`,
  `crates/ugaris-core/src/world/character_values.rs`, summing
  `Effect::target_character`-matched effects' `.light`, capped at the
  four lowest-id effects to approximate C's fixed four-slot `ch.ef[]`
  array). 2 new tests in `world/tests/character_values.rs`; ledger
  section "Ralph Loop - `update_char` Stat Recomputation" and the
  `create.c` `update_char` row in the Ported table extended. Task
  checkbox flipped to `[x]` - all four recompute slices plus every
  call-site and sub-gap are now ported, with only the trivial
  `player_reset_map_cache` no-op and the four-slot approximation
  remaining as intentional, documented deviations.
- 2026-07-03: Equipment slot rules on swap (`CL_SWAP` into worn slots)
  (P1, iteration 29) - ported `can_wear`/`check_requirements`
  (`tool.c:943-1098`) into `World::can_wear` +
  `crates/ugaris-core/src/world/items.rs`, wired the gate into
  `inventory_swap_slot` (`crates/ugaris-server/src/inventory.rs`); ledger
  section "Ralph Loop - Equipment Slot Rules on Swap (`CL_SWAP`)". 15 new
  tests (6 core + 9 server), all green; boot-smoked past tick 232.
- 2026-07-03: Ground item decay (P1, iteration 30) - wired
  `World::set_item_expire` (already existed for body decay in
  `world/death.rs`) into `World::complete_drop`
  (`crates/ugaris-core/src/world/actions.rs`), mirroring C
  `set_item_map` (`map.c:36-85`)'s `if (it[in].flags & IF_TAKE)
  set_expire(in, item_decay_time)` combined with `set_expire`
  (`expire.c`)'s own `IF_NODECAY` no-op - gated on `TAKE && !NODECAY` at
  the call site since Rust's `set_item_expire` has no built-in
  `IF_NODECAY` check. 2 new tests in `world/tests/items.rs` (decays at
  exactly `item_decay_time` ticks; `IF_NODECAY` items never armed);
  ledger section "Ralph Loop - Ground Item Decay". Boot-smoked
  (game loop ticking, no panics).
- 2026-07-03: Serial validation everywhere (P1, iteration 32) - audited
  `player_driver.c` and ported the missing `PAC_KILL` pre-switch serial
  guard into `World::apply_player_action_setup`
  (`crates/ugaris-core/src/world/actions.rs`); found and fixed the actual
  live-traffic gap in `crates/ugaris-server/src/player_actions.rs::
  apply_player_action`, which hardcoded serial `0` for Kill/Give/
  CharacterSpell/character-targeted MapSpell instead of capturing
  `ch[co].serial` like C's `cl_kill`/`cl_give`/`player_driver_charspell`,
  silently defeating the existing fireball/ball character-serial checks
  in real gameplay; threaded `&World::characters` through
  `ServerRuntime::queue_action` (`crates/ugaris-server/src/main.rs`).
  7 new tests (2 core stale/matching-serial Kill tests, 5 server live-
  capture tests); fixed a pre-existing test (`setup_world_actions_
  promotes_deferred_legacy_player_fightback`) that used a mismatched
  mock serial the new guard now correctly rejects; ledger section
  "Ralph Loop - Serial Validation Everywhere". Boot-smoked (game loop
  ticking, no panics).
- 2026-07-03: PostgreSQL login hardening (P1, partial/`[~]`) - wired the
  `SessionEvent::Login` handler (`crates/ugaris-server/src/main.rs`) to
  send the exact legacy `SV_EXIT` reject (C `player_client_exit`,
  `src/system/player.c:260-276`/`396-444`) and disconnect instead of
  falling through to a scaffold spawn for every non-`Ready`
  `LoginOutcome` and DB error; added `login_reject_message()`
  (`crates/ugaris-server/src/login.rs`) with the nine legacy reject
  strings (`crates/ugaris-server/src/constants.rs`) plus 2 focused tests
  (`crates/ugaris-server/src/tests/login.rs`); ledger row for
  `database_character.c`/`player.c` login paths extended. REMAINING:
  mocked-pool/`DATABASE_URL`-gated tests for `begin_login_tx`'s row
  branching, `Duplicate`/`TooManyBadPasswords` construction, cross-area
  `NewArea` redirect (separate deferred task).
- 2026-07-03: PostgreSQL login hardening (P1, iteration 35, still
  partial/`[~]`) - constructed the previously-dead `LoginOutcome::
  Duplicate`/`TooManyBadPasswords` variants. Added `bad_passwords` table
  (`migrations/0004_bad_passwords.sql`, mirrors C's `badip` table) plus
  `is_ip_rate_limited`/`record_bad_password_attempt` in
  `crates/ugaris-db/src/character.rs` porting C `is_badpass_ip`/
  `add_badpass_ip` (`src/system/badip.c`) with the exact `>3`/60s,
  `>8`/1h, `>25`/24h thresholds, called from `begin_login` (before the
  transaction opens) and `begin_login_tx` (only on an existing row with a
  wrong password, matching C's anti-enumeration-preserving call site).
  Ported C `load_char_dup` (`database_character.c:731-753`) as an
  online-duplicate-account query inside `begin_login_tx`, run after the
  password/locked/paid checks with the same `account_id == 1` test-
  account exemption C hardcodes. Confirmed `clean_badpass_ips` is C dead
  code (never called) and intentionally left unported. 5 new
  `ugaris-db` tests. `cargo fmt --all` / `cargo test --workspace` (1130
  core + 12 db + 3 net + 33 protocol + 368 server, all green) / `cargo
  build -p ugaris-server` clean; boot-smoked past tick 230 with no
  panics. Still `[~]`: mocked-pool/`DATABASE_URL`-gated tests and a live
  end-to-end TCP reject test remain blocked on a real Postgres instance,
  unavailable in this environment.
- 2026-07-04: `quiet_say`/`say`/`emote` NPC speech helpers in core (P2,
  iteration 42) - added `World::npc_say`/`npc_quiet_say`/`npc_emote`/
  `npc_murmur` to `crates/ugaris-core/src/world/text.rs` plus
  `murmur_message`/`quiet_say_message` to `crates/ugaris-core/src/
  log_text.rs`; migrated `lab2_undead.rs`/`npc_idle.rs`/`merchant.rs`'s
  ad-hoc `pending_area_texts` pushes onto the new helpers (fixing two
  latent format/distance bugs found along the way); ledger section
  "Ralph Loop - NPC Speech Helpers (`quiet_say`/`say`/`emote`/`murmur`)".
- 2026-07-04: Gatekeeper NPC (P2, iteration 51, `[~]`) - ported the pure
  dialogue/precondition logic slice from `src/system/gatekeeper.c` into
  `crates/ugaris-core/src/character_driver.rs` (`GATEKEEPER_QA`,
  `gate_welcome_dialogue_step`, `gate_welcome_state_after_repeat`,
  `gate_enter_test_precheck`, `CDR_GATE_WELCOME`/`CDR_GATE_FIGHT`) and a
  `DRD_GATE_PPD` block to `crates/ugaris-core/src/player.rs`. World/tick
  wiring (room spawning, `turn_seyan`, fight driver, tick-loop dispatch)
  remains - see the task's REMAINING note. Ledger section "Gatekeeper
  NPC".
- 2026-07-04: Achievements (P3, iteration 65, `[~]`) - ported the core
  data model and stat-driven award logic from
  `src/module/achievements/achievement.c`/`achievement.h` as a new
  standalone leaf module `crates/ugaris-core/src/achievement.rs`: the
  full 127-entry `AchievementType` enum and `achievement_defs` table
  (Steam ids, names, descriptions, categories, progress targets - copied
  digit for digit via a source-parsing script to avoid transcription
  error, then spot-checked against the C source), `PentArea`/`AchCategory`
  enums, `Achievement`/`AccountAchievements`/`AchievementStats` structs,
  `AccountAchievements::award`/`add_progress`/`is_unlocked`/
  `get_progress`, `get_stat_progress` (the full stat-to-progress switch
  incl. u64->u32 saturating casts for demon/silver/gold/wealth counters),
  `area_to_pent_index`, and every `achievement_add_*`/`achievement_check_*`
  function (flowers/mushrooms/berries/potions/demons/pents/chests/stones/
  enemy_killed/pvp_kill/military_mission/tunnel_level/silver_mined/
  gold_mined/gold_earned/play_time/login_streak/level/skill/profession/
  exploration/clear_all), each returning the list of newly-unlocked
  achievements for a future caller to route through logging/Steam-sync/DB
  side effects this leaf module has no access to. 41 new tests covering
  the table's integrity and digit-for-digit content, every threshold
  ladder, the per-pent-area/hardcore/profession branch tables, login
  streak day-rollover semantics (first login/same-day/consecutive/gap),
  and the achieved-by name truncation to the C struct's 40-byte buffer.
  Not wired into any live call site yet (no persistence, no protocol
  packets, no command dispatch, no gameplay call sites) - see the task's
  REMAINING note for the itemized follow-up list. `cargo fmt --all`,
  `cargo test --workspace` (1386 core [+41] + 36 db + 3 net + 33 protocol
  + 406 server, all green, zero failures), `cargo build -p ugaris-server`
  clean with zero warnings, and a 10s boot-smoke showed ticking with no
  panics. Ledger section "Ralph Loop - Achievements Core Data Model".
- 2026-07-04 (iteration 66): Achievements (P3, still `[~]`) - closed
  REMAINING gap (1), persistence. Added `PlayerRuntime::achievement_data:
  AccountAchievements`/`achievement_stats: AchievementStats` fields
  (`crates/ugaris-core/src/player.rs`); added `Serialize`/`Deserialize`
  to `Achievement`/`AccountAchievements`/`AchievementStats`
  (`crates/ugaris-core/src/achievement.rs`, with a manual
  `achievement_array_serde` shim for the 128-entry array since serde's
  derive doesn't cover non-`Copy` const-generic arrays); added
  `crates/ugaris-server/src/achievement.rs` with byte-exact
  `DRD_ACHIEVEMENT_DATA`(7176B)/`DRD_ACHIEVEMENT_STATS`(176B)
  subscriber-blob block codecs (offsets verified against `achievement.h`
  with a throwaway C `sizeof`/`offsetof` probe, matching the exact
  `time_t`/`u64` alignment padding); wired into
  `apply_character_snapshot`/`character_save_request`
  (`crates/ugaris-server/src/snapshots.rs`) alongside the existing
  `DRD_ACCOUNT_WIDE_DEPOT` block, following its exact
  parse/replace-block/omit-if-default pattern. Added
  `DRD_ACHIEVEMENT_DATA`/`DRD_ACHIEVEMENT_STATS` constants
  (`crates/ugaris-server/src/constants.rs`). 14 new tests (3 core
  serde-roundtrip/short-buffer + 11 server byte-layout/roundtrip/
  subscriber-blob-block tests). Left `[~]`: gaps
  (2)-(5) (protocol packets, DB first-unlock tracking, command dispatch,
  gameplay call sites) are unstarted. `cargo fmt --all`, `cargo test
  --workspace` (1389 core [+3] + 36 db + 3 net + 33 protocol + 417
  server [+11], all green, zero failures), `cargo build -p
  ugaris-server` clean with zero warnings, and a 10s boot-smoke showed
  "entering Rust game loop" with no panics. Ledger section "Ralph Loop -
  Achievements Core Data Model" extended.
- 2026-07-04 (iteration 67): Achievements (P3, still `[~]`) - closed
  REMAINING gap (2), the Steam-achievement mod packets. New
  `crates/ugaris-protocol/src/mod_achievements.rs`: `SV_ACH_UNLOCK`
  (0x30)/`_PROGRESS` (0x31)/`_SYNC` (0x32)/`_STATS` (0x33) subtype
  constants, `ach_unlock` (51-byte packet) and `AchSyncEntry`/
  `ach_sync_batch` (5-byte header + 56-byte entries) builders, byte
  layout copied from the sibling `Ugaris_Protocol` repo's
  `include/ugaris/protocol/mod_achievements.h` (the actual header C
  `achievement.c:1291-1415` builds against; not part of the
  `Ugaris_Server` source tree itself). New `achievement_unlock_payload`/
  `achievement_sync_payloads` functions in
  `crates/ugaris-server/src/achievement.rs`, porting `achievement_
  send_to_client`/`achievement_sync_all` (`achievement.c:1291-1415`)
  including the batching-by-16 and the empty-trailing-final-packet edge
  case. Wired the login trigger: `player::DEFERRED_ACHIEVEMENTS` is now
  set in `ServerRuntime::login` (`main.rs`, previously only
  `DEFERRED_AUCTION` was set there), and a new tick-loop sweep mirrors C
  `tick_player`'s `ticks >= 2 && (deferred_init & DEFERRED_ACHIEVEMENTS)`
  gate (`player.c:3668-3674`): sends the batched sync payloads, then
  awards `ACHIEVEMENT_STARTED_UGARIS` and runs `check_level`/
  `check_exploration`/`check_login_streak`, queuing an `SV_ACH_UNLOCK`
  for each newly-unlocked achievement via the existing `sessions_for_
  character`/`send_to_session` fan-out (same pattern the auction
  login-notice sweep next to it uses). 10 new tests (4 protocol
  byte-layout + 3 server send-payload + wiring covered indirectly by the
  boot-smoke). Left `[~]`: gaps (3) DB first-unlock tracking/cross-server
  announce, (4) `/achievements`-family command dispatch, and (5) the
  ~15 real gameplay call sites (chests, gathering, combat, mining,
  quests, clans) are still unstarted. `cargo fmt --all`, `cargo test
  --workspace` (1389 core + 36 db + 3 net + 37 protocol [+4] + 420
  server [+3], all green, zero failures), `cargo build -p ugaris-server`
  clean with zero warnings, and a 10s boot-smoke showed "entering Rust
  game loop" with no panics. Ledger section "Ralph Loop - Achievements
  Core Data Model" extended.
- 2026-07-04: Achievements (P3, still `[~]`) - closed the "play time"
  gameplay call site (`src/system/player.c:3448-3462`,
  `achievement_add_play_time`): new `award_play_time_minute` helper in
  `crates/ugaris-server/src/achievement.rs`, wired into `main.rs`'s tick
  loop on the existing once-a-minute gate for every connected character;
  3 new tests in `tests/achievement.rs`; ledger section "Ralph Loop -
  Achievements Core Data Model" extended.
- 2026-07-05: Clan system (P3, still `[~]`) - ported `destroy_dungeon`'s
  `build_remove`/`build_empty` map-teardown sweep (`dungeon.c:725-786,
  1343-1364`) as `World::destroy_dungeon`/`build_remove_tile`/
  `build_empty_tile` in `crates/ugaris-core/src/world/dungeon_master.rs`:
  evicts a player via a same-area teleport chain (with a system-text
  warning) or removes an NPC outright, scatters/destroys any item
  (arming the standard item-decay timer for a body that found space),
  and clears every effect anchored to the tile, then a second sweep
  resets every tile to bare indoors floor. Documented the one real
  simplification this needed: C's `change_area` fallback (moving a
  cornered player to their stored rest point, possibly in a different
  area) is only reachable for its same-area case here, since this
  codebase runs one area per server process with no cross-area transfer
  yet (matching the existing "target area server is down" precedent in
  `crates/ugaris-server/src/transport.rs`); the cross-area case falls
  through to the same `remove_character` eviction C's `exit_char`
  fallback would produce. 13 new tests in
  `crates/ugaris-core/src/world/tests/dungeon_master.rs`. `cargo fmt
  --all`, `cargo test --workspace` (1900 core + 55 db + 3 net + 40
  protocol + 602 server, all green, zero failures), `cargo build -p
  ugaris-server` clean with zero warnings. Ledger section "Ralph Loop -
  Clan System" extended.
- 2026-07-05: Events (P3, now `[~]`) - ported `src/module/events/events.c`'s
  generic calendar-matching primitives (`is_date_in_range`/
  `is_time_in_range`/`is_day_matching`/`is_week_matching`, plus a
  from-scratch `days_from_civil`/`weekday_from_days`/`week_number` (glibc
  `strftime("%W")`) trio validated against real `date +%W` output) and all
  five `src/module/events/recurring/*.c` boosted-rate events (Double
  Experience Thursday, Double Drop Rate Tuesday, Double Experience & Drop
  Rate Weekend, Mining Monday, Mining Wednesday) as
  `crates/ugaris-server/src/events.rs`, wired into `main.rs`'s existing
  once-a-minute tick gate (mirroring C's `add_scheduled_task(check_events,
  60, ...)`). Added a `GameSettings::loot_modifiers` named-scalar registry
  (`get_loot_modifier`/`set_loot_modifier`, default 1.0) matching C
  `loot_get_modifier`/`loot_set_modifier` for the `event_drop_rate` hook.
  Preserved two real C quirks rather than "fixing" them: Mining
  Monday/Wednesday's `_end` hooks hardcode their multipliers back to `1.0`
  instead of restoring the snapshot their `_start` hooks captured (dead
  snapshot code in the C source), while Double Experience
  Thursday/Weekend's `_end` hooks do restore their captured
  `original_exp_modifier` snapshot exactly. 16 new tests in
  `crates/ugaris-server/src/tests/events.rs` plus 1 in
  `crates/ugaris-core/src/game_settings.rs`. `cargo fmt --all`, `cargo test
  --workspace` (1950 core + 55 db + 3 net + 40 protocol + 620 server, all
  green, zero failures), `cargo build -p ugaris-server` clean with zero
  warnings, 12s boot-smoke confirmed "entering Rust game loop" with no
  panics. Ledger section "Ralph Loop - Events (Recurring Boosted-Rate)"
  added.
- 2026-07-05: Events (`src/module/events/**`) (P3) - no new code; closed
  this `[~]` task to `[x]` after confirming its last blocker
  (`event_drop_rate` had no loot consumer) was already resolved by the
  "Death-mode loot tables" task's `world/loot.rs::compose_loot_modifier`
  (iteration 155), and that `RecurrenceType::Daily`/`Monthly` plus C's
  generic `schedule_one_time_event`/`cancel_event`/
  `get_event_bonus_multiplier` API have zero call sites anywhere in the
  legacy C tree outside `events.c` itself (verified via grep) - genuinely
  dead code, not a porting gap. `mining_*_multiplier`'s consumer stays
  explicitly out of scope (belongs to the unstarted P4 "Area 12 -
  `mine.c`" task). Ledger row for `events.c`/`.h`/`recurring/*.c`/
  `easter_event.c` updated accordingly.
- 2026-07-06: Cross-area transfer (P3, partial) - wrote the design plan
  (single-process stance: N OS processes, one area+mirror each, DB-
  mediated handoff), added `PacketBuilder::server_redirect` (`SV_SERVER`
  encoder) to `ugaris-protocol`, wired `AreaRepository` into
  `ugaris-server` (startup `mark_alive`/shutdown `mark_down`, new
  `--public-addr` CLI arg), and wired the login-side redirect
  (`LoginOutcome::NewArea` -> live-target lookup -> `SV_SERVER` +
  disconnect, falling back to the existing down-reject text only when
  unregistered/offline); ledger "Continuation Handoff" design-plan
  section and the new `crates/ugaris-db/src/area.rs` ledger row.
- 2026-07-06: Cross-area transfer (P3, still `[~]`) - wired two more of
  the six remaining mid-game teleport call sites into the existing
  `attempt_cross_area_transfer` helper: clan-spawn exit and mine
  gateway (both `ItemDriverOutcome` arms in
  `crates/ugaris-server/src/main.rs`), using `config.mirror_id` as both
  source and target mirror (C's `change_area` always resolves the
  target mirror as the character's own current mirror via `ch[cn].
  mirror`, and this codebase's per-process config already represents
  that). Remaining 4 call sites (`/office`+`/goto`, `/jail`/`/unjail`,
  dungeon-master rescue) and the periodic `mark_alive` heartbeat are
  unchanged; ledger row for `src/system/database/database_area.h`/`.c`
  updated with the iteration-222 note.
- 2026-07-06: Cross-area transfer (P3, still `[~]`) - wired the
  `/office` and `/goto`/`/jump` (`finish_goto_jump`) call sites into
  `attempt_cross_area_transfer` via a new deferred
  `KeyringCommandResult::cross_area_transfer` field, following the same
  established pattern as `/kick`'s save and `/setclanjewels`'s clan-log
  write (the command layer has no DB handle or `ServerRuntime` of its
  own); target mirror resolves to `mirror_changed` when the command set
  one, else the caller's own current area/mirror, matching C's
  `change_area` reading `ch[cn].mirror`. Updated 4 existing unit tests
  in `crates/ugaris-server/src/tests/commands_admin.rs` accordingly.
  Remaining 3 call sites (`/jail`/`/unjail`, dungeon-master rescue) and
  the periodic `mark_alive` heartbeat are unchanged; ledger row for
  `src/system/database/database_area.h`/`.c` updated with the
  iteration-223 note.
- 2026-07-06: Sector skip optimization (`skipx_sector`) (P3, now `[~]`) -
  health check was green (1084/1084 `ugaris-server` tests, full workspace
  passing) so picked this, the only remaining unchecked P3 item. Rather
  than guess at "likely fine for small player counts", added a real
  `#[ignore]`d profiling harness
  (`profile_map_diff_payloads_cost_at_realistic_player_counts` in
  `crates/ugaris-server/src/tests/map_sync.rs`) measuring
  `map_diff_payloads`'s current unconditional per-tile recompute cost
  (the LOS/`char_see_char` work C's `skipx_sector` would let it skip) at
  100 concurrent players, `view_distance=15`: ~27µs/player/tick, ~2.7ms
  total/tick, against the 24-tick/second (~41.6ms) tick budget - ~6.5% at
  a player count far above any real Ugaris concurrency, negligible at
  realistic counts. Confirms the deferral is still correct with real
  data. No behavior change; `crates/ugaris-core/src/sector.rs`'s already-
  ported `DirtySectors`/`skip_x_sector` remain unwired, ready for a
  future iteration if load data ever says otherwise. `cargo fmt --all`,
  `cargo test --workspace`, `cargo build -p ugaris-server` all green;
  boot-smoked (10s run, tick loop advancing, no panics).
- 2026-07-06: Area 1 - `gwendylon.c` (P4, now `[~]`) - re-verified the
  sector-skip deferral above independently (confirmed the same "dozens of
  scattered tile-mutation call sites, large cross-cutting change" finding
  via a fresh grep of `DirtySectors`/`mark_dirty_sector` call sites), then
  moved to this, the topmost P4 task, since P0-P3 have no other actionable
  work. Ported a self-contained first NPC slice: `camhermit_driver`
  (`CDR_CAMHERMIT = 14`, the forest hermit's two-quest bear-kill/tooth-
  necklace chain) end to end - message-loop dispatch (`NT_CHAR` state
  machine, `NT_TEXT` small-talk via a new `GWENDYLON_QA` table shared by
  every area-1 NPC driver, `NT_GIVE` give-back), the `questlog_open`/
  `_done`/`_reopen` wiring (first live NPC-driven questlog completion in
  this codebase), and a new general `World::give_char_item_smart`
  primitive (`give_char_item_smart`, `tool.c:3408-3494`, money/inventory/
  hand/drop/destroy cascade) for future NPC ports to reuse. New `crates/
  ugaris-core/src/world/camhermit.rs` (+tests), `crates/ugaris-server/src/
  area1.rs` (facts snapshot + event application, wired into `main.rs`'s
  tick loop), `IID_AREA1_SMALL_BEAR_TEETH`/`DEV_ID_RH` in `item_driver`,
  10 new `CAMHERMIT_STATE_*` constants promoted to `pub(crate)` in
  `quest.rs`. 9 new core tests. See `world::camhermit`'s module doc
  comment for the two documented gaps (color-marker styling dropped from
  one reminder line; `camhermit_kills` never advances yet since
  `monster_dead`, the shared area-1 death hook, is still unported).
  `cargo fmt --all`, `cargo test --workspace` (2270 core + 1084 server,
  all green), `cargo build -p ugaris-server` all clean with zero
  warnings; boot-smoked (12s run, tick loop advancing, no panics).
- 2026-07-06: Area 1 - `gwendylon.c` (P4, continuing, still `[~]`) -
  ported the second area-1 NPC slice: `yoakin_driver` (`CDR_YOAKIN = 9`,
  the hunter's bear-hunt quest at the knight castle, plus the shrike-
  talisman exp reward and generic leftover-give branches it also
  handles) end to end - `NT_CHAR` state machine (states 0-5, including
  the 120s intro-chain reset and the `logain_state`-gated state 2 -> 3
  transition), `NT_TEXT` small-talk via the same shared `GWENDYLON_QA`
  table `world::camhermit` already wired up, `NT_GIVE` (bear-tooth
  quest turn-in with first-completion-only gold reward, shrike-talisman
  exp reward, generic give-back fallback). New `crates/ugaris-core/src/
  world/yoakin.rs` (+13 tests), `World::destroy_items_by_template_id`
  (generic `destroy_item_byID` port, equipment+inventory+cursor scope)
  in `world/items.rs`, `IID_AREA1_BIGBEAR_TOOTH`/`IID_SHRIKE_TALISMAN` in
  `item_driver::ids`, `QLOG_YOAKIN` in `quest.rs`, `CDR_YOAKIN`/
  `YoakinDriverData`/`CharacterDriverState::Yoakin` in
  `character_driver.rs` (plus the three exhaustive-match call sites that
  needed the new variant), `CDR_YOAKIN` default driver-state wiring in
  `zone.rs`. Extended `crates/ugaris-server/src/area1.rs` (facts
  snapshot + event application) and wired into `main.rs`'s tick loop
  right after camhermit's. See `world::yoakin`'s module doc comment for
  the two documented gaps (color-marker styling dropped from the state-4
  reminder line, same as camhermit; the bear-tooth turn-in's
  `destroy_item_byID` sweep does not reach the account depot, since that
  storage lives outside `World`). `cargo fmt --all`, `cargo test
  --workspace` (2283 core + 1084 server, all green), `cargo build -p
  ugaris-server` all clean with zero warnings; boot-smoked (10s run,
  "entering Rust game loop" `area_id=1`, tick loop advancing, no
  panics).
- 2026-07-06: Area 1 - `gwendylon.c` (P4, continuing, still `[~]`) -
  ported the third area-1 NPC slice: `terion_driver` (`CDR_TERION = 11`,
  the village's ambient lore/storyteller NPC, `:1228-1472`) end to end -
  the `NT_NPC`/`NTID_DIDSAY` cross-NPC talk-throttle pre-pass unique to
  this driver (bumps `last_talk` whenever another nearby NPC just
  finished a line, without consuming the message), the 14-state ambient
  dialogue chain gated on `gwendy_state`/`reskin_state` (including two
  silent state jumps that change state without counting as "didsay"),
  `NT_TEXT` small-talk via the same shared `GWENDYLON_QA` table
  `world::camhermit`/`world::yoakin` already wired up (with its own
  4-bucket "repeat"/"restart" state-reset ranges, and no `current_victim`
  gate before the qa match - a genuine asymmetry vs. yoakin's own C
  source, preserved rather than "fixed"), and the generic `NT_GIVE`
  give-back fallback. New `crates/ugaris-core/src/world/terion.rs` (+13
  tests), `CDR_TERION`/`TerionDriverData`/`CharacterDriverState::Terion`
  in `character_driver.rs` (plus the three exhaustive-match call sites
  that needed the new variant), `CDR_TERION` default driver-state wiring
  in `zone.rs`. Extended `crates/ugaris-server/src/area1.rs` (facts
  snapshot + event application) and wired into `main.rs`'s tick loop
  right after yoakin's.   Terion is pure ambient dialogue - no quest log,
  no item reward, no gold - so `world::terion`'s module doc comment
  records no outstanding gaps of its own. `cargo fmt --all`, `cargo test
  --workspace` (2296 core + 1084 server, all green), `cargo build -p
  ugaris-server` all clean with zero warnings; boot-smoked (10s run,
  "entering Rust game loop" `area_id=1`, tick loop advancing, no
  panics).
- Iteration 247 (Area 1 - `gwendylon.c`, fourth NPC slice): ported
  `gwendylon_driver` (`CDR_GWENDYLON`, the main quest-giver mage's
  four-skull quest chain, `:234-673` - new `CDR_GWENDYLON = 8` constant).
  20-state `NT_CHAR` chain covering all four `QLOG_GWENDY_*` quests
  including all three `questlog_isdone` skip-ahead jumps ported as
  literal jump targets exactly as C writes them (a real, verified
  asymmetry: skipping skull 1 jumps to the next tier's own "done"
  checkpoint, but skipping skulls 2/3 jump to the tier *after that one's*
  "wait" checkpoint instead), plus the final `GWENDYLON_STATE_DONE_BLESS`
  periodic-bless branch. `NT_TEXT` small-talk via the same shared
  `GWENDYLON_QA` table with its own 5-bucket "repeat" state-reset ranges.
  `NT_GIVE`'s five branches share a new `gwendylon_turn_in_skull` helper
  for the four skull turn-ins (quest completion, inventory sweep, state
  advance, first-completion-only gold reward) plus the `IID_CALIGARLETTER`
  teleport-letter hand-off and the generic give-back fallback. The
  `DONE_BLESS` branch reproduces a genuine C `return`-before-
  `remove_message` quirk (the triggering `NT_CHAR` message is spliced
  back onto the NPC's own `driver_messages` and the turn/idle-move tail
  skipped for that tick, matching C's early `return` exactly) via
  `World::setup_bless_spell` (no new spell-system code needed).
  `IID_CALIGARLETTER`'s `change_area(co, 36, 240, 10)` needed a new
  `GwendylonCrossAreaTransfer` pending-queue on `World` (mirroring
  `world::jail`/`world::macro_npc`'s existing pattern), resolved by
  `crates/ugaris-server/src/area1.rs::apply_gwendylon_cross_area_transfers`
  through the existing shared `attempt_cross_area_transfer` helper, with
  Gwendylon audibly saying the C fallback line on failure (a `quiet_say`,
  not a private message - a genuine difference from every other
  cross-area call site). Nine new item IDs in `item_driver/ids.rs`. New
  `crates/ugaris-core/src/world/gwendylon.rs` (+9 tests),
  `CDR_GWENDYLON`/`GwendylonDriverData`/`CharacterDriverState::Gwendylon`
  in `character_driver.rs` (plus the two exhaustive-match call sites that
  needed the new variant), `CDR_GWENDYLON` default driver-state wiring in
  `zone.rs`. Extended `crates/ugaris-server/src/area1.rs` (facts snapshot,
  event application, cross-area transfer application) and wired into
  `main.rs`'s tick loop right after terion's. One documented gap shared
  with yoakin: `destroy_items_by_template_id` does not sweep the account
  depot. `cargo fmt --all`, `cargo test --workspace` (2305 core + 1084
  server, all green, zero failures, zero warnings), `cargo build -p
  ugaris-server`/`--workspace` all clean; boot-smoked (8s run, "entering
  Rust game loop" `area_id=1`, tick loop advancing, no panics).
- Iteration 248 (Area 1 - `gwendylon.c`, fifth NPC slice): ported
  `greeter_driver` (`CDR_GREETER = 13`, the tutorial-town Governor
  "Cameron" at the stronghold, `:1485-1798` - new `CDR_GREETER` constant).
  15-state `NT_CHAR` chain (states 0-14): class-gated (`CF_WARRIOR`/
  `CF_MAGE`) entry greeting with a silent Seyan'Du (both flags)
  fast-path straight to the terminal state, a level-7-gated weapon
  tutorial (small blade -> two-handed/staff -> fists -> outro), the
  "learn"-prompt/James-whimpering hint (conditional on `james_state`),
  a level-7-gated rest-area/recall-scrolls/movement/look-ground civics
  tutorial, and the final `QLOG_LYDIA`-gated understand-prompt/reminder
  pair (state 12/13, each reading `questlog_isdone(co, QLOG_LYDIA)`
  exactly like C - caught and fixed a first-draft mistake that dropped
  this quest-log read entirely). `NT_TEXT` small-talk via the shared
  `GWENDYLON_QA` table/`analyse_text_qa`, with "repeat" resetting to
  state 0 and "learn" rewinding to state 8 only from the state-7 "empty"
  checkpoint or state >= 13 (matching C's `current_victim`/`last_talk`
  reset-then-gate shape, the same as `world::yoakin`'s `NT_TEXT` handler
  rather than `world::terion`'s ungated one). `NT_GIVE` is the same
  give-back-or-destroy fallback every other area-1 ambient NPC uses. New
  `crates/ugaris-core/src/world/greeter.rs` (+20 tests),
  `CDR_GREETER`/`GreeterDriverData`/`CharacterDriverState::Greeter` in
  `character_driver.rs` (plus the two exhaustive-match call sites in
  `npc_fight.rs`/`npc_idle.rs` that needed the new variant), `CDR_GREETER`
  default driver-state wiring in `zone.rs` (`greeter_state`/
  `greeter_seen_timer`/`james_state` `area1_ppd` accessors already
  existed from earlier iterations' `showppd` work). Extended
  `crates/ugaris-server/src/area1.rs` (facts snapshot incl.
  `quest_log.is_done(QLOG_LYDIA)`, event application) and wired into
  `main.rs`'s tick loop right after the gwendylon cross-area-transfer
  block. No gameplay gaps beyond the documented `COL_LIGHT_BLUE`/
  `COL_RESET` marker drop (cosmetic, matches every other ambient NPC in
  this file). `cargo fmt --all`, `cargo test --workspace` (2325 core +
  1084 server, all green, zero failures, zero warnings), `cargo build -p
  ugaris-server`/`--workspace` all clean; boot-smoked (10s run, "entering
  Rust game loop" `area_id=1`, tick loop advancing, no panics).
- 2026-07-06: Area 1 `jessica_driver` (P4, slice) - ported the
  robber-operations two-quest chain (`CDR_JESSICA 125`, `gwendylon.c:
  1809-2065`) to new `crates/ugaris-core/src/world/jessica.rs` (+13
  tests): `NT_CHAR` 13-state dialogue machine (entry gated on
  `QLOG_NOOK` being done, five intro lines, `QUEST1_DO`/`QUEST2_DO`
  60-second reminder gates, `QUEST1_FINISH`/`QUEST2_FINISH` auto-advance
  with `questlog_open`/`questlog_done`), `NT_TEXT` via the shared
  `GWENDYLON_QA`/`analyse_text_qa` table with the "repeat" case resetting
  either quest's `_GIVE_1` checkpoint, and `NT_GIVE` turning in
  `IID_AREA1_ROBBER2NOTE` to finish quest 1 (new `IID_AREA1_ROBBER2NOTE`/
  `DEV_ID_KW` constants in `item_driver`). Preserved a genuine C behavior
  difference: jessica's own unwanted-item give-back calls plain
  `give_char_item` (`tool.c:3371-3394`), not `give_char_item_smart` like
  every sibling NPC in this file - added a new shared
  `World::give_char_item` in `world/items.rs` (promoted from a private
  `trader.rs` duplicate, which now calls the shared method instead) since
  no plain-give port existed yet. New `CDR_JESSICA`/`JessicaDriverData`/
  `CharacterDriverState::Jessica` in `character_driver.rs` (plus the two
  exhaustive-match call sites in `npc_fight.rs`/`npc_idle.rs`), new
  `pub(crate)` `JESSICA_STATE_QUEST1_DO`/`QUEST2_DO` constants and
  `pub(crate)` visibility on `JESSICA_STATE_QUEST1_FINISH`/
  `QUEST2_FINISH` in `quest.rs` (state accessors/`Area1QuestState`
  already existed from earlier `showppd`/`questlog_init_area1` work),
  `CDR_JESSICA` default driver-state wiring in `zone.rs`. Extended
  `crates/ugaris-server/src/area1.rs` (facts snapshot incl.
  `quest_log.is_done(QLOG_NOOK)`, event application - no achievement
  wiring needed since jessica's own quests carry no gold reward) and
  wired into `main.rs`'s tick loop right after the greeter block.
  Documented gap: the `QUEST2_DO` -> `QUEST2_FINISH` transition is driven
  by a separate `bredel_dead` monster-death hook (`gwendylon.c:2825-
  2842`) that is not ported anywhere yet (same class of gap as
  camhermit's `monster_dead` bear-kill counter), so the kill-quest cannot
  fully complete on a live server until that hook (or the broader
  `monster_dead` death-dispatch table) is ported; this driver's own
  dialogue already handles `QUEST2_FINISH` correctly once reached. `cargo
  fmt --all`, `cargo test --workspace` (2338 core + 1084 server, all
  green, zero failures, zero warnings), `cargo build -p ugaris-server`/
  `--workspace` all clean; boot-smoked (10s run, "entering Rust game
  loop" `area_id=1`, tick loop advancing, no panics).
- 2026-07-06: Area 1 - `src/area/1/gwendylon.c` (P4, `[~]`) - ported its
  seventh self-contained NPC slice: `jiu_driver` (`CDR_JIU = 127`, the
  forest sanctuary pilgrim's riverbeast-kill quest, `QLOG_JIU`,
  `:2074-2247`) to new `crates/ugaris-core/src/world/jiu.rs`
  (`JiuDriverData`/`CDR_JIU`/`CharacterDriverState::Jiu` added to
  `character_driver.rs` with its two exhaustive-match update sites in
  `world/npc_fight.rs`/`world/npc_idle.rs`, `CDR_JIU` default driver-
  state wiring in `zone.rs`). Ports the full `NT_CHAR` 5-state dialogue
  machine (level-39-gated entry split, `STORY1`'s `questlog_open`,
  `WAIT_FOR_KILL`'s silent no-op, `BEAST_KILLED`'s thanks-plus-
  `questlog_done`), `NT_TEXT`'s "repeat" via the shared `GWENDYLON_QA`
  table, and the generic `NT_GIVE` give-back. `QLOG_JIU`/
  `area1_jiu_state`/`area1_jiu_seen_timer` already existed as unused
  surface, so no new `PlayerRuntime` accessors were needed. Preserved a
  genuine C dead-code quirk instead of "fixing" it: `jiu_driver`'s two
  consecutive `NT_CHAR` throttle checks both gate on the identical
  `TICKS*10` threshold (unlike yoakin's distinct `TICKS*5`/`TICKS*10`
  pair), making the second unreachable - ported as the single effective
  check. Extended `crates/ugaris-server/src/area1.rs` (`jiu_player_facts`/
  `apply_jiu_events`, no achievement wiring needed since Jiu's quest
  carries no gold reward) and wired into `main.rs`'s tick loop right
  after the jessica block. Documented gap: `riverbeast_dead`
  (`gwendylon.c:2255-2272`) is not ported anywhere yet - same class of
  gap as jessica's `bredel_dead`/camhermit's `monster_dead` - so the
  `_WAIT_FOR_KILL` -> `_BEAST_KILLED` transition cannot fire on a live
  server until that hook (or the broader `monster_dead`/`ch_died_driver`
  death-dispatch table) is ported; this driver's own dialogue already
  handles `_BEAST_KILLED`/`_DONE` correctly once reached. `cargo fmt
  --all`, `cargo test --workspace` (2348 core + 1084 server, all green,
  zero failures, zero warnings), `cargo build -p ugaris-server`/
  `--workspace` all clean; boot-smoked (10s run, "entering Rust game
  loop" `area_id=1`, tick loop advancing, no panics).
- 2026-07-07: Area 1 (`gwendylon.c`) (P4, continued) - closed the
  `monster_dead`/`bredel_dead`/`riverbeast_dead` death-hook gap that
  camhermit/jessica/jiu's own module doc comments had all independently
  flagged as their remaining blocker. Added `CDR_RIVERBEAST` (128),
  `CDR_CAMERON_FORESTMONSTER` (129), `CDR_BREDEL` (154) driver-ID
  constants (`crates/ugaris-core/src/character_driver.rs`, values from
  `src/system/drvlib.h:177,178,205`). Ported area 1's `monster_dead`
  (`gwendylon.c:5201-5231`) split in two, mirroring the existing
  `apply_swamp_monster_death_driver`/`apply_swamp_monster_death_from_
  hurt_event` shape: `World::apply_area1_monster_death_driver`
  (`crates/ugaris-core/src/world/hurt.rs`, the noon/stone-circle weapon-
  glow half, `+= 5` charge vs area 15's `+= 12`) plus
  `apply_area1_monster_death_from_hurt_event` (`crates/ugaris-server/src/
  world_events.rs`, the `camhermit_kills` counter half, needs
  `PlayerRuntime`). Ported `bredel_dead` (`:2825-2842`) and
  `riverbeast_dead` (`:2255-2272`) directly as
  `apply_bredel_death_from_hurt_event`/`apply_riverbeast_death_from_hurt_
  event` (`world_events.rs`), each checking the killer's `CF_PLAYER`
  flag and current `jessica_state`/`jiu_state` before advancing it and
  queuing the exact `log_char` text. All three wired into the existing
  `apply_pk_hate_from_hurt_events` per-`LegacyHurtEvent` dispatch loop
  alongside the swamp-monster/teufel-rat/caligar-skelly hooks it already
  runs. Added 2 `World`-level tests (`world/tests/hurt.rs`: noon-trigger
  weapon glow + rejection of repeat/off-hour kills) and 4
  `ServerRuntime`-level tests (`crates/ugaris-server/src/tests/world_
  events.rs`: riverbeast advances `jiu_state` 2->3 with the reward line,
  a non-`WAIT_FOR_KILL` player is ignored, bredel advances `jessica_
  state` 10->11 with its reward line, and a forest-monster kill counts
  `camhermit_kills` up to 10 and emits the "killed 10 big bears" line).
  Now all three previously-blocked quest chains
  (camhermit/jessica/jiu) can complete end-to-end on a live server.
  `cargo fmt --all`, `cargo test --workspace` (2350 core + 1088 server,
  all green, zero failures, zero warnings), `cargo build -p ugaris-
  server` clean; boot-smoked (10s run, "entering Rust game loop"
  `area_id=1`, tick loop advancing, no panics). REMAINING for Area 1:
  every other NPC/death-hook branch listed in the task's own REMAINING
  note above (forest_ranger/brithildie/bigbadspider_dead/james/nook/
  lydia/balltrap_skelly/robber/sanoa/reskin/asturin/guiwynn/logain, plus
  the shared `gwendylon_dead`/`asturin_dead` tail).
- 2026-07-07: Area 1 (`gwendylon.c`) (P4, continued) - ported
  `forest_ranger_driver` (`CDR_FOREST_RANGER` = 155, `:2284-2473`), the
  bear-attack warning sentry near the forest stone circle. New
  `crates/ugaris-core/src/world/forest_ranger.rs`
  (`process_forest_ranger_actions`/`ForestRangerPlayerFacts`/
  `ForestRangerOutcomeEvent`, same `World`/`PlayerRuntime` split as
  `world::terion`/`world::yoakin`), `ForestRangerDriverData`/
  `CharacterDriverState::ForestRanger` (`character_driver.rs`),
  `area1_forest_ranger_state`/`area1_forest_ranger_seen_timer` accessors
  at the correct `area1_ppd` field offsets 37/38
  (`crates/ugaris-core/src/player.rs`), zone-spawn wiring for the
  `forest_ranger` template (`crates/ugaris-core/src/zone.rs`), and
  server-side facts/apply wiring
  (`crates/ugaris-server/src/area1.rs`/`main.rs`, appended after the
  jiu wiring in the tick loop). Ported the `ENTRY`/`WARNING_1`/
  `WARNING_2`/`HINT_1`/`GREET` state machine exactly, including two
  genuine C-source quirks preserved rather than "fixed": the `ENTRY`
  branch gates on the *ranger's own* `ch[cn].level`, not the greeted
  player's (unique among this file's ambient NPCs), and the `NT_CHAR`
  branch's second throttle check is unreachable dead code in C itself
  (both conditions test the identical `ticker < last_talk + TICKS*10`,
  so the second can never fire once the first has already passed) -
  documented in the module doc comment, not silently dropped. Also
  ported the wider `char_dist(cn, co) > 15` greet range (every sibling
  NPC in this file uses `10`). Deliberately NOT ported: the idle body's
  `WN_LHAND` torch-relight upkeep (`gwendylon.c:2438-2451`) - a cosmetic
  light-radius detail around a single stationary `CF_IMMORTAL` NPC that
  would require threading the full `execute_item_driver_request`/
  `apply_item_driver_outcome` pipeline (built for player-initiated `use`
  requests) through a new NPC-idle call site; documented as a gap in the
  module doc comment. Added 13 `World`-level tests
  (`crates/ugaris-core/src/world/tests/forest_ranger.rs`): both entry
  branches (ranger level above/below 30), each dialogue state's text and
  transition, the greet-repeat timing window (both sides), the wider
  15-tile greet distance vs. the shared 10-tile threshold, the `NT_TEXT`
  "repeat" reset (both the `GREET`-only gate and the non-`GREET` no-op),
  the `NT_GIVE` item-return, and the `TICKS*10` talk throttle. Extended
  `player.rs`'s existing `area1_ppd` codec tests
  (`area1_ppd_codec_matches_legacy_c_layout`/
  `area1_ppd_exposes_remaining_fields_for_showppd`) to cover the two new
  fields and corrected the latter's doc comment, which previously
  (accurately, before this iteration) called `forest_ranger_state`/
  `forest_ranger_seen_timer` "never read" - they now back a real
  gameplay driver, even though `cmd_showppd` itself still never prints
  them (confirmed by re-reading the whole C function). `cargo fmt --all`,
  `cargo test --workspace` (2363 core + 1088 server, all green, zero
  failures, zero warnings), `cargo build -p ugaris-server` clean;
  boot-smoked (10s run, "entering Rust game loop" `area_id=1`, tick loop
  advancing, no panics). REMAINING for Area 1: every other NPC/death-hook
  branch listed in the task's own REMAINING note above (brithildie/
  bigbadspider_dead/james/nook/lydia/balltrap_skelly/robber/sanoa/
  reskin/asturin/guiwynn/logain, plus the shared `gwendylon_dead`/
  `asturin_dead` tail).
