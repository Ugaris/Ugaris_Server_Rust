# Porting TODO

This is the forward-looking work list for porting the legacy C server
(`/home/eddow/Development/UgarisProjects/Ugaris_Server/src/**`) to this Rust
workspace. Work through it top to bottom: earlier sections block gameplay
more than later ones. `PORTING_LEDGER.md` records what is already done; this
file records what to do next. Keep both in sync as you work.

Read `AGENTS.md` first. The C server is the behavioral oracle: when in doubt,
open the C function and copy its observable behavior exactly, including
constants, message text, byte layouts, and stupid-looking edge cases.

---

## How To Work A Task

Follow this recipe for every checkbox. Do not skip steps.

1. Pick the topmost unchecked task in the highest-priority section you can
   make progress on. Do one task at a time.
2. Read the referenced C source completely before writing Rust. Also grep the
   Rust tree for related code that already exists: most systems are partially
   ported and you must extend them, not duplicate them.
3. Place code in the module the task names. Never grow a file past ~2,000
   lines; split like the existing module layout (see `AGENTS.md`,
   `Module Layout Rules`).
4. Write focused tests for every ported behavior in the matching
   `tests/<domain>.rs` file. Test the C-visible behavior (values, text,
   packet bytes, state transitions), not implementation details.
5. Verify before marking done. All three must pass with **zero warnings**:

   ```bash
   cargo fmt --all
   cargo test --workspace
   cargo build -p ugaris-server
   ```

6. Boot-smoke when your change touches the runtime loop, login, map sync, or
   protocol:

   ```bash
   target/debug/ugaris-server --bind-addr 127.0.0.1:5556
   # expect: "legacy TCP listener ready", "loaded area zone map",
   #         "entering Rust game loop", no panics for 10+ seconds
   ```

7. Update the paperwork:
   - Tick the checkbox here (`- [x]`), and add a one-line note if you
     deviated from C or left a follow-up gap.
   - Add or extend the matching row in `PORTING_LEDGER.md` (Ported/Partial
     table plus a short progress bullet at the end of the file).

### Hard Rules

- Never delete or weaken an existing test to make yours pass. If an existing
  test conflicts with C behavior you just verified in the C source, fix the
  test and say so in the ledger note.
- Never change packet byte layouts, `IDR_*`/`CDR_*`/`AC_*`/flag constants,
  message strings, or formulas away from C. Copy them digit for digit.
- Do not refactor unrelated code, do not update dependencies, do not touch
  `.ralph/`, and do not "improve" C logic. Port it; note oddities in a
  comment (`// C: ...`).
- Prefer typed outcomes over direct mutation across layers: item/character
  drivers return outcome enums, `World` applies them, `ugaris-server` does
  I/O. Follow the existing patterns in the module you edit.
- **One file per NPC**: a new NPC gets exactly one core file
  (`world/npc/<area>/<name>.rs` - driver data, arg parser, QA table, and
  `impl World` logic together) plus its server tick pass in
  `tick_npc/<area>.rs`. Register the pass in `tick_npc::run_all` in driver
  order. `main.rs`, `character_driver.rs`, and `world_events/` must not
  grow when adding an NPC.
- **Persistence is typed serde now**: new persistent player state is a
  `#[serde(default)]` field on `PlayerRuntime` (submodule of
  `crates/ugaris-core/src/player/`) - it persists automatically through
  `characters.player_state_json`. Do NOT write new legacy `DRD_*_PPD`
  codecs unless C gameplay logic reads the byte layout at runtime; the
  blob codecs are read-only fallback for pre-0020 rows.
- When you touch `crates/ugaris-db`, also run the live container suite:
  `cargo test -p ugaris-db -- --ignored` (requires Docker; it is excluded
  from the default run).
- File size limit: ~800 lines for driver/NPC files, ~2,000 hard cap
  elsewhere. For mechanical splits use `tools/rust_split/splitter.py`
  (see the specs referenced in `PORTING_LEDGER.md` for examples).
- If a task is too big for one sitting, port a self-contained slice, test it,
  mark the checkbox as `- [~]` (in progress) with a note about what remains.

### Where Things Live (quick map)

| Concern | Location |
|---|---|
| Item drivers (use/timer behavior) | `crates/ugaris-core/src/item_driver/<domain>.rs`; unique drivers get their own file |
| World mutation, actions, combat | `crates/ugaris-core/src/world/<system>.rs` |
| NPCs (data + parser + dialogue + world logic) | `crates/ugaris-core/src/world/npc/<area>/<name>.rs` (one file per NPC) |
| NPC/queued-event server tick passes | `crates/ugaris-server/src/tick_npc/<area>.rs`, ordered in `tick_npc::run_all` |
| Driver registry (ids, dispatch, state enum, shared QA framework) | `crates/ugaris-core/src/character_driver.rs` |
| Player session/persistent state | `crates/ugaris-core/src/player/<system>.rs` |
| World event drains (text, death hooks, admin DB tasks) | `crates/ugaris-server/src/world_events/` |
| Wire packets | `crates/ugaris-protocol/src/packet.rs`, `command.rs` |
| Server loop phases, client sync, text commands | `crates/ugaris-server/src/<concern>.rs` |
| DB repositories + migrations | `crates/ugaris-db/src/*.rs`, `migrations/` |
| Local dev DB / live DB tests | `compose.yaml`, `crates/ugaris-db/tests/postgres_roundtrip.rs` |
| Legacy client (for checking what the client expects) | `../astonia_community_client/src/client/protocol.c` |

---

## P0 - Playability Blockers

These make the game actually playable solo on area 1. Do these first, in
order.

- [x] **Regeneration tick** - characters never recover HP/endurance/mana. *(done - details in PORTING_LEDGER.md)*
- [x] **Skill raising (`CL_RAISE`)** - parsed but ignored; players cannot *(done - details in PORTING_LEDGER.md)*
- [x] **Speed mode (`CL_SPEED`) and fight mode (`CL_FIGHTMODE`)** - both *(done - details in PORTING_LEDGER.md)*
- [x] **Player death saves** - `die_character` never consults `saves`. *(done - details in PORTING_LEDGER.md)*
- [x] **Game clock advancement** - `world.date` never moves; it is always *(done - details in PORTING_LEDGER.md)*
- [x] **Look at character (`CL_LOOK_CHAR`)** - parsed, ignored. *(done - details in PORTING_LEDGER.md)*
- [x] **Look at map item (`CL_LOOK_ITEM`)** - parsed, ignored. Reuse *(done - details in PORTING_LEDGER.md)*
- [x] **Junk item (`CL_JUNK_ITEM`)** - C `cl_junk_item` destroys the cursor *(done - details in PORTING_LEDGER.md)*
- [x] **Ping (`CL_PING`)** - C echoes `SV_PING`/`SV_LPING` with the client *(done - details in PORTING_LEDGER.md)*
- [x] **Fast sell (`CL_FASTSELL`)** - C `cl_fastsell` sells an inventory *(done - details in PORTING_LEDGER.md)*
- [x] **NPC sighting messages (`NT_CHAR` emission)** - NPCs only "see" *(done - details in PORTING_LEDGER.md)*
---

## P0.5 - Structural Maintenance (do these before new area content)

- [x] **Finish main() phase decomposition** - `main.rs` is ~7.1K lines; the
  remaining tick-branch phases (world stepping, client command loop, sync)
  should extract into phase functions like `tick_npc::run_all` did for NPC
  passes. Verbatim moves with superset params; keep execution order.
  REMAINING: the "world stepping" phase is extracted into
  `tick_world::world_step`, the post-NPC-pass "sync" phase is extracted
  into `tick_sync::sync_phase`, and now the queued-client-action drain/
  dispatch/feedback-flush phase (draining `drain_actions_for_tick`,
  matching every `ClientAction` variant, then flushing feedback/inventory/
  container/name-refresh packets) is extracted into
  `tick_client_actions::process_queued_client_actions`
  (`crates/ugaris-server/src/tick_client_actions.rs`, 1,388 lines cut from
  `main.rs`, now ~5.3K). Still inline in `main.rs`: the huge
  completed-action-outcome handling block (`if !completed_actions.is_empty()
  { ... }` following `tick_basic_actions_with_attack_policy`) - this one is
  too large to move verbatim into one file (would blow the 2,000 line cap)
  and needs splitting by completed-action-kind family across several
  files, not just relocation. First family slice done: the Warp-area
  (`src/area/25/warped.c`) `ItemDriverOutcome` family (17 variants -
  teleport/bonus-level/key-spawn/key-door/trial-door) is extracted into
  `tick_item_use_warp::dispatch_warp_outcome`
  (`crates/ugaris-server/src/tick_item_use_warp.rs`, 389 lines). Second
  family slice done: the chest family (16 variants - `ChestTreasure`/
  `RandomChest`/`RatChest`/`InfiniteChest*`/`ForestChest*`/`PickChest*`/
  `ChestSpawn*`) is extracted into
  `tick_item_use_chests::dispatch_chest_outcome`
  (`crates/ugaris-server/src/tick_item_use_chests.rs`, 353 lines;
  `main.rs` down to ~4.7K). Unlike Warp's contiguous span, this family's
  variants were scattered across 5 spots in the match; each spot's code
  was either replaced with one combined or-pattern call arm (the first
  spot) or deleted (the other 4 spots), since match-arm order doesn't matter.
  Third family slice done: the dungeon family (7 contiguous variants -
  `DungeonTeleport`/`DungeonFake`/`DungeonKey`/`DungeonKeyCursorOccupied`/
  `DungeonDoorMissingKeys`/`DungeonDoorTooManyDefenders`/
  `DungeonDoorSolved`) is extracted into
  `tick_item_use_dungeon::dispatch_dungeon_outcome`
  (`crates/ugaris-server/src/tick_item_use_dungeon.rs`, 127 lines;
  `main.rs` down to ~4.7K/4,683 lines).
  Fourth family slice done: the ice + palace/Islena-door family (12
  contiguous variants - `IceItemSpawn`/`IceItemSpawnCursorOccupied`/
  `WarmFireCursorOccupied`/`IceItemSpawnBug`/`WarmFire`/`BackToFire`/
  `MeltingKeyTick`/`PalaceDoorKeyRequired`/`IslenaDoorBusy`/
  `IslenaDoorRespawning`/`IslenaDoorResting`/`PalaceDoorTick`) is
  extracted into `tick_item_use_ice::dispatch_ice_outcome`
  (`crates/ugaris-server/src/tick_item_use_ice.rs`, 149 lines; `main.rs`
  down to 4,610).
  Fifth family slice done: the Teufel family (16 contiguous variants -
  `TeufelArena`/`TeufelArenaExit`/`TeufelArenaNeedsSuit`/
  `TeufelArenaLevelTooHigh`/`TeufelArenaEquipmentEnhanced`/
  `TeufelArenaEquipmentBound`/`TeufelArenaBusy`/
  `TeufelArenaExitLowHealth`/`TeufelDoor`/`TeufelDoorNoHumans`/
  `TeufelDoorNoBeggars`/`TeufelDoorOnlyNobles`/`TeufelDoorBusy`/
  `TeufelDoorBug`/`TeufelRatNestSpawn`/`TeufelRatNestDestroyed`/
  `TeufelRatNestGuarded`) is extracted into
  `tick_item_use_teufel::dispatch_teufel_outcome`
  (`crates/ugaris-server/src/tick_item_use_teufel.rs`, 141 lines;
  `main.rs` down to 4,569).
  Sixth family slice done: the skeleton-raise family (4 contiguous
  variants - `SkelRaiseDust`/`SkelRaiseTouch`/`SkelRaiseRaise`/
  `SkelRaiseTimer`) is extracted into
  `tick_item_use_skelraise::dispatch_skelraise_outcome`
  (`crates/ugaris-server/src/tick_item_use_skelraise.rs`, 68 lines;
  `main.rs` down to 4,556).
  Seventh family slice done: the Edemon/Fdemon boss-machinery family
  (19 variants - `EdemonSwitchStuck`/`EdemonDoorLocked`/
  `EdemonDoorLifeless`/`EdemonBlockBlocked`/`EdemonBlockMove`/
  `EdemonTubePulse`/`FdemonLoaderBlocked`/`FdemonCannonLifeless`/
  `EdemonLoaderBlocked`/`FdemonFarmHarvest`/`FdemonFarmCursorOccupied`/
  `FdemonFarmNotReady`/`FdemonFarmBug`/`FdemonBloodBlocked`/
  `FdemonBloodDestroyedFlask`/`FdemonBloodFilled`/`FdemonLavaBlocked`/
  `FdemonLavaActivated`/`EdemonDoorToggle`) is extracted into
  `tick_item_use_edemon_fdemon::dispatch_edemon_fdemon_outcome`
  (`crates/ugaris-server/src/tick_item_use_edemon_fdemon.rs`, 225
  lines; `main.rs` down to 4,359). `EdemonDoorToggle` needed care: it
  originally appeared twice in the match (once with a `key_name:
  Some(..)` field guard producing feedback text, once bare inside the
  large no-op catch-all further down for the `key_name: None` case) -
  both arms are now inside the extracted dispatcher in the same
  relative order, and both original main.rs occurrences were removed.
  Eighth family slice done: the transport-point family (3 contiguous
  variants - `TransportOpen`/`TransportInvalid`/`TransportTravel`) is
  extracted into `tick_item_use_transport::dispatch_transport_outcome`
  (`crates/ugaris-server/src/tick_item_use_transport.rs`, 128 lines;
  `main.rs` down to 4,300).
  Ninth family slice done: the clan-spawn/LQ/arena family (13 contiguous
  variants - `ClanSpawnExit`/`ClanSpawnExitBusy`/`ClanSpawnLevelTooHigh`/
  `ClanSpawnContested`/`ClanSpawnCountdown`/`ClanSpawnAward`/
  `ClanSpawnTimer`/`LqTicker`/`LqEntranceClosed`/`LqEntranceLevelBlocked`/
  `LqEntranceUndefined`/`LqEntrancePenalty`/`ArenaToplist`) is extracted
  into `tick_item_use_clan_lq_arena::dispatch_clan_lq_arena_outcome`
  (`crates/ugaris-server/src/tick_item_use_clan_lq_arena.rs`, 203 lines;
  `main.rs` down to 4,220).
  Tenth family slice done: the shrine family (8 contiguous variants -
  `ZombieShrine`/`ZombieShrineNeedsOffering`/`RandomShrineNeedsKey`/
  `RandomShrineAlreadyUsed`/`RandomShrineBug`/`RandomShrineUse`/
  `SpecialShrine`/`DemonShrine`) is extracted into
  `tick_item_use_shrines::dispatch_shrine_outcome`
  (`crates/ugaris-server/src/tick_item_use_shrines.rs`, 380 lines;
  `main.rs` down to 3,889). Several nested `RandomShrineUse` sub-arms used
  `continue` to skip to the next queued action in the enclosing
  `for completion in &completed_actions` loop; since the function is now
  called once per outcome these became `return`, the equivalent
  "stop processing this outcome" behavior at function scope.
  Eleventh family slice done: the two-city burndown-barrel family (5
  contiguous variants - `BurndownTooHot`/`BurndownAlreadyBurned`/
  `BurndownTouch`/`BurndownIgnite`/`BurndownTimerTick`) is extracted into
  `tick_item_use_burndown::dispatch_burndown_outcome`
  (`crates/ugaris-server/src/tick_item_use_burndown.rs`, 52 lines;
  `main.rs` down to 3,882).
  Twelfth family slice done: the xmas + swamp-spawn family (4 contiguous
  variants - `XmasMaker`/`SwampSpawn`/`SwampSpawnPulse`/`XmasTree`) is
  extracted into `tick_item_use_xmas_swamp::dispatch_xmas_swamp_outcome`
  (`crates/ugaris-server/src/tick_item_use_xmas_swamp.rs`, 108 lines;
  `main.rs` down to 3,834).
  Thirteenth family slice done: the Caligar family (14 variants, scattered
  across 4 spots - `CaligarWeightBlocked`/`DoorLocked`/`DoorBusy`/`Move`/
  `Door`/`Timer`/`GunProjectile`, `CaligarKeyAssemble` (both `final_key`
  guards)/`KeyNeedsCursor`/`KeyDoesNotFit`, `CaligarSkellyDoor`/
  `SkellyDoorLocked`/`SkellyDoorBusy`, `CaligarTraining`) is extracted into
  `tick_item_use_caligar::dispatch_caligar_outcome`
  (`crates/ugaris-server/src/tick_item_use_caligar.rs`, 179 lines;
  `main.rs` down to 3,743).
  Fourteenth family slice done: the key-assembly family (51 variants
  scattered across 6 spots - staffer/saltmine `StafferBookText`/
  `StafferAnimationBook`/`StafferMineExhausted`/`StafferBlockBlocked`/
  `StafferSpecDoorLocked`/`StafferMineDig`/`StafferMineTimer`/
  `StafferBlockMove`/`StafferBlockTimer`/`StafferSpecDoorToggle`/
  `SaltmineDoorBlocked`/`SaltmineLadderUse`/`SaltmineSaltbagUse`,
  `BoneHint` + `BoneHolder*` (rune stand), `PalaceKey*`/`EnchantCursorItem`/
  `AntiEnchantCursorItem`/`EnchantNeedsCursor`, `ShrikeAmulet*`,
  `MineGateway*`/`MineKeyDoor*`, `Arkhata*`, `LizardFlower*`) is extracted
  into `tick_item_use_keyassembly::dispatch_keyassembly_outcome`
  (`crates/ugaris-server/src/tick_item_use_keyassembly.rs`, 536 lines;
  `main.rs` down to 3,451). `SaltmineSaltbagUse`'s original `continue`
  (valid inside the enclosing `for completion in &completed_actions` loop)
  became `return`, same precedent as the shrines slice.
  Fifteenth family slice done: the labyrinth family (18 contiguous
  variants - `BranningtonUnderwaterBerry`/`Lab3YellowBerry`/
  `Lab3WhiteBerry`/`Lab3WhiteBerryLightTick`/`Lab3BrownBerry`/
  `Lab2WaterWell`/`Lab2WaterAltar`/`Lab2WaterDrink`/
  `Lab2WaterCursorOccupied`/`Lab2StepActionClear`/
  `Lab2StepActionDaemonCheck`/`Lab2StepActionDaemonWarning`/
  `Lab2GraveClueBook`/`Lab2GraveClose`/`Lab2GraveCheckOpen`/
  `Lab2GraveOpen`/`LabEntranceSolvedAll`/`LabEntranceTooLow`/
  `LabExitWrongOwner`) is extracted into
  `tick_item_use_lab::dispatch_lab_outcome`
  (`crates/ugaris-server/src/tick_item_use_lab.rs`, 230 lines; `main.rs`
  down to 3,345).
  Sixteenth family slice done: the mine-wall digging family (5 contiguous
  variants - `MineWallInitialized`/`MineWallDig`/`MineWallCursorOccupied`/
  `MineWallExhausted`/`MineWallCollapse`) is extracted into
  `tick_item_use_minewall::dispatch_minewall_outcome`
  (`crates/ugaris-server/src/tick_item_use_minewall.rs`, 68 lines;
  `main.rs` down to 3,317).
  Seventeenth family slice done: the forest-spade/junkpile/pick-door
  digging-and-lockpicking family (8 contiguous variants -
  `ForestSpadeFind`/`ForestSpadeCollapse`/`ForestSpadeNothing`/
  `ForestSpadeCursorOccupied`/`JunkpileSearch`/`JunkpileCursorOccupied`/
  `PickDoorToggle`/`PickDoorLocked`) is extracted into
  `tick_item_use_dig_pick::dispatch_dig_pick_outcome`
  (`crates/ugaris-server/src/tick_item_use_dig_pick.rs`, 186 lines;
  `main.rs` down to 3,245).
  Eighteenth family slice done: the special-consumables/reading-material
  family (12 contiguous variants - `LollipopLicked`/`LollipopMemories`/
  `ChristmasPopInspected`/`SpecialPotionDrunk`/`SpecialPotionAntidote`/
  `SpecialPotionInfravision`/`SpecialPotionSecurity`/
  `SpecialPotionProfessionReset`/`SpecialPotionBug`/`BookText`/
  `BookcaseText`/`BookcaseLocked`) is extracted into
  `tick_item_use_books_potions::dispatch_books_potions_outcome`
  (`crates/ugaris-server/src/tick_item_use_books_potions.rs`, 199 lines;
  `main.rs` down to 3,072).
  Nineteenth family slice done: the keyring/assemble/gathering/alchemy-
  flask family (22 contiguous variants - `KeyringShow`/`Extinguish`/
  `KeyedDoorToggle`/`KeyringAddCursorItem`/`AssembleItem`/
  `AssembleNeedsCursor`/`AssembleDoesNotFit`/`AssembleUnknownItem`/
  `ParkShrine`/`ParkShrineBug`/`PickBerry`/`PickBerryCursorOccupied`/
  `PickAlchemyFlower`/`PickAlchemyFlowerCursorOccupied`/
  `FlaskIngredientAdded`/`FlaskWrongCursor`/`FlaskFull`/
  `FlaskFinishedNoMoreIngredients`/`FlaskEmptyShaken`/
  `FlaskIngredientBug`/`FlaskMixed`/`FlaskRuined`) is extracted into
  `tick_item_use_crafting::dispatch_crafting_outcome`
  (`crates/ugaris-server/src/tick_item_use_crafting.rs`, 368 lines;
  `main.rs` down to 2,848). Final slice done: the remaining scaffolding
  around the giant `match outcome { ... }` (auto-keyring pickup, the
  item-use dispatch loop that calls every per-family `tick_item_use_*`
  dispatcher plus the small no-op catch-all arms, item-use feedback/
  container-refresh flush, the post-completion per-session map/inventory/
  effects refresh, and the queued sound-specials drain) - the whole
  `if !completed_actions.is_empty() { ... }` block following
  `tick_basic_actions_with_attack_policy` - is extracted verbatim into
  `tick_item_use_completion::process_completed_action_outcomes`
  (`crates/ugaris-server/src/tick_item_use_completion.rs`, 1,312 lines;
  `main.rs` down to 1,586, under the 2,000 hard cap for the first time).
  This closes the task: every tick-loop phase (world stepping, NPC
  passes, sync, queued-client-actions, completed-action outcomes) now
  lives in its own module and `main.rs` only orchestrates the calls in
  order.
- [x] **Split `tests/commands_admin/character.rs` (~8K)** by command
  keyword using `tools/rust_split/splitter.py` with a spec like the ones
  described in the ledger; keep shared helpers in the tests `mod.rs`.
  *(done - details in PORTING_LEDGER.md)*
- [x] **Area-text color markers** - `WorldAreaText.message: String` drops
  legacy `COL_*` byte markers from every NPC line (documented deviation in
  several `world/npc/**` module docs). Carry bytes end-to-end and restore
  the C markers in the QA tables that had them. *(done - all 13
  originally-listed deviation sites restored, `area32/military.rs` closed
  last; details in PORTING_LEDGER.md)*
- [x] **Retire legacy blob writes** - after a few clean iterations with
  `player_state_json` (migration 0020): stop populating
  `ppd_blob`/`subscriber_blob` in the three `snapshots.rs` builders, add a
  backfill migration converting remaining blob-only rows through the
  legacy decoders, then mark the decoders `#[deprecated]`. Keep the raw
  `PlayerRuntime::ppd_blob` field (it preserves unknown legacy blocks
  inside the JSON document). *(done - backfill startup routine closes the
  task; details in PORTING_LEDGER.md)*
- [x] **`military.rs` (3.2K) split** - `world/npc/area32/military.rs`
  holds two NPCs plus shared mission logic; split into
  `military_master.rs`, `military_advisor.rs`, and `missions.rs`.
  *(done - details in PORTING_LEDGER.md)*

---

## P1 - Core Framework

Systems every later port depends on. Order within the section is a
suggestion; dependencies are noted.

- [x] **`update_char` stat recomputation** - the big one. C *(done - details in PORTING_LEDGER.md)*
- [x] **Equipment slot rules on swap (`CL_SWAP` into worn slots)** - C *(done - details in PORTING_LEDGER.md)*
- [x] **Experience/level-up side effects** - C `give_exp` -> *(done - details in PORTING_LEDGER.md)*
- [x] **Ground item decay** - dropped items never disappear (bodies do). *(done - details in PORTING_LEDGER.md)*
- [x] **`SV_SETVAL`/resource streaming on change** - C pushes value/exp/ *(done - details in PORTING_LEDGER.md)*
- [x] **Serial validation everywhere** - C guards every queued action with *(done - details in PORTING_LEDGER.md)*
- [x] **Logout/exit flow** - C `cl_exit`/lostcon: linger timer *(done - details in PORTING_LEDGER.md)*
- [x] **PostgreSQL login hardening** - wrong password must send the legacy *(done - details in PORTING_LEDGER.md)*
- [x] **Merchant store DB persistence** - C `database_merchant.c` *(done - details in PORTING_LEDGER.md)*
- [x] **Special stores** - C `add_special_store`/`create_special_item` *(done - details in PORTING_LEDGER.md)*
- [x] **Client command audit completion** - handle the remaining parsed *(done - details in PORTING_LEDGER.md)*
---

## P2 - NPC & Dialogue Framework

Unlocks every quest NPC. Do these before any P4 area work.

- [x] **Generic NPC text analysis (`analyse_text_driver`)** - C *(done - details in PORTING_LEDGER.md)*
- [x] **Driver memory (`mem_*`)** - C `src/system/mem.c`: *(done - details in PORTING_LEDGER.md)*
- [x] **`quiet_say`/`say`/`emote` NPC speech helpers in core** - several *(done - details in PORTING_LEDGER.md)*
- [x] **Idle NPC chatter** - merchant/citizen random murmur tables *(done - details in PORTING_LEDGER.md)*
- [x] **`CDR_BANK` banker NPC** - C `src/module/bank.c`: deposit/withdraw *(done - details in PORTING_LEDGER.md)*
- [x] **`CDR_TRADER` player-to-player trade NPC** (`src/module/base.c` *(done - details in PORTING_LEDGER.md)*
- [x] **Aclerk / auction NPC** - C `merchant.c::aclerk_driver` + *(done - details in PORTING_LEDGER.md)*
- [x] **Gatekeeper NPC (`src/system/gatekeeper.c`)** - lab entrance *(done - details in PORTING_LEDGER.md)*
---

## P3 - World Systems

- [x] **Questlog initialization & quest state machine** *(done - details in PORTING_LEDGER.md)*
- [x] **Achievements (`src/module/achievements/achievement.c`)** - runtime *(done - details in PORTING_LEDGER.md)*
- [x] **Clan system (`src/system/clan.c` + DB)** - membership lives in DB; *(done - details in PORTING_LEDGER.md)*
- [x] **Military ranks (`src/module/military.c`)** - military points exist *(done - details in PORTING_LEDGER.md)*
- [x] **Arena rankings (`src/system/arena.c`)** - toplist formatter is *(done - details in PORTING_LEDGER.md)*
- [x] **Weather driver (`src/module/weather/weather.c`)** - server-side *(done - details in PORTING_LEDGER.md)*
- [x] **Events (`src/module/events/**`)** - recurring boosted-rate events *(done - details in PORTING_LEDGER.md)*
- [x] **Death-mode loot tables (`src/system/loot/loot.c`)** - JSON tables *(done - details in PORTING_LEDGER.md)*
- [x] **Remaining `/` and `#` text commands** - diff *(done - details in PORTING_LEDGER.md)*
- [x] **Cross-area transfer** - the big multi-server feature. Every *(done - details in PORTING_LEDGER.md)*
- [x] **Player-side fight-driver auto-combat (lostcon self-defense + *(done - details in PORTING_LEDGER.md)*
- [x] **Macro-detection engine (`macro_driver`, `src/module/base.c:802- *(done - details in PORTING_LEDGER.md)*
- [x] **`.pre` zone preprocessor parity** - `src/system/create.c` expands *(done - details in PORTING_LEDGER.md)*
- [~] **Sector skip optimization (`skipx_sector`)** - C skips unchanged
  sectors in the per-tick map scan. Port once per-tick diff CPU becomes a
  measured problem (profile first; likely fine for small player counts).
  REMAINING: added a real profiling baseline instead of guessing -
  `profile_map_diff_payloads_cost_at_realistic_player_counts`
  (`crates/ugaris-server/src/tests/map_sync.rs`, `#[ignore]`d, run with
  `cargo test --release -p ugaris-server profile_map_diff_payloads_cost --
  --ignored --nocapture`) measures `map_diff_payloads`'s unconditional
  per-tile recompute cost (the exact thing `skipx_sector` would let C
  skip) at 100 concurrent players and `view_distance=15` (a diamond of
  ~450 tiles per player, each running a full `char_see_char`/line-of-sight
  check) against a full `MAX_MAP`x`MAX_MAP` world. Result: ~27µs per
  player per tick, ~2.7ms total per tick for all 100 players combined -
  against a 24-tick/second (~41.6ms) tick budget that is ~6.5% at a player
  count far above any real Ugaris concurrent population; at a realistic
  handful of concurrent players the cost is well under 1% of one tick.
  This confirms the task's own deferral condition still holds with real
  data, not just assumption - genuinely not worth the large, cross-cutting
  `set_sector` call-site integration (dozens of area `.c` files, most
  still unported in P4) the actual optimization would require. Left `[~]`
  rather than `[x]` since the optimization itself remains unimplemented;
  re-run the profiling harness (or a real load test) if a future
  iteration's player count or `view_distance` assumptions change, and
  implement the real `DirtySectors`/`skip_x_sector` wiring (already
  ported in `crates/ugaris-core/src/sector.rs`, just not called from
  `map_sync.rs`) only if that shows a real problem.

---

## P4 - Area Content

Every area's `.c` file mixes item drivers (mostly ported - check the
ledger) and character drivers (mostly NOT ported). For each area task:
port the character drivers (dialogue via P2 framework, quest PPD, special
movement), then boot with that area's data and smoke it.

Ordered by player progression; the C file is the oracle.

- [x] **Area 1 - `src/area/1/gwendylon.c` (6,286 lines)** - the tutorial
  and main city NPCs: Gwendylon quest chain, Lydia tutorial give, skeleton
  quests, `tutorial_ppd` hints (player_driver.c has the tutorial hook -
  port together). This is the highest-value area work: new players see it
  first. *(done - details in PORTING_LEDGER.md)*
- [x] **Area 2 - `src/area/2/area2.c`** - remaining character drivers
  (zombie lord, priests). Item drivers done. *(done - details in
  PORTING_LEDGER.md)*
- [x] **Area 3 - `src/area/3/area3.c`** - palace story NPCs, lamp ghost
  quest flow (lamps themselves are ported).
  REMAINING: `astro1_driver` (ambient moon-telescope monologue, no
  dialogue/quest/item interaction) is ported (`world/npc/area3/astro1.rs`,
  `CDR_ASTRO1`), establishing the file's NPC scaffolding. `thomas_driver`+
  `sir_jones_driver` (the crypt entrance guard + the crypt quest chain
  itself, sharing `crypt_state`/`crypt_bonus`) are now also ported
  (`world/npc/area3/thomas.rs`+`sir_jones.rs`, `CDR_THOMAS`/
  `CDR_SIRJONES`), cross-referencing the already-ported `CDR_VAMPIRE`/
  `CDR_VAMPIRE2` death hooks that gate on `area3_crypt_state`; a new
  shared `AREA3_QA` table (`world/npc/area3/mod.rs`) carries only the 13
  canned-greeting/repeat/restart/aye/nay entries these two drivers need -
  the `list`/`money`/`shortcut to caligar`/`explain`/`engrave:`/~86-entry
  raise-lower-skill block remain for `kassim_driver`/`supermax_driver`.
  `astro2_driver` (quest 16, lost astronomer notes) is now also ported
  (`world/npc/area3/astro2.rs`, `CDR_ASTRO2`), sharing `AREA3_QA`.
  `seymour_driver` (quest 10-12, Aston entry NPC and army-enrollment
  chain) is now also ported (`world/npc/area3/seymour.rs`,
  `CDR_SEYMOUR`), sharing `AREA3_QA`; the C `set_army_rank(co, 1)`
  enrollment call is reproduced as a direct `military_points = 1` write
  (see the module's own doc comment for why that's equivalent).
  `kelly_driver` (the biggest chain: quests 13-15/54/60, park shrines,
  swamp-beast-head bounties, the Caligar-plaque hunt) is now also ported
  (`world/npc/area3/kelly.rs`, `CDR_KELLY`), sharing `AREA3_QA` plus a new
  `shortcut to caligar`(7) QA entry for `kelly_driver`'s own god-only
  fast-forward; new `kelly_found1-3`/`kelly_found_cnt` `PlayerRuntime`
  accessors were added alongside it.
  `carlos_driver` (two independent chains gated on `questlog_count(co,
  61)`: the ritual quest, `carlos2_state`/quest 61, and the repeatable
  dragon-staff quest, `carlos_state`/quest 20, whose
  `achievement_award(ACHIEVEMENT_DRAGONSBANE)` fires unconditionally on
  every turn-in) is now also ported (`world/npc/area3/carlos.rs`,
  `CDR_CARLOS`), sharing `AREA3_QA`; new `staffer_carlos2_state`
  `PlayerRuntime` accessor, `AchievementType::Dragonsbane` award wrapper,
  and 8 `IID_CARLOS_DOOR`/`IID_STAFF_DRAGON*`/`IID_MAX_*` item-id
  constants were added alongside it.
  `kassim_driver` (the jewelry engraver, `world/npc/area3/kassim.rs`,
  `CDR_KASSIM`) is now also ported: full state machine (greeting/
  inscription-text/item-wait/engrave), `engrave: <text>` command parsing,
  gold charge + `IF_ENGRAVED`/item-description writes, and idle
  mutterings; a new `CharacterDriverState::Engrave` variant (same
  "player, not NPC" precedent as `ClanFound`) carries the pending
  inscription text, plus new `kassim_seen_timer`/
  `kassim_item_wait_starttime` `PlayerRuntime` accessors and a new
  `explain`(9) `AREA3_QA` entry.
  `supermax_driver` (the past-maxes raiser: greeting sequence, `list`/
  `money`/`raise <skill>`/`lower <skill>` commands) is now also ported
  (`world/npc/area3/supermax.rs`, `CDR_SUPERMAX`), sharing `AREA3_QA`
  plus its own 82 new entries (`list`(5)/`money`(6) and the 80-row
  raise/lower block keyed on `CharacterValue as i32 + 100/200`); new
  `supermax_canraise`/`supermax_cost` helpers added to
  `item_driver::scrolls` alongside the existing `skillmax`/`raise_cost`,
  and new `supermax_state`/`supermax_gold` `PlayerRuntime` accessors
  (C's global `misc_ppd`, not the area-3-specific `area3_ppd`).
  `lampghost_driver`/`_respawn`/`_dead` (the palace-light puzzle janitor:
  self-defense/aggressive-sighting cascade plus a nearest-lit-lamp
  claim/walk/extinguish job loop) is now also ported
  (`world/npc/area3/lampghost.rs`, `CDR_LAMPGHOST`); C's `lamp[MAXLAMP]`
  registry's `cn`/`cost` claim fields become a new `World::
  area3_lamp_claims: HashMap<ItemId, (CharacterId, i32)>` (registration
  membership was already `Item::driver_data[6]`), the respawn light gate
  lives in `ugaris-server`'s `spawns::respawn_npc_character`, and the
  claim-release death hook lives in `world_events::death_hooks::
  apply_lampghost_death_from_hurt_event`. This closes Area 3.
- [x] **Area 4 - `src/area/4/pents.c`** - pentagram quest NPCs + demon
  wave logic beyond the ported item boundary. Also wire the achievement
  calls this file's reward mechanic makes in C (`achievement_add_pents`,
  `achievement_award(FIVE_IN_A_ROW/HAPPY_GO_LUCKY/FAVORED_BY_FORTUNE/
  DEMON_LORDS_DEMISE)`) using the existing `award_*` helper pattern in
  `crates/ugaris-server/src/achievement.rs` (Achievements task, closed
  iteration 84).
  REMAINING: the whole per-player solve/reward pipeline is now ported and
  wired end-to-end - `activate_pentagram`/`deactivate_pentagram`/
  `check_for_quest_completion`/`complete_pentagram_quest`/
  `distribute_rewards_to_player`/`add_pentagram_to_player`/
  `update_player_pentagram_stats`/`check_for_color_combo`/
  `handle_lucky_pentagram`/`log_pentagram_info`/`check_for_record`/
  `calculate_required_pentagrams`/`update_power_levels` (`World::
  pentagram_quest: PentagramQuestState` + `crates/ugaris-core/src/
  pentagram.rs`'s pure per-player math over `PlayerRuntime::
  pentagram_debug`, drained every tick by `crates/ugaris-server/src/
  pents.rs::process_pentagram_activations`, wired into
  `tick_item_use_completion.rs` right after the item-use dispatch loop);
  all four reachable achievement call sites (`FIVE_IN_A_ROW`/
  `HAPPY_GO_LUCKY`/`FAVORED_BY_FORTUNE`/`achievement_add_pents`) are wired
  through `award_pentagram_*` helpers in `achievement.rs`. The
  `handle_demon_lord_door` access-time gate and the `IDR_PENT`/
  `IDR_PENTBOSSDOOR` item drivers were already ported before this.
  (1)+(2) now CLOSED together: demon spawning (`spawn_demons_at_pentagram`/
  `enhance_elite_demon`/`adjust_lesser_demon`/`enhance_demon_character`/
  slot bookkeeping in the pentagram item's `driver_data[6..]`) is ported
  in `World` (`world/pents.rs`) plus a `ugaris-server`-side `pents.rs`
  glue (`process_pentagram_demon_spawns`) that instantiates each planned
  `penterN` template (needs `ZoneLoader`); spawned demons get a new
  `CDR_PENTER` driver id whose own tick AI is the `CDR_SIMPLEBADDY`
  self-defense/idle-wander driver reused wholesale (widened
  `character.driver == CDR_SIMPLEBADDY` gates in `world/npc_fight.rs`/
  `world/npc_idle.rs`, same precedent as `CDR_DUNGEONFIGHTER`), so no
  separate `demon_character_driver` dispatch was needed. The
  `pent_demon_{low,mid,high}[_elite]` JSON loot tables already exist
  under `ugaris_data/loot/pents/` (rolled once per spawn via
  `loot_apply_to_npc`, matching C's `process_demon_messages`).
  `update_demon_profession` and `handle_demon_death`'s power-level-
  reduction/`DEMON_LORDS_DEMISE` achievement (wired through a new
  `award_pentagram_demon_lords_demise_achievement` helper) are also
  ported (`World::update_demon_profession`/`apply_penter_demon_death`,
  the latter hooked into `World::kill_character_followup` so it fires
  for every `CDR_PENTER` death regardless of whether a killer exists,
  matching C's `ch_died_driver` semantics - unlike the `LegacyHurtEvent`-
  based death-hook family, which only fires when there's a killer).
  (4) is now also CLOSED: `pentagram_record`/`pentagram_record_holder`
  restart-persistence (C `load_pentagram_record`/
  `save_pentagram_record_scheduled`, `database_pent_record.c`) is ported
  via a new `ugaris-db::PgPentagramRecordRepository` mirroring C's own
  `pentagram_record` table one-for-one (`migrations/
  0021_pentagram_record.sql`), loaded once at startup
  (`pents::load_pentagram_record_at_startup`) and re-saved on both of
  C's non-shutdown call sites: the periodic `add_scheduled_task(...,
  3600 * 4, ...)` cadence (checked inline in `main.rs`'s tick loop, not
  threaded through `tick_npc::run_all` - deliberately avoided touching
  its ~150-function-signature blast radius for a system-wide,
  non-NPC concern) and `/saveall`'s explicit call
  (`tick_client_actions.rs`). (3) is now also CLOSED: `pentagram_tester_
  driver` (`CDR_TESTER = 77`, a test-only QA bot never spawned by any C
  code path - confirmed not player-facing) is ported in
  `world/npc/area4/tester.rs`, registered as `tick_npc::area4::
  tester_driver_87`. This closes Area 4. The macro-daemon challenge-room
  `saved_pent_*` restore remains a documented no-op in `macro_daemon.rs`
  (unrelated to any of the four numbered items above; not a blocker for
  any other P4 area).
- [x] **Area 6 - `src/area/6/edemon.c`** - Earth Demon boss driver
  (`CDR_EDEMON*` characters); machinery items are ported.
  REMAINING: confirmed C's own `ch_driver`/`ch_died_driver`/
  `ch_respawn_driver` in `edemon.c` are all empty (`switch { default: return
  0; }`) - no `CDR_EDEMON` character driver exists in C; `edemon2s`/
  `edemon6s` (`ugaris_data/zones/6/edemon.chr`) use plain `driver=7`
  (`CDR_SIMPLEBADDY`, already ported) plus `flag=CF_EDEMON`, so this task is
  really "port every `CF_EDEMON` branch in shared combat code", not a new
  driver. All 8 `IDR_EDEMON*` item drivers were already ported (confirmed,
  extensively tested per the ledger). A previous slice closed two
  `CF_EDEMON` combat mechanics: `do_walk`'s earthmud-spell movement
  slowdown (`do.c:93-99`) and `check_strike_near`'s earth-demon damage
  reduction against ball/flash strikes (`effect.c:864`). This iteration
  closed the last gap: `act_attack` (`act.c:747-748`) was resolving melee
  to-hit from the raw `V_ATTACK`/`V_PARRY` stat instead of calling
  `get_attack_skill`/`get_parry_skill` - a pre-existing, cross-cutting P1
  bug affecting every character's melee to-hit chance, not just earth
  demons (their `CF_EDEMON` fallback branch, `fight_skill * 3.5`, was
  simply unreachable dead code before this fix). `do_action::act_attack`
  now takes an `items: &HashMap<ItemId, Item>` parameter and calls the
  already-ported `attack::attack_skill`/`parry_skill` (fight-skill lookup
  via the already-ported `simple_baddy_fight_skill`), matching C exactly,
  including the earth-demon/magic-shield/spellcaster fallback branches.
  `rage` stays hardcoded to `0` (undocumented pre-existing gap: `Character`
  has no `rage` field yet, same as every other `attack_skill`/`parry_skill`
  caller - see `values.rs`'s `show_values_lines` doc comment). This closes
  every `CF_EDEMON` call site and Area 6 for real.
- [x] **Area 8 - `src/area/8/fdemon.c`** - Fire Demon boss + farm NPCs;
  cannon/loader items are ported. *(done - details in PORTING_LEDGER.md)*
  Ported `CDR_FDEMON_DEMON` (the roaming Fire Demon/Fire Golem
  hunt AI - `world::fdemon`'s new waypoint-graph `find_waypoints`/
  `hunt_driver`/`may_hunt_there` port plus `world::npc::area8::
  fdemon_demon`'s gohome-hysteresis/wander driver, reusing
  `CharacterDriverState::SimpleBaddy` wholesale for combat/messages same as
  `CDR_PENTER`/`CDR_DUNGEONFIGHTER`) and its `fdemon_demon_dead` death hook
  (`farmy_ppd.boss_stage` 16/17->18). The `sprite==190` "Fire Golem" boss
  variant is spawned as plain `CDR_SIMPLEBADDY` (100% observably identical
  to C's own unconditional tail-call, see the module doc comment) so the
  death hook matches on `area_id==8 && sprite==190` instead of driver id.
  Ported `CDR_FDEMON_BOSS` (the Commander's 33-stage mission-giver dialogue
  chain, `world::npc::area8::fdemon_boss`): the full `boss_stage` state
  machine (`fdemon_boss_greet_player`, a direct-sighting-scan replacement
  for C's `NT_CHAR` loop, same precedent as `fdemon_demon`), `platoon_exp`'s
  always-live player-exp/rank-promotion half (`fdemon_platoon_exp` - the
  soldier-exp loop is a documented gap, unreachable without
  `CDR_FDEMON_ARMY`), the open-ended stage-28+ Defense-Station scouting
  phase, and the shared `NT_TEXT` "repeat" stage-reset ladder
  (`fdemon_boss_repeat_reset`, wired through the real `driver_messages`
  queue since player speech is already reliably delivered there). The
  matching loader-side half - `IDR_FDEMONLOADER`'s defense-station
  boss-mission bookkeeping (`fdemon_loader_station_report`, a new
  `station_id` field on `FdemonLoaderChanged`) - is also now wired end to
  end, so a player can solo the entire mission chain (crystal-insert ->
  stage advance -> platoon exp -> next mission) without any soldiers.
  Still unported: `CDR_FDEMON_ARMY` (the recruitable-soldier "take"/"drop"/
  formation-following/emote system, `farmy_data`/`farmy_ppd.soldier[]`) -
  the boss's own "take"/"drop" `NT_TEXT` tail and `platoon_exp`'s soldier-
  exp loop are documented gaps pending that driver. First two slices of
  that driver ported, both pure and spawning-independent:
  `world::npc::area8::fdemon_army` carries `struct profile`/
  `assign_profile`/`take_soldiers`'s type-and-profile eligibility logic
  (`plan_soldier_recruitment`) plus the `struct soldier` PPD field
  accessors (`PlayerRuntime::farmy_soldier_type`/`_rank`/`_base`/
  `_profile`/`_exp`/`_cn`/`_serial` in `player/areas_misc.rs`), and now
  also `update_soldier`'s stat-scaling (`scale_soldier_skill`/
  `scale_soldier_values`), skill-tiered equipment selection
  (`soldier_equipment_items`), and exp/level recompute
  (`calc_exp` in `world/exp.rs`, C `skill.c:174-196`, plus
  `finalize_soldier_exp_and_level` composing it with the already-ported
  `exp2level`). Fourth slice done: `take_soldiers`/`drop_soldiers` spawning
  is now fully wired end to end (`ugaris-server`'s new `area8_army.rs`,
  needing `ZoneLoader`/`ServerRuntime::allocate_character_id`) - saying
  "take"/"drop" to the Commander (boss `NT_TEXT` handling extended in
  `area8.rs`, with the `boss_stage 1..=30` gate and "cannot take soldiers"
  message) actually spawns/despawns real `army1s`/`army2s` soldier
  characters, fully equipped and stat-scaled via the previously-ported
  pure helpers. The new `CDR_FDEMON_ARMY` driver id + `CharacterDriverState
  ::FdemonArmy(FarmyData)` variant plus `World::army_follow_driver`/
  `fdemon_army_tick` (`world/npc/area8/fdemon_army.rs`) port the
  `MIS_FOLLOW`/leader-lost-disintegration slice of the per-soldier tick, so
  recruited soldiers now actually follow their leader around (wired into
  `tick_npc::area8::fdemon_boss_driver_89`). Fifth slice done: the
  "follow"/"back"/"retreat"/"front"/"behind" `NT_TEXT` command reception
  (`World::fdemon_army_process_text_messages`) plus `army_back_driver`/
  `army_front_driver` are now ported, so `MIS_BACK`/`MIS_RETREAT`/
  `MIS_FRONT` are fully live missions a leader can command a soldier into
  (only the leader's own speech, gated by C's `find_platoon` platoon-
  membership check, can issue a command). Sixth slice done:
  `army_behind_driver` (`fdemon.c:688-705`) is now also ported - it looks
  up whatever character the leader is facing (a direct map-tile lookup),
  computes the tile directly behind that character (opposite of its own
  facing direction, reusing the `opposite_direction` helper
  `army_back_driver` already established), walks there via
  `World::setup_walk_toward` (already an exact `move_driver` equivalent -
  `pathfinder` + `walk_or_use_driver`) if not already positioned, and
  attacks with `do_action::do_attack` once in position - so `MIS_BEHIND`
  is now a fully live mission. Combat/heal/bless self-defense
  (`fight_driver_update`/`do_heal`/`do_bless`/`fight_driver_attack_
  visible`) is now also ported (`world::npc::area8::fdemon_army_combat`):
  a direct-scan replacement for `fdemon_army`'s own `NT_CHAR` handling
  (leader tracking + bless/heal target selection, same "replace message-
  driven sighting with a scan" precedent as `fdemon_demon`/`fdemon_boss`)
  plus real message-driven `NT_GOTHIT`/`NT_SEEHIT` aggro tracking (the
  previous `fdemon_army_process_text_messages` silently discarded these -
  renamed to `fdemon_army_process_messages` and broadened) and a direct
  call to the already-ported multi-enemy `fight_driver_attack_visible_
  and_follow`. Also fixed a real gap this uncovered: soldier spawning
  never seeded the driver-independent `DRD_FIGHTDRIVER` slot C's own
  `fight_driver_set_dist(cn, 0, 20, 0)` NT_CREATE handler sets, so no
  self-defense enemy could ever be recorded - added to `area8_army.rs`'s
  `spawn_army_soldier`. Confirmed the previous note's `it_driver`
  drdata[6] take/drop-soldier item triggers do not exist in the C source
  (only `fdemon_boss`'s NT_TEXT-based take/drop, already ported) - removed
  from this list. Seventh slice done: the `do_emote`/`got_emote`
  personality/chat engine (`fdemon.c:323-1325`) is now also ported
  (`world/npc/area8/fdemon_army_emote.rs`'s new `SoldierEmote` struct +
  `World::fdemon_army_do_emote`/`fdemon_army_got_emote`/
  `fdemon_army_emote_stats_line`), including a real C quirk reproduced
  digit-for-digit (`do_emote`'s `bestscore`/`bestco` are shared, never
  reset, across its four lonely/boredom/fear/praise blocks). The `NT_TEXT`
  handler's `res >= 20` emote-reaction dispatch and case `7`'s emote-stats
  debug command are wired in `fdemon_army_combat.rs`; a new needs-name-
  gated matcher (`character_driver::analyse_text_qa_needs_name` +
  `FDEMON_ARMY_EMOTE_QA`, 40 rows) reproduces C's `qa[].needs_name` gate
  without adding a field to the other ~300 `TextQaEntry` call sites
  (see that function's own doc comment). `NT_DEAD`-triggered praise
  (killing something) and the HP<50%-triggered fear/boredom shift are also
  wired into `fdemon_army_process_messages`/`fdemon_army_tick`. Freshly
  spawned soldiers get their four base tendencies from `assign_profile`;
  cross-recruit-cycle persistence of the "current"/relationship fields
  (C's `ppd->soldier[n].emote`, byte layout already reserved in
  `player/misc.rs`) remains a documented gap. `fdemon_army.rs`'s four
  `army_*_driver` movement functions were also split into a new
  `fdemon_army_movement.rs` to stay under the ~800-line guideline after
  this addition. `platoon_exp`'s soldier-exp/promotion loop (`fdemon.c:
  729-751`) is now also ported: `World::fdemon_platoon_exp` credits each
  live recruited soldier's PPD-tracked exp (folding back its accumulated-
  but-unspent exp, capping promotion one rank below the player's own) via
  new `SoldierPlatoonFacts`/`SoldierPlatoonExpUpdate` types riding on
  `FdemonBossPlayerFacts`/`FdemonBossStageUpdate::soldier_updates`; the
  `ZoneLoader`-needing re-equip half (`update_soldier`'s stat rescale +
  item swap) is `area8_army::reequip_soldier_for_promotion`, applied by
  `area8.rs` for every promoted slot. Eighth (final) slice done: the
  emote-state cross-recruit persistence gap is now closed -
  `PlayerRuntime::farmy_soldier_emote`/`set_farmy_soldier_emote` expose the
  previously-reserved byte range, wired into `area8_army.rs`'s
  `take_soldiers`/`drop_soldiers`/`spawn_army_soldier` so a soldier's
  personality/relationship state survives a drop/re-recruit cycle. Area 8
  is now fully ported.
- [x] **Area 10 - `src/area/10/ice.c`** - ice NPCs, ice demon curse
  integration (curse spell side is ported). *(done - C's own `ch_driver`
  is empty, all NPCs are plain `CDR_SIMPLEBADDY`; found and fixed a real
  `CF_IDEMON` freeze-modifier bug in `npc_fight.rs` combat AI; details in
  PORTING_LEDGER.md)*
- [x] **Area 11 - `src/area/11/palace.c`** - palace guards, Islena fight
  driver (door/bomb/cap items ported). *(done - `palace_islena` and
  `palace_guard` both ported; details in PORTING_LEDGER.md)*
- [x] **Area 12 - `src/area/12/mine.c`** - keyholder golems, miners. Also
  wire `achievement_add_silver_mined`/`_gold_mined` from the
  `handle_mining_result` reward cascade using the existing `award_*`
  helper pattern in `crates/ugaris-server/src/achievement.rs`
  (Achievements task, closed iteration 84).
  REMAINING: the `handle_mining_result` weighted-event roll and four of
  its six branches are now ported - silver/gold finds (`give_mine_item`'s
  cursor/inventory-pile placement, `check_military_silver` mission
  tracking, and the `achievement_add_silver_mined`/`_gold_mined` wiring
  this checkbox specifically called out), golem spawning
  (`spawn_normal_golem`/`spawn_rare_golem`, reusing plain `CDR_SIMPLEBADDY`
  templates same as other area mob spawns), and the cave-in endurance
  mechanic (`handle_cave_in`, including the miner-avoid-chance and
  athlete-reduction quirks). New `crates/ugaris-core/src/world/mining.rs`
  (pure roll/amount/cave-in math) + `crates/ugaris-server/src/mine.rs`
  (`ZoneLoader`/achievement glue), wired from `tick_item_use_minewall.rs`
  when a wall's `MineWallDig` outcome carries `opened: true`. `handle_
  orb_find` (the 5-in-100,000 orb-of-skill reward) and `handle_artifact_
  find` (the 200-in-100,000 12-flavor relic table with exp/silver/
  military-point tiers) are now also ported (`mine.rs::apply_mine_orb_
  find`/`apply_mine_artifact_find`), reusing `World::give_char_item`/
  `grant_created_orb`'s `"empty_orb"` template convention and the
  already-ported `World::give_exp`/`give_military_pts`/`achievement::
  give_money` helpers; `dispatch_minewall_outcome`/`apply_mine_wall_
  reward` now take an `area_id` parameter for these two branches' `give_
  exp`/`give_military_pts` calls. `CDR_GOLEMKEYHOLDER`/`keyhold_fight_
  driver` (the locked-treasure-room boss golem `keyholder_door`/
  `IDR_MINEKEYDOOR` spawns) is now also ported (`world/npc/area12/
  golemkeyholder.rs`, byte-for-byte identical to the already-ported
  `gate_fight_driver` except for the self-destruct timeout and victim
  assignment, see the module doc comment); the door's own outcome was
  reshaped into a new `MineKeyDoorOpened` variant carrying the picked
  room coordinates out to `ugaris-server::mine::spawn_keyholder_golem`.
  This closes Area 12.
- [x] **Area 13 - `src/area/13/dungeon.c` + `dungeon_tab.c`** - dungeon
  master/fighter drivers, clan jewel raid protocol. *(already fully
  ported across earlier "Clan system"/P0.5/cross-area-transfer
  iterations; audited function-by-function against C in iteration 79 -
  no gaps found. Details in PORTING_LEDGER.md.)*
- [x] **Area 14 - `src/area/14/random.c`** - remaining shrine effects
  (indecisiveness/bribes/welding) + questlog resend after shrines.
  *(done - `shrine_welding` plus its `can_receive_mod`/`can_give_mod`
  helpers now ported via a new `World::apply_random_shrine_welding`/
  `World::recompute_item_requirements`; details in PORTING_LEDGER.md)*
- [x] **Area 15 - `src/area/15/swamp.c`** - Clara dialogue runtime (state
  helpers exist), military reward application. *(done - runtime NT_CHAR/
  NT_TEXT/NT_GIVE message-loop integration, military-point rewards, and
  the `CDR_SWAMPMONSTER`->`CDR_SIMPLEBADDY` AI-gate widening all ported;
  details in PORTING_LEDGER.md)*
- [x] **Area 16 - `src/area/16/forest.c`** - forest NPCs/robber quest.
  *(done - `imp_driver`/`william_driver`/`hermit_driver` (`CDR_FORESTIMP`/
  `CDR_FORESTWILLIAM`/`CDR_FORESTHERMIT`) ported as one file each under
  `world/npc/area16/`, sharing a new `FOREST_QA` table; `CDR_FORESTMONSTER`
  reuses `CDR_SIMPLEBADDY` AI (same precedent as `CDR_SWAMPMONSTER`) plus
  its `monster_dead` death hook (`imp_kills`/`hermit_state` counters +
  hardkill-weapon glow). `IDR_FORESTCHEST` was already fully ported.
  Details in PORTING_LEDGER.md.)*
- [~] **Area 17 - `src/area/17/two.c`** - Two-City thief/skeleton NPC
  drivers (`CDR_TWOSKELLY` has state scaffolding).
  REMAINING: `CDR_TWOSKELLY` ("Scarcewind", the raised governor's-ghost
  quest giver, quest 30) is now fully ported (`world/npc/area17/
  two_skelly.rs`), including its 30-second self-destruct timer and the
  new file-local `TWOCITY_QA` shared table (`world/npc/area17/mod.rs`,
  currently only the entries this NPC needs). `CDR_TWOALCHEMIST`
  ("Cervik", the spider-poison quest giver, quest 31) is now also ported
  (`world/npc/area17/alchemist.rs`), sharing `TWOCITY_QA`; its 1st/3rd/
  7th/10th-completion potion reward (`combo_potion3`/`security_potion`,
  level-gated) is finished server-side (`ugaris-server/area17.rs`) since
  it needs the quest-log completion count and `ZoneLoader`. Still
  unported: `CDR_TWOGUARD`/`CDR_TWOBARKEEPER`/`CDR_TWOSERVANT`/
  `CDR_TWOTHIEFGUARD`/`CDR_TWOTHIEFMASTER`/`CDR_TWOROBBER`'s death hook/
  `CDR_TWOSANWYN` - see `PORTING_LEDGER.md` for the full driver breakdown
  and suggested next-slice order.
- [ ] **Area 18 - `src/area/18/bones.c`** - rune quest completion
  (`exec_rune` rewards), bone NPCs.
- [ ] **Area 19 - `src/area/19/nomad.c`** - nomad camp NPCs/trading.
- [ ] **Area 20 - `src/area/20/lq.c`** - live-quest admin command table,
  LQ NPC dialogue (spawn/raise/equipment ported).
- [ ] **Area 22 - `src/area/22/lab*.c`** - remaining lab mechanics per
  lab; lab2 undead mostly ported; gatekeeper depends on P2.
- [ ] **Areas 23/24 - `src/area/23_24/strategy.c` (3,599 lines)** - the
  strategy minigame (mission ownership, worker spawning, resources).
  Item dispatch is stubbed as no-ops; this is a full subsystem - plan in
  ledger first.
- [ ] **Area 25 - `src/area/25/warped.c`** - warped NPC dialogue,
  `DRD_WARPFIGHTER` full fight driver.
- [ ] **Area 26 - `src/area/26/staffer.c`** - vault skull PPD/quest, Rouven
  smuggler dialogue.
- [ ] **Area 28 - `src/area/28/brannington_forest.c`** - forest NPCs.
- [ ] **Area 29 - `src/area/29/brannington.c`** - Brannington quest NPCs,
  `DRD_STAFFER_PPD` remaining fields.
- [ ] **Area 30 - `src/area/30/clanmaster.c`** - clan master NPC (needs P3
  clan system).
- [ ] **Area 31 - `src/area/31/warrmines.c`** - Warr mines NPCs.
- [ ] **Area 32 - `src/area/32/missions.c`** - governor mission NPCs
  (needs P3 military).
- [ ] **Area 33 - `src/area/33/tunnel.c`** - long tunnel events. Also wire
  `achievement_add_tunnel_level` using the existing `award_*` helper
  pattern in `crates/ugaris-server/src/achievement.rs` (Achievements
  task, closed iteration 84).
- [ ] **Area 34 - `src/area/34/teufel.c`** - rat/gambler NPCs, arena score
  rewards (rat nest items ported).
- [ ] **Area 36 - `src/area/36/caligar.c`** - Caligar quest NPCs, PPD
  quest state beyond skelly doors.
- [ ] **Area 37 - `src/area/37/arkhata.c` (4,764 lines)** - Arkhata clerk/
  quest NPC chain (pool/stopwatch/key items ported).
- [ ] **Area 38 - `src/area/38/shrike.c`** - Shrike NPCs (amulet assembly
  ported).
- [ ] **Common NPCs - `src/common/professor.c`, `src/common/npc_states.h`,
  `src/common/ice_shared.c` remainder** - shared NPC helpers referenced
  by multiple areas. Also wire `achievement_check_profession` from
  `learn_prof`/`improve_prof` using the existing `award_*` helper pattern
  in `crates/ugaris-server/src/achievement.rs` (Achievements task, closed
  iteration 84).

---

## Not Applicable / Deferred (do not port)

- `src/system/mem.c` memory-pool allocator - Rust ownership replaces it.
- `src/system/io.c` low-level socket pumping beyond the tick frame - tokio
  replaces it (frame envelope is ported).
- `src/system/chat/chat.c` cross-server chat transport - single-server
  setup for now; local channels are ported. Revisit with cross-area.
- Dynamic `.so` driver loading (`src/system/libload.c`) - replaced by the
  static registry.
- `src/module/anticheat/*` runtime heartbeats - scaffolded DB side exists;
  defer until multiplayer testing starts.
- `.pre` zone/map files (`ugaris_data/zones/*/*.pre`) - closed-source
  Windows map editor paint-palette sources (`PRESET:`/`CHANCE:`/
  `LINEWALL:`/`GROUND:` blocks); `create.c` never reads them (confirmed:
  `load_zones` only ever runs with masks `.itm`/`.chr`/`.map`), and no
  area's `.map`/`.itm`/`.chr` data references one. The Rust `ZoneLoader`
  already has full parity by loading the same 3 extensions the C server
  does. See the closed "`.pre` zone preprocessor parity" P3 task
  (iteration 219) for the full cross-reference.

---

## Progress Log

Keep entries to at most three lines: date, task, one-line result.
Anything longer belongs in `PORTING_LEDGER.md`; historical verbose
notes live in `PROGRESS_ARCHIVE.md`.

- 2026-07-08: Area 17 STARTED: ported `CDR_TWOSKELLY` (`world/npc/
  area17/two_skelly.rs`, quest 30) plus a new `raise_skeleton_from_
  template` rest-position fix it depends on. 2929 core + 1138 server
  tests pass, clean build/boot-smoke.
- 2026-07-08: Area 14 STARTED: ported `shrine_indecisiveness`/
  `shrine_bribes` plus the `sendquestlog` resend after every successful
  random-shrine use (all 10 ported kinds). Only `shrine_welding` remains.
  2872 core + 1136 server tests pass, clean build/boot-smoke.
- 2026-07-08: Area 13 CLOSED: function-by-function audit of `dungeon.c`/
  `dungeon_tab.c` against the Rust tree found it already fully ported by
  earlier "Clan system"/P0.5/cross-area-transfer iterations - no gap
  found. No code changes; checkbox marked `[x]`.
- 2026-07-08: Area 12 CLOSED: ported `CDR_GOLEMKEYHOLDER`/`keyhold_fight_
  driver` (`world/npc/area12/golemkeyholder.rs`, reused `gate_fight`'s
  shape) + `mine::spawn_keyholder_golem`. 2872 core + 1130 server tests
  pass, clean build/boot-smoke (area 12).
- 2026-07-08: Area 12 `handle_orb_find`/`handle_artifact_find` ported
  (`mine.rs::apply_mine_orb_find`/`apply_mine_artifact_find`), closing every
  gap but `CDR_GOLEMKEYHOLDER`. 2865 core + 1128 server tests pass, clean
  build, boot-smoke ok.

- 2026-07-08: Area 10 (`ice.c`) closed - C's `ch_driver` is empty (all
  NPCs plain `CDR_SIMPLEBADDY`); found/fixed a real `CF_IDEMON`
  freeze-modifier bug in `npc_fight.rs`. 2833 core + 1113 server tests pass.

- 2026-07-08: Area 8 `CDR_FDEMON_ARMY` emote-state cross-recruit persistence
  closed (`PlayerRuntime::farmy_soldier_emote`, wired into `area8_army.rs`).
  Area 8 fully ported. 2832 core + 1113 server tests pass, clean build.

- 2026-07-08: Area 8 `platoon_exp`'s soldier-exp/promotion loop ported
  (`World::fdemon_platoon_exp` soldier facts/updates, `area8_army::
  reequip_soldier_for_promotion` for the `ZoneLoader` re-equip half).
  2831 core + 1112 server tests pass, clean build/boot-smoke.

- 2026-07-08: Area 8 `CDR_FDEMON_ARMY` `do_emote`/`got_emote` personality/
  chat engine ported (new `fdemon_army_emote.rs`, `analyse_text_qa_needs_
  name`), incl. the `bestscore` shared-across-blocks C quirk. 2828 core +
  1111 server tests pass, clean build/boot-smoke.

- 2026-07-08: Area 8 `CDR_FDEMON_ARMY` combat: self-defense/heal/bless
  fallback ported (new `fdemon_army_combat.rs`), plus fixed a spawn-time
  gap (`DRD_FIGHTDRIVER` never seeded) that silently blocked it. 2812
  core + 1111 server tests pass, clean build/boot-smoke (area 1 + 8).

- 2026-07-07: Area 1 `brithildie_driver` (`CDR_BRITHILDIE`) ported:
  ambient lore NPC unlocking `QLOG_BRITHILDIE`, plus its
  `bigbadspider_dead` death hook (`CDR_BIGBADSPIDER`) completing the
  quest. 1091 tests pass, clean build/boot-smoke.
- 2026-07-07: P0.5 main() decomposition: extracted the tick loop's
  "world stepping" phase into `tick_world::world_step` (461 lines cut
  from `main.rs`, now 6,689). Client-command-loop/sync phases remain.
  1091 tests unchanged, clean build/boot-smoke.
- 2026-07-07: P0.5 main() decomposition: extracted the post-NPC-pass
  "sync" phase into `tick_sync::sync_phase` (`main.rs` down to 6,632).
  Only the client-command-loop phase remains inline. 1091 tests
  unchanged, clean build/boot-smoke.
- 2026-07-07: P0.5 main() decomposition: extracted the queued-client-
  action drain/dispatch/feedback-flush phase into
  `tick_client_actions::process_queued_client_actions` (`main.rs` down
  to 5,262). Only completed-action-outcome handling (~3.7K lines)
  remains inline. 1091 tests unchanged, clean build/boot-smoke.
- 2026-07-07: Area 1 `nook_driver` (`CDR_NOOK`) ported: the identity-
  crisis judge/knight/jester's greeting chain plus the stolen-cap
  side quest (`QLOG_NOOK`) and its idle mutterings. 2393 core + 1091
  server tests pass, clean build/boot-smoke.
- 2026-07-07: P0.5 main() decomposition: sliced the first completed-
  action-outcome family (Warp-area, 17 variants) into
  `tick_item_use_warp::dispatch_warp_outcome` (`main.rs` down to
  ~4.9K). 1091 server tests unchanged, clean build/boot-smoke.
- 2026-07-07: P0.5 main() decomposition: sliced the second completed-
  action-outcome family (chests, 16 non-contiguous variants) into
  `tick_item_use_chests::dispatch_chest_outcome` (`main.rs` down to
  ~4.7K). 1091 server tests unchanged, clean build/boot-smoke.
- 2026-07-07: P0.5 main() decomposition: sliced the third completed-
  action-outcome family (dungeon, 7 contiguous variants) into
  `tick_item_use_dungeon::dispatch_dungeon_outcome` (`main.rs` down to
  4,683). 1091 server tests unchanged, clean build/boot-smoke.
- 2026-07-07: P0.5 main() decomposition: sliced the fourth completed-
  action-outcome family (ice + palace/Islena doors, 12 contiguous
  variants) into `tick_item_use_ice::dispatch_ice_outcome` (`main.rs`
  down to 4,610). 1091 server tests unchanged, clean build/boot-smoke.
- 2026-07-07: Area 1 `lydia_driver` (`CDR_LYDIA`) ported: the mage's-
  daughter hangover-potion quest chain (`QLOG_LYDIA`), reward-potion
  grant deferred to `ugaris-server`. 2406 core + 1091 server tests pass,
  clean build/boot-smoke.
- 2026-07-07: P0.5 main() decomposition: sliced the fifth completed-
  action-outcome family (Teufel, 16 contiguous variants) into
  `tick_item_use_teufel::dispatch_teufel_outcome` (`main.rs` down to
  4,569). 1091 server tests unchanged, clean build/boot-smoke.
- 2026-07-07: P0.5 main() decomposition: sliced the sixth completed-
  action-outcome family (skel-raise, 4 contiguous variants) into
  `tick_item_use_skelraise::dispatch_skelraise_outcome` (`main.rs`
  down to 4,556). 1091 server tests unchanged, clean build/boot-smoke.
- 2026-07-07: Area 1 `robber_driver` (`CDR_ROBBER`) ported: the
  midnight-meeting forest patrol NPC's nine-waypoint walk, ladder/hole
  use-triggers, torch upkeep, and single-victim self-defense. 2415
  core + 1091 server tests pass, clean build/boot-smoke.
- 2026-07-07: P0.5 main() decomposition: sliced the seventh completed-
  action-outcome family (Edemon/Fdemon boss machinery, 19 variants)
  into `tick_item_use_edemon_fdemon::dispatch_edemon_fdemon_outcome`
  (`main.rs` down to 4,359). 1091 server tests unchanged, clean build/boot-smoke.
- 2026-07-07: P0.5 main() decomposition: sliced the eighth completed-
  action-outcome family (transport-point, 3 contiguous variants) into
  `tick_item_use_transport::dispatch_transport_outcome` (`main.rs` down
  to 4,300). 1091 server tests unchanged, clean build/boot-smoke.
- 2026-07-07: P0.5 main() decomposition: sliced the ninth completed-
  action-outcome family (clan-spawn/LQ/arena, 13 contiguous variants)
  into `tick_item_use_clan_lq_arena::dispatch_clan_lq_arena_outcome`
  (`main.rs` down to 4,220). 1091 server tests unchanged, clean build/boot-smoke.
- 2026-07-07: P0.5 main() decomposition: sliced the tenth completed-
  action-outcome family (shrines, 8 contiguous variants) into
  `tick_item_use_shrines::dispatch_shrine_outcome` (`main.rs` down to
  3,889). 1091 server tests unchanged, clean build/boot-smoke.
- 2026-07-07: P0.5 main() decomposition: sliced the eleventh completed-
  action-outcome family (burndown barrel, 5 contiguous variants) into
  `tick_item_use_burndown::dispatch_burndown_outcome` (`main.rs` down
  to 3,882). 1091 server tests unchanged, clean build/boot-smoke.
- 2026-07-07: P0.5 main() decomposition: sliced the twelfth completed-
  action-outcome family (xmas + swamp-spawn, 4 contiguous variants)
  into `tick_item_use_xmas_swamp::dispatch_xmas_swamp_outcome`
  (`main.rs` down to 3,834). 1091 server tests unchanged, clean
  build/boot-smoke.
- 2026-07-07: P0.5 main() decomposition: sliced the thirteenth completed-
  action-outcome family (Caligar, 14 variants scattered across 4 spots)
  into `tick_item_use_caligar::dispatch_caligar_outcome` (`main.rs` down
  to 3,743). 1091 server tests unchanged, clean build/boot-smoke.
- 2026-07-07: P0.5 main() decomposition: sliced the fourteenth completed-
  action-outcome family (key-assembly: staffer/saltmine/bone-holder/
  arkhata/lizard-flower/palace-key/mine-gateway/shrike-amulet, 51
  variants across 6 spots) into
  `tick_item_use_keyassembly::dispatch_keyassembly_outcome` (`main.rs`
  down to 3,451). 1091 server + 2415 core tests unchanged, clean
  build/boot-smoke.
- 2026-07-07: P0.5 main() decomposition: sliced the fifteenth completed-
  action-outcome family (labyrinth: Lab2/Lab3/Brannington berries +
  lab-entrance/exit, 18 contiguous variants) into
  `tick_item_use_lab::dispatch_lab_outcome` (`main.rs` down to 3,345).
  1091 server + 2415 core tests unchanged, clean build/boot-smoke.
- 2026-07-07: P0.5 main() decomposition: sliced the sixteenth completed-
  action-outcome family (mine-wall digging, 5 contiguous variants) into
  `tick_item_use_minewall::dispatch_minewall_outcome` (`main.rs` down to
  3,317). 1091 server + 2415 core tests unchanged, clean build/boot-smoke.
- 2026-07-07: P0.5 main() decomposition: sliced the seventeenth completed-
  action-outcome family (forest-spade/junkpile/pick-door, 8 contiguous
  variants) into `tick_item_use_dig_pick::dispatch_dig_pick_outcome`
  (`main.rs` down to 3,245). 1091 server + 2415 core tests unchanged,
  clean build/boot-smoke.
- 2026-07-07: P0.5 main() decomposition: sliced the eighteenth completed-
  action-outcome family (lollipops/Christmas pop/special potions/books/
  bookcase, 12 contiguous variants) into
  `tick_item_use_books_potions::dispatch_books_potions_outcome`
  (`main.rs` down to 3,072). 1091 server + 2415 core tests unchanged,
  clean build/boot-smoke.
- 2026-07-07: P0.5 main() decomposition: sliced the nineteenth completed-
  action-outcome family (keyring/assemble/gathering/alchemy-flask, 22
  contiguous variants) into
  `tick_item_use_crafting::dispatch_crafting_outcome` (`main.rs` down to
  2,848). 1091 server + 2415 core tests unchanged, clean build/boot-smoke.
- 2026-07-07: P0.5 main() decomposition COMPLETE: extracted the last
  scaffolding around the outcome match into
  `tick_item_use_completion::process_completed_action_outcomes`
  (`main.rs` down to 1,586, under the 2,000 cap). 1091 server + 2415 core
  tests unchanged, clean build/boot-smoke.
- 2026-07-07: P0.5 split `tests/commands_admin/character.rs` (7,933
  lines) into 13 command-family files under
  `tests/commands_admin/character/` via `tools/rust_split/splitter.py`
  (largest now 1,038 lines). 1091 server + 2415 core tests unchanged,
  clean build.
- 2026-07-07: Area 1 `sanoa_driver` (`CDR_SANOA`) ported: ambient
  dialogue-free twelve-waypoint city walker plus self-defense cascade.
  1091 server + 2423 core tests pass, clean build/boot-smoke.
- 2026-07-07: Area 1 `reskin_driver` (`CDR_RESKIN`) ported: tavern-keeper
  dialogue chain unlocking `QLOG_RESKIN`, plus its alchemy-ingredient
  turn-in (gold + `ACHIEVEMENT_WELL_PAID_GATHERER`). 1091 server + 2443
  core tests pass, clean build/boot-smoke.
- 2026-07-07: Area 1 `asturin_driver` (`CDR_ASTURIN`) ported: the
  private-quarters guard's positional greeting/warning state machine plus
  its self-defense cascade and `asturin_dead` death hook. 1091 server +
  2454 core tests pass, clean build/boot-smoke.
- 2026-07-07: Area 1 `guiwynn_driver` (`CDR_GUIWYNN`) ported: the
  town-mage's two-part "Order of Mages" quest chain (`QLOG` 7-8), money
  reward kept as a literal carried item (not gold), matching C. 1091
  server + 2470 core tests pass, clean build/boot-smoke.
- 2026-07-07: Area 1 `james_driver` (`CDR_JAMES`) ported: Lydia-quest
  hand-off/hardcore-recruiter dialogue plus the full `james_raisehint`
  advice-only weighted priority computation; the `CF_GOD`-only "raise
  me"/equipment-grant tail is a documented, deliberate gap. 1091 server +
  2488 core tests pass, clean build/boot-smoke.
- 2026-07-07: Area 1 `balltrap_skelly_driver` (`CDR_BALLTRAP`) ported: the
  stationary ball-trap guard skeleton's self-defense cascade plus its
  3-second-gated `do_use(DX_LEFT, 0)` trap trigger. Only `logain_driver`
  remains unported in this file. 1091 server + 2494 core tests pass, clean
  build/boot-smoke.
- 2026-07-07: Area 1 `logain_driver` (`CDR_LOGAIN`) ported (the last NPC
  in `ch_driver`'s dispatch table) plus the shared `gwendylon_dead`
  death-hook branches for all 10 remaining quest-giver drivers. Only
  `tutorial_ppd` (`player_driver.c`) is left before this checkbox closes.
  1094 server + 2512 core tests pass, clean build/boot-smoke.
- 2026-07-07: `tutorial_ppd`/`tutorial()` (`player_driver.c:374-711`)
  ported: all 17 newbie hint branches wired into the tick loop, closing
  Area 1's last gap. 1098 server + 2531 core tests pass, clean
  build/boot-smoke.
- 2026-07-07: P0.5 area-text color markers: built the `COL_STR_*` sentinel
  + `expand_color_sentinels` + `WorldAreaTextBytes`/`npc_quiet_say_bytes`
  mechanism and restored it end-to-end on `camhermit.rs`'s reminder line
  (the worked example). ~12+ other deviation sites remain (see checkbox
  note). 1098 server + 2534 core tests pass, clean build/boot-smoke.
- 2026-07-07: P0.5 area-text color markers: restored the remaining 12
  documented/undocumented sites (gwendylon/greeter/yoakin/jessica/reskin/
  lydia/james/guiwynn/logain/brithildie/trader/bank) via `COL_STR_*`
  sentinels + `_bytes` siblings; added `text::has_color_sentinels` helper
  for bank's shared reply loop. Only `area32/military.rs` remains. 1098
  server + 2534 core tests pass, clean build/boot-smoke.
- 2026-07-08: P0.5 area-text color markers COMPLETE: restored the last
  site, `area32/military.rs` (27 `COL_LIGHT_BLUE`/`COL_RESET` call sites
  across mission-offer/accept/hear/reroll/greet text and both Advisor
  favor flows) via `COL_STR_*` sentinels + `_bytes` siblings. 1098
  server + 2534 core tests pass, clean build/boot-smoke.
- 2026-07-08: P0.5 "Retire legacy blob writes" write-path slice: the three
  `snapshots.rs` save builders and `SAVE_CHARACTER_*_SQL` no longer write
  `ppd_blob`/`subscriber_blob` (columns now frozen); legacy decoders marked
  `#[deprecated]`, now-test-only encoders `#[allow(dead_code)]`. Backfill
  migration (needs Rust decode logic, not plain SQL) not yet started - `[~]`.
  2534 core + 1098 server tests pass, live DB suite green, clean build/
  boot-smoke.
- 2026-07-08: P0.5 "Retire legacy blob writes" CLOSED: added the backfill
  startup routine (`ugaris-server/src/legacy_backfill.rs`, called from
  `main.rs` after `run_migrations()`) plus two new `CharacterRepository`
  methods; verified against a live Docker Postgres with real legacy rows
  (decode-success and decode-failure-retry paths both confirmed). 2534
  core + 1101 server tests pass, live DB suite green, clean build/boot.
- 2026-07-08: P0.5 `military.rs` split CLOSED: mechanically split the
  3,615-line file into `military/{mod,missions,military_master,
  military_advisor}.rs` (largest 1,300 lines) via `tools/rust_split/
  splitter.py` with a name-keyed `ASSIGN` spec exploding the multiple
  `impl World` blocks by method name. 2534 core + 1101 server tests
  unchanged, clean build/boot-smoke.
- 2026-07-08: Area 2 (`src/area/2/area2.c`) CLOSED: ported all four
  character drivers (`CDR_SUPERIOR`/`CDR_MOONIE`/`CDR_VAMPIRE`/
  `CDR_VAMPIRE2`) as one file each under `world/npc/area2/`, plus the
  `vampire`/`vampire2_dead` crypt-quest completion death hooks. 2553 core
  + 1101 server tests pass, clean build/boot-smoke.
- 2026-07-08: Area 3 (`src/area/3/area3.c`) STARTED: documented the full
  9-remaining-driver breakdown, ported the simplest (`astro1_driver`,
  ambient monologue, `world/npc/area3/astro1.rs`). 2558 core + 1101
  server tests pass, clean build/boot-smoke.
- 2026-07-08: Area 3: ported `thomas_driver`+`sir_jones_driver` (crypt
  entrance guard + full 16-state crypt quest chain), plus a new shared
  `AREA3_QA` table. 2575 core + 1101 server tests pass, clean
  build/boot-smoke (area 1 and area 3 zone data both verified).
- 2026-07-08: Area 3: ported `astro2_driver` (lost-astronomer's-notes
  quest, `QLOG` 16), including the `IID_AREA2_ASTRONOTE` NT_GIVE handoff
  and its first-completion `MONEY_AREA3_MOONIES` reward. 2581 core + 1101
  server tests pass, clean build/boot-smoke.
- 2026-07-08: Area 3: ported `seymour_driver` (`QLOG` 10-12, Aston-entry
  greeting/army-enrollment chain), including the 3-branch `NT_GIVE`
  handler (Loisan's note/zombie skull 1/zombie skull 2) and the
  `set_army_rank(co, 1)` enrollment deviation. 2592 core + 1101 server
  tests pass, clean build/boot-smoke.
- 2026-07-08: Area 3: ported `kelly_driver` (27-state chain: quests
  13-15/54/60, park shrines, swamp-beast-head bounties, Caligar-plaque
  hunt), plus a new `shortcut to caligar` `AREA3_QA` entry. 2616 core +
  1101 server tests pass, clean build/boot-smoke.
- 2026-07-08: Area 3: ported `carlos_driver` (two independent quest
  chains: ritual/quest 61 + repeatable dragon-staff/quest 20 with
  unconditional `Dragonsbane` achievement), plus `staffer_carlos2_state`
  and 8 new item-id constants. 2629 core + 1101 server tests pass, clean
  build/boot-smoke.
- 2026-07-08: Area 3: ported `kassim_driver` (jewelry engraver: greeting/
  inscription/item-wait/engrave state machine, `engrave:` command, gold
  charge, `IF_ENGRAVED` item write), plus a new `CharacterDriverState::
  Engrave` variant and 2 new `PlayerRuntime` timer accessors. 2639 core +
  1101 server tests pass, clean build/boot-smoke.
- 2026-07-08: Area 3: ported `supermax_driver` (past-maxes raiser:
  greeting sequence, `list`/`money`/`raise`/`lower` commands), plus new
  `supermax_canraise`/`supermax_cost` core helpers and 82 new `AREA3_QA`
  entries. 2653 core + 1101 server tests pass, clean build/boot-smoke.
- 2026-07-08: Area 3 CLOSED: ported `lampghost_driver`/`_respawn`/`_dead`
  (self-defense + nearest-lit-lamp claim/walk/extinguish job loop), new
  `World::area3_lamp_claims` registry substituting C's `lamp[]` array.
  2663 core + 1104 server tests pass, clean build/boot-smoke.
- 2026-07-08: Area 4 STARTED: ported the full pentagram solve/reward
  pipeline (activate/deactivate/quest-completion/reward-distribution/
  color-combo/lucky-pent/record-tracking) plus all 4 reachable pentagram
  achievement call sites; demon spawning/`CDR_PENTER`/`CDR_TESTER`/DB
  record persistence remain (see checkbox REMAINING). 2680 core + 1106
  server tests pass, clean build/boot-smoke.
- 2026-07-08: Area 4: ported demon spawning + the `CDR_PENTER` demon
  driver (reuses `CDR_SIMPLEBADDY` AI) + `update_demon_profession` +
  `handle_demon_death`'s power-reduction/`DEMON_LORDS_DEMISE` award; only
  `CDR_TESTER` and `pentagram_record` DB persistence remain. 2696 core +
  1108 server tests pass, clean build/boot-smoke.
- 2026-07-08: Area 4: ported `pentagram_record` DB restart-persistence
  (new `PgPentagramRecordRepository`, `migrations/
  0021_pentagram_record.sql`, startup load + 4h-cadence/`/saveall`
  saves). Only `CDR_TESTER` remains. 2696 core + 1108 server + 81 db
  tests pass, clean build/boot-smoke.
- 2026-07-08: Area 4 CLOSED: ported `pentagram_tester_driver`
  (`CDR_TESTER`, `world/npc/area4/tester.rs`), the QA-only test bot -
  last remaining piece. 2704 core + 1108 server tests pass, clean
  build/boot-smoke.
- 2026-07-08: Area 6 audit + two `CF_EDEMON` combat gaps closed:
  `do_walk`'s earthmud-spell movement slowdown (previously a total no-op)
  and `check_strike_near`'s earth-demon ball/flash damage reduction.
  2711 core + 1108 server tests pass, clean build/boot-smoke.
- 2026-07-08: Area 6 CLOSED: fixed `act_attack` to call `get_attack_skill`/
  `get_parry_skill` (`attack_skill`/`parry_skill`) instead of the raw
  `V_ATTACK`/`V_PARRY` stat - a cross-cutting P1 melee to-hit bug affecting
  every character, not just earth demons. 2712 core + 1108 server tests
  pass, clean build/boot-smoke.
- 2026-07-08: Area 8 STARTED: ported `CDR_FDEMON_DEMON` (new
  `world::fdemon` waypoint-hunt-graph module + `world::npc::area8::
  fdemon_demon` gohome/wander driver) and its `fdemon_demon_dead` death
  hook; `CDR_FDEMON_ARMY`/`CDR_FDEMON_BOSS` (soldier recruitment + mission
  dialogue) remain. 2724 core + 1108 server tests pass, clean build/
  boot-smoke (verified live against real `zones/8/fire.map` data).
- 2026-07-08: Area 8: ported `CDR_FDEMON_BOSS` (Commander's 33-stage
  mission dialogue, `platoon_exp` player reward) plus the matching
  `IDR_FDEMONLOADER` defense-station bookkeeping; only `CDR_FDEMON_ARMY`
  remains. 2753 core + 1108 server tests pass, clean build/boot-smoke
  (verified live against real `zones/8/fire.map` data with the Commander).
- 2026-07-08: Area 8 `CDR_FDEMON_ARMY` first slice: pure `profile[]`/
  `assign_profile`/type-eligibility logic (`fdemon_army.rs`) + `struct
  soldier` PPD accessors; spawning/formation-AI/emotes still remain.
  2762 core + 1108 server tests pass, clean build/boot-smoke.
- 2026-07-08: Area 8 `CDR_FDEMON_ARMY` second slice: `update_soldier`'s
  pure stat-scaling + skill-tiered equipment selection in
  `fdemon_army.rs`. `calc_exp`/spawning/formation-AI still remain.
  2766 core + 1108 server tests pass, clean build.
- 2026-07-08: Area 8 `CDR_FDEMON_ARMY` third slice: ported `calc_exp`
  (`world/exp.rs`, C `skill.c:174-196`) plus `update_soldier`'s exp/level
  recompute (`fdemon_army::finalize_soldier_exp_and_level`). Spawning/
  driver id/formation-AI/emotes still remain. 2771 core + 1108 server
  tests pass, clean build.
- 2026-07-08: Area 8 `CDR_FDEMON_ARMY` fourth slice: wired real
  `take_soldiers`/`drop_soldiers` spawning (`area8_army.rs`) plus the
  `MIS_FOLLOW`/leader-lost tick (`army_follow_driver`/`fdemon_army_tick`)
  - soldiers now spawn/despawn on "take"/"drop" and follow their leader.
  2778 core + 1111 server tests pass, clean build/boot-smoke.
- 2026-07-08: Area 8 `CDR_FDEMON_ARMY` fifth slice: ported the "follow"/
  "back"/"retreat"/"front"/"behind" `NT_TEXT` command reception plus
  `army_back_driver`/`army_front_driver` - `MIS_BACK`/`MIS_RETREAT`/
  `MIS_FRONT` are now live; only `army_behind_driver`'s combat remains
  unported. 2794 core + 1111 server tests pass, clean build/boot-smoke.
- 2026-07-08: Area 8 `CDR_FDEMON_ARMY` sixth slice: ported
  `army_behind_driver` (leader-facing-target lookup + flank positioning
   + `do_attack`) - `MIS_BEHIND` is now fully live. 2798 core + 1111
  server tests pass, clean build/boot-smoke.
- 2026-07-08: Area 11 STARTED: ported `palace_islena`/`CDR_PALACEISLENA`
  (dialogue/aggro state machine, three "Power of X" heal triggers,
  `islena_dead`'s `ACHIEVEMENT_LADYKILLER` award). `palace_guard` remains.
  2844 core + 1113 server tests pass, clean build/boot-smoke (area 11).
- 2026-07-08: Area 11 CLOSED: ported `palace_guard`/`CDR_PALACEGUARD`
  (patrol/reserve-ambush/scream-alert/freeze-chokepoint/`Ice Eye` line
  walk, single-victim self-defense). 2856 core + 1113 server tests pass,
  clean build/boot-smoke (area 11, 139 characters, no panics).
- 2026-07-08: Area 12 STARTED: ported `handle_mining_result`'s silver/
  gold/golem-spawn/cave-in branches (new `world/mining.rs` + server-side
  `mine.rs`), `check_military_silver`, and the silver/gold-mined
  achievement wiring. Orb/artifact finds and `CDR_GOLEMKEYHOLDER` remain.
  2865 core + 1123 server tests pass, clean build/boot-smoke.
- 2026-07-08: Area 15 CLOSED: wired `clara_driver`'s full `NT_CHAR`/
  `NT_TEXT`/`NT_GIVE` message loop + military-point rewards onto the
  existing pure `clara_dialogue_step`, widened the `CDR_SWAMPMONSTER`->
  `CDR_SIMPLEBADDY` AI gates (same precedent as `CDR_PENTER`), and fixed a
  real bug: a duplicate `EXP_AREA15_HARDKILL` constant was `5000` instead
  of C's `7500`. 2891 core + 1136 server tests pass, clean build/boot-
  smoke (area 15, 11 characters, no panics).
- 2026-07-08: Area 16 CLOSED: ported `imp_driver`/`william_driver`/
  `hermit_driver` (`world/npc/area16/{imp,william,hermit}.rs`) plus
  `CDR_FORESTMONSTER`'s `monster_dead` death hook (weapon-glow in
  `World::apply_forest_monster_death_driver`, `imp_kills`/`hermit_state`
  counters in `ugaris-server::apply_forest_monster_death_from_hurt_event`).
  2918 core + 1138 server tests pass, clean build/boot-smoke (area 16,
  116 characters, no panics).

