# Rust Rewrite Porting Ledger

The C source inventory is tracked with:

```bash
git ls-files "src/**/*.c" "src/**/*.h"
```

Current tracked C/C header file count: 302.

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
| `src/system/do.c` `use_item` dispatch boundary, `src/module/base.c` `potion_driver` non-template path, simple `food_driver` path, `teleport_driver` core path, `recall_driver` core path, `door_driver` core path, `teleport_door_driver` core path, and normal `chest_driver` slices | `crates/ugaris-core/src/item_driver.rs`, `crates/ugaris-core/src/world.rs`, `crates/ugaris-core/src/player.rs`, `crates/ugaris-server/src/main.rs` | Legacy account-depot-first, container open, depot open, item-driver dispatch ordering, `IDR_POTION` resource restoration/caps/consumption for potions that do not require empty-bottle template creation, typed deferred outcome for empty-bottle template creation, `IDR_FOOD` simple food consumption for kinds 0/1, `IDR_TELEPORT` drdata decoding/access checks/same-area drop-char-extended teleport, `IDR_RECALL` level/arena/dying checks plus same-area recall/scroll consumption, `IDR_DOOR` core open/close state with item flags, tile flags, sprite toggles, closed-state flag storage, and blocked-doorway close refusal, `IDR_TELE_DOOR` direction/level checks plus exact opposite-side same-area teleport, and `IDR_CHEST` treasure-index decoding, `treasure_N` template instantiation to cursor, per-player last-access runtime state, `drdata[5..7]` hour cooldowns, exact-key and skeleton-key chest access via cursor/inventory slots 30+, runtime keyring exact-key chest checks, death-gated access via `drdata[7]`, key-use feedback, and legacy system feedback text ported with tests. Account depot loading, container permission model, empty-potion item template creation, special food/event handling, cross-area transfer execution, keyring item/command management and door checks, chest achievements/persistent PPD storage, random chest loot tables, door auto-close timers, extended/multi-tile door foreground shifts, double-door pairing, tele-door clan-area messaging/guards, Brannington transport PPD gate, teleport/recall logging, and remaining concrete item driver effects remain. |
| `src/server.h` | `crates/ugaris-core/src/entity.rs`, `crates/ugaris-core/src/effect.rs`, `crates/ugaris-core/src/legacy.rs`, `crates/ugaris-core/src/map.rs` | Core flags, values, map/item/character/effect shapes including character sprite, item template ID, death counter, inventory ranges, version, tick constants ported. |
| `src/system/act.h` | `crates/ugaris-core/src/legacy.rs` | Action IDs ported. |
| `src/system/questlog.h` | `crates/ugaris-core/src/quest.rs` | Quest IDs, flags, and fixed-size quest log behavior ported with tests. |
| `src/system/io.c` / `src/system/io.h` | `crates/ugaris-protocol/src/frame.rs`, `crates/ugaris-net/src/*`, `crates/ugaris-server/src/main.rs` | Legacy tick frame envelope, TCP session skeleton, per-session server command channels, runtime-to-session framed payload sending, listener readiness/error reporting, default info logging, IPv4 plus IPv6 localhost listening for `localhost`, multi-payload login bootstrap queueing, and chunked full-map bootstrap below legacy frame limits ported. Full gameplay send buffering, compression modes, and backpressure policy remain partial. |
| `src/system/map.h` / `src/system/map.c` primitives | `crates/ugaris-core/src/map.rs`, `crates/ugaris-core/src/item_ops.rs` | Legacy map indexing, bounds checks, movement/sight blocker helpers, grid wrapper, item map placement/removal, character map placement/removal, `NOMAGIC` flag sync, simple 3x3 drop order, extended pathfinder-backed drop order, carried-item removal, and carried-item replacement ported with tests. Light/expire/trap/notify callbacks remain. |
| `src/system/los.h` / `src/system/los.c` primitive LOS | `crates/ugaris-core/src/map.rs` | Conservative line-of-sight helper with blocker tests ported. Full per-character cached LOS table remains. |
| `src/system/path.h` / `src/system/path.c` | `crates/ugaris-core/src/path.rs` | A* path shape, legacy heuristic, first-direction result, node cap, 2/3 movement costs, inner-bound successor checks, and `path_ignore_char`-style character-ignoring movement mode ported with tests. Other custom target callbacks and exact global-state compatibility remain. |
| `src/system/sector.h` / `src/system/sector.c` | `crates/ugaris-core/src/sector.rs` | Dirty sector tick tracking, `skipx_sector`, 8x8 character sector head/links, sound sector flood fill, shout sector flood fill, temporary sound door traversal, and hearing checks ported with tests. Runtime character/map integration remains. |
| `src/system/see.h` / `src/system/see.c` | `crates/ugaris-core/src/see.rs`, `crates/ugaris-core/src/entity.rs` | Character and item visibility rules ported with tests: invisible checks, light/daylight blending, infrared/infravision boosts, light/dark profession boosts, stealth/perception scoring, carried item hiding, takeable item light requirement, and front-wall item side checks. Uses current conservative Rust LOS helper. |
| `src/system/date.h` / `src/system/date.c` | `crates/ugaris-core/src/game_time.rs` | Game date units, moon phases, solstice/equinox flags, sunrise/sunset, area light overrides, and daylight calculation ported with tests. |
| `src/system/do.h` / `src/system/do.c` primitive actions, `src/system/act.c` primitive completions | `crates/ugaris-core/src/do_action.rs`, `crates/ugaris-core/src/entity.rs`, `crates/ugaris-core/src/world.rs` | Error codes, duration constants, `speed`, `end_cost`, `do_idle`, deterministic `do_walk` movement checks/reservation, terrain/underwater movement costs, fast-mode endurance cost, timed `do_take`/`do_use`/`do_drop` setup, `act_walk`, `act_take`, `act_drop`, `act_use` validation to typed item-driver request, action step readiness/reset, world-backed action completion helpers, minimal world action tick dispatcher, and `turn` ported with tests. Weather/effect movement modifiers, full central action dispatcher, actual use-item driver effects, combat, and spells remain. |
| `src/system/drvlib.c` distance helpers | `crates/ugaris-core/src/drvlib.rs`, `crates/ugaris-core/src/entity.rs` | `map_dist`, `char_dist`, `tile_char_dist`, and `step_char_dist` including `tox`/`toy` target-position handling ported with tests. Broader driver library remains. |
| `src/system/act.c` / `src/system/do.c` pure attack formulas | `crates/ugaris-core/src/attack.rs` | Attack chance breakpoint table, strict roll comparison, attack/parry skill math, side/back/assassin bonuses, direct melee damage units, and armor/lifeshield reduction helpers ported with tests. Full `PAC_KILL`, `do_attack`, `act_attack`, `can_attack`, and death/hurt integration remain. |
| `src/system/do.c` / `src/system/act.c` pure spell formulas | `crates/ugaris-core/src/spell.rs` | Spell constants, effect IDs, spell item driver IDs, spellpower/duration math, variable mana spend formulas, speed modifier formulas, and combat spell helper formulas ported with tests. Queue execution, spell setup/completion, spell item/effect lifecycle, and world integration remain. |
| `src/system/timer.h` / `src/system/timer.c` | `crates/ugaris-core/src/scheduler.rs` | Sorted tick timer queue and due-event drain behavior ported with tests. Function-pointer callbacks represented as named timer events. |
| `src/system/task_scheduler.h` / `src/system/task_scheduler.c` | `crates/ugaris-core/src/scheduler.rs`, `crates/ugaris-server/src/main.rs` | 128-task periodic scheduler, seconds-to-ticks conversion, due-task detection, and server tick-loop integration ported. |
| `src/system/talk.h` / `src/system/talk.c` low-level logging | `crates/ugaris-core/src/log_text.rs`, `crates/ugaris-protocol/src/packet.rs` | `LOG_*` constants, text sanitization, scrollback behavior, raw color-marker preservation, `say`/`shout`/`holler`/`emote`/`whisper` message formats, and `SV_TEXT` little-endian length layout ported with tests. Area broadcast/hearing sectors remain. |
| `src/system/player.c` `log_player` | `crates/ugaris-protocol/src/packet.rs`, `crates/ugaris-core/src/log_text.rs` | Text packet framing and scrollback rules ported with tests. |
| `src/system/player.c` login block and initial client sync scaffold | `crates/ugaris-protocol/src/login.rs`, `crates/ugaris-net/src/session.rs`, `crates/ugaris-server/src/main.rs` | Login block size, endian layout, vendor protocol version, password obfuscation, runtime character-id assignment, scaffolded in-world player spawn/despawn, temporary `new_warrior_m` player-template instantiation with starter equipment/items, runtime login/bootstrap response (`SV_LOGINDONE`, `SV_TICKER`, `SV_MIRROR`, `SV_PROTOCOL`, `SV_ORIGIN`, full visible diamond `SV_MAP11`, visible character `SV_MAP10`, `SV_SETVAL*`, resources, exp, gold, cursor item, initial equipment/inventory `SV_SETITEM`, `SV_TEXT`), C-mapped `SV_SCROLL_*` plus origin, character clear/update, and newly visible diamond fringe tile/character packets for one-tile walk completions, coarse full refresh for non-walk completed actions, and coarse post-action cursor/inventory snapshot sync ported with tests. Server smoke-tested listening without DB. Authentication, PostgreSQL character load/selection, real spawn coordinates, true inventory delta cache, full cached map diff parity, and full player state machine still partial. |
| `src/system/player_driver.h` / `src/system/player_driver.c` action setters and primitive runtime bridge | `crates/ugaris-core/src/player.rs`, `crates/ugaris-core/src/world.rs`, `crates/ugaris-server/src/main.rs` | `player_driver_stop`, `halt`, direct action setters, serial-preserving item/character actions, teleport, spell queue insertion/last-slot overwrite behavior, server-side use of driver setters for direct/spell client actions, and primitive tick-loop setup/completion for idle, walk-dir including diagonal wall-slide fallback, `PAC_MOVE`, adjacent/path-to-item take, adjacent/path-to-target drop, adjacent/path-to-item use including front-wall pathing, `PAC_TELEPORT` as facing item-use with legacy `spec = teleport + 1`, immediate `PAC_LOOK_MAP` turn/LOS/request handling plus server `SV_TEXT` feedback for hidden targets, coordinate fallback, and rest/clan/arena/peace flags, and `PAC_GIVE` adjacent/path-to-recipient setup plus `AC_GIVE` cursor-item transfer ported with tests. Queued spell priority execution, actual item use effects beyond potion, combat, full area section-name/level rendering for look output, serial validation, wall-use/door interaction during movement, and action error side effects remain. |
| Zone template/map parser scaffolding from `src/system/create.c` / `src/system/map.c` | `crates/ugaris-core/src/zone.rs` | Legacy token parsing, `.itm`/`.chr` template record parsing including item `ID`, `.map` directive parsing with origin offsets, live item template ID retention, and tiny sample application into `World` ported with tests. Production zone validation, startup integration, full character template fields, item-driver creation side effects, and respawn/random-loot behavior remain. |
| Area terrain startup loading from `ugaris_data/zones/<area>/*.map` | `crates/ugaris-server/src/main.rs`, `crates/ugaris-core/src/zone.rs` | Server startup resolves `UGARIS_ZONE_ROOT` or default `ugaris_data/zones` / `../ugaris_data/zones`, loads generic and area `.itm`/`.chr` templates best-effort, loads the first area `.map`, accepts signed legacy sprite IDs, tolerates missing item/character templates while preserving terrain, sanitizes `from/to` range copies so live item/character IDs and temporary item blockers are not duplicated across terrain ranges, reports load counts, keeps the loader alive for runtime template instantiation, and chooses an open spawn tile. Area 1 `above1.map` smoke-tested: 65,533 ground tiles, 16,969 blocked tiles, 1,780 item templates, 188 character templates, 2,236 placed items, and 446 placed characters. Process-level legacy login smoke confirmed map bootstrap payloads after loading real area data. Full `.pre` expansion/generator parity, complete template metadata, respawn/random loot, and all object driver side effects remain. |

## Partial

| C File | Rust Location | Remaining Work |
|---|---|---|
| `src/system/database/database.h` | `crates/ugaris-db/src/lib.rs` | PostgreSQL pool, module boundary, and database handle ported. |
| `src/system/database/database_area.h` | `crates/ugaris-db/src/area.rs`, `migrations/0001_core_accounts_characters.sql` | Area server records and alive/down/get operations scaffolded. |
| `src/system/database/database_character.h` / `database_character.c` login and snapshot paths | `crates/ugaris-db/src/character.rs`, `migrations/0001_core_accounts_characters.sql`, `migrations/0002_sessions_questlog_anticheat.sql`, `migrations/0003_character_snapshots.sql` | Login status semantics, character target lookup, current-area update, login session insert, release semantics, guarded backup save, logout save, Rust character JSON snapshot load/save, and character item snapshot rows scaffolded for PostgreSQL. Password hash verification, live DB migration verification, and full legacy binary blob decode/encode remain. |
| `src/system/database/database_anticheat.h` | `migrations/0002_sessions_questlog_anticheat.sql` | Session/event schema scaffolded; repository methods remain. |
| `src/system/player.c` / `src/system/player.h` | `crates/ugaris-net`, `crates/ugaris-core`, `crates/ugaris-server` | Player states, `PAC_*`, command recognition, command payload parsing, login parse, runtime registry, session send channels, scaffold character spawn, direct action setters, action queue, and primitive world action bridge exist; full action execution, map cache/client sync, inventory delta sync, text logging, transfer, anti-cheat integration remain. |
| `src/system/libload.c`, `src/system/drvlib.h`, module driver switch mapping | `DRIVER_PORTING_PLAN.md` | Static Rust driver registry architecture and prioritized character/item driver port order documented. Implementation remains. |

## Continuation Handoff

Use this section as the starting point for the next session.

### Current Runnable State

- Rust workspace lives under `rust_server/` and is still untracked by git.
- The legacy C server remains untouched; current rewrite is isolated to `rust_server/`.
- The Rust server loads real area 1 data from `../ugaris_data/zones/1/above1.map` plus generic/area templates.
- Latest real area 1 smoke counts: 65,533 ground tiles, 16,969 blocked tiles, 1,780 item templates, 188 character templates, 2,236 placed items, and 446 placed characters.
- Default login scaffold uses `generic/player.chr` template `new_warrior_m`, then renames the character to the login name and sends starter equipment/inventory to the client.
- Known unrelated dirty/untracked files outside the rewrite: `.gitignore` and `scripts/extract_npc_lore.py`; do not modify/revert them unless explicitly asked.

### How To Run

Build and run the Rust area server:

```bash
cd rust_server
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

Run from `rust_server/`:

```bash
cargo fmt --all
cargo test --workspace
cargo build -p ugaris-server
```

Last verified after the first random-chest slice:

- `cargo test --workspace`: passed.
- `cargo build -p ugaris-server`: passed.
- Process-level area 1 login smoke: passed and received bootstrap payload bytes.

### Recent Client-Facing Fixes

- NPC duplication was mostly caused by `.map` `from/to` range copies duplicating live `item`/`character` tile IDs. `ZoneLoader` now sanitizes range copies so terrain is copied without live object IDs or temporary object blockers.
- Movement stutter was caused by sending a full visible diamond after every completed walk. One-tile walks now send `SV_SCROLL_*`, `SV_ORIGIN`, old-position character clear, center character update, and newly visible fringe tile/character packets.
- Full visible-diamond refresh is still used for non-walk actions until a proper cached map-diff system is ported.
- `PAC_LOOK_MAP` no longer drops pending output in the server loop. It now sends legacy `SV_TEXT` feedback for hidden targets (`Too far away or hidden.`), coordinate fallback for visible targets while area sections remain unported, and the exact rest/clan/arena/peace zone flag messages.

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

Chest gaps still to port:

- Keyring commands, remove/show text integration, registered-key validation, auto-add pickup integration, and persistent `DRD_KEYRING_PPD` load/save.
- Door keyring checks.
- Achievement persistence/protocol sending beyond runtime marker updates.
- Exact persistent PPD storage behavior for chest access across logout/server restart.
- `IDR_RANDCHEST = 34` persistent `DRD_RANDCHEST_PPD`, exact RNG parity, and full live-data smoke coverage.

Recommended next chest steps:

1. Port keyring item driver/commands/auto-add and persistent `DRD_KEYRING_PPD` load/save.
2. Add persistent PPD load/save for treasure chest last-access state so cooldowns survive logout/server restart.
3. Port door keyring checks using the same runtime/persistent keyring state.
4. Persist/runtime-load chest achievement state and send achievement protocol updates.
5. Persist/runtime-load `IDR_RANDCHEST` daily access state and verify full loot table behavior against live data.

### Other High-Value Next Steps

- Implement a real per-session map cache and diff output matching `player.c` instead of the current coarse refresh paths.
- Port full area section-name/level rendering for `PAC_LOOK_MAP` output.
- Integrate PostgreSQL-backed login/character selection and logout save instead of the temporary `new_warrior_m` scaffold.
- Port combat action execution: `PAC_KILL`, `do_attack`, `act_attack`, hurt/death integration.
- Port spell queue execution and spell setup/completion.
- Continue door family details: keyed doors/keyring checks, auto-close timers, extended/multi-tile door foreground shifts, and `IDR_DOUBLE_DOOR` pairing.

## Pending Priority Order

1. Stabilize client-visible interaction loop: cached map diffs, action feedback text, chest cooldown/messages, look output.
2. Continue `src/system/player.c`, `src/system/player_driver.c`, `src/system/do.c`, `src/system/act.c` behavior beyond the primitive actions already ported.
3. Continue `src/system/map.c`, `src/system/los.c`, `src/system/path.c` parity for light, traps, notify callbacks, and exact LOS cache behavior.
4. Port `src/system/database/*.c` to PostgreSQL repositories and migrations, especially real login/load/save.
5. Port `src/system/game/*.c`, `src/system/skill.c`, `src/system/effect.c`, `src/system/death.c`, `src/system/respawn.c`.
6. Port `src/system/drvlib.c`, `src/system/libload.c`, all `src/module/**` drivers using the static Rust registry plan.
7. Port area modules under `src/area/**`.
8. Port chat, auction, anti-cheat, weather, event, command/admin systems.
