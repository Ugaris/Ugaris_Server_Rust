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
- New persistent player state must round-trip through the legacy PPD blob
  codecs in `crates/ugaris-core/src/player.rs` (see `DRD_*_PPD` examples).
- If a task is too big for one sitting, port a self-contained slice, test it,
  mark the checkbox as `- [~]` (in progress) with a note about what remains.

### Where Things Live (quick map)

| Concern | Location |
|---|---|
| Item drivers (use/timer behavior) | `crates/ugaris-core/src/item_driver/<domain>.rs` |
| World mutation, actions, combat, NPC AI | `crates/ugaris-core/src/world/<system>.rs` |
| Character drivers (NPC brains) | `crates/ugaris-core/src/character_driver.rs` (+ `world/npc_*.rs` runtime) |
| Player session state, PPD codecs | `crates/ugaris-core/src/player.rs` |
| Wire packets | `crates/ugaris-protocol/src/packet.rs`, `command.rs` |
| Server loop, client sync, text commands | `crates/ugaris-server/src/<concern>.rs` |
| DB repositories | `crates/ugaris-db/src/*.rs` |
| Legacy client (for checking what the client expects) | `../astonia_community_client/src/client/protocol.c` |

---

## P0 - Playability Blockers

These make the game actually playable solo on area 1. Do these first, in
order.

- [x] **Regeneration tick** - characters never recover HP/endurance/mana.
  - C: `regenerate()` in `src/system/act.c:2101` (not `tool.c` - corrected
    during implementation) and the HP/endurance/mana idle regen in
    `act_idle()` (`act.c:99`), both called once per tick per character from
    `tick_char()`. Ported the exact formulas: skill-gated endurance/lifeshield
    regen throttled per real second via `last_regen`, idle HP/endurance/mana
    regen gated by `regen_ticker + regen_time`, `MF_NOREGEN`/`CF_PLAYER`/
    area-33 special cases, and the warcry-without-magicshield lifeshield leak.
  - Rust: `crates/ugaris-core/src/world/regen.rs` (`World::regenerate_characters`,
    called from `main.rs` each tick after `world.advance()`). Also added the
    C `act()` `regen_ticker` non-idle-action stamp to
    `tick_basic_actions_with_attack_policy` (`world/actions.rs`), and a new
    `Character.last_regen` field (mirrors C `ch.last_regen`, distinct from
    `regen_ticker`).
  - Tests: `world/tests/regen.rs` (13 tests) + one in `world/tests/actions.rs`
    for the `regen_ticker` stamp.
  - REMAINING: `reduce_rage`/`increase_rage` not ported (`rage` field does not
    exist on `Character` yet); `NT_CHAR` notify-area emission at the end of
    `act_idle` deferred to the "NPC sighting messages" task below;
    `check_endurance` fast-mode revert deferred to the "Speed mode" task
    below. Idle regen runs continuously per real tick instead of C's
    `act1`-scaled batch (Rust's tick loop skips `action == 0` characters
    entirely, so there's no per-batch idle-completion event to hook into);
    the steady-state rate matches C, only the batching granularity differs -
    see the `regen.rs` module doc comment.

- [x] **Skill raising (`CL_RAISE`)** - parsed but ignored; players cannot
  spend experience.
  - C: `cl_raise` in `src/system/player.c` -> `raise_value` in
    `src/system/skill.c` (not `raise_value_exp` - that one is scroll/shrine
    only and also grants `exp`, `cl_raise` only spends already-unspent exp
    via `exp_used` vs `exp`). Ported a new `raise_value` helper alongside
    the existing scroll helpers in
    `crates/ugaris-core/src/item_driver/scrolls.rs`, reusing `raise_cost`,
    `skillmax`, `bare_value`, `skill_raise_cost_factor` - no duplicated
    math.
  - Rust: `World::raise_skill` (`crates/ugaris-core/src/world/skills.rs`)
    wraps the helper and returns a `RaiseSkillOutcome`; `main.rs` handles
    `ClientAction::Raise { value }` by calling it and, on success, sending a
    small packet with `SV_SETVAL0/1` for the raised value plus `exp`/
    `exp_used` (mirrors the fields `login.rs` sends, but only for the one
    changed value instead of a full 43-value dump). C's `cl_raise` sends no
    feedback packet at all on failure, so the Rust handler stays silent on
    `RaiseSkillOutcome::Blocked` too.
  - Tests: `crates/ugaris-core/src/world/tests/skills.rs` (9 tests) cover
    success (bare/effective bump, exp_used spent, exp untouched, effective
    never lowered to match bare), and blocked cases: insufficient unspent
    exp, `CF_NOEXP`, skill not present (bare value 0), at `skillmax`,
    unraisable skill (`cost == 0`, e.g. Armor), out-of-range value index,
    unknown character.
  - REMAINING: no `update_char` recompute, level-up, or achievement checks
    fire on raise (matches the pre-existing gap already noted in the ledger
    for the scroll path - those are separate unported P1 tasks). No
    dedicated `ugaris-server`-crate test exists for the `main.rs` match arm
    itself (same precedent as other simple inline actions like
    `GetQuestLog`/`ReopenQuest`, which also have no server-crate tests) -
    the packet assembly only reuses already-tested `PacketBuilder` methods.

- [x] **Speed mode (`CL_SPEED`) and fight mode (`CL_FIGHTMODE`)** - both
  parsed, both ignored.
  - C: `cl_speed` in `src/system/player.c` validates the mode byte against
    `SM_NORMAL`/`SM_FAST`/`SM_STEALTH`, gates `SM_FAST` on
    `endurance >= POWERSCALE`, then sets `ch[cn].speed_mode` (no feedback
    packet either way). `cl_fightmode` is a genuine no-op stub
    (`return;` - `ch[cn].fight_mode` is otherwise unused anywhere in the C
    tree), confirmed by reading the full function body and grepping for
    other `fight_mode` references. Also ported the sibling `check_endurance()`
    (act.c:1838), called unconditionally right before `regenerate()` in the
    same `tick_char()` loop: reverts `SM_FAST` to `SM_NORMAL` and logs
    "You're exhausted." once endurance drops below `POWERSCALE`.
  - Rust: `World::set_speed_mode` (`crates/ugaris-core/src/world/speed.rs`)
    plus `SpeedMode::from_client_mode` (`entity.rs`); wired
    `ClientAction::Speed`/`ClientAction::FightMode` in
    `crates/ugaris-server/src/main.rs` (fight mode is an explicit
    documented no-op match arm). `check_endurance` added to
    `World::regenerate_characters` in `crates/ugaris-core/src/world/regen.rs`
    (runs before the position-gated regen/idle-regen logic, matching C
    ordering), using the existing `queue_system_text`/`drain_pending_system_texts`
    plumbing (already consumed by `send_pending_world_system_texts` in
    `ugaris-server/src/world_events.rs`).
  - Tests: `world/tests/speed.rs` (6 tests: normal/stealth always succeed,
    fast requires endurance >= POWERSCALE exactly, invalid mode byte
    ignored, unknown character ignored) and 4 new tests in
    `world/tests/regen.rs` for `check_endurance` (revert + message below
    POWERSCALE, no revert at exactly POWERSCALE, non-fast modes untouched,
    runs even outside map bounds). Fixed a pre-existing test
    (`regenerate_endurance_blocked_in_fast_speed_mode`) that relied on
    endurance=0 while asserting fast-mode regen block - that combination
    now also triggers `check_endurance`'s revert per real C behavior, so
    the fixture was changed to hold endurance at exactly `POWERSCALE` to
    isolate the regen-block behavior it actually tests.
  - REMAINING: nothing - task fully done as scoped (fight mode has no C
    behavior to port).

- [x] **Player death saves** - `die_character` never consults `saves`.
  - C: `god_save_char` in `src/system/death.c:851` is called from inside
    `hurt()` (`death.c:1262`), not from `die_char`/`kill_char` - it runs
    immediately at the fatal blow, before `kill_char` ever schedules the
    `AC_DIE` animation, and takes priority-checked position right after the
    PK-death branch (`cc && CF_PLAYER(cn) && CF_PLAYER(cc)` - PK kills never
    get a save). Ported exactly: decrement `saves` then cap at 10 (odd but
    literal C order), `got_saved++`, `hp = 1*POWERSCALE`,
    `remove_all_poison`, `extinguish` (burn effects only), the two Ishtar
    log lines, `transfer_to_restarea` (same-area case). Also found and
    ported the C death-sound gating detail: the "killed with"
    sound/threshold check plays for NODEATH *and* the saves branch too, not
    only an actual kill (`death.c:1204-1229`, before the NODEATH/save/kill
    branch split) - `apply_legacy_hurt`'s death sound condition now includes
    `god_saved`.
  - Rust: new `World::god_save_character` in
    `crates/ugaris-core/src/world/death.rs`, called from
    `World::apply_legacy_hurt` (`world/hurt.rs`) at the same decision point
    as C - added a `cause_is_player` precheck and a new
    `LegacyHurtOutcome::god_saved` flag so a saved player never gets
    `CF_DEAD`/`deaths++`/the death-animation timer at all (matches C: the
    entire `die_char` body/item/exp-loss path never runs for a god save).
    Moved `legacy_save_number` (formerly server-crate-only in
    `area_apply.rs`) into `world::death` as a public `legacy_save_number`
    so both the shrine-security path and the new save message can share it.
  - Tests: `world/tests/hurt.rs` - save consumed + `got_saved` incremented +
    exp/items/position untouched except rest-teleport + feedback text
    (`legacy_hurt_god_saves_player_with_unspent_saves_instead_of_killing`),
    poison/burn removal
    (`legacy_hurt_god_save_removes_poison_and_burn_effects`), the saves>10
    decrement-then-cap quirk (`legacy_hurt_caps_saves_at_ten_after_decrement`),
    PK death ignoring saves
    (`legacy_hurt_pk_death_ignores_saves_and_kills_normally`), and
    saves=0 falling through to a normal kill
    (`legacy_hurt_no_saves_left_kills_normally`).
  - REMAINING: the other `hurt()` special-case death branches sharing the
    same C `else` chain (`shutdown_save_char`, `area_save_char` for areas
    11/12/22/25/31/32/33/36, `arena_save_char`, Teufelheim PK, LQ-area
    death, area-21 death) are still unported - out of scope for this task,
    left as future `die_character`/`hurt.rs` work. The "Killed with X.XX
    damage by a lvl N NAME." log line (`death.c:1222`) is also still
    unported (only the sound was fixed to match its gating).

- [x] **Game clock advancement** - `world.date` never moves; it is always
  the same hour, so daylight/nightlight logic is frozen.
  - C: `tick_date()` in `src/system/date.c:267`, called once per iteration
    of the main loop (`src/server.c:618`) with `time_now = time(NULL)`
    taken just beforehand (`server.c:616`) - i.e. the game clock is not
    incremented at a fixed per-tick rate, it is *recomputed from real
    wall-clock time* every tick (`game_time = time_now - STARTTIME`,
    `DAYLEN = 2 real hours` per in-game day). `player.c:2357`
    (`if (dlight != player[nr]->dlight) redo = 1;`) is the only "periodic
    refresh on hour change" - it forces a full per-player visible-map
    resend, it does NOT recompute the static `tile.dlight` geometry
    (`compute_dlight`/`reset_dlight` only change when indoor/outdoor map
    structure changes, e.g. a door opening - confirmed by reading
    `src/system/light.c` and grepping all `compute_dlight`/`reset_dlight`
    call sites in `create.c`, which are exclusively door/structure
    mutations, never `tick_date`/`tick_char`).
  - Rust: new `World::advance_date` (`crates/ugaris-core/src/world/date.rs`)
    wraps the already-ported `GameDate::calculate` math
    (`crates/ugaris-core/src/game_time.rs`), called once before the tick
    loop starts and once per tick in `crates/ugaris-server/src/main.rs`
    (mirroring `tick_date()`'s pre-`tick_char()` position each real-time
    loop iteration), using a new `current_unix_time()` helper (same
    `SystemTime::now().duration_since(UNIX_EPOCH)` idiom as
    `rng.rs`/`xmas.rs`/`stacks.rs`) and forwarding `runtime.dlight_override`
    exactly like the existing `/dlight` admin command
    (`commands_admin.rs`). Also fixed a real, previously-masked bug: the
    timer-driven item-driver context built in
    `World::execute_item_driver_timer_request`
    (`crates/ugaris-core/src/world/item_outcomes.rs`) never populated
    `ItemDriverContext::daylight`/`hour`/`fullmoon`/`newmoon`/`solstice`/
    `equinox` at all (always the `0`/`false` `Default`), so
    `nightlight_driver`/`swampwhisp_driver`
    (`crates/ugaris-core/src/item_driver/lights.rs`,
    `area15_swamp.rs`) always believed it was permanently night (the bug
    was invisible before because `world.date` was *also* always frozen at
    its zeroed default, so the two zeros matched) - now populated from
    `self.date` on every timer-driven driver call, matching C's globals
    being live at every `call_item` invocation.
  - Tests: `world/tests/date.rs` (6 tests) - delegates to
    `GameDate::calculate` correctly, reports no change while daylight is
    unchanged, reports a change across a real sunrise boundary (daylight
    0 -> 255), respects the `/dlight`-style numeric override, respects the
    per-area light override table (area 23 underground), and advances one
    `yday` per `DAY_LEN` real seconds.
  - Closed (2026-07-03 re-review): the "mark light-dirty sectors when
    daylight changes" half of the original task note does not apply to the
    current Rust architecture and is not a correctness gap. In C,
    `player.c:2357`'s `redo = 1` on `dlight` change exists purely to defeat
    the `skipx_sector` early-continue optimization inside
    `plr_map_update`'s per-tile loop (`player.c:2374-2380`) - i.e. it forces
    C to *recompute* tiles it would otherwise skip for performance. Rust's
    `map_diff_payloads`/`tile_visibility` (`crates/ugaris-server/src/map_sync.rs`)
    has no equivalent skip-sector fast path at all: it unconditionally
    recomputes every visible tile's effective light from
    `world.date.daylight` each tick and diffs the result against the cached
    `VisibleMapCell`, so a daylight change is already detected and sent to
    every affected player with no extra plumbing needed. `world.dirty_sectors`
    remains unused by `map_sync.rs` (confirmed by grep) and should stay that
    way unless a future task adds a real skip-sector optimization to
    `map_diff_payloads`, at which point `advance_date`'s existing bool
    return value is the right signal to force a full recompute that tick.
    Live boot-smoked again this iteration: server enters the tick loop and
    runs without panics with the wired date advancement in place.

- [x] **Look at character (`CL_LOOK_CHAR`)** - parsed, ignored.
  - C: `cl_look_char` -> `look_char` in `src/system/player.c` /
    `src/system/act.c` (sends `SV_LOOK*` packets with sprite, name,
    description, equipment worn sprites for players; text description for
    NPCs). Check the exact packet in the C client `sv_look`.
  - Rust: protocol builder in `crates/ugaris-protocol/src/packet.rs` +
    handler in `main.rs`; follow `legacy_item_look_text` in
    `crates/ugaris-server/src/inventory.rs` for the text-side conventions.
  - Tests: packet layout against C client expectations; NPC vs player
    variants.
  - REMAINING: `look_char`'s labyrinth-solved count, first-kill Hell
    flavor text, army rank, PK info, clan info, and club info lines are
    not ported (no `count_solved_labs`/`check_first_kill`/`DRD_RANK_PPD`/
    `DRD_PK_PPD`/clan/club system exists yet - each is its own P2/P3 todo
    item). The looker-`CF_GOD` debug branch (dumping the target's carried
    non-worn items + active effect slots) is also deferred since
    `CL_LOOK_ITEM`'s text builder (next task below) doesn't exist yet
    either. See `PORTING_LEDGER.md` "Ralph Loop - Look At Character
    (CL_LOOK_CHAR)" for the full gap list.

- [x] **Look at map item (`CL_LOOK_ITEM`)** - parsed, ignored. Reuse
  `legacy_item_look_text`; gate by `char_see_item` and distance like C
  `cl_look_item`. Tests in `tests/inventory.rs`.

- [x] **Junk item (`CL_JUNK_ITEM`)** - C `cl_junk_item` destroys the cursor
  item (with `IF_NOJUNK` guard, not `IF_QUEST` - corrected during
  implementation; see below).

- [x] **Ping (`CL_PING`)** - C echoes `SV_PING`/`SV_LPING` with the client
  timestamp (see client `sv_ping`, `svl_ping`). Wire it so client RTT
  display works. Trivial: builder + handler + test.
  - C: `cl_ping` (`src/system/player.c:1352-1358`) reads the raw 4-byte
    opaque value the client sent (its own `SDL_GetTicks()`, per the
    community client's `cmd_ping`) and echoes it back unmodified, prefixed
    with `SV_PING` (49) - 5 bytes total, native/little-endian, no
    transformation. There is no separate `SV_LPING` packet type; the
    client's `sv_ping`/`svl_ping` are just its two-pass (length/process)
    naming convention applied to the one `SV_PING` type.
  - Rust: `ClientAction::Ping`/`CL_PING` parsing already existed
    (`crates/ugaris-protocol/src/command.rs`, `client.rs`) but had no
    builder or handler. Added `PacketBuilder::ping` (mirrors the existing
    `ticker`/`mirror` `u8 + put_u32_le` shape) to
    `crates/ugaris-protocol/src/packet.rs`, and wired a
    `ClientAction::Ping { value }` match arm in
    `crates/ugaris-server/src/main.rs` that echoes the value straight back
    to the same session - no character/world state touched, matching C's
    pure-transport handler.
  - Tests: `command.rs` (`parses_ping_opaque_value_little_endian`),
    `packet.rs` (`ping_echoes_opaque_value_unmodified_like_c_cl_ping`).
  - REMAINING: nothing - task fully done as scoped.

- [x] **Fast sell (`CL_FASTSELL`)** - C `cl_fastsell` sells an inventory
  slot directly to the active merchant (`player_store`-adjacent path).
  Extend `crates/ugaris-server/src/merchants.rs`; reuse
  `merchant_store_sell` semantics but from an inventory slot. Tests in
  `tests/commands_chat.rs`... no - `tests/merchants.rs` (create it).
  - C: `cl_fastsell` (`src/system/player.c:877`) calls `swap(cn, pos)` to
    pick the slot item up onto the cursor (swapping back whatever was
    already held - so with an empty slot and a held item, the held item
    lands in the slot and the sell attempt becomes a no-op), then
    `check_merchant(cn)`, then blocks quest items with a hold-SHIFT
    message before calling `player_store(cn, 0, 1, 0)`
    (`src/module/merchants/store.c:325` `buy()`, already ported as
    `World::merchant_store_sell`).
  - Rust: added `apply_fast_sell` in `crates/ugaris-server/src/merchants.rs`
    reusing `inventory_swap_slot` (existing simplified `swap`) +
    `World::check_merchant` + `World::merchant_store_sell`; wired
    `ClientAction::FastSell { slot }` in `crates/ugaris-server/src/main.rs`
    to refresh inventory whenever the swap ran and the merchant store view
    only when a sale actually happened (mirrors C sending `SV_SETITEM`-ish
    inventory updates regardless, but store repaint only on a real trade).
  - Tests: `tests/merchants.rs` (new file) - sells to an open merchant with
    the C `buyprice` formula, swaps back into an empty slot when nothing
    sells, blocks quest items with the exact C message while leaving the
    item on the cursor, rejects the equip/spell slot range, and no-ops
    without an active merchant.
  - REMAINING: C also falls through to `check_container_item` +
    `player_depot`/`account_depot_store`/`container` when no merchant is
    open (the `ch[cn].con_in` branch). The per-character legacy depot
    (`DRD_DEPOT_PPD`/`MAXDEPOT`, `src/system/depot.c`) isn't ported at all
    yet (only the account-wide depot exists), and fast-selling into an
    open item container or account depot from an inventory slot is not
    wired either - only the merchant branch is implemented.

- [x] **NPC sighting messages (`NT_CHAR` emission)** - NPCs only "see"
  players through ad-hoc scans (merchant greeting, simple-baddy attack
  scan). C emits `NT_CHAR` notify messages from character movement so
  *every* driver reacts through its message queue.
  - C: `notify_area(x, y, NT_CHAR, cn, 0, 0)` call sites in
    `src/system/act.c` (walk completion, and nearly every other `act_*`
    completion: idle/take/use/drop/attack/give/every spell cast).
    Correction found while implementing: `notify_area` itself has **no**
    `char_see_char` gate - it's an unconditional `NOTIFY_SIZE` (32-tile) box
    broadcast; the visibility gate is applied downstream by each driver's
    own message consumer (`merchant.c`/`simple_baddy.c` both check
    `char_see_char` themselves after receiving the queued message). Also,
    `src/system/create.c` (spawn) never calls `notify_area` at all - only a
    self-targeted `NT_CREATE`, already ported - so there is no spawn-time
    `NT_CHAR` call site to add.
  - Rust: `World::complete_walk` (`crates/ugaris-core/src/world/actions.rs`)
    now emits `notify_area(.., NT_CHAR, ..)` gated on `!CF_NONOTIFY`,
    matching C `act_walk`. Also fixed an independent radius bug found along
    the way: `notify_area` used +-16 tiles instead of C's `NOTIFY_SIZE = 32`
    (`crates/ugaris-core/src/world/text.rs`).
  - Tests: `world/tests/actions.rs` - walking queues `NT_CHAR` exactly once
    to every character in the (now-correct) 32-tile box including the mover
    itself, `CF_NONOTIFY` suppresses it, a failed walk sends nothing, and a
    character outside the box gets nothing. Fixed 3 pre-existing tests whose
    "far away" fixtures were inside the corrected 32-tile radius (see
    ledger).
  - Rust (iteration 13): wired the remaining inventory/social `act_*`
    completions in `World::complete_take`/`complete_drop`/`complete_use`/
    `complete_give` (`crates/ugaris-core/src/world/actions.rs`). Each mirrors
    its exact C call site: `act_take` (act.c:333-335) and `act_drop`
    (act.c:440-441) fire `NT_CHAR` gated on `!CF_NONOTIFY` right after the
    item moves; `act_drop` additionally fires an *unconditional*
    `NT_ITEM` (act.c:443) for the dropped item regardless of `CF_NONOTIFY`;
    `act_use` (act.c:376-379) fires `NT_CHAR` once target/item validation
    passes, before the deeper item-driver outcome is known (so it fires even
    if the eventual `use_item` driver declines); `act_give` (act.c:871-875)
    fires `NT_CHAR` from the giver's position after `notify_char(co,
    NT_GIVE, ...)` (already wired via `transfer_cursor_item`). Added
    `NT_ITEM` to the `world/mod.rs` re-export list (only `NT_CHAR`/`NT_GIVE`
    etc. were imported before).
  - Tests: `world/tests/actions.rs` - one notify + one no-notify
    (`CF_NONOTIFY` or failed validation) test per call site
    (`complete_take_notifies_nearby_characters_with_nt_char`,
    `complete_take_skips_notify_when_cf_nonotify_set`,
    `complete_drop_notifies_nt_char_and_unconditional_nt_item`,
    `complete_use_notifies_nt_char_once_validation_passes`,
    `complete_use_skips_notify_when_validation_fails`,
    `complete_give_notifies_nt_char_after_nt_give`,
    `complete_give_skips_nt_char_when_cf_nonotify_set`). Updated the
    pre-existing `world/tests/lab2_undead.rs::give_completion_notifies_lab2_undead_receiver`
    test, which now correctly observes 2 driver messages (`NT_GIVE` then
    `NT_CHAR`) since the receiver sits inside its own notify box - this is
    correct C behavior, not a regression.
  - Rust (iteration 14): wired `act_attack` (act.c:763-793) - added `NT_CHAR`
    emission to `World::complete_attack_with_rolls_and_clash_roll`
    (`crates/ugaris-core/src/world/combat.rs`), gated on `!CF_NONOTIFY`,
    firing from the attacker's position after `apply_legacy_hurt` regardless
    of hit or miss (matches C: `sub_attack` runs unconditionally, then the
    surround/rage/notify tail runs regardless of roll outcome). Added a
    defensive "attacker still alive" check (`!CharacterFlags::DEAD`) mirroring
    C's `if (!ch[cn].flags) return 0` guard, even though nothing today can
    kill the attacker mid-`apply_legacy_hurt` (no reflect-damage effect
    exists yet).
  - Tests: `world/tests/combat.rs` -
    `completed_attack_notifies_nearby_characters_with_nt_char_on_hit_and_miss`
    (asserts `NT_CHAR` fires on both a hit and a miss roll, filtering out the
    unrelated `NT_SEEHIT` that `apply_legacy_hurt` also queues to the same
    bystander on a hit) and `completed_attack_skips_notify_when_cf_nonotify_set`
    (uses a miss roll to isolate the `CF_NONOTIFY` gate from
    `apply_legacy_hurt`'s own unconditional `NT_SEEHIT` broadcast).
  - Rust (iteration 15): wired the 12 spell-cast `act_*` completions in
    `World::complete_*` (`crates/ugaris-core/src/world/spells.rs`):
    `complete_bless`/`complete_flash`/`complete_fireball`/`complete_ball`/
    `complete_firering`/`complete_magicshield`/`complete_pulse`/
    `complete_freeze`/`complete_warcry`/`complete_heal` each now emit
    `NT_CHAR` (gated on `!CharacterFlags::NONOTIFY`) followed by an
    unconditional `NT_SPELL` carrying the matching `CharacterValue::*`
    payload (`Bless`/`Flash`/`Fireball`/`Freeze`/`MagicShield`/`Pulse`/
    `Warcry`/`Heal` - all match the legacy `V_*` numeric constants exactly),
    mirroring each C call site (`act.c:936-940` fireball, `1057-1061` ball,
    `929-933`/`935-941` firering (plus its "did `hurt` kill the caster"
    `!DEAD` guard), `1041-1044` flash, `1090-1093` magicshield, `1237-1241`
    bless+`sound_area`, `1399-1402` warcry, `1556-1560` freeze+`sound_area`,
    `1637-1640` pulse, `1671-1674` heal). `complete_ball` intentionally uses
    `CharacterValue::Flash` as the `NT_SPELL` payload (not a "Ball" value,
    which doesn't exist) - copied digit-for-digit from C's own
    `notify_area(..., V_FLASH, fn)`. `complete_earthrain`/`complete_earthmud`
    were left unchanged: C's own `act_earthrain`/`act_earthmud` have their
    `notify_area` calls commented out (dead code), so there is no C behavior
    to port there.
  - Tests: `world/tests/spells.rs` gained
    `completed_firering_notifies_nearby_characters_with_nt_char_and_nt_spell`
    plus `NT_CHAR`/`NT_SPELL` assertions in the existing
    `player_magicshield_spell_sets_up_and_completes_lifeshield_gain`,
    `player_heal_spell_restores_target_hp_on_completion`,
    `player_bless_spell_installs_carried_spell_item_on_completion`,
    `player_flash_spell_installs_timed_speed_spell_on_self`, and
    `player_freeze_spell_installs_negative_speed_spell_on_nearby_target`
    tests; `world/tests/effects.rs`'s
    `player_pulse_damages_low_health_target_and_creates_visible_effects` and
    `world/tests/effect_tick.rs`'s `targeted_fireball_sets_up_projectile_action`/
    `targeted_ball_sets_up_projectile_action`; `world/tests/text.rs`'s two
    warcry tests (including one proving the broadcast is unconditional even
    when a soundblock wall stops the warcry effect itself from reaching the
    target). Fixed one pre-existing test whose fixture asserted the
    old "no messages" behavior:
    `world/tests/spells.rs::action_tick_attack_policy_can_block_area_spell_targets`
    now asserts the blocked target still observes `NT_CHAR`/`NT_SPELL` (the
    area broadcast is unconditional, independent of the attack-policy gate
    on the per-target damage), which matches the C source exactly.
  - REMAINING (all documented as intentional/deferred, not oversights):
    `sub_surround`/`V_SURROUND` (act.c:697-705) and `increase_rage` are still
    not ported (no `rage`/`V_SURROUND` fields exist on `Character` yet), so
    `act_attack`'s surround-weapon and rage side effects remain a gap
    independent of all `NT_CHAR`/`NT_SPELL` wiring in this item. The
    `act_idle` equivalent (`world/regen.rs`) is intentionally deferred:
    Rust's idle regen runs every tick continuously rather than once per C's
    `act1`-sized batch, so wiring `NT_CHAR` there now would emit far more
    often than C until that batching gap is closed. Migrating the merchant
    greeting scan (`world/merchant.rs::greet_nearby_players`) to consume
    `NT_CHAR` via `process_merchant_messages` instead of its current
    per-tick brute-force scan is also not done (optional per this item's own
    wording).

---

## P1 - Core Framework

Systems every later port depends on. Order within the section is a
suggestion; dependencies are noted.

- [~] **`update_char` stat recomputation** - the big one. C
  `update_char(cn)` in `src/system/tool.c` recomputes `values[0]` from
  `values[1]` plus worn equipment modifiers, spell items, profession
  bonuses, race base, and clamps; it also recomputes `V_ARMOR`/`V_WEAPON`
  from gear and sets `CF_ITEMS`/`CF_UPDATE` fan-out.
  - Rust today: only ad-hoc modifier deltas exist
    (`world/character_values.rs`). Port the full recompute as
    `World::update_character(cn)` and call it where C calls `update_char`
    (equip/unequip, spell install/expiry, level up, login, death respawn).
  - Do this in slices: (1) equipment modifier sum + caps, (2) driver-spell
    flags (partially exists as `refresh_driver_spell_flags`), (3) armor and
    weapon values, (4) HP/end/mana max clamps.
  - Tests: `world/tests/character_values.rs` - wearing/removing items
    changes effective values exactly like C, including the +20 modifier cap
    and non-stacking rules.
  - Blocks: proper equip flow, enchant effects, level-ups feeling right.
  - REMAINING: `World::update_character(cn)` now ports the full
    `create.c:1710` algorithm (all four slices: item/spell modifier sum
    with the 50%/72.5% seyan cap and `IF_BEYONDMAXMOD` bypass, skill-value
    base-attribute averaging, Body Control/Armor Skill/spell-average armor
    bonuses, Speed Skill/Athlete/Thief/Demon profession bonuses, day/
    night/clan attribute bonuses, and the HP/endurance/mana current-value
    clamp), and is wired into worn-slot equip/unequip
    (`inventory_swap_slot`, `pos < 12` only, matching C `do.c:1294`).
    Iteration 17: wired every remaining `world/spells.rs` install/expire
    call site (`install_bless_spell`/`install_bonus_spell`/
    `install_beyond_potion_spell`/`install_speed_spell` (warcry/freeze)/
    `install_curse_spell`/`install_firering_spell`/
    `install_timed_identity_spell` (infravision/oxygen/uwtalk)/
    `remove_driver_spells`/`poison_character`/`remove_poison_by_driver`/
    `schedule_existing_spell_timers`/`remove_spell_from_timer`), each
    matched 1:1 against its C call site (`bless_someone`/`bless_self`
    `act.c:1117/1158`, `add_bonus_spell` `drvlib.c:2646`,
    `add_potion_spell` `alchemy.c:1007`, `warcry_someone`/`freeze_someone`
    `act.c:1324/1522`, `ice_curse` `act.c:1470`, `add_spell`
    `tool.c:1683`, `poison_someone`/`remove_poison`/`remove_all_poison`
    `poison.c:61/128/148`, `remove_spell` `tool.c:1591`) - removed the now
    fully-superseded `apply_item_modifier_deltas`/`add_character_value_delta`
    helpers from `character_values.rs`. Also wired `World::raise_skill`
    (`world/skills.rs`, C `raise_value` `skill.c:256`), player-death
    respawn (`World::die_character`, C `die_char` `death.c:807`, gated on
    the same-area `place_character_on_map` success matching C's
    `transfer_to_restarea` return value), and login (both the DB-snapshot
    path in `ugaris-server/src/snapshots.rs::apply_character_snapshot` and
    the template/scaffold path in `main.rs`'s login handler), matching C
    `login_ok` (`database_character.c:1512`). Uncovered and fixed a real
    pre-existing bug along the way: `poison_callback_from_timer`'s
    `tick == 0` HP-modifier decrement and `remove_poison_by_driver` never
    triggered a stat recompute at all (only set `CF_UPDATE`), so poison's
    permanent max-HP reduction was silently inert; both now call
    `update_character` matching `poison_callback`/`remove_poison`/
    `remove_all_poison` in C. Several `world/tests/*.rs` fixtures needed
    `values[1]` (raised base) baselines added since the wired recompute now
    genuinely enforces C's per-value floor clamp (`n <= V_STR` -> 0) and
    the 50%-of-raised-base cap on item/bless modifiers, which the old
    delta-only helper never enforced - see `world/tests/spells.rs`,
    `world/tests/text.rs`, `world/tests/hurt.rs`,
    `world/tests/death.rs`, `world/tests/skills.rs` for the corrected
    fixtures/expectations and comments explaining each.
  - Iteration 18: closed the "item-driver-level raise" sub-gap by having
    the `World`-level caller recompute after applying the driver's outcome
    (the second option the previous note left open), rather than
    threading `&mut World` through the whole item-driver dispatch. C
    `raise_value_exp` (`src/system/skill.c:315-377`, used by
    `item_driver/scrolls.rs::stat_scroll_driver` for `IDR_STATSCROLL`)
    calls `update_char(cn)` after every successful bare-value raise; the
    Rust driver loops calling `raise_value_exp` per scroll charge purely
    on `&mut Character` and returns `ItemDriverOutcome::StatScrollUsed`, so
    `World::apply_item_driver_outcome` (`world/item_outcomes.rs`) now
    matches that outcome and calls `self.update_character(character_id)`
    once after the loop completes - equivalent to C's per-raise calls
    since `update_char` is idempotent on the final `value[1]` state. Test:
    `world/tests/item_outcomes.rs::stat_scroll_use_triggers_update_character_recompute`
    raises Body Control via a scroll and asserts the derived Armor bonus
    (`body_control * 5`) updates immediately instead of staying stale.
    Verified the other two named gaps are non-issues after reading their C
    sources directly: `enchant_item`/`anti_enchant_item`
    (`src/module/base.c:3543`/`5781`, backing `orbs.rs::enchant_driver`/
    `anti_enchant_driver`) mutate only the target item's `mod_index`/
    `mod_value` and never call `update_char` in C either (the recompute
    only happens later when the enhanced item is worn, which is already
    wired via `inventory_swap_slot`); `potions.rs`'s drivers
    (`potion_driver`/`special_potion_driver`/`beyond_potion_driver`) only
    heal/restore current `hp`/`mana`/`endurance` or install spells via
    `install_beyond_potion_spell` (already wired in iteration 17), never
    touch `values[]`, so they need no additional wiring.
  - STILL REMAINING (iteration 20 update): `raise_value_exp`'s
    `check_levelup(cn)` call is now wired too (see the "Experience/level-up
    side effects" task's iteration 20 note - `world/item_outcomes.rs`'s
    `StatScrollUsed` handler calls `check_levelup` before
    `update_character`). The `src/area/18/bones.c:317-431` and
    `src/area/37/arkhata.c:800-801`
    call sites of `raise_value_exp` are not yet ported to Rust at all (no
    `raise_value_exp` usage exists outside `scrolls.rs` in the Rust tree),
    so those specific area drivers remain out of scope for this note.
    Documented gaps in the recompute itself (`ch.ef[]` area-effect light,
    `P_CLAN`/`areaID == 13`, sprite reselection) are unchanged from the
    previous iteration's notes.
  - Iteration 21: closed the sprite-reselection gap. Ported
    `recompute_character_sprite` (`world/character_values.rs`, C
    `create.c:1969-2120`) - class/gender/weapon-in-hand `sbase`/`off`
    selection (all 12 warrior/mage/male/female/arch combinations) plus the
    full six-slot (`head`/`arms`/`legs`/`body`/`cloak`/`feet`)
    `IID_DEMONSKIN1/2/3` full-suit override (added those three item-ID
    constants, absent from Rust before now), gated on C's
    `CF_PLAYER && (!CF_GOD || sprite in admin-exempt ranges)` check.
    `World::update_character` calls it after the value recompute and
    marks the character's tile dirty (`mark_dirty_sector`) on an actual
    sprite change, matching C's `set_sector` call. `reset_name(cn)`
    (colored-name cache invalidation on demon-sprite transitions) is
    documented as an intentional no-op: Rust has no server-side
    name-color cache to invalidate. Tests in
    `world/tests/character_values.rs` cover unarmed class/gender base
    sprite, two-handed-weapon offset, full demon-skin-1 suit override,
    the god-admin-sprite exemption, and the dirty-sector marking on
    change. While porting this, found and fixed a real latent bug in the
    already-shipped Body Control bare-handed Weapon-bonus check (same
    file): `item.flags.contains(ItemFlags::WEAPON)` is wrong since
    `IF_WEAPON` is a composite of several single-bit weapon-class flags
    (`IF_AXE|IF_DAGGER|...`) and C's check is `flags & IF_WEAPON` (any
    bit) - `.contains()` requires *every* bit simultaneously, which no
    real weapon item ever has, so the bare-handed bonus was silently
    never suppressed by a real weapon in hand. Fixed both this call site
    and the new sprite one to use `.intersects()` (matching the correct
    pattern already used in `world/npc_fight.rs:2381`), with a new
    regression test
    (`body_control_bare_handed_bonus_is_suppressed_by_a_real_weapon_in_hand`)
    since the existing test only exercised the empty-hand path.
    STILL REMAINING: `ch.ef[]` area-effect light and `P_CLAN`/
    `areaID == 13` (documented above; `area_id` is a real, already-
    threaded per-instance value but wiring it through `update_character`
    touches ~32 call sites, deferred as its own slice) are unchanged.

- [ ] **Equipment slot rules on swap (`CL_SWAP` into worn slots)** - C
  `cl_swap`/`swap` checks `place_item_typed` rules: worn slot flag match
  (`IF_WN*`), min level, class gates, two-handed vs left hand, and calls
  `update_char`. Verify the Rust `inventory_swap_slot`
  (`crates/ugaris-server/src/inventory.rs`) against C and port the missing
  gates. Tests exist in `tests/inventory.rs` - extend them.

- [~] **Experience/level-up side effects** - C `give_exp` ->
  `check_levelup` in `src/system/skill.c`/`tool.c`: level recompute from
  exp, `SV_TEXT` "You have reached level N!", HP/end/mana refill on level,
  `update_char`, achievements hook. Rust has exp modifiers server-side but
  no level recompute. Port `exp2level`/`level2exp` into core (variants
  already exist in `crates/ugaris-server/src/spawns.rs` - consolidate into
  `ugaris-core` and re-export) and apply on every exp grant (kill exp path
  in `world/death.rs` + admin/quest grants).
  - Progress: ported `exp2level`/`level2exp`/`level_value`
    (`src/system/tool.c:1272-1283`) into the new canonical
    `crates/ugaris-core/src/world/exp.rs` and deleted the three duplicate
    copies that had accreted (`ugaris-server/src/spawns.rs`
    `legacy_level2exp`/`legacy_exp_to_level`, `ugaris-server/src/
    area_apply.rs` `legacy_level_value`/`legacy_level_exp`,
    `ugaris-core/src/item_driver/helpers.rs` `legacy_level_value` now
    delegates to `world::level_value`); all former call sites (LQ
    raise/reset, random shrines, quest rewards, food/area17/area29 item
    drivers, `/god exp` NOLEVEL cap) now use the one core copy. Ported
    `World::check_levelup(character_id)` (`tool.c:1318-1356`) with tests
    in `world/tests/exp.rs`: level-increment loop over `max(exp,
    exp_used)`, "Thou gained a level!" text, save grant/reset
    (hardcore resets to 0, others +1 capped at 10) with its two feedback
    lines, the level-20 profession unlock (`value[1][V_PROFESSION] = 1`)
    guarded on it not already being set, and the `set_sector` dirty-map
    refresh (`World::mark_dirty_sector`). Wired it into the two "give_exp"
    call sites the C source actually has hooked to a live player: killer
    exp (`world/death.rs` `KillExpAward` -> `main.rs` tick loop) and the
    `/god exp` admin command, both of which route through
    `commands_admin.rs::give_exp_with_runtime_modifiers` (kept in the
    server crate since its two multipliers, `exp_modifier`/
    `hardcore_exp_bonus`, are live-tunable `ServerRuntime` fields, not
    `ugaris-core` `GameSettings`); it now takes `&mut World` +
    `CharacterId` instead of `&mut Character` and calls
    `world.check_levelup(character_id)` after updating `exp`, gated on
    `!NOLEVEL` exactly like C's `if (!(ch[cn].flags & CF_NOLEVEL))
    check_levelup(cn);` tail call. Extended
    `tests/commands_admin.rs::god_exp_command_uses_runtime_exp_modifiers_and_legacy_gates`
    to assert the target actually levels up (1 -> 3) and the NOLEVEL
    character does not, despite being one exp shy of the next threshold.
    Re-read `check_levelup` directly in C: it does **not** refill HP/
    endurance/mana on level-up (that text in this task's own description
    doesn't match the C source) and does not call `update_char` itself
    either - only `raise_value_exp` calls `update_char` after its own
    `check_levelup` call, so no HP/mana-refill or update_char gap actually
    exists here.
  - Iteration 20: closed the `raise_value_exp` gap noted above. C
    `raise_value_exp` (`skill.c:315-361`) calls `check_levelup(cn)` right
    after adding the raise cost to `exp`/`exp_used`, once per successful
    raise; the stat scroll driver (`item_driver/scrolls.rs`,
    `base.c:6031` `IDR_STATSCROLL`) loops calling it per scroll charge on
    `&mut Character` only, so - following the same outcome-based pattern
    iteration 18 used for `update_character` - `World`'s
    `ItemDriverOutcome::StatScrollUsed` handler
    (`world/item_outcomes.rs::apply_item_driver_outcome`) now calls
    `self.check_levelup(character_id)` before `self.update_character(...)`,
    matching C's per-charge `check_levelup`/`update_char` ordering; a
    single batched call after the loop is equivalent since both are
    idempotent/monotonic on the final `exp`/`value[1]` state (documented
    on the `ItemDriverOutcome::StatScrollUsed` doc comment, including why
    the `V_PROFESSION` unlock edge case cannot diverge - raising
    `V_PROFESSION` itself requires it already non-zero, which is exactly
    when `check_levelup`'s unlock is a no-op). Test:
    `world/tests/item_outcomes.rs::stat_scroll_use_triggers_check_levelup`
    raises a cheap skill from a low bare value and asserts the character's
    `level` field actually increments, not just `exp`.
   - REMAINING: the level-10-multiple "Grats" server-wide broadcast
     (`server_chat(6, ...)`), `achievement_check_level`, and
     `reset_name(cn)` have no Rust equivalents anywhere (documented as gaps
     in `check_levelup`'s doc comment, not silently dropped).
   - Iteration 22: ported the canonical `World::give_exp(character_id,
     base_exp, area_id)` (C `give_exp` `tool.c:1371-1423`) into
     `ugaris-core/src/world/exp.rs` - the full algorithm (hardcore/global
     multipliers now read from `self.settings.exp_modifier`/
     `hardcore_exp_bonus`, the `CF_NOEXP`/area-21 no-op gate, the
     `CF_NOLEVEL` exp-band clamp, the i32 range clamp, the
     decrease-prevention guard, and the `check_levelup` tail call) so it is
     usable from `ugaris-core` item drivers, which only ever have `&mut
     World` (not `ServerRuntime`). Made `world.settings.exp_modifier`/
     `hardcore_exp_bonus` the single source of truth: removed the
     duplicate `ServerRuntime::exp_modifier`/`hardcore_exp_bonus` fields
     (server crate never read `world.settings` before this, so the two
     copies would otherwise silently diverge) and repointed
     `/setexpmod`/`/sethardcoreexpbonus` to mutate `world.settings`
     directly; `commands_admin.rs::give_exp_with_runtime_modifiers` is now
     a thin `world.give_exp` wrapper. Wired two of the six remaining
     direct-mutation call sites named in the previous note: `/milexp`
     (`commands_admin.rs`, C `cmd_milexp`/`give_military_pts_no_npc`
     `command.c:3014`/`tool.c:3281`) now routes its fixed `give_exp(co, 1)`
     call through `World::give_exp` instead of a raw `+= 1`, and also fixed
     a latent bug found while reading the C source: the hardcore
     `military_points` multiplier was hardcoded to `1.10` instead of
     reading the already-live-tunable `runtime.hardcore_military_exp_bonus`
     (default 1.10, so previously invisible unless an admin changed it).
     `item_driver/food.rs`'s lollipop exp bonus (C `lollipop` `base.c:3250`
     calling `give_exp`) no longer mutates `character.exp` directly inside
     the driver (which only has `&mut Character`); the driver now returns
     the base amount via `ItemDriverOutcome::LollipopLicked.exp_added` and
     `World::apply_item_driver_outcome`'s new arm
     (`world/item_outcomes.rs`) grants it through `self.give_exp`, matching
     the existing `StatScrollUsed` outcome-based pattern. Also fixed
     `ugaris-core/src/player.rs::touch_demonshrine` (C `demonshrine_driver`
     `base.c:3189-3235`, the `player.rs:2921` site named in the previous
     note): it previously mutated `character.exp` directly too (missing
     the multipliers/`CF_NOEXP`/`check_levelup`, and never called
     `update_char` for the Demon value bump either); it now only mutates
     the Demon value/`CF_ITEMS` and returns `exp_added` unapplied, with the
     `ItemDriverOutcome::DemonShrine` handler in `ugaris-server/src/
     main.rs` calling `World::update_character` then `World::give_exp`,
     matching C's `update_char(cn); give_exp(cn, ...);` call order exactly.
     Tests: `world/tests/exp.rs` (`give_exp_*`, 8 new cases covering the
     modifier math, `NOEXP`/area-21 gates, `NOLEVEL` clamp both directions,
     the decrease-prevention guard, and the `check_levelup` tail call),
     `world/tests/item_outcomes.rs::lollipop_lick_grants_exp_through_give_exp_not_a_raw_mutation`,
     `tests/commands_admin.rs::milexp_routes_its_fixed_one_exp_through_give_exp_and_honors_runtime_military_bonus`,
     updated `player::tests::demonshrine_touch_updates_value_and_blocks_repeats`
     to assert the caller-side application contract.
     STILL REMAINING: `area_apply.rs`'s four random-shrine reward sites and
     `main.rs`'s inline quest/area reward grants (~4 sites) still bypass
     `give_exp`/`check_levelup` entirely (no hardcore/exp_modifier/NOEXP/
     NOLEVEL handling, no level-up). Each is server-crate-only code with
     `&mut World` already in scope, so wiring them is now a mechanical
     `world.give_exp(...)` swap-in, no further infrastructure needed - a
     good next slice.
  - Iteration 23: closed the "`area_apply.rs`'s four random-shrine reward
    sites" half of the previous note. C `shrine_edge`/`shrine_vitality`/
    `shrine_braveness`/`shrine_continuity` (`src/area/14/random.c:2028`/
    `2078`/`2176`/`2126`) all grant their exp via `give_exp(cn, ...)`, not
    a raw `ch[cn].exp += ...`; `apply_random_shrine_edge`/`_vitality`/
    `_braveness`/`_continuity` (`area_apply.rs`) each took only `&mut
    Character` (no `&mut World`), so they now return the computed amount
    through their existing `Used { exp, .. }` result variants without
    touching `character.exp` themselves, and the four call sites in
    `main.rs`'s `RandomShrineKind` match arms call `world.give_exp(...)`
    (and, for vitality, `world.update_character(...)` matching C's
    trailing `update_char(cn)`) once the `&mut Character` borrow has
    ended - same outcome-based pattern as the `StatScrollUsed`/
    `LollipopLicked` item-driver outcomes from earlier iterations. Also
    found and fixed the same bug in `apply_zombie_shrine`'s experience
    branch (C `area2.c:259/325/390`, a different file but the identical
    raw-mutation issue): since that function already takes `&mut World`
    directly, it now calls `world.give_exp(...)` inline instead of
    mutating `character.exp`. Updated the pre-existing unit tests for
    these four `apply_random_shrine_*` functions (`tests/area_apply.rs`,
    `tests/item_apply.rs`, `tests/commands_admin.rs`) to stop asserting
    `character.exp` (no longer this function's responsibility - see the
    inline comments added at each call site) and added a new test,
    `apply_zombie_shrine_experience_routes_through_give_exp_and_honors_noexp_and_modifier`,
    proving the `NOEXP` gate blocks the grant and the runtime
    `exp_modifier` multiplier scales it, which the old raw-mutation code
    silently ignored.
  - Iteration 24: closed the last "main.rs's inline quest/area reward
    grants" sub-gap - all 4 remaining raw-mutation sites (grep
    `character.exp = character.exp.saturating_add` in `main.rs`, now zero
    hits). Cross-referenced each against C: the two warp/reward-sphere
    sites (`ItemDriverOutcome::WarpBonus` handler) are C
    `warpbonus_driver` (`src/area/25/warped.c:423` sphere-kind-1 reward,
    `:453` the per-step trickle exp) - both call `give_exp(cn, ...)`, so
    both now call `world.give_exp(character_id, ..., u32::from(args.area_id))`;
    the reward-sphere match had to be restructured (the exp arm can no
    longer sit inside the `world.characters.get_mut(&character_id)`
    borrow used by the save/military/gold/lollipop arms, since
    `give_exp` needs `&mut World` itself) into a top-level match on
    `reward_sphere_kind` where only the non-exp arms re-borrow
    `world.characters` individually - behaviorally identical, verified by
    reading each arm against `warped.c:397-441` line by line (the
    `Some(2)` guard-on-saves-and-not-hardcore condition is preserved
    as an `if` instead of a match guard). The other two sites are C
    `bookcase` (`src/area/17/two.c:2622`, the library-solved-once reward,
    `give_exp(cn, min(level_value(level)/5, 80000))` - matches the
    existing `bookcase_library_exp` helper exactly) and C
    `staffer_animation_book` (`src/area/29/brannington.c:521`,
    `give_exp(cn, min(level_value(60)/5, level_value(level)/4))` - matches
    the existing driver-side `exp_added` computation in
    `area29_brannington.rs`); both now call `world.give_exp(...,
    u32::from(args.area_id))` instead of mutating `character.exp`
    directly. All four sites pass the real `args.area_id` from the
    dispatch loop (already used by the sibling `RandomShrineKind`/`Chest`
    arms a few hundred lines up), so the `CF_NOEXP`/area-21/hardcore/
    exp_modifier/`check_levelup` handling now applies uniformly. No
    dedicated new tests: these are inline dispatch-loop match arms (no
    testable pure function boundary, matching the existing precedent for
    the sibling `RandomShrineKind::Edge` arm, which is likewise untested
    at the `main.rs` wiring level - only the extracted pure functions
    `apply_random_shrine_edge`/`bookcase_library_exp`/`warpbonus_driver`/
    `staffer2_driver` have direct tests); `cargo test --workspace` stayed
    at the same 342/1105/etc pass counts (no regressions), confirming the
    refactor didn't change any currently-tested behavior. Grepped the
    whole workspace for any other raw `character.exp` grant mutations
    (excluding `exp_used`/`exp_cost`/`exp_added` counters and the
    intentional subtraction sites in `potions.rs`/`death.rs` which are
    refunds/losses, not grants, and correctly stay raw): none remain.
    STILL REMAINING (unchanged from earlier iterations, and out of this
    task's original C-`give_exp`-routing scope): the level-10-multiple
    "Grats" server-wide broadcast, `achievement_check_level`, and
    `reset_name(cn)` documented in `check_levelup`'s doc comment have no
    Rust equivalents.

- [ ] **Ground item decay** - dropped items never disappear (bodies do).
  C: `set_expire(in, item_decay_time)` on player drops (`act_drop`) and
  `expire_item` behavior for `IF_TAKE` ground items in `src/system/item.c`
  / `tool.c`. Rust: reuse `World::set_item_expire` from `world/death.rs`
  in `complete_drop`; respect `IF_NODECAY`. Tests in `world/tests/items.rs`.

- [ ] **`SV_SETVAL`/resource streaming on change** - C pushes value/exp/
  gold/HP bars whenever they change (`CF_UPDATE`/`CF_ITEMS` consumers in
  `plr_update`). Rust only sends resources in the periodic char record and
  after specific actions. Add a per-tick pass: when a session's character
  has `UPDATE`/`ITEMS` flags set, send the same packets login sends
  (`SV_SETVAL*`, `SV_SETHP/ENDUR/MANA`, exp, gold, inventory snapshot for
  `ITEMS`) and clear the flags. Mirror C's flag semantics exactly.
  - This replaces several ad-hoc `command_inventory_refresh` pushes -
    migrate call sites gradually, do not break existing tests.

- [ ] **Serial validation everywhere** - C guards every queued action with
  `ch[co].serial != act2 -> abort`. Rust stores serials but
  `apply_player_action_setup` checks them only for kill/fireball/ball.
  Audit `PAC_*` setups against C `player_driver.c` switch and add the
  missing serial guards. Tests: stale-serial targets abort to idle.

- [ ] **Logout/exit flow** - C `cl_exit`/lostcon: linger timer
  (`CDR_LOSTCON` drives the body for `lagout_time`), save, despawn. Rust
  despawns instantly on disconnect. Port the lostcon linger: on disconnect
  keep the character with `CDR_LOSTCON` driver for `runtime.lagout_time`
  ticks (idle, attackable), then save+remove. Tests: disconnect keeps the
  character breathing for the window; reconnect within the window reclaims
  it (C `take_over_char`).

- [ ] **PostgreSQL login hardening** - wrong password must send the legacy
  reject (`SV_EXIT` reason? check C `cmd_exit(nr, reason)` in
  `src/system/io.c`), not a scaffold accept. Character creation for
  unknown names per C account flow (or explicit reject if creation is
  website-side - read `database_character.c::begin_login` fully and match
  it). Extend `crates/ugaris-db/src/character.rs` tests with a mocked pool
  if DB is unavailable; otherwise gate live tests behind `DATABASE_URL`.

- [ ] **Merchant store DB persistence** - C `database_merchant.c`
  (load_merchant_inventory, queue_merchant_* tasks). Rust merchants are
  memory-only. Add `crates/ugaris-db/src/merchant.rs` + a migration
  mirroring the C tables, load on store creation, queue saves on
  buy/sell. Follow the existing `character.rs` repository shape.

- [ ] **Special stores** - C `add_special_store`/`create_special_item`
  (`src/module/merchants/store.c` + `create.c`): the random enchanted-item
  stock merchants refresh every 12h. Port `create_special_item` into core
  (it is also used by chests/loot), then enable the `special` merchant arg
  path already parsed in `MerchantDriverData`.

- [ ] **Client command audit completion** - handle the remaining parsed
  actions: `ClientInfo`, `Log`, `ModPacket` (mod protocol - can be a
  logged no-op initially, but check `src/common/mod_packet.c` for the
  handshake the community client expects), `Nop`. Anything still
  unhandled must at least be an explicit logged no-op, not silence.

---

## P2 - NPC & Dialogue Framework

Unlocks every quest NPC. Do these before any P4 area work.

- [ ] **Generic NPC text analysis (`analyse_text_driver`)** - C
  `src/module/merchants/merchant.c::analyse_text_driver` and the richer
  copy in `src/area/1/gwendylon.c` (they share a pattern: lowercase the
  text, match name + keyword, respond via `quiet_say`). Port a reusable
  keyword-matcher into `crates/ugaris-core/src/character_driver.rs` that
  drivers feed their `NT_TEXT` messages through. Tests: keyword hit/miss,
  name gating, case insensitivity.

- [ ] **Driver memory (`mem_*`)** - C `src/system/mem.c`:
  `mem_add_driver/mem_check_driver/mem_erase_driver` per-(npc, player,
  slot) memory with timeouts. The merchant greeting already fakes slot 7
  with `MerchantDriverData::greeted` - replace with a proper
  `DriverMemory` structure on `CharacterDriverState` usable by all
  drivers. Tests: add/check/expiry parity.

- [ ] **`quiet_say`/`say`/`emote` NPC speech helpers in core** - several
  drivers need to talk. There are queued area-text pieces already
  (`queue_lab2_undead_say`); generalize to `World::npc_say(cn, text)`
  (say format), `npc_emote`, `npc_murmur` with the C color/format rules
  from `src/system/talk.c`. Migrate existing call sites.

- [ ] **Idle NPC chatter** - merchant/citizen random murmur tables
  (`merchant_driver` RANDOM(25) block, citizen equivalents). Needs the
  speech helpers. Low complexity, high flavor.

- [ ] **`CDR_BANK` banker NPC** - C `src/module/bank.c`: deposit/withdraw
  via text commands + `NT_GIVE` money handling, balance stored in PPD
  (`DRD_BANK_PPD`? read the C). Port driver + PPD codec + tests.

- [ ] **`CDR_TRADER` player-to-player trade NPC** (`src/module/base.c`
  trader section) and **`CDR_JANITOR`** (item cleanup NPC). Both have
  registry stubs already - fill in behavior.

- [ ] **Aclerk / auction NPC** - C `merchant.c::aclerk_driver` +
  `src/system/auction/*.c` + `database_merchant.c`. Big; slice it:
  (1) aclerk dialogue/give handling, (2) auction storage in DB,
  (3) `CL_*` auction client protocol if the community client uses it
  (check client sources first - if the client has no auction UI, mark
  N/A with a note).

- [ ] **Gatekeeper NPC (`src/system/gatekeeper.c`)** - lab entrance
  dialogue/fight driver. The lab item drivers are ported; this is the
  character in front. Depends on text analysis + memory.

---

## P3 - World Systems

- [ ] **Questlog initialization & quest state machine**
  (`src/system/questlog.c`) - quest open/done transitions driven by NPC
  dialogue flags, `sendquestlog` on change (packet already ported), exp
  rewards per quest (`quest_exp.h`). Port the quest table + the
  `questlog_open/done` helpers; wire the already-ported `CL_REOPENQUEST`
  reset side effects per area.

- [ ] **Achievements (`src/module/achievements/achievement.c`)** - runtime
  markers partially exist (chests, transport). Port the achievement
  table, progress PPD, `SV_*` packets the community client expects
  (check client), and the grant/announce path. Wire existing markers.

- [ ] **Clan system (`src/system/clan.c` + DB)** - membership lives in DB;
  Rust has direct clan fields only. Port clan repository
  (`crates/ugaris-db/src/clan.rs`), clan trade bonus (merchants call
  `clan_trade_bonus` - currently 0), clan-vs-clan attack policy in
  `can_attack`, clan chat channel gating, clan hall transport access
  (transport module has the seam).

- [ ] **Military ranks (`src/module/military.c`)** - military points exist
  on `Character`; port rank thresholds, `#rank` style commands, mission
  PPD (`mission_ppd.h`) and the governor mission flow (`check_military_solve`
  is referenced by the death path - port it there when this lands).

- [ ] **Arena rankings (`src/system/arena.c`)** - toplist formatter is
  ported but rankings are never stored. Port `DRD_ARENA_PPD`, win/loss
  recording on arena kills, and the ranking table persistence.

- [ ] **Weather driver (`src/module/weather/weather.c`)** - server-side
  state machine exists in `crates/ugaris-server/src/weather.rs` (admin
  commands only). Port the actual per-tick weather effects: `SV_*`
  weather packets to clients (check client protocol), movement slow,
  visibility reduction, damage weather, area gating.

- [ ] **Events (`src/module/events/**`)** - recurring boosted-rate events
  and seasonal events (christmas partially ported). Port the scheduler +
  each recurring event's modifier hooks (`event_drop_rate` modifier is
  referenced by loot JSON).

- [ ] **Death-mode loot tables (`src/system/loot/loot.c`)** - JSON tables
  under `ugaris_data/loot/`. Port the loader + roll engine + pity
  counters + `apply_death_loot_for_template` into the body-container fill
  in `world/death.rs`. Only pents data exists today; add tests with
  fixture JSON.

- [ ] **Remaining `/` and `#` text commands** - diff
  `src/system/command.c` against `crates/ugaris-server/src/commands_*.rs`
  and port what's missing (there are dozens; make a checklist in the PR
  note as you go). Priority: `/help` completeness, `/who` variants,
  `/allow`/clan invite commands, admin teleports (`/goto`), `/mirror`,
  `/seen`, `/top`.

- [ ] **Cross-area transfer** - the big multi-server feature. Every
  cross-area teleport currently returns "target server down". Decide the
  single-process stance first (likely: run multiple areas in one process
  or reject cleanly). If porting: C `change_area` in
  `src/system/database/database_area.c` + area handoff blobs. This is a
  design task - write a plan in the ledger before coding.

- [ ] **`.pre` zone preprocessor parity** - `src/system/create.c` expands
  `.pre` template includes; the Rust `ZoneLoader` skips them. Check which
  areas' data actually use `.pre` and port expansion so those areas load
  fully.

- [ ] **Sector skip optimization (`skipx_sector`)** - C skips unchanged
  sectors in the per-tick map scan. Port once per-tick diff CPU becomes a
  measured problem (profile first; likely fine for small player counts).

---

## P4 - Area Content

Every area's `.c` file mixes item drivers (mostly ported - check the
ledger) and character drivers (mostly NOT ported). For each area task:
port the character drivers (dialogue via P2 framework, quest PPD, special
movement), then boot with that area's data and smoke it.

Ordered by player progression; the C file is the oracle.

- [ ] **Area 1 - `src/area/1/gwendylon.c` (6,286 lines)** - the tutorial
  and main city NPCs: Gwendylon quest chain, Lydia tutorial give, skeleton
  quests, `tutorial_ppd` hints (player_driver.c has the tutorial hook -
  port together). This is the highest-value area work: new players see it
  first. Slice by NPC.
- [ ] **Area 2 - `src/area/2/area2.c`** - remaining character drivers
  (zombie lord, priests). Item drivers done.
- [ ] **Area 3 - `src/area/3/area3.c`** - palace story NPCs, lamp ghost
  quest flow (lamps themselves are ported).
- [ ] **Area 4 - `src/area/4/pents.c`** - pentagram quest NPCs + demon
  wave logic beyond the ported item boundary.
- [ ] **Area 6 - `src/area/6/edemon.c`** - Earth Demon boss driver
  (`CDR_EDEMON*` characters); machinery items are ported.
- [ ] **Area 8 - `src/area/8/fdemon.c`** - Fire Demon boss + farm NPCs;
  cannon/loader items are ported.
- [ ] **Area 10 - `src/area/10/ice.c`** - ice NPCs, ice demon curse
  integration (curse spell side is ported).
- [ ] **Area 11 - `src/area/11/palace.c`** - palace guards, Islena fight
  driver (door/bomb/cap items ported).
- [ ] **Area 12 - `src/area/12/mine.c`** - keyholder golems, miners.
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
- [ ] **Area 33 - `src/area/33/tunnel.c`** - long tunnel events.
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
  by multiple areas.

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

---

## Progress Log

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
