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
- [x] **Area 17 - `src/area/17/two.c`** - Two-City thief/skeleton NPC
  drivers (`CDR_TWOSKELLY` has state scaffolding).
  `CDR_TWOSKELLY` ("Scarcewind", the raised governor's-ghost
  quest giver, quest 30) is now fully ported (`world/npc/area17/
  two_skelly.rs`), including its 30-second self-destruct timer and the
  new file-local `TWOCITY_QA` shared table (`world/npc/area17/mod.rs`,
  currently only the entries this NPC needs). `CDR_TWOALCHEMIST`
  ("Cervik", the spider-poison quest giver, quest 31) is now also ported
  (`world/npc/area17/alchemist.rs`), sharing `TWOCITY_QA`; its 1st/3rd/
  7th/10th-completion potion reward (`combo_potion3`/`security_potion`,
  level-gated) is finished server-side (`ugaris-server/area17.rs`) since
  it needs the quest-log completion count and `ZoneLoader`. `CDR_TWOSANWYN`
  ("Sanwyn", the military quest giver, quest 29) is now also ported
  (`world/npc/area17/sanwyn.rs`), sharing `TWOCITY_QA`; its per-note
  military-points reward is applied directly via `World::
  give_military_pts_from_npc` (no server-side deferral needed).
  `CDR_TWOBARKEEPER` (the tavern barkeeper/guest-pass broker) is now also
  ported (`world/npc/area17/barkeeper.rs`), the first driver needing the
  new shared `legal_status`/`legal_fine`/`citizen_status` `twocity_ppd`
  accessors (plus its own `barkeeper_state`/`barkeeper_last`) and the new
  shared `LS_CLEAN`/`LS_FINE`/`LS_DEAD`/`CS_ENEMY`/`CS_GUEST`/
  `CS_CITIZEN`/`CS_HONOR` constants in `world/npc/area17/mod.rs`.
  `CDR_TWOGUARD` (the Exkordon territory-enforcement city guard patrol,
  the biggest single driver in this file) is now also ported
  (`world/npc/area17/guard.rs` + `guard_messages.rs`, split across two
  files to stay under the ~800-line NPC guideline): the day/night torch
  sensor, the full `NT_CHAR` illegal-territory leave-warning/fine ladder
  state machine (with `LS_DEAD` immediate-attack and `LS_CLEAN` one-time
  guest-pass-intro branches), the `NT_TEXT` "pay" command (including its
  bank-account fallback, new `twocity_current_guard`/
  `twocity_current_guard_time`/`twocity_last_attack`/`twocity_guard_intro`
  `PlayerRuntime` accessors) and god-only citizen-status admin commands,
  `NT_GOTHIT`'s low-HP `call_guard` alert (new shared `World::
  two_city_call_guard` in `mod.rs`, reused by the still-unported
  `servant_driver`) plus attacking-a-guard fine, `NT_SEEHIT`'s
  protect-an-ally fine, `NT_NPC`'s `NTID_TWOCITY` called-to-help
  destination and `NTID_TWOCITY_PICK` lockpicking fine, and the patrol-
  waypoint/return-to-post movement tail. The `guard_dead` death hook is
  ported in `ugaris-server`'s `world_events::death_hooks::
  apply_two_guard_death_from_hurt_event`. 30 new focused tests.
  `CDR_TWOROBBER` (the forest-camp robbers, `two.c:3163-3165`, an
  unconditional `char_driver(CDR_SIMPLEBADDY, ...)` tail call) is now
  reused end-to-end (new `character.driver == CDR_SIMPLEBADDY` gate
  widening in `world/npc_fight.rs`/`world/npc_idle.rs`, same precedent as
  `CDR_PENTER`/`CDR_FORESTMONSTER`) plus its `robber_dead` death hook
  (`ugaris-server`'s `apply_two_robber_death_from_hurt_event`, new
  `PlayerRuntime::set_twocity_thief_killed`). `CDR_TWOSERVANT` (the
  forbidden-territory palace maids/mistress/governor's-double,
  `world/npc/area17/servant.rs`) is now also ported, including its
  `servant_dead` death hook. `CDR_TWOTHIEFGUARD` (the thieves-guild sewer
  entrance guard, `world/npc/area17/thiefguard.rs`) is now also ported,
  including its own fight-driver hostility toward players caught inside
  the sewers before joining the guild. `CDR_TWOTHIEFMASTER` ("Guild
  Master", the 18-state lockpick-chain quest giver behind the sewer
  entrance covering quests 25-28, `world/npc/area17/thiefmaster.rs`) is
  now also ported, closing Area 17.
- [x] **Area 18 - `src/area/18/bones.c`** - rune quest completion
  (`exec_rune` rewards), bone NPCs. *(done - C's own `ch_driver` is an
  empty stub (no bone-specific NPCs; area 18's monsters use plain
  `driver=7`/`CDR_SIMPLEBADDY`, already ported elsewhere), so this task was
  entirely the item-driver/rune-puzzle half. Closed the rune-combination
  reward table, the full boneholder rune-stand pipeline, and the
  `bonebridge` partial-add/remove-bones-from-a-carried-bridge path
  (`bones.c:236-270`) via new `BoneBridgeAddBone`/`BoneBridgeFinished`/
  `BoneBridgeWrongCursorItem`/`BoneBridgeRemoveBone`/
  `BoneBridgeNotEnoughBones` outcomes + `World::add_bone_to_bridge`/
  `remove_bone_from_bridge`, the new "bone" item landing on the cursor via
  `ZoneLoader::instantiate_item_template`; details in PORTING_LEDGER.md)*
- [x] **Area 19 - `src/area/19/nomad.c`** - nomad camp NPCs/trading.
  *(done - `CDR_NOMAD` (all 6 personas: Kalanur/tribe recruiter quest 32,
  Irakar/dice seller, the `Llakal Sla` game host, the two Kir monastery
  monks/quests 33-34, the statue seller) and `CDR_MADHERMIT`
  (flower-guarding self-defense NPC) both ported end-to-end, including the
  full `Llakal Sla` dice-betting minigame (`nomad_bet`/`nomad_roll`) and
  its `IDR_NOMADDICE` -> `NT_NPC`/`NTID_DICE` `notify_area` wiring, which
  was a real pre-existing gap (the roll was computed but never delivered
  to the nomad NPC). `world/npc/area19/{nomad,nomad_dialogue,nomad_text,
  nomad_give,nomad_bet,madhermit}.rs` + `ugaris-server/src/area19.rs` +
  `tick_npc/area19.rs`; details in PORTING_LEDGER.md)*
- [x] **Area 20 - `src/area/20/lq.c`** - live-quest admin command table,
  LQ NPC dialogue (spawn/raise/equipment ported).
  REMAINING: `lqnpc`'s per-tick dialogue/movement driver (`NT_CHAR`
  greeting, `NT_GOTHIT` hurt-mark plus aggressive-mode self-defense,
  `NT_TEXT` trigger/reply plus "followme"/"stopfollow" admin-mirroring,
  `NT_GIVE` quest-item turn-in) and `lqnpc_died`'s respawn-scheduling/
  kill-hurt-mark death hook are now ported
  (`world/npc/area20/lqnpc.rs`, `ugaris-server/src/area20.rs` +
  `tick_npc/area20.rs`, `world_events::death_hooks::
  apply_lqnpc_death_from_hurt_event`), including a new `LqItemSpec`/
  `make_lq_item_template_id` (`create_lq_item` port, `Item::template_id`
  as C's `it[in].ID`) and a new typed `PlayerRuntime::lq_marks`
  (`DRD_LQ_PLR_DATA`). `LqNpcState`/`LqNpcSpawnRequest` widened with the
  dialogue/reward fields C's `spawn_npc` copies into `DRD_LQ_NPC_DATA`.
  Deliberately NOT ported (unreachable without it): the `usurp` god/
  LQMaster-possession `domirror` movement branch, since only the
  `special_driver` admin command table below can ever set `dat->usurp`
  (see `lqnpc.rs`'s own module doc comment). The `CDR_LQPARSER` admin
  command table's first slice (18 of ~45 subcommands - NPC-template CRUD:
  `#npc`/`#npcname`/`#npcgold`/`#npcsprite`/`#npcpos`/
  `#npcdescription`/`#npcgreeting`/`#npcreply`/`#npclist`/
  `#npcdelete`/`#npcwantitem`/`#npcitem`/`#npcshow`/`#npckillmark`/
  `#npchurtmark`/`#npcrewarditem`/`#npcmodlevel`/`#npcrespawn`) is now
  ported (`world/lq_admin.rs`'s `World::apply_lq_admin_command`, wired
  into `tick_client_actions.rs` right after `apply_clan_command`) - pure
  `World` logic needing no `ZoneLoader`/`PlayerRuntime`, since every field
  these commands touch already lived on `LqNpcState`. Still unported:
  `#thrall`/`#killthrall` (need `DRD_LQ_NPC_DATA.thrallname`), `#usurp`/
  `#follow`/`#stop`/`#exit` (need a new `PlayerRuntime.usurp` field),
  `#nspawn`/`#nremove`/`#nsay`/`#nimmortal`/`#nemote`/`#nattack`/`#wimp`
  (live-instance control), the whole quest-lifecycle family
  (`#questsave`/`#questdelete`/`#questend`/`#questload`/`#questshow`/
  `#questreward`/`#questlevel`/`#questreset`/`#questentrance`/
  `#queststart`, `#xinfo` - needs a new `lq_data` `World` field C never
  got ported here), and `#questsave`/`#questload`'s file I/O (a
  genuinely new pattern for this codebase). Second slice done: the
  `#doorlist`/`#doorlock` pair (`lq.c:2443-2503`) is now ported
  (`world/lq_admin.rs`'s `lq_admin_cmd_doorlist`/`lq_admin_cmd_doorlock`),
  reusing the pre-existing `World::lq_doors`/`discover_lq_doors_once`/
  `write_lq_door_key_id` scaffolding from the `LqTicker` port - pure
  `World` logic, no `ZoneLoader`/`PlayerRuntime` needed. Third slice
  done: the live-instance-control family (`#nremove`/`#nsay`/
  `#nimmortal`/`#nemote`/`#nattack`, all pure `World` logic, plus
  `#nspawn`, the one command in this family needing `ZoneLoader` for a
  brand new character) is now ported. `#nspawn` is dispatched via its own
  `World::try_dispatch_lq_nspawn`/`LqNspawnDispatch` (see that type's doc
  comment) from `tick_client_actions.rs`, ahead of `apply_lq_admin_command`,
  reusing `spawns::spawn_lq_npc_character` (the same instantiation path
  `#npcrespawn`'s scheduled respawns already use) and a new shared
  `build_lq_npc_spawn_request` (factored out of `queue_due_lq_npc_respawns`).
  `lq_admin_remove_npc_instance` (shared by `#npcdelete` and the new
  `#nremove`) now returns C `remove_npc`'s real `bool` (a pending
  scheduled respawn counts as "removed" even with no live instance) and
  resets the template's `character_id`/`character_serial` on removal,
  matching C's `lq_npc[n].cn = lq_npc[n].cserial = 0`. Still unported:
  `#thrall`/`#killthrall` (need `DRD_LQ_NPC_DATA.thrallname`), `#usurp`/
  `#follow`/`#stop`/`#exit`/`#wimp` (need a new `PlayerRuntime.usurp`
  field, `#wimp` also needs a `teleport_char_driver`-equivalent free-tile
  search), the whole quest-lifecycle family (`#questsave`/`#questdelete`/
  `#questend`/`#questload`/`#questshow`/`#questreward`/`#questlevel`/
  `#questreset`/`#questentrance`/`#queststart`, `#xinfo` - needs a new
  `lq_data` `World` field C never got ported here), and
  `#questsave`/`#questload`'s file I/O (a genuinely new pattern for this
  codebase). Fourth slice done: `#thrall`/`#killthrall` (`lq.c:427-503`,
  `spawn_npc`'s on-the-fly template-detached `isthrall` spawn/despawn
  pair) are now ported. `LqNpcDriverData` gained the `thrallname` field;
  `LqNpcSpawnRequest` gained `is_thrall`/`thrall_name`; `spawn_lq_npc_
  character` branches on `is_thrall` to skip `slot`/`greeting`/`trigger`/
  `reply`/the `lq_npcs` slot-bookkeeping registration, matching C's own
  `isthrall` guard. `#thrall` is `World::try_dispatch_lq_thrall`/
  `LqThrallDispatch` (same `#nspawn`-precedent split, dispatched ahead of
  `apply_lq_admin_command`), resolving only the *first* matching template
  slot (unlike `#nspawn`'s all-matches-plus-`"all"`) and rolling an
  independent `RANDOM(4)`-based drop position per spawned thrall.
  `#killthrall` is pure `World::lq_admin_cmd_killthrall`, scanning every
  live `CDR_LQNPC` character directly (a thrall has no template row to
  resolve via `lq_npcs`). Fifth slice done: `#usurp`/`#follow`/`#stop`/
  `#exit`/`#wimp`, the possessed-NPC "me"/"emote" relay sub-command, the
  possessed-NPC plain-speech relay, and the per-tick `domirror` movement-
  mirroring branch this all drives are now ported (new `world/lq_usurp.rs`
  - `lq_admin.rs` was already near the 2,000-line cap - plus a new
  `Character::lq_usurp` field, C's `pdat->usurp`; `LqNpcDriverData` gained
  `usurp`/`udx`/`udy`). Dispatched from `tick_client_actions.rs` right
  after alias expansion (before any chat/tell/who handling), matching C's
  real priority (`special_driver` runs before `command()`'s own switch) -
  this is what lets the plain-speech relay actually intercept ordinary
  `say` text. `#wimp`'s free-tile search reuses the already-ported
  `teleport_char_driver` (turned out not to need a new helper, despite
  this file's earlier note); `cmd_wimp`'s `ppd->last_lq_death` write is a
  new `World::pending_lq_wimps` drain queue (`PlayerRuntime::
  set_last_lq_death`, `misc_ppd` offset 8) applied by `tick_client_
   actions.rs` right after the queued-action loop. Deliberately NOT
   ported: the `c9`/`mirror` relay sub-command (needs `chat.c`'s
   `server_chat`, permanently deferred cross-server transport per
   `AGENTS.md`). 20 new focused tests (`world/tests/lq_usurp.rs` +
   3 `domirror` tests in `world/tests/lqnpc.rs`). Sixth slice done: the
   non-file-I/O half of the quest-lifecycle family - `#questlevel`/
   `#questreward`/`#questshow`/`#questentrance`/`#queststart`/
   `#questreset` - is now ported (`world/lq_admin.rs`, new `LqData`
   struct = C's `struct lq_data`, plus a new `World::
   lq_reset_drop_body_item` for `#questreset`'s already-on-the-map body
   relocation). All six are pure `World` logic; `#npcshow`'s hurt/
  killmark exp-preview line now reads the real table instead of a
  placeholder. Seventh slice done: `#questend`/`#xinfo` (new
  `world/lq_quest_admin.rs` - `lq_admin.rs` was already near the
  2,000-line cap - plus `tick_client_actions.rs`'s new
  `dispatch_lq_questend_or_xinfo`, checked right before
  `apply_lq_admin_command`'s fallback like `#nspawn`/`#thrall`). Same
  split as those two: `World::lq_admin_wants_questend`/
  `lq_admin_wants_xinfo` gate the command/permission/area match,
  `World::apply_lq_questend_reward`/`report_lq_xinfo` do the pure-`World`
  reward-math/formatting half, and `ugaris-server` supplies the
  `PlayerRuntime::lq_marks` `World` can't see by iterating
  `ServerRuntime::players`. Eighth (final) slice done: `#questsave`/
  `#questdelete`/`#questload` (`world/lq_quest_file.rs`, another new
  sibling file for the same "`lq_admin.rs` near/over the 2,000-line cap"
  reason) - the last file-I/O gap. `LqQuestSnapshot`/`LqQuestFile` are
  this port's JSON replacement for C's raw `lq_data`+`lq_npc[]`+
  `lq_door[]` byte dump (no cross-version binary-compat requirement to
  preserve for a save format this port owns end to end); the pure-`World`
  half (area/permission gate, `get_str` name/password parsing, the
  per-character `isalpha` name validation, `LqQuestFileDispatch`) stays in
  `ugaris-core`, the actual `quest/<name>.qst` `read`/`write`/`remove_file`
  (plus the stored-password compare gating overwrite/delete/load) is new
  `ugaris-server::area20::handle_lq_quest_file_dispatch`, dispatched from
  `tick_client_actions.rs` ahead of `apply_lq_admin_command`'s fallback,
  same split precedent as `#nspawn`/`#thrall`. C's `cmd_questload`'s
  `init_done = 0` (defers the door-`keyID` restore to the next
  `lq_ticker` rescan tick) is reproduced as a synchronous
  `discover_lq_doors_once` rescan inside `apply_lq_quest_snapshot`
  instead, restoring each rescanned slot's `key_id` immediately rather
  than a tick later - see that function's own doc comment for why this is
  an equivalent, not a behavior change. This closes every subcommand in
  the `CDR_LQPARSER` table and closes Area 20.
- [x] **Area 22 - `src/area/22/lab*.c`** - remaining lab mechanics per
  lab; lab2 undead mostly ported; gatekeeper depends on P2.
  lab1's `CDR_LABGNOMEDRIVER` torch-gnome triad and
  `IDR_DEATHFIBRIN` (shrine + staff) are now ported. The shared
  `create_lab_exit`/`IDR_LABEXIT` reward loop (spawn on master death +
  `set_solved_lab`/`change_area` on use) is now also ported end to end
  (`world::lab`'s queue + `ugaris-server`'s `lab::create_lab_exit` +
  `tick_item_use_lab::dispatch_lab_outcome`'s `LabExitUse` handling),
  wired into `labgnome_died_driver`'s `dat->master` branch; the other four
  lab areas' own master-kill hooks still need porting to actually
  call `World::queue_lab_exit_spawn` (the shared machinery they'll call
  now exists). `CDR_LAB2HERALD` (the graveyard chapel keeper's full
  greeting dialogue/keyword jumps/ring turn-in/gate reward) is now also
  ported (`world/npc/area22/lab2_herald.rs`). `CDR_LAB2DEAMON` (the
  family-vault masquerade-detection/seek-and-destroy guardian) is now also
  ported (`world/npc/area22/lab2_deamon.rs`), including its
  `lab2_deamon_create`/`lab2_deamon_is_elias` creation helpers (split
  between `World::init_lab2_deamon`/`lab2_deamon_already_tracking` and the
  `ugaris-server` `Lab2StepActionDaemonWarning` dispatcher, since only the
  server caller knows the real spawn coordinates). `CDR_LAB3PASSGUARD`
  (the password-gate guard) and `CDR_LAB3PRISONER` (the mute note-giving
  prisoner) are now also ported (`world/npc/area22/lab3_passguard.rs` +
  `lab3_prisoner.rs`), including new `PlayerRuntime::legacy_lab3_
  password1/2`/`_guard_talkstep`/`_prisoner_talkstep` accessors and the
  `IID_LAB3_PRISONKEY` item id. `IDR_LAB3_SPECIAL` (`lab3_special`,
  `src/area/22/lab3.c:897-1068`) is now also ported: the password-protected
  teleport door (`drdata[0]==1` - `World::apply_lab3_teleport_door` in the
  new `world/lab.rs` addition, resolving `teleport_char_driver` plus the
  underwater torch-extinguish/bubble/"Hrgblub."/`create_lab_exit`-reward
  tail entirely in `World` since none of it needs `ZoneLoader`/
  `PlayerRuntime`; the guard-locked check reads a new `ItemDriverContext::
  lab3_guard_talkstep` field), the note-giving skeleton (`drdata[0]==2` -
  `ugaris-server`'s new `create_lab3_note_for_character`, sibling to the
  prisoner's own `create_lab3_note_on_cursor`), and the note-reading switch
  (`drdata[0]==3`, cases `1..=6` canned lore text plus `20`/`21`'s
  `lab3_init_password` - `tick_item_use_lab.rs`'s new `lab3_note_text`,
  the first real writer of `PlayerRuntime::legacy_lab3_password1`/`_2`,
  closing the gap `lab3_passguard.rs`'s own module doc comment used to
  flag). Lab4 is now closed: `CDR_LAB4SEYAN` (the "Observer" crown/
  szepter quest giver, `world/npc/area22/lab4_seyan.rs`) and
  `CDR_LAB4GNALB` (the patrol-guard/crazy-gnalb triad, including a new
  branching-path-graph patrol mechanism transcribed digit-for-digit from
  C's `gnalb_path[]` table, `world/npc/area22/lab4_gnalb.rs`) are both
  ported end to end, plus `IDR_LAB4_ITEM`'s fireplace-key branch (new
  `Lab4FireplaceKeyGive`/`Blocked` outcomes). New plain `PlayerRuntime::
  lab4_seyan_state`/`_got` fields (C's non-persistent `DRD_LAB4_PLAYER`,
  intentionally made persistent per `AGENTS.md`) and three new
  `IID_LAB4_MAGEKEY`/`_SZEPTER`/`_CROWN` item ids. Lab5 progress: `CDR_
  LAB5SEYAN` ("Laros", the three-demon-head quest giver,
  `world/npc/area22/lab5_seyan.rs`) and `CDR_LAB5DAEMON` (the shared
  servant/master/gunned demon fight driver - `CF_IMMORTAL` toggle gated
  on `IID_LAB5_WEAPON` in `WN_RHAND` for masters, `world/npc/area22/
  lab5_daemon.rs`) are both ported end to end; new `PlayerRuntime::
  lab5_seyan_state`/`_got` fields and `IID_LAB5_HEAD1`/`_2`/`_3`/
  `_WEAPON` item ids. The trophy heads drop via the existing generic
  death-drop mechanic (no scripted reward call in C), so this closes a
  fully playable "kill 3 master demons, turn in heads, get the lab
  exit" loop on its own. `CDR_LAB5MAGE` ("Mathor", `world/npc/area22/
  lab5_mage.rs`) is now also ported: the full intro/force/demon/ritual-
  explanation dialogue ladder, the `REPEAT`/`FORCE`/`DEMON`/`RITUAL`
  `NT_TEXT` keyword jumps (`DEMONS` is unreachable dead code in C itself -
  documented, not ported), the god-only `SET 1/2/3` ritual-state debug
  command, and the full "inside the name square, shouted the real name"
  ritual invocation - `ritual_hurt` (pure `World`) plus the dynamic
  `ritual_start`/`ritual_create_char` room-spawning system (room search/
  clear/statue-placement in `World::attempt_ritual_start`, the
  `ZoneLoader`-needing demon instantiation in `ugaris-server`'s new
  `lab5_ritual.rs`, wired through a new `Lab5MageOutcomeEvent::
  AttemptRitualStart` + `World::finish_ritual_start`). New `PlayerRuntime::
  lab5_mage_state`/`lab5_ritual_daemon`/`lab5_ritual_state` fields and a
  new `World::lab5_namecoords` dynamic override array (mage's own
  `NT_CREATE` writes index 0, also closing a documented gap in the
  already-ported `lab5_daemon`'s gunned-demon aggro line).
  `IDR_LAB5_ITEM` is now ported for all 13 of its `drdata[0]` flavors
  (obelisk/chestbox/combopotion/manapotion/nameplate/realnameplate/
  entrance/backdoor/gun/pike/no-potion-door/fireface/lightface,
  `crates/ugaris-core/src/item_driver/area22_lab.rs::lab5_item_driver` +
  `crates/ugaris-server/src/tick_item_use_lab.rs`) - the force-summon
  ritual (nameplate -> realnameplate -> entrance) is reachable through
  normal gameplay, not just the god-only `SET` command. The last two
  decorative flavors, `fireface`/`lightface` (pure ambient "shoot a
  projectile down the corridor forever" statues, `cn==0`-only), needed a
  narrow slice of the generic "nothing arms an always-on ambient item
  driver's very first timer call" gap: since both are always static
  `.itm` zone data (never runtime-created), extending the existing
  `World::schedule_existing_light_timers` per-area-load batch-prime
  sweep with an `IDR_LAB5_ITEM` entry was sufficient (verified against
  real zone data: `scheduled_light_timers` 0 -> 29 on `--area-id 22`,
  matching the real placement count). This closes Area 22.
- [x] **Areas 23/24 - `src/area/23_24/strategy.c` (3,599 lines)** - the
  strategy minigame (mission ownership, worker spawning, resources).
  Item dispatch is stubbed as no-ops; this is a full subsystem - plan in
  ledger first.
  REMAINING: plan + first slice done - `crates/ugaris-core/src/world/
  strategy.rs` (order constants, the 14-row `MISSIONS` table and 24-row
  `AI_PRESETS` table digit-for-digit from C, `str_exp_cost`/
  `str_increment`/`str_raise` upgrade economy) plus a new
  `PlayerRuntime::strategy: StrategyPpd` persistent field
  (`crates/ugaris-core/src/player/strategy.rs`). Second slice done: the
  `str_area` registry's `init_areas` plus the per-tick mission-lifecycle
  driver itself (`str_ticker`/`did_party_lose`/`remove_party`/
  `close_area`/`reward_winner`/`init_mission`), `IDR_STR_TICKER` now
  dispatches a real `ItemDriverOutcome::StrTicker`. Third slice done: the
  mission entry queue (`queue_validate`/`queue_remove`/`queue_mission`/
  `queue_check`/`show_queue`, `strategy.c:3200-3276`) is now ported
  (  `World::queue_validate`/`queue_remove`/`queue_mission`/`queue_check`/
  `show_queue`), using `CharacterId` identity directly instead of C's
  `cn`+`ID` pair (same simplification as `ArenaContender`). Fourth slice
  done: `special_driver`'s player-facing `#`/`/` command table itself
  (`CDR_STRATEGY_PARSER`, `strategy.c:3278-3626`) is now ported in
  `crates/ugaris-core/src/world/strategy_special.rs` - `#jp`/`#list`/
  `#info`/`#raise`/`#reset`/`#mission`/`#enter`/`#surrender`/`#queue`,
  including a lazily-initialized jump-point registry and the real
  `#enter` mission-join flow (`str_init_mission`/`take_spawner`), wired
  into `ugaris-server`'s `tick_client_actions.rs`. This mission system is
  now reachable through live gameplay for the first time.
  REMAINING: the worker character driver (`strategy_driver`), the
  `mine`/`storage`/`depot`/`spawner` item drivers (currently no-op), the
  AI-opponent driver (`ai_main`, 538 lines), and `#eguard` (needs
  `ZoneLoader` character-spawning for the still-unported worker driver).
  Fifth slice done: the player-facing `CF_PLAYER` "look" branches of the
  `mine`/`storage`/`depot` item drivers (`strategy.c:1122-1241`) are now
  ported (`item_driver::area23_24::{str_mine_driver,str_storage_driver,
  str_depot_driver}`, new `ItemDriverOutcome::StrMineLook`/`StrDepotLook`/
  `StrStorageInteract` + `StrStorageConversion`, dispatched via new
  `crates/ugaris-server/src/tick_item_use_strategy.rs`): reading a
  building's current Platinum total, and storage's carried-`IDR_ENHANCE`
  silver/gold-to-Platinum conversion. `IDR_STR_SPAWNER` and the `cn==0`/
  NPC-worker halves of all three remain no-ops (still gated on the
  unported `strategy_driver`).
  Sixth slice done: the worker character driver's (`strategy_driver`,
  `strategy.c:713-1120`) NT_TEXT order-assignment cascade is now ported as
  pure/testable `World` methods (`crates/ugaris-core/src/world/
  strategy_worker.rs`, new `StrategyWorkerOrder` enum replacing C's
  `order`/`or1`/`or2` triple, `World::strategy_worker_apply_order_text`
  plus the `finditem`/`finddepot` ring-spiral map searches
  (`strategy_find_item_near`/`strategy_find_depot_or_storage_near`) it
  needs) - the "mine"/"follow"/"guard"/"fight"/"home"/"take"/"transfer"/
  "train" spoken-command keywords, their item-lookup validation and
  "sorry, ..." failure text, digit-for-digit. Not wired to a live
  character yet (no `CDR_*`/`CharacterDriverState` exists for a worker -
  same gap the item-driver doc comment above already calls out).
  Seventh slice done: `setname`'s three pure pieces - `strategy_train_price`
  (`TRAINPRICE` macro), `strategy_worker_name` (per-order name template),
  `strategy_worker_description` - plus `findstorage`
  (`World::strategy_find_storage_owned_by_group`) and `restplace`
  (`World::strategy_worker_rest_place`, C's `dat->restplace` persisted as
  an `Option<(dx, dy)>` tile-delta instead of a raw `m`-space offset) are
  now also ported in `world/strategy_worker.rs`, all pure/testable without
  a live worker character (18 new tests).
  REMAINING: `strategy_driver`'s NT_CREATE handling, the per-tick
  order-execution switch (movement/`use_driver` dispatch per order), the
  `CDR_STRATEGY`/`CharacterDriverState`/`spawner_sub` spawning wiring
  needed to ever construct a live worker, the `mine`/`storage`/`depot`/
  `spawner` item drivers' NPC-worker branches, and the full `ai_main`/
  `ai_init` AI-opponent driver.
  Eighth slice done: the `strategy_boss` NPC dialogue driver (Cinciac,
  `CDR_STRATEGY_BOSS = 80`, a static zone-placed NPC needing no
  `ZoneLoader` spawning) is now ported end to end
  (`world/npc/area23_24/boss.rs` + `ugaris-server/src/area23_24.rs`,
  `tick_npc::area23_24::strategy_boss_driver_118`) - the full 12-stage
  greeting/mission-briefing dialogue plus the "repeat"/"military rank"/
  "levels and experience" `NT_TEXT` commands. This is the first live path
  that can ever advance `StrategyPpd::boss_stage` past 0, unlocking the
  entire already-ported `CDR_STRATEGY_PARSER` command table
  (`#jp`/`#list`/`#info`/`#raise`/`#mission`/`#enter`/`#surrender`)
  through real gameplay for the first time.
  Ninth slice done: `strategy_driver`'s full per-tick body (`CDR_STRATEGY
  = 78`, new `world/npc/area23_24/worker.rs`) - NT_CREATE, NT_TEXT/
  NT_GIVE/NT_GOTHIT message handling (reusing the already-ported order-
  text cascade), single-victim self-defense, and the complete `OR_MINE`/
  `OR_TRANSFER`/`OR_TAKE`/`OR_FOLLOW`/`OR_FIGHTER`/`OR_GUARD`/
  `OR_ETERNALGUARD`/`OR_TRAIN`/default order-execution switch. Also
  closed the `mine`/`storage`/`depot` item drivers' NPC-worker branches
  (new `ItemDriverOutcome::StrMineWorkerDig`/`StrBuildingWorkerTransfer`/
  `StrDepotWorkerTakeover`, applied in `World::apply_item_driver_outcome`)
  so a worker's orders actually move Platinum between buildings. No live
  `CDR_STRATEGY` character can exist yet - `spawner_sub`/`take_spawner`
  spawning remains unported - so this is tested via directly-constructed
  test characters only (registered in `tick_npc::run_all` regardless, per
  precedent).
  Tenth slice done: `IDR_STR_SPAWNER`'s player-facing `spawner`/
  `spawner_sub` (`strategy.c:1244-1381`) is now ported end to end - a
  live `CDR_STRATEGY` worker can finally be recruited through real
  gameplay. `World::try_dispatch_strategy_spawner_use` (new in
  `world/strategy_worker.rs`) ports the ownership/storage-lookup/
  Platinum-cost/worker-count-cap eligibility checks (all pure `World`
  logic, including a real C quirk preserved deliberately: the `NPCPRICE`
  Platinum is deducted *before* character creation is ever attempted, so
  a drop failure still spends it - not "fixed"); `ugaris-server`'s new
  `tick_item_use_strategy::spawn_strategy_worker` builds the actual fresh
  `"strategy_npc"` character via `ZoneLoader` +
  `World::spawn_character_from_item_drop` (C `create_char`+
  `item_drop_char`), applies the `value[1]` warcry/endurance/speed
  bonuses before `update_char`, restores hp/endurance/mana to max, and
  finishes via a new `World::finish_strategy_worker_spawn` driver-state
  stamp. 7 new tests in `world::tests::strategy`.
  Eleventh slice done: the `ai_main`/`ai_init` AI-opponent driver's
  structural building blocks - `AiData`/`AiNpc`/`AiPlace` (new
  `world/strategy_ai.rs`), the place-graph navigation primitives
  (`update_npc_place`/`subtask_move`), all seven `task_*` order-assignment
  functions, the roster bookkeeping (`assign_npc`/`add_worker`/
  `add_etguard`/`add_guard`/`remove_guard`/`remove_worker`), and the
  guard-defense allocation logic (`wantguardcnt`/`assign_guards`/
  `remove_free_guards`/`nag_attack`) - are now ported, all fully testable
  without a live spawned AI army (37 new tests).
  Twelfth slice done: `World::ai_init` itself (`strategy.c:2269-2427`) -
  the place-graph construction from `IDR_STR_MINE`/`_DEPOT`/`_STORAGE`
  items sharing a spawner's area slot, the `pathfinder` distance/parent
  BFS, `enemy_possible` up-propagation, and live-roster discovery/
  classification (factored into a separately-tested `AiData::
  register_npc`, since `ai_init`'s own `code`-vs-`Character::group`
  match can never succeed today - `Character::group` is `u16`-narrowed
  and every valid `ai_init` `code` exceeds `u16::MAX`, the same
  pre-existing, documented gap as `World::str_did_party_lose`). 12 new
  tests, still not wired to any live tick call site.
  Thirteenth slice done: `World::ai_refresh_places` (`strategy.c:2505-
  2630`) - `ai_main`'s per-place owned/platin/threat refresh loop (the
  enemy-presence scan populating `threat`/`threatlevel`/`threatcount`/
  `threatncount`/`threatnlevel`, threat propagation up/down the parent
  chain, `panic`/`pplace`/`pdist`) plus the "project threats to
  neighboring places" pass. C's sector-grid scan is replaced with a
  plain linear `self.characters` scan (same final distance check, same
  precedent as other "no sector index" ports); `ragnarok`/`nogoldleft`
  are returned via a new `AiPlaceRefreshResult` rather than committed to
  `AiData` (C only commits them at the very end of `ai_main`, after
  still-unported blocks that read the *previous* tick's values). 11 new
  tests.
  Fourteenth slice done: `AiData::update_guard_list`/`update_nag_guard`/
  `update_place_worker_and_eguard_counts`/`update_free_npc_count`
  (`strategy.c:2484-2500,2509-2520,2531-2539,2632-2642`) - the remaining
  pure roster-bookkeeping refreshes from `ai_main`'s outer body that need
  no live-character/item access, closing every part of the per-place loop
  besides the already-ported threat scan. 15 new tests.
  Fifteenth slice done: `World::ai_update_npc_list` (`strategy.c:2461-
  2482`, the "update npc list" NPC refresh) - resolved by widening
  `AiNpc::cn` to `Option<CharacterId>` (C's `an[n].cn = 0` sentinel)
  instead of attempting index-preserving `Vec` removal, so every other
  index-based reference (`worker[]`/`eguard`/`guard[]`/`nagguard`) keeps
  working unchanged. 5 new tests.
  Sixteenth slice done: `AiData::assign_tasks_to_workers` (`strategy.c:
  2674-2796`) - the panic/non-panic "assign tasks to workers" loop, the
  core per-tick planning decision: panic sends every non-eternal-guard
  NPC to fight at `pplace`; otherwise each NPC keeps its job if still
  productive/safe, gets promoted to/recalled from elite-guard duty, gets
  redirected to a busier parent place, or falls back to the nearest
  depot/mine/storage with spare capacity, else goes idle. New
  `AiData::ragnarok`/`nogoldleft` committed fields (read as the previous
  tick's values) and a new `AiPlaceRefreshResult::mindist` field support
  this; one documented deviation (C's `ap[-1]` OOB read on a storage-
  parented NPC is treated as "no parent threat"). 11 new tests.
  Seventeenth slice done: the final per-npc task-dispatch `switch`
  (`strategy.c:2932-2972`) is now ported as `World::ai_dispatch_tasks`
  (`crates/ugaris-core/src/world/strategy_ai.rs`) - dispatches every
  roster NPC to its already-ported `task_*` function by `AiTask` (the
  `T_EGUARD` train-vs-idle-vs-guard nested `if` kept verbatim), then
  writes the resulting raw `order`/`or1`/`or2` back onto the live
  worker's typed `StrategyWorkerOrder` via a new `raw_to_strategy_
  worker_order` (the inverse of the existing `strategy_worker_
  order_to_raw`), auto-vivifying driver state same as `ai_task_idle`.
  7 new tests. `strategy_ai.rs` is now 1,923 lines - split it before
  adding the next slice.
  Eighteenth slice done: split `strategy_ai.rs` into the pure `AiData`/
  `AiPlace`/`AiNpc` types file plus a new sibling `strategy_ai_tasks.rs`
  carrying every `impl World` method over them, then ported `ai_main`'s
  "create new workers" loop (`:2644-2672`) - `AiData::register_new_worker`
  plus `World::ai_wants_more_workers`/`ai_plan_worker_spawn` (the
  eligibility/`NPCPRICE`-deduction half; the actual `ZoneLoader`-needing
  character-creation tail is deliberately deferred until a live `ai_main`
  call site exists to call it, avoiding a dead-code function in the
  `ugaris-server` binary crate).
  Nineteenth slice done: `World::ai_threat_and_worklevel_tick`
  (`crates/ugaris-core/src/world/strategy_ai_tasks.rs`, `strategy.c:
  2798-2916`) - the "find places with too little workers"/threat-list
  maintenance (expire/record/sort-via-`tcomp`/dispatch-via-
  `World::ai_assign_guards`/truncate)/worklevel-adjustment tail. New
  `AiThreat` type plus `AiData::threats`/`lastchange` fields; `tcomp`'s
  two real comparator bugs (empty-slot side always sorts "less"
  regardless of side, and the distance branch always returns "less"
  regardless of direction) are kept verbatim, not fixed. 10 new tests.
  Twentieth slice done: `create_eguard`'s eligibility/plan/roster-
  registration halves (`strategy.c:2892-2920,2987-3029`) are now ported -
  `World::ai_wants_more_eguards`/`ai_eguard_spawn_candidates`/
  `ai_plan_eguard_spawn` (new `AiEguardSpawnPlan`) plus
  `AiData::register_new_eguard`, mirroring the `ai_plan_worker_spawn`/
  `register_new_worker` split precedent exactly. 7 new tests.
  Twenty-first slice done: the two `ZoneLoader`-needing character-creation
  tails are now also ported - `ugaris-server`'s new `tick_item_use_
  strategy::spawn_ai_worker`/`spawn_ai_eguard` build the actual fresh
  `"strategy_npc"` character from `AiWorkerSpawnPlan`/`AiEguardSpawnPlan`
  (`spawner_sub`'s `create_char`/`item_drop_char` half and
  `create_eguard`'s `create_char`/`drop_char` half respectively,
  `strategy.c:1259-1279,2991-3023`), reusing the exact same value-bonus/
  `update_char`/hp-endurance-mana-to-max/dir/sprite/group shape as the
  tenth slice's `spawn_strategy_worker`; a new `World::
  finish_ai_eguard_spawn` sibling to `finish_strategy_worker_spawn`
  additionally stamps the `OR_ETERNALGUARD` order C's `create_eguard`
  sets. `#[allow(dead_code)]`'d (same precedent as `dungeon.rs`/
  `snapshots.rs`/`depot.rs`/`events.rs`) since neither has a live caller
  yet - exercised directly by 2 new `tests::strategy` tests.
  Twenty-second slice done: the two prerequisite fixes flagged by the
  previous note are both closed. First, the real reschedule bug: the
  `IDR_STR_TICKER`/`IDR_LQ_TICKER` cn==0 timer reschedule used to only be
  applied by `tick_item_use_clan_lq_arena.rs`'s `StrTicker`/`LqTicker`
  arms - dead code, since a `character_id==0` timer-fired outcome never
  flows through that player-`item_use`-completion pipeline. The
  reschedule (`World::schedule_item_driver_timer`) now lives in `World::
  apply_item_driver_outcome`'s own `LqTicker`/`StrTicker` arms - the real
  dispatch point both the timer path and (theoretically) the item-use
  path funnel through - with `str_ticker`'s reward-event drain
  (`apply_strategy_reward_events`, needs `ServerRuntime`) moved to
  `tick_world.rs`'s `timer_outcomes` loop instead (the correct precedent,
  same as `EdemonGateSpawn`/`ChestSpawn`). Second, priming: `IDR_LQ_TICKER`/
  `IDR_STR_TICKER` are now included in `World::
  schedule_existing_light_timers`'s zone-load priming sweep (a narrower,
  existing precedent that already substitutes for C's fully generic
  `create_item_nr` priming call for every "always-on ambient `cn==0`
  driver" - porting that fully generic mechanism remains out of scope, per
  the previous note). Boot-smoke against areas 20/23 confirms both tickers
  now self-perpetuate forever (`processed timer callbacks` firing every
  ~24 ticks indefinitely) instead of going silent after one call.
  With both prerequisites real, `World::ai_main` itself (`crates/
  ugaris-core/src/world/strategy_ai_main.rs`) now assembles every
  previously-ported piece (`ai_init`/`ai_update_npc_list`/
  `update_guard_list`/`update_nag_guard`/`update_place_worker_and_eguard_
  counts`/`ai_refresh_places`/`update_free_npc_count`/`ai_wants_more_
  workers`+`ai_plan_worker_spawn`/`assign_tasks_to_workers`/`ai_threat_and_
  worklevel_tick`/`ai_wants_more_eguards`+`ai_eguard_spawn_candidates`+
  `ai_plan_eguard_spawn`/`ai_nag_attack`/`ai_dispatch_tasks`) into one real
  per-tick call, in C's exact order, plus a new `World::ai_parties:
  HashMap<u32, AiData>` registry (C's `ai_data[MAX_AI]`) and `World::
  register_ai_worker`/`register_ai_eguard` for `ugaris-server` to call back
  once it actually builds a planned character. Two documented
  simplifications (both already flagged by `AiWorkerSpawnPlan`/
  `AiEguardSpawnPlan`'s own doc comments): at most one worker/eguard is
  planned per call instead of C's unbounded loop (converges over a few
  more ticks instead of bursting in one), and no live `IDR_STR_SPAWNER`
  `cn==0` timer tick calls `ai_main` yet - see REMAINING. 10 new focused
  tests (`world/tests/strategy_ai_main.rs`) plus 2 reschedule-bug
  regression tests.
  Twenty-third (final) slice done: the live `IDR_STR_SPAWNER` `cn==0`
  ambient/AI-init timer tick (`spawner`, `strategy.c:1319-1356`) is now
  wired end to end - `World::str_spawner_ambient_tick` (new
  `ItemDriverOutcome::StrSpawnerAmbientTick`, dispatched from `World::
  apply_item_driver_outcome` since it needs the LCG random seed for its
  jittered `TICKS + RANDOM(TICKS)` reschedule) resolves the spawner's
  owner code, runs the one-time rename/storage-income-seed setup
  (`World::str_spawner_first_activation`) on the first tick after a
  mission assigns it, then calls `World::ai_main` every tick after and
  queues any returned worker/eternal-guard plan onto two new pending-spawn
  `World` queues for `ugaris-server` to drain unconditionally every tick
  (`tick_world.rs`, reusing the already-implemented, now-live
  `spawn_ai_worker`/`spawn_ai_eguard`). `World::ai_init` also now seeds
  `ad.ppd.npc_color` directly from the spawner's own `drdata[10]` byte
  (C mutates the static `preset[].ppd.npc_color` in place before its own
  `ai_init` call; `AI_PRESETS` is an immutable `const` table in this port,
  so the override applies straight to the fresh `AiData` instead).
  `IDR_STR_SPAWNER` is now included in `World::
  schedule_existing_light_timers`'s zone-load priming sweep alongside the
  two tickers. Boot-smoke against areas 23/24 confirms spawners
  self-perpetuate forever with no panics. This closes Areas 23/24 for
  real - every checkbox in this task's own plan is now live.
- [x] **Area 25 - `src/area/25/warped.c`** - warped NPC dialogue,
  `DRD_WARPFIGHTER` full fight driver. *(done - `CDR_WARPMASTER`/
  `CDR_WARPFIGHTER` both ported, plus `warped_raise`'s full stat-rescale/
  equipment-item synthesis wired into the trial-door spawn and the
  player-teleport-back-on-kill death hook; item drivers were already
  ported in earlier iterations. REMAINING (deferred, documented no-op):
  `warpfighter`'s rare "spoiled potion of freeze" self-curse sub-branch,
  which needs a `create_spell_timer` mechanism this codebase has not
  ported anywhere yet. Details in PORTING_LEDGER.md.)*
- [x] **Area 26 - `src/area/26/staffer.c`** - vault skull PPD/quest, Rouven
  smuggler dialogue. *(done - `IDR_STAFFER` subtypes 1-5, `smugglecom_
  driver`/`smugglelead_died`, and `rouven_driver` (`CDR_ROUVEN = 130`, the
  Imperial Vault guard, quests 62/63, `world/npc/area26/rouven.rs`) are
  all ported. Details in PORTING_LEDGER.md.)*
- [x] **Area 28 - `src/area/28/brannington_forest.c`** - forest NPCs.
  *(done - `aristocrat_driver`/`yoatin_driver` (`CDR_ARISTOCRAT`/`CDR_YOATIN`,
  quests 38/39) and `CDR_WHITEROBBERBOSS`'s `robberboss_dead` death hook
  (quest 46) all ported; `IDR_BRANNINGTONFOREST` item driver was already
  done. Details in PORTING_LEDGER.md.)*
- [x] **Area 29 - `src/area/29/brannington.c`** - Brannington quest NPCs,
  `DRD_STAFFER_PPD` remaining fields. *(done - `grinnich_driver`/
  `shanra_driver` (`CDR_GRINNICH`/`CDR_SHANRA`, the shared tower-dungeon
  hint/reward/teleport flow) close out every NPC in this file; all other
  `brannington.c` drivers were already ported in earlier iterations.
  Details in PORTING_LEDGER.md.)*
- [x] **Area 30 - `src/area/30/clanmaster.c`** - clan master NPC (needs P3
  clan system). *(done - found already almost fully ported (`CDR_CLANMASTER`/
  `CDR_CLANCLERK`, all 4 item drivers, tick registration, event
  application) from earlier work whose checkbox was never ticked; closed
  the one real gap, the `clanmaster_dead` charlog-only death hook for both
  drivers. Details in PORTING_LEDGER.md.)*
- [x] **Area 31 - `src/area/31/warrmines.c`** - Warr mines NPCs. *(done -
  all four character drivers - `dwarfchief_driver`/`lostdwarf_driver`/
  `dwarfshaman_driver`/`dwarfsmith_driver` - ported in
  `world/npc/area31/{dwarfchief,lostdwarf,dwarfshaman,dwarfsmith}.rs`;
  item drivers were already done. Details in PORTING_LEDGER.md.)*
- [x] **Area 32 - `src/area/32/missions.c`** - governor mission NPCs
  (needs P3 military). *(done - `CDR_MISSIONGIVE` ("Mister Jones")
  dialogue/reward-shop, `start_mission`/`build_fighter` instance-dungeon
  spawn, `mission_fighter_dead` kill-counter hook, `missionchest_driver`/
  `mission_done`, the rotating "special offer" purchase, and `CTPOT`'s
  multi-turn stat-potion flow were all ported in earlier iterations.
  `RNORB` ("Random Orb", C's `create_orb()`, `tool.c:3678-3778`) was the
  last of the 24 reward-shop entries left unported - closed by having
  `ugaris-server::area32::apply_mission_giver_events`'s `GiveItemReward`
  handler special-case `itmtmp == "RNORB"` to roll one of the 32 `V_*`
  skills via `world.roll_legacy_random(32)`/`area_apply::
  legacy_orb_value_from_seed` and build the item with the already-existing
  `area_apply::instantiate_orb_with_modifier` instead of
  `loader.instantiate_item_template`, rather than adding a 5th duplicate
  orb-naming implementation. All 24/24 reward-shop entries are now
  functional. Details in PORTING_LEDGER.md.)*
- [x] **Area 33 - `src/area/33/tunnel.c`** - long tunnel events. Also wire
  `achievement_add_tunnel_level` using the existing `award_*` helper
  pattern in `crates/ugaris-server/src/achievement.rs` (Achievements
  task, closed iteration 84). *(done - details in PORTING_LEDGER.md)*
- [x] **Area 34 - `src/area/34/teufel.c`** - rat/gambler NPCs, arena score
  rewards. `CDR_TEUFELQUEST`, `CDR_TEUFELDEMON`, `CDR_TEUFELGAMBLER` (the
  3 chip-tier dice game plus its 3 `give_rewardN` tables, `world/npc/
  area34/teufelgambler.rs`), and `CDR_TEUFELRAT` (pure `CDR_SIMPLEBADDY`
  tail call, gate-widened in `world/npc_fight.rs`/`world/npc_idle.rs` +
  `zone.rs`; death-scoring was already ported) are all now ported.
  *(done - details in PORTING_LEDGER.md)*
- [x] **Area 36 - `src/area/36/caligar.c`** - Caligar quest NPCs, PPD
  quest state beyond skelly doors. *(done - the final four quest NPCs
  (`glori_driver`/`arquin_driver`/`smith_driver`/`homden_driver`, the
  quest-54-59 obelisk/key-part/dungeon-key/ring chain) are now ported in
  `world/npc/area36/{glori,arquin,smith,homden}.rs`; details in
  PORTING_LEDGER.md)*
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

- 2026-07-12: Area 36 CLOSED: ported the last four quest NPCs -
  `glori_driver`/`arquin_driver`/`smith_driver`/`homden_driver` (quest
  54-59 obelisk/key-part/dungeon-key/ring chain). 4057 core [+21] + 1204
  server tests pass, clean build/boot-smoke (area 36, no panics).
- 2026-07-12: Area 36 STARTED: ported the three non-quest-chain drivers -
  `CDR_CALIGARGUARD` (Eulc/Margana riddle guards), `CDR_CALIGARGUARD2`
  (taunting sentry), `CDR_CALIGARSKELLY` AI gate-widening. 4036 core [+19]
  + 1204 server tests pass, clean build/boot-smoke (area 36, no panics).
- 2026-07-12: Area 34 CLOSED: ported `CDR_TEUFELGAMBLER` (3 chip-tier dice
  game + `give_reward`/`2`/`3` tables, `world/npc/area34/teufelgambler.rs`)
  and `CDR_TEUFELRAT` (pure `CDR_SIMPLEBADDY` tail-call gate widening).
  4017 core [+15] + 1204 server tests pass, clean build, boot-smoke.
- 2026-07-12: Area 34 STARTED: ported `CDR_TEUFELQUEST` (rat-hunt reward
  NPC: greeting, give experience/military/money/godly, `special_rat_reward`
  ladder) + shared `TEUFEL_QA`/`teufel_analyse_text`. 3998 core [+13] +
  1204 server tests pass, clean build, boot-smoke (area 34, no panic).
- 2026-07-12: fixed a broken build left by iteration 50 (unhandled
  `TunnelDoorAreaCheck`/`TunnelDoorFlavor` match arms in
  `tick_item_use_completion.rs`) and added the missing tests for
  `mean_door_driver`/`tunnel_mean_door_area_clear` (`IDR_TUNNELDOOR2`).
  3973 core [+9] + 1204 server tests, boot-smoke.
- 2026-07-12: Area 33 progress: ported `IDR_TUNNELDOOR`'s exp/military
  exit-pillar branches + `give_reward` (auto-promote ladder,
  `achievement_add_tunnel_level` wiring). `DOOR_ENTRY`/`DOOR_CONTINUE`
  maze generator + `IDR_TUNNELDOOR2` still unported. 3964 core [+17] +
  1204 server tests, boot-smoke.
- 2026-07-12: Area 32 - ported `RNORB` (`create_orb()`), the last reward-
  shop gap, via existing `area_apply` orb helpers. 24/24 rewards now
  functional; task closed [x]. 3944 core + 1204 server [+1] tests, boot-smoke.
- 2026-07-12: Area 32 progress: ported `CTPOT`'s multi-turn custom-stat-
  potion skill-naming flow (`find_skill_text`, `MissionGiveOutcomeEvent::
  GiveCustomStatPotion`) - the pre-existing `Item::modifier_index`/`_value`
  fields already covered the "deeper gap" the old note described; only
  `RNORB` remains unported. 3944 core [+11] + 1203 server [+21] tests pass,
  boot-smoke.
- 2026-07-12: Area 32 progress: ported the rotating "special offer" gear
  purchase (qa codes 18/19, `ugaris-server::area32::
  regenerate_mission_giver_special_offers` + `MissionGiveOutcomeEvent::
  ShowSpecialOffer`) - 23/24 reward-shop entries now functional. 3933
  core [+8] + 1199 server [+4] tests pass, boot-smoke.
- 2026-07-12: Area 32 progress: ported `missionchest_driver`/`mission_done`
  (`IDR_MISSIONCHEST`, `item_driver::area32_missions` + `ugaris-server::
  area32::apply_mission_chest_open`) - the reward chest can now be opened
  and `find_item`/auto-solve work end to end. 3929 core [+2] + 1195
  server [+7] tests pass, boot-smoke.
- 2026-07-12: Area 32 progress: ported `mission_fighter_dead`'s kill-
  counter hook (new `CDR_MISSIONFIGHT` driver id, `world_events::
  death_hooks::apply_mission_fighter_death_from_hurt_event`) - kills now
  bump kill counters and auto-solve jobs. 3927 core + 1188 server [+4]
  tests pass, boot-smoke.
- 2026-07-11: Area 32 progress: ported `start_mission`/`build_fighter`
  (the 41x41 instance-dungeon spawn, `world/npc/area32/mission_start.rs`
  + `ugaris-server/src/area32.rs::spawn_mission_fighter`) - accepting a
  job now builds the dungeon, spawns fighters, wires door/chest keys, and
  teleports the player in. 3927 core [+10] + 1184 server [+2] tests pass,
  boot-smoke (area 32, real zone data).
- 2026-07-11: Area 32 progress: ported `CDR_MISSIONGIVE` ("Mister Jones"
  job-board/reward-shop NPC, `world/npc/area32/governor.rs`, new
  `PlayerRuntime::governor` field) - dialogue, job rolling, and 22/24
  rewards fully live. 3917 core [+11] + 1182 server tests pass, boot-smoke.
- 2026-07-11: Area 31 CLOSED: ported all four `warrmines.c` character
  drivers (dwarfchief/lostdwarf/dwarfshaman/dwarfsmith quest chains,
  `world/npc/area31/*.rs`); item drivers were already done. 3906 core
  [+32] + 1182 server tests pass, clean build/boot-smoke.
- 2026-07-11: Area 30 CLOSED: found the NPCs/item-drivers already fully
  ported from earlier work (stale checkbox); closed the last gap, the
  `clanmaster_dead` charlog-only death hook for `CDR_CLANMASTER`/
  `CDR_CLANCLERK`. 3876 core + 1182 server [+2] tests pass, boot-smoke.
- 2026-07-11: Area 29 progress: ported `guard_brannington_driver`
  (`CDR_GUARDBRAN`, quest 64 "Finding Arkhata", `world/npc/area29/
  guardbran.rs`), incl. `case1->2`/`case6->7` real fallthrough and a
  read-only `arkhata_ppd.rammy_state` cross-area gate. 3854 core [+17]
  + 1180 server tests pass, clean build/boot-smoke (area 29).
- 2026-07-11: Paperwork catch-up: `broklin_driver` (`CDR_BROKLIN`, quests
  45/46 + gold<->silver trade) was fully ported and tested by iteration 36
  but its `PORTING_TODO.md`/`PORTING_LEDGER.md` entries were never
  updated; documented now, no code changes.
- 2026-07-11: Area 29 progress: ported `brenneth_brannington_driver`
  (`CDR_BRENNETHBRAN`, quests 41-43, `world/npc/area29/brennethbran.rs`),
  including the `case 5`/`9` questlog-fast-forward guards. 3821 core [+11]
  + 1180 server tests pass, clean build/boot-smoke (area 29).
- 2026-07-11: Area 29 progress: ported `forest_brannington_driver`
  (`CDR_FORESTBRAN`, no quest, `world/npc/area29/forestbran.rs`), reading
  the already-ported `forestbran_done` counter for dig-location hints.
  3810 core [+12] + 1180 server tests pass, clean build/boot-smoke.
- 2026-07-11: Area 29 progress: ported `count`/`countessa`/
  `daughter_brannington_driver` (quest 40, shared `countbran_bits`,
  `world/npc/area29/{countbran,countessabran,daughterbran}.rs`). 3798 core
  [+24] + 1176 server tests pass, clean build/boot-smoke (area 29).
- 2026-07-11: Area 29 STARTED: ported `spirit_brannington_driver`
  (`CDR_SPIRITBRAN`, quest 44, `world/npc/area29/spiritbran.rs`), saves
  reward instead of gold/item. 3774 core [+11] + 1176 server tests pass,
  clean build/boot-smoke (area 29, `placed_characters=285`).
- 2026-07-11: Area 28 CLOSED: ported `aristocrat_driver`/`yoatin_driver`
  (`CDR_ARISTOCRAT`/`CDR_YOATIN`, quests 38/39) and `robberboss_dead`
  (`CDR_WHITEROBBERBOSS`, quest 46). 3763 core [+23] + 1176 server tests
  pass, clean build/boot-smoke (area 28, `placed_characters=247`).
- 2026-07-11: Area 26 CLOSED: ported `rouven_driver` (`CDR_ROUVEN`, the
  Imperial Vault guard, quests 62/63, `world/npc/area26/rouven.rs`),
  including the `vault_key1`/`IID_MAX_VAULTKEY` grant. 3741 core [+17] +
  1176 server tests pass, clean build/boot-smoke (area 26).
- 2026-07-11: Area 26 progress: ported `smugglecom_driver`/
  `smugglelead_died` (Contraband quest chain, quests 35-37,
  `CDR_SMUGGLECOM`/`CDR_SMUGGLELEAD`). 3724 core [+21] + 1176 server tests
  pass, clean build/boot-smoke (area 26). Only `rouven_driver` remains.
- 2026-07-11: Area 25 CLOSED: ported `CDR_WARPMASTER` (key-for-stone
  trader) and `CDR_WARPFIGHTER` (trial-room fighter, full `warped_raise`
  stat/equipment scaling, self-destruct, death-hook teleport-back). 3702
  core [+20] + 1176 server [+6] tests pass, clean build/boot-smoke (area 25).
- 2026-07-11: Areas 23/24 strategy minigame CLOSED: twenty-third slice -
  wired the live `IDR_STR_SPAWNER` `cn==0` ambient tick to `World::ai_main`
  (`str_spawner_ambient_tick`/`str_spawner_first_activation`, zone-load
  priming, `tick_world.rs` spawn-plan drain). 3682 core [+5] + 1170 server
  tests pass, clean build/boot-smoke (areas 23/24, no panics).
- 2026-07-11: Areas 23/24 strategy minigame: twenty-second slice - fixed
  the LqTicker/StrTicker reschedule+priming prerequisite bugs (both now
  self-perpetuate forever, boot-smoke confirmed) and assembled `World::
  ai_main` from every previously-ported piece. 3677 core [+12] + 1170
  server tests pass, clean build/boot-smoke. Live spawner wiring remains.
- 2026-07-11: Areas 23/24 strategy minigame: twenty-first slice - ported
  the `ZoneLoader`-needing character-creation tails for both AI worker/
  eguard spawn plans (`tick_item_use_strategy::spawn_ai_worker`/
  `spawn_ai_eguard`, `World::finish_ai_eguard_spawn`). 3665 core + 1170
  server tests [+2] pass, clean build/boot-smoke. `ai_main` assembly and
  the timer-priming prerequisite still remain.
- 2026-07-11: Areas 23/24 strategy minigame: twentieth slice - ported
  `create_eguard`'s eligibility/plan/roster-registration halves
  (`ai_wants_more_eguards`/`ai_eguard_spawn_candidates`/
  `ai_plan_eguard_spawn`/`AiData::register_new_eguard`). 3665 core [+7]
  + 1168 server tests pass, clean build/boot-smoke.
- 2026-07-11: Areas 23/24 strategy minigame: nineteenth slice - ported
  `World::ai_threat_and_worklevel_tick` (missing-worker detection,
  `at[]` threat-list expire/record/`tcomp`-sort/`assign_guards`-dispatch/
  truncate, worklevel adjustment). 3658 core [+9] + 1168 server tests
  pass, clean build.
- 2026-07-11: Areas 23/24 strategy minigame: eighteenth slice - split
  `strategy_ai.rs` into a types file + new `strategy_ai_tasks.rs`, then
  ported the "create new workers" loop's pure eligibility/plan half.
  3649 core [+9] + 1168 server tests pass, clean build/boot-smoke.
- 2026-07-11: Areas 23/24 strategy minigame: sixteenth slice - ported
  `AiData::assign_tasks_to_workers` (the panic/non-panic per-tick
  task-assignment loop, `ai_main`'s core planning decision). 3633 core
  [+12] + 1168 server tests pass, clean build/boot-smoke.
- 2026-07-11: Areas 23/24 strategy minigame: fifteenth slice - ported
  `World::ai_update_npc_list` (the "update npc list" NPC refresh),
  widening `AiNpc::cn` to `Option<CharacterId>` to resolve the previous
  slice's own "doesn't map onto Vec roster" blocker. 3621 core [+5] +
  1168 server tests pass, clean build.
- 2026-07-11: Areas 23/24 strategy minigame: fourteenth slice - ported
  the remaining pure per-place/guard-list bookkeeping refreshes
  (`update_guard_list`/`update_nag_guard`/
  `update_place_worker_and_eguard_counts`/`update_free_npc_count`).
  3616 core [+15] + 1168 server tests pass, clean build.
- 2026-07-11: Areas 23/24 strategy minigame: thirteenth slice - ported
  `World::ai_refresh_places` (`ai_main`'s per-place owned/platin/threat
  refresh + neighbor threat projection). 3601 core [+11] + 1168 server
  tests pass, clean build. Roster refresh/task-assignment still remain.
- 2026-07-11: Areas 23/24 strategy minigame: twelfth slice - ported
  `World::ai_init` (place graph + live-roster classification via new
  `AiData::register_npc`). 3590 core [+12] + 1168 server tests pass,
  clean build. `ai_main`'s outer per-tick body still remains.
- 2026-07-11: Areas 23/24 strategy minigame: ninth slice - ported
  `strategy_driver`'s full per-tick body (`CDR_STRATEGY`, `world/npc/
  area23_24/worker.rs`) plus the `mine`/`storage`/`depot` NPC-worker item-
  driver branches. 3534 core [+21] tests pass, clean build/boot-smoke.
  Still no live worker (spawning unported).
- 2026-07-11: Areas 23/24 strategy minigame: fifth slice - ported
  `mine`/`storage`/`depot`'s player "look" branches (info messages +
  storage's silver/gold-to-Platinum conversion). 3467 core [+65] tests
  pass, clean build/boot-smoke. Worker driver/spawner/AI still remain.
- 2026-07-11: Areas 23/24 strategy minigame: fourth slice - ported
  `special_driver`'s `#`/`/` command table (jp/list/info/raise/reset/
  mission/enter/surrender/queue), now live. 3457 core [+21] tests pass,
  clean build/boot-smoke. Only `#eguard` remains (needs `ZoneLoader`).
- 2026-07-11: Areas 23/24 strategy minigame: third slice - ported the
  mission entry queue (`queue_validate`/`queue_remove`/`queue_mission`/
  `queue_check`/`show_queue`). 3436 core [+8] + 1168 server tests pass,
  clean build/boot-smoke. Not yet reachable live (no "go" command caller).
- 2026-07-11: Areas 23/24 strategy minigame: second slice - ported the
  per-tick mission-lifecycle driver (`str_ticker`/`did_party_lose`/
  `remove_party`/`close_area`/`reward_winner`/`init_mission`);
  `IDR_STR_TICKER` now dispatches a real outcome instead of a no-op.
  Not yet reachable live (no caller seeds a real mission). 3428 core
  [+22] tests pass, clean build/boot-smoke.
- 2026-07-11: Areas 23/24 strategy minigame: first slice - ported the
  pure `MISSIONS`/`AI_PRESETS` content tables, order constants, and
  `str_exp_cost`/`str_increment`/`str_raise` upgrade economy plus a new
  persistent `PlayerRuntime::strategy` field. Full plan for the
  remaining worker/AI/item-driver machinery in `world::strategy`'s doc
  comment. 3400 core [+19] tests pass, clean build/boot-smoke.
- 2026-07-11: Area 22 CLOSED: ported `fireface`/`lightface`
  (`IDR_LAB5_ITEM` drdata[0]==2/13), extending `schedule_existing_light_
  timers` to prime them (both are static zone data). 3381 core [+6] +
  1168 server tests pass, clean build/boot-smoke (29 statues live).
- 2026-07-10: Area 22 progress: ported `IDR_LAB5_ITEM` for 11/13
  `drdata[0]` flavors (obelisk/chestbox/potions/nameplate/realnameplate/
  entrance/backdoor/gun/pike/no-potion-door) - the ritual is now reachable
  via normal gameplay. fireface/lightface remain (pre-existing ambient-
  timer-priming gap). 3375 core + 1168 server tests pass, clean boot-smoke.
- 2026-07-09: Area 22 progress: ported `CDR_LAB5MAGE` (dialogue + the
  full force-summon ritual invocation/room-spawn system). Only
  `IDR_LAB5_ITEM` remains for Area 22. 3358 core + 1162 server tests
  pass, clean build/boot-smoke (area 22, no panics).

- 2026-07-09: Area 22 progress: ported Lab5's `CDR_LAB5SEYAN` (head-
  collection quest giver) and `CDR_LAB5DAEMON` (servant/master/gunned
  demon fight driver, `IID_LAB5_WEAPON` immortal-toggle). `CDR_LAB5MAGE`
  + `IDR_LAB5_ITEM` + the ritual room-spawn system remain. 3344 core +
  1162 server tests pass, clean build/boot-smoke (area 22, 12 daemons
  ticking, no panics).

- 2026-07-09: Area 22 progress: ported Lab4 (`CDR_LAB4SEYAN`/`CDR_LAB4GNALB`
  + `IDR_LAB4_ITEM`, new `world/npc/area22/lab4_seyan.rs`+`lab4_gnalb.rs`,
  a new branching-path patrol mechanism). Only lab5 remains for Area 22.
  3325 core + 1162 server tests pass, clean build/boot-smoke (area 22, 3
  gnalb NPCs ticking, no panics).
- 2026-07-09: Area 22 progress: ported `IDR_LAB3_SPECIAL` (teleport door +
  note-giving skeleton + note-reading/password switch, closing the
  `lab3_passguard.rs` password-write gap). 3299 core + 1162 server tests
  pass, clean build/boot-smoke (area 22).
- 2026-07-09: Area 22 progress: ported `CDR_LAB3PASSGUARD`/`CDR_LAB3PRISONER`
  (new `world/npc/area22/lab3_passguard.rs`+`lab3_prisoner.rs`, new
  `PlayerRuntime::legacy_lab3_*` accessors, `IID_LAB3_PRISONKEY`).
  `IDR_LAB3_SPECIAL` (password assignment/door) remains unported. 3286
  core + 1154 server tests pass, clean build/boot-smoke.

- 2026-07-09: Area 22 progress: ported `CDR_LAB2DEAMON` (new
  `world/npc/area22/lab2_deamon.rs`, family-vault masquerade-detection
  guardian: is-Elias check, warn/elias/quick-elias/masquerade dialogue
  ladders, seek-and-destroy fight AI, teleport-if-stuck, self-destruct);
  fixed the `Lab2StepActionDaemonWarning` spawn placeholder to dedup,
  fall back to `(x, y+3)`, and set `co`/`serial`/`dir`. 3268 core + 1154
  server tests pass, clean build/boot-smoke.
- 2026-07-09: Area 22 progress: ported `CDR_LAB2HERALD` (new
  `world/npc/area22/lab2_herald.rs`, graveyard chapel keeper full
  dialogue/gate reward). 3254 core + 1154 server tests pass, clean
  boot-smoke (area 22).
- 2026-07-09: Area 22 progress: ported the shared `create_lab_exit`/
  `IDR_LABEXIT` reward loop (`world::lab` queue, `ugaris-server::lab`,
  `tick_item_use_lab::dispatch_lab_outcome`'s `LabExitUse`), wired into
  lab1's master-kill hook. 3245 core + 1154 server tests pass, clean boot-smoke.
- 2026-07-09: Area 22 progress: ported lab1's `CDR_LABGNOMEDRIVER`
  (torch-gnome guard/fighter/immortal-master triad) and `IDR_DEATHFIBRIN`
  (shrine + staff). 3236 core + 1148 server tests pass, clean boot-smoke.
- 2026-07-09: Area 20 CLOSED: ported `#questsave`/`#questdelete`/
  `#questload` (new `world/lq_quest_file.rs` + `ugaris-server::area20::
  handle_lq_quest_file_dispatch`, JSON save files under `quest/`). 3226
  core + 1148 server tests pass, clean build/boot-smoke.
- 2026-07-09: Area 20 progress: ported `#questend`/`#xinfo` (new
  `world/lq_quest_admin.rs`, `tick_client_actions.rs::
  dispatch_lq_questend_or_xinfo`). Only `#questsave`/`#questdelete`/
  `#questload` file I/O remains. 3210 core + 1141 server tests pass.
- 2026-07-09: Area 20 progress: ported the non-file-I/O quest-lifecycle
  commands `#questlevel`/`#questreward`/`#questshow`/`#questentrance`/
  `#queststart`/`#questreset` (new `LqData`, `World::
  lq_reset_drop_body_item`). 3196 core + 1141 server tests pass, clean
  build/boot-smoke.
- 2026-07-09: Area 20 progress: ported `#usurp`/`#follow`/`#stop`/`#exit`/
  `#wimp` + the possessed-NPC relay + `domirror` tick mirroring (new
  `world/lq_usurp.rs`, `Character::lq_usurp`). 3184 core + 1141 server
  tests pass, clean build/boot-smoke.
- 2026-07-09: Area 20 progress: ported the `CDR_LQPARSER` admin command
  table's `#thrall`/`#killthrall` pair (`world/lq_admin.rs`, new
  `LqNpcDriverData::thrallname`/`LqNpcSpawnRequest::is_thrall`). 3164
  core + 1141 server tests pass, clean build/boot-smoke.
- 2026-07-09: Area 20 progress: ported the `CDR_LQPARSER` admin command
  table's live-instance-control family (`#nspawn`/`#nremove`/`#nsay`/
  `#nimmortal`/`#nemote`/`#nattack`, `world/lq_admin.rs`). 3150 core +
  1140 server tests pass, clean build/boot-smoke.
- 2026-07-09: Area 20 progress: ported the `CDR_LQPARSER` admin command
  table's `#doorlist`/`#doorlock` pair (`world/lq_admin.rs`, reusing the
  existing `LqDoorState`/`discover_lq_doors_once` scaffolding). 3131 core
  + 1140 server tests pass, clean build/boot-smoke.
- 2026-07-09: Area 20 progress: ported the `CDR_LQPARSER` admin command
  table's NPC-template CRUD slice (18 subcommands, `world/lq_admin.rs`,
  pure `World` logic, no `ZoneLoader` needed). 3124 core + 1140 server
  tests pass, clean build/boot-smoke.
- 2026-07-09: Area 20 STARTED: ported `lqnpc`'s per-tick dialogue/
  movement driver + `lqnpc_died`'s death hook (new `world/npc/area20/`,
  `LqItemSpec`, `PlayerRuntime::lq_marks`). Admin command table (~45
  subcommands) remains. 3095 core + 1140 server tests pass, clean
  build/boot-smoke (areas 1 + 20).
- 2026-07-09: Area 19 CLOSED: ported `CDR_NOMAD` (all 6 personas incl. the
  `Llakal Sla` dice-betting minigame) + `CDR_MADHERMIT`, and fixed a real
  gap (dice rolls never reached the nomad NPC). 3088 core + 1138 server
  tests pass, clean build/boot-smoke (area 19, 178 characters, no panics).
- 2026-07-09: Area 18 CLOSED: ported `bonebridge`'s partial-add/remove-
  bones-from-a-carried-bridge path (`bones.c:236-270`), the last gap in
  this file. 3064 core + 1138 server tests pass, clean build/boot-smoke
  (area 18, no panics).
- 2026-07-09: Area 18 STARTED: ported the rune-combination reward table
  (`exec_rune`, all ~44 combos) plus the full boneholder rune-stand
  insert/remove/expire/activate pipeline (new `world/bones.rs`,
  `PlayerRuntime::rune_check`/`rune_set`). Only `bonebridge`'s partial-add
  path remains. 3061 core + 1138 server tests pass, clean build/boot-smoke.
- 2026-07-09: Area 17 CLOSED: ported `CDR_TWOTHIEFMASTER` ("Guild
  Master", the 18-state lockpick-chain quest giver, quests 25-28,
  `world/npc/area17/thiefmaster.rs`), including its own `NT_GOTHIT`
  self-defense (no shared helper existed for it). 3049 core + 1138 server
  tests pass, clean build/boot-smoke (area 1 + 17, no panics).

- 2026-07-09: Area 17 progress: ported `CDR_TWOTHIEFGUARD` (thieves-guild
  entrance guard, `world/npc/area17/thiefguard.rs`) + a new `may_follow_
  invisible` param on the shared fight-driver cascade (also fixed a
  latent `CDR_FDEMON_ARMY` bug). 3023 core + 1138 server tests pass,
  clean build/boot-smoke (area 1 + 17).

- 2026-07-09: Area 17 progress: ported `CDR_TWOSERVANT` (palace maids/
  mistress/governor's-double, `world/npc/area17/servant.rs`) + its
  `servant_dead` death hook. 3004 core + 1138 server tests pass, clean
  build/boot-smoke (area 17).

- 2026-07-09: Area 17 progress: ported `CDR_TWOROBBER` (reuses
  `CDR_SIMPLEBADDY` AI, new gate widening) + its `robber_dead` death hook.
  2986 core + 1138 server tests pass, clean build/boot-smoke (area 17).

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
- 2026-07-08: Area 17 progress: ported `CDR_TWOSANWYN`/`sanwyn`
  (`world/npc/area17/sanwyn.rs`), the military quest giver "Sanwyn"
  (quest 29, "Dirty Hands"); 2953 core + 1138 server tests pass, clean
  build/boot-smoke. Remaining: guard/barkeeper/servant/thiefguard/
  thiefmaster/robber-death-hook (see PORTING_LEDGER.md).
- 2026-07-08: Area 17 progress: ported `CDR_TWOBARKEEPER`/`barkeeper`
  (`world/npc/area17/barkeeper.rs`), the tavern guest-pass broker, plus
  shared `legal_status`/`legal_fine`/`citizen_status` PPD accessors and
  `LS_*`/`CS_*` constants. 2966 core + 1138 server tests pass, clean
  build/boot-smoke (area 17).
- 2026-07-08: Area 17 progress: ported `CDR_TWOGUARD`/`guard_driver`
  (`world/npc/area17/guard.rs`+`guard_messages.rs`), the Exkordon city
  guard patrol, plus its `guard_dead` death hook. 2984 core + 1138 server
  tests pass, clean build/boot-smoke (area 17).
- 2026-07-11: Areas 23/24 progress: ported the `ai_main`/`ai_init`
  AI-opponent driver's structural building blocks (new
  `world/strategy_ai.rs`: `AiData`/`AiNpc`/`AiPlace`, place-graph nav,
  all 7 `task_*` functions, roster bookkeeping, guard-defense allocation).
  3578 core (+37) + 1168 server tests pass, clean build; only the outer
  per-tick bodies/`create_eguard`/threat-scan remain for this task.
- 2026-07-11: Area 29 progress: ported `CDR_CENTINEL` (wooden marionette
  sentinels, `zones/29/wrtower.chr`'s `centinel_count` template) - reuses
  SimpleBaddy fight/idle AI plus `centinel_dead`'s kill-counter death hook
  (milestones at 1/10/20, teleport+reset at 30). 3799 core (+1) + 1180
  server (+4) tests pass, clean build, boot-smoke on area 29.
- 2026-07-11: Area 29 closed: ported `grinnich_driver`/`shanra_driver`
  (`CDR_GRINNICH`/`CDR_SHANRA`, tower-entrance hint + basement reward/
  teleport flow, `grinnich_state`/`shanra_state` PPD fields). 3876 core
  (+23) + 1180 server tests pass, clean build/boot-smoke on area 29.
- 2026-07-12: Area 33 started: ported `gorwin_driver`/`CDR_TUNNELER_GORWIN`
  (`world/npc/area33/gorwin.rs`, new `CDR_TUNNELER_GORWIN`/`IID_TUNNEL*`
  ids). Item drivers/dungeon generator remain. 3954 core (+12) + 1204
  server tests pass, clean build/boot-smoke (areas 1 and 33).
- 2026-07-12: Area 33 closed: ported `tunneldoor`'s `DOOR_ENTRY`/
  `DOOR_CONTINUE` creeper-dungeon instance scan (`World::
  plan_tunnel_entry`, `find_unused_sector`/`build_fighter`/marker
  handlers) plus its `ugaris-server` creeper-spawn wiring. 3985 core
  (+31) + 1204 server tests pass, clean build/boot-smoke (areas 1, 33).

