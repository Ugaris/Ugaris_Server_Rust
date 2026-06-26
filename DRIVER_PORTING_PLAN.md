# Driver Porting Plan

This note maps the legacy driver registry/module dispatch to a Rust architecture. It is intentionally isolated from the current Rust implementation and does not require changes to `world.rs` or `item_driver.rs`.

## Legacy Dispatch Shape

- `src/system/libload.c` loads every `.dll`/`.so` from `runtime/` and `runtime/<areaID>/`, resolves an exported `driver` symbol, and probes loaded libraries until one returns non-zero.
- Character dispatch is `char_driver(nr, type, cn, ret, last_action)`. Driver `0` is reserved for the player driver when `type == CDT_DRIVER`.
- Item dispatch is `item_driver(nr, in, cn)`. Driver `0` means `look_item`, and drivers `>= 1000` are identity/timer tags, not normal runtime handlers.
- Both character and item dispatch use `fast_chdrv[nr]` and `fast_itdrv[nr]` caches after the first successful library probe.
- Area guards are currently hard-coded in `item_driver` before module dispatch for `IDR_BONEBRIDGE`, `IDR_BONEHINT`, `IDR_NOMADDICE`, `IDR_STAFFER2`, `IDR_OXYPOTION`, `IDR_LIZARDFLOWER`, `IDR_CALIGAR`, and `IDR_ARKHATA`.
- Every module exports the same `driver(int type, int nr, int obj, int ret, int lastact)` multiplexer, usually forwarding `CDT_DRIVER`, `CDT_ITEM`, `CDT_DEAD`, and `CDT_RESPAWN` to local switches.
- `src/module/base.c` owns the most important generic base item drivers plus `CDR_MACRO`, `CDR_TRADER`, and `CDR_JANITOR`.

## Current Rust State

- `crates/ugaris-core/src/item_driver.rs` already models `use_item` ordering and an outcome-oriented `execute_item_driver` boundary.
- Implemented item coverage includes `IDR_POTION`, `IDR_FOOD` simple paths, `IDR_DOOR` core open/close toggles, `IDR_TELEPORT` core checks/target decoding, `IDR_RECALL` core checks, driver `0` look handling, and typed unsupported outcomes.
- `crates/ugaris-core/src/world.rs` applies same-area teleport/recall effects after `ItemDriverOutcome`, which is the right boundary for map mutation.
- `crates/ugaris-core/src/player.rs` already ports the player-driver action setter surface, queue behavior, and driver stop/halt primitives.
- `crates/ugaris-core/src/drvlib.rs` currently only contains distance helpers from `src/system/drvlib.c`.
- There is no Rust character-driver registry yet, and `Character` does not currently carry a message queue or typed per-driver state equivalent to legacy `set_data`/`DRD_*`.

## Recommended Rust Architecture

Use a static typed registry rather than dynamic library loading. The legacy dynamic probing solved hot-loading and area module discovery; the Rust rewrite benefits more from compile-time coverage, explicit dependencies, and testable outcomes.

Suggested modules:

- `driver_ids`: constants or enums for `CDR_*`, `IDR_*`, `CDT_*`, and identity driver ranges. Keep exact numeric compatibility tests against `drvlib.h`.
- `driver_registry`: maps driver IDs to handlers and preserves legacy dispatch semantics, including driver `0`, `>=1000` identity tags, unsupported logging, and area guards.
- `character_driver`: owns `CharacterDriverKind`, `CharacterDriverContext`, `CharacterDriverOutcome`, normal tick dispatch, death dispatch, and respawn dispatch.
- `item_driver`: keep behavior functions small and pure where possible. They should return typed outcomes for world mutation, item creation, container/depot access, logging, timer scheduling, and cross-area transfer.
- `driver_state`: typed per-character/per-player state replacing `set_data(cn, DRD_*, size)`. Start with enum-backed states for simple baddy, chest/orb cooldown PPD, merchant/bank memory timers, and transport seen flags.
- `driver_effects`: shared effect vocabulary used by both item and character drivers, for example `Log`, `AreaLog`, `ScheduleItem`, `CreateItemTemplate`, `DestroyItem`, `ReplaceCarriedItem`, `OpenDoor`, `SetMapFlags`, `TeleportSameArea`, `ChangeArea`, `QueuePlayerAction`, and `CombatIntent`.

Dispatch policy:

- Resolve IDs once through static match tables. Do not emulate dynamic `.so` probing unless hot-loading becomes a real requirement.
- Keep C return-code compatibility at the registry edge: `Handled`, `NotHandled`, and `RetryLater` can map to legacy `1`, `0`, and door-style `2` semantics.
- Keep world mutation in `World` or an effect applier. Individual drivers should not need broad mutable access to all characters, items, map, timers, and database at once.
- Represent legacy timer callbacks by scheduling typed events such as `TimerEvent::ItemDriver { driver, item_id }` instead of storing function pointers.
- Treat area-specific modules as ordinary registry providers gated by `area_id`, not as dynamically loaded libraries.

## Porting Dependencies

- Remaining door, torch, nightlight, and trap driver details depend on map flags, item light mutation, LOS/light invalidation, timer scheduling, and sound/log effects.
- Chest, randchest, orbspawn, and account depot depend on item-template creation, cursor/inventory mutation, player persistent data, and database/container policy.
- Stat scrolls and beyond potions depend on value-raising, experience spending, stat caps, and item requirement recomputation.
- Enchant/anti-enchant/orb drivers depend on modifier rules, wearable flags, requirement recomputation, and item inspection output.
- `CDR_SIMPLEBADDY` depends on message queues, fight driver helpers, pathing, spells, poison, regeneration, and item use.
- `CDR_MERCHANT` and `CDR_BANK` depend on text parsing, NPC memory, storage/depot/database systems, and transaction rules.

## Prioritized Drivers To Port Next

1. Finish door family details: keyed doors/keyring checks, auto-close timers, extended/multi-tile foreground shifts, and `IDR_DOUBLE_DOOR` pairing.
2. `IDR_TELE_DOOR`: same navigation impact as doors, but simpler than full door state. Builds on same-area teleport outcomes and player stop/update behavior.
3. `IDR_CHEST` and `IDR_RANDCHEST`: high player-facing reward impact. Requires a reusable `CreateItemTemplate` plus cursor placement and per-player cooldown state.
4. `IDR_ACCOUNT_DEPOT`: current Rust dispatch recognizes it but returns unsupported. Porting it unlocks a major storage path and validates database-backed item containers.
5. `IDR_TORCH`, `IDR_NIGHTLIGHT`, and `IDR_TOYLIGHT`: important for visibility and already referenced by player movement/area logic. These should follow after timer and light effects are in place.
6. `IDR_STATSCROLL`: self-contained once value raising/experience spending is available. It has high progression impact and is easier to test than combat AI.
7. `IDR_CITY_RECALL`: extends existing recall/teleport work with fixed destination decoding, stack count handling, and cross-area transfer outcomes.
8. `IDR_ENCHANTITEM`, `IDR_ANTIENCHANTITEM`, `IDR_SPECIALANTIENCHANTITEM`, `IDR_ORBSPAWN`, and `IDR_ANTIORBSPAWN`: high economy/progression impact, but should wait for robust modifier and item-template support.
9. `IDR_ASSEMBLE`: quest-critical but mostly item-ID/template mapping. Port after item ID constants and item template creation are stable.
10. `IDR_BALLTRAP`, `IDR_USETRAP`, `IDR_STEPTRAP`, `IDR_SPIKETRAP`, and `IDR_FLAMETHROW`: useful once projectile/effect/combat hooks exist.
11. `CDR_SIMPLEBADDY`: first character driver to port after primitive combat/fight-driver helpers and message queues. It unlocks many area-specific NPCs that delegate to it.
12. `CDR_MERCHANT` and `CDR_BANK`: high social/economy impact, but require text command parsing, database/storage, transaction logging, and NPC memory support.
13. `CDR_TRADER`: depends on robust give/take/cursor semantics and player-to-player transaction guarantees.
14. `CDR_MACRO` and `CDR_JANITOR`: useful operationally, but lower impact for core gameplay than doors, rewards, storage, and basic enemies.

## First Implementation Slice

The lowest-risk slice is a registry scaffold plus `IDR_DOOR` outcomes:

- Add driver ID constants with tests for the dispatch-critical IDs.
- Add a registry function that forwards existing implemented item drivers and returns `Unsupported` for all others.
- Add door-specific outcomes instead of mutating the world inside the item driver.
- Apply door outcomes in `World` in one place: flags, sprite, light/LOS invalidation markers, sounds/logs, and timer scheduling.
- Test closed-to-open, open-to-closed, blocked auto-close retry, locked door key rejection, and no-auto-close behavior.

This slice preserves the current Rust item-driver boundary and avoids entangling character AI before map/item/timer effects are solid.
