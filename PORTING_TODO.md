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
- [ ] **Retire legacy blob writes** - after a few clean iterations with
  `player_state_json` (migration 0020): stop populating
  `ppd_blob`/`subscriber_blob` in the three `snapshots.rs` builders, add a
  backfill migration converting remaining blob-only rows through the
  legacy decoders, then mark the decoders `#[deprecated]`. Keep the raw
  `PlayerRuntime::ppd_blob` field (it preserves unknown legacy blocks
  inside the JSON document).
- [ ] **`military.rs` (3.2K) split** - `world/npc/area32/military.rs`
  holds two NPCs plus shared mission logic; split into
  `military_master.rs`, `military_advisor.rs`, and `missions.rs`.

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
- [ ] **Area 2 - `src/area/2/area2.c`** - remaining character drivers
  (zombie lord, priests). Item drivers done.
- [ ] **Area 3 - `src/area/3/area3.c`** - palace story NPCs, lamp ghost
  quest flow (lamps themselves are ported).
- [ ] **Area 4 - `src/area/4/pents.c`** - pentagram quest NPCs + demon
  wave logic beyond the ported item boundary. Also wire the achievement
  calls this file's reward mechanic makes in C (`achievement_add_pents`,
  `achievement_award(FIVE_IN_A_ROW/HAPPY_GO_LUCKY/FAVORED_BY_FORTUNE/
  DEMON_LORDS_DEMISE)`) using the existing `award_*` helper pattern in
  `crates/ugaris-server/src/achievement.rs` (Achievements task, closed
  iteration 84).
- [ ] **Area 6 - `src/area/6/edemon.c`** - Earth Demon boss driver
  (`CDR_EDEMON*` characters); machinery items are ported.
- [ ] **Area 8 - `src/area/8/fdemon.c`** - Fire Demon boss + farm NPCs;
  cannon/loader items are ported.
- [ ] **Area 10 - `src/area/10/ice.c`** - ice NPCs, ice demon curse
  integration (curse spell side is ported).
- [ ] **Area 11 - `src/area/11/palace.c`** - palace guards, Islena fight
  driver (door/bomb/cap items ported).
- [ ] **Area 12 - `src/area/12/mine.c`** - keyholder golems, miners. Also
  wire `achievement_add_silver_mined`/`_gold_mined` from the
  `handle_mining_result` reward cascade using the existing `award_*`
  helper pattern in `crates/ugaris-server/src/achievement.rs`
  (Achievements task, closed iteration 84).
- [ ] **Area 13 - `src/area/13/dungeon.c` + `dungeon_tab.c`** - dungeon
  master/fighter drivers, clan jewel raid protocol.
- [ ] **Area 14 - `src/area/14/random.c`** - remaining shrine effects
  (indecisiveness/bribes/welding) + questlog resend after shrines.
- [ ] **Area 15 - `src/area/15/swamp.c`** - Clara dialogue runtime (state
  helpers exist), military reward application.
- [ ] **Area 16 - `src/area/16/forest.c`** - forest NPCs/robber quest.
- [ ] **Area 17 - `src/area/17/two.c`** - Two-City thief/skeleton NPC
  drivers (`CDR_TWOSKELLY` has state scaffolding).
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

