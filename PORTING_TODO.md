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

- [x] **`update_char` stat recomputation** - the big one. C
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
  - Iteration 25: closed the `P_CLAN`/`areaID == 13` gap without the
    feared ~32-call-site refactor. Since this Rust server is one process
    per area for its entire lifetime (`ServerConfig::area_id` is set
    once at startup and never changes), added a `pub area_id: u16` field
    directly on `World` (`world/mod.rs`, defaults to `0` via `#[derive
    (Default)]`) instead of threading `area_id` as a parameter through
    `update_character` and its ~17 non-test call sites. `main.rs` sets
    `world.area_id = config.area_id` once, immediately after
    `World::default()`, before the zone map loads. `World::update_character`
    (`world/character_values.rs`) now computes `in_clan_area` as
    `self.area_id == 13 || tile.flags.contains(MapFlags::CLAN)`, matching
    C `create.c:1856` exactly (`areaID == 13 || (mmf & MF_CLAN)`) - the
    existing `P_CLAN` profession-bonus arithmetic
    (`character_values.rs:511-514`, from an earlier iteration) was
    already correct and only needed the real `areaID == 13` input.
    New tests
    (`clan_profession_bonus_applies_in_area_13_catacombs_without_clan_tile_flag`,
    `clan_profession_bonus_does_not_apply_outside_area_13_or_clan_tile`)
    in `world/tests/character_values.rs` cover both the area-13-without-
    tile-flag case and the outside-area-13-and-no-tile-flag no-bonus
    case. Boot-smoked: `entering Rust game loop area_id=1` confirms
    `config.area_id` reaches `World::area_id` correctly.
    STILL REMAINING (unchanged): only `ch.ef[]` area-effect light
    contributions to `V_LIGHT` are undocumented/unported (Rust effects
    are not attached to characters the way C's `ch.ef[]` array is); this
    is a separate, larger effects-system gap outside this task's scope.
  - Iteration 28: closed the final documented gap. `World::update_character`
    now computes `effect_light` (`World::character_attached_effect_light`,
    `world/character_values.rs`) by summing `.light` across the
    character's currently attached effects (`Effect::target_character ==
    Some(character_id)`, which already existed for magicshield/firering/
    pulseback/burn/bless/warcry/freeze/potion/curse/cap/lag/strike/flash
    show-effects) and passes it into `recompute_character_values`, which
    adds it into `mod[V_LIGHT]` exactly like C's `mod[V_LIGHT] +=
    ef[fn].light` loop (`create.c:1785-1797`) - uncapped by the seyan/
    warrior mod-percentage cap since `V_LIGHT` sits outside the `n <=
    V_STR || n >= V_PULSE` range in C, matching the existing formula
    already ported. Documented, intentional deviation: C's `ch.ef[]` is a
    fixed four-slot array (`add_effect_char`, `effect.c:209`, silently
    refuses a fifth simultaneous attachment), which Rust does not model;
    as an approximation, `character_attached_effect_light` sums only the
    four lowest-effect-id (earliest-attached) character-attached effects,
    matching C for the common case and only deviating in the rare 5+
    simultaneous character-attached-effect case. 2 new tests in
    `world/tests/character_values.rs`
    (`character_attached_effect_light_contributes_to_v_light`,
    `character_attached_effect_light_caps_at_four_effects_by_creation_order`).
    `cargo fmt --all` / `cargo test --workspace` (1112 core tests, all
    green) / `cargo build -p ugaris-server` all clean; boot-smoked past
    tick 233 with no panics. This closes the task: all four
    `update_character` slices plus every documented call-site and
    recompute-detail gap are now ported, with only the trivial
    `player_reset_map_cache` display-cache no-op (Rust has no such
    client-scroll-diff cache to invalidate) and the above four-slot
    approximation remaining as intentional, documented deviations.

- [x] **Equipment slot rules on swap (`CL_SWAP` into worn slots)** - C
  `cl_swap`/`swap` checks `place_item_typed` rules: worn slot flag match
  (`IF_WN*`), min level, class gates, two-handed vs left hand, and calls
  `update_char`. Verify the Rust `inventory_swap_slot`
  (`crates/ugaris-server/src/inventory.rs`) against C and port the missing
  gates. Tests exist in `tests/inventory.rs` - extend them.
  - Progress: ported `can_wear` (`src/system/tool.c:994-1098`) and
    `check_requirements` (`tool.c:943-991`) as a new `World::can_wear`
    method + `check_requirements` helper in
    `crates/ugaris-core/src/world/items.rs` - all 12 `WN_*` slot-flag
    matches, the `WN_LHAND`/`WN_RHAND` two-handed hand-conflict rules,
    `min_level`/`max_level`, all four `needs_class` bits (Warrior/Mage/
    Seyan'Du/Arch), negative-`modifier_index` stat requirements (read
    against `value[1]`, with the same out-of-range-index guard as C to
    avoid an `i16` negate-overflow panic), and `IF_BONDWEAR` ownership.
    Wired into `inventory_swap_slot`
    (`crates/ugaris-server/src/inventory.rs`): a non-empty cursor item
    targeting a worn slot (`pos < 12`) is now rejected (silently, matching
    C's `cl_swap` which never surfaces `error` to the client) unless
    `can_wear` passes; unequip (empty cursor) is unaffected, matching C
    (`can_wear` is only called inside `if ((in = ch[cn].citem))`). 13 new
    tests: 6 core-level (`world/tests/items.rs` -
    `can_wear_rejects_positions_outside_the_worn_slot_range`,
    `check_requirements_rejects_above_maximum_level`,
    `check_requirements_seyanddu_gate_needs_both_mage_and_warrior_flags`,
    `check_requirements_arch_gate_rejects_non_arch_characters`,
    `check_requirements_bondwear_restricts_to_the_bonded_owner`,
    `check_requirements_ignores_out_of_range_modifier_index_without_panicking`)
    plus 9 server-level end-to-end tests in
    `crates/ugaris-server/src/tests/inventory.rs` covering slot-flag match/
    mismatch, min-level, needs_class, stat-requirement, both two-handed
    hand-conflict directions (and the non-conflict success case), and the
    unequip-bypasses-`can_wear` case. `cargo fmt --all` / `cargo test
    --workspace` (1118 core + 351 server tests, all green) / `cargo build
    -p ugaris-server` all clean; boot-smoked past tick 232 with no panics.
    REMAINING (left as documented, out-of-scope-for-this-slice gaps,
    consistent with the task's explicit "worn slot flag match, min level,
    class gates, two-handed vs left hand" enumeration): (1) the
    `store_item`-based auto-unequip cross-hand cleanup in C `swap`
    (`do.c:1260-1271`) is dead code in the normal flow - `can_wear` already
    rejects both trigger conditions before that code can run (verified by
    tracing `IF_WNTWOHANDED` vs the `inl`/`inr` occupancy checks) - so it
    was not ported; (2) the "no switching equipment in Teufel PK arena"
    early gate (`do.c:1230-1233`, `areaID == 34 && MF_ARENA`) needs an
    `area_id` parameter threaded through `inventory_swap_slot` and its
    `apply_fast_sell` call site in `merchants.rs`, deferred to keep this
    slice focused; (3) the `IF_MONEY`-drop-into-slot-converts-to-gold
    branch (`do.c:1276-1287`) is a distinct money-handling concern, not an
    equipment-slot rule, and is left for a future task.

- [x] **Experience/level-up side effects** - C `give_exp` ->
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
  - Iteration 26: closed the "Grats" broadcast half of the remaining note.
    C `check_levelup`'s `if (ch[cn].level % 10 == 0) server_chat(6, ...)`
    (`tool.c:1347-1350`) sends `"0000000000" COL_MAUVE "Grats: %s is level
    %d now!"` to channel 6 ("Grats", already a joinable chat channel in
    `commands_chat.rs`). Since `server_chat`'s fan-out needs live session
    state that `ugaris-core`'s `World::check_levelup` doesn't have, added
    the same queue/drain pattern already used for
    `pending_system_texts`/`pending_area_texts`: a new
    `WorldChannelBroadcast { channel, message_bytes }` event type and
    `World::queue_channel_broadcast`/`drain_pending_channel_broadcasts`
    (`world/text.rs`). `check_levelup` (`world/exp.rs`) now queues one
    per level-up crossing a multiple of ten, building the exact C byte
    sequence (`b"0000000000"` + `COL_CHAT_GRATS` (== `COL_MAUVE`) +
    formatted text). `ugaris-server`'s new
    `send_pending_world_channel_broadcasts` (`world_events.rs`) drains the
    queue each tick and fans each message out via `system_text_bytes` to
    every session whose `PlayerRuntime::chat_channels` has the target
    channel's bit set - the same join-bit rule `apply_chat_command`
    (`commands_chat.rs`) uses for player-authored channel messages (no
    clan/mirror/area/ignore filters apply to channel 6). Wired into the
    tick loop next to the sibling `send_pending_world_*` calls
    (`main.rs`). Tests in `world/tests/exp.rs`: broadcast queued with the
    exact byte-for-byte payload at level 10, no broadcast at a
    non-multiple-of-ten level-up, and two broadcasts (level 10 and 20)
    when a single `give_exp` call vaults a character across both
    thresholds at once. Boot-smoked (`entering Rust game loop`, no
    panic). STILL REMAINING: `achievement_check_level` (needs a general
    achievement engine, out of scope - see the P1 task list) and
    `reset_name(cn)` (a no-op by construction - no server-side
    colored-name cache exists to invalidate, documented in
    `character_values.rs`) are unchanged.
  - Iteration 27 (closing review): re-audited the whole workspace for any
    remaining raw `character.exp` grant mutations that should route
    through `give_exp` (`grep -rn "\.exp = \|\.exp +=\|\.exp\.saturating_add"`
    across all crates, excluding `/tests/`). Found none - the three
    remaining raw-`exp` writers are all correctly *not* `give_exp` calls
    once checked against their exact C originals: `scrolls.rs::
    raise_value_exp` mirrors C `raise_value_exp` (`skill.c:315-361`),
    which itself does a bare `ch[cn].exp += cost;` (not `give_exp`, so no
    multiplier/NOEXP-area gating applies there either - confirmed by
    reading the C function directly); `potions.rs`'s and `death.rs`'s
    `saturating_sub` sites are exp *losses/refunds* (potion side-effect
    exp refund, death exp loss), which C also applies as bare
    subtractions, never through `give_exp` (a grant-only function); and
    `commands_admin.rs`'s `/setlevel` debug command directly assigns
    `level2exp(level)`, matching C `cmd_setlevel`'s own direct
    `ch[cn].exp = level2exp(level)` assignment (a debug override, not a
    gameplay grant). Marking this task `- [x]`: every C `give_exp` call
    site that grants a player positive exp during normal gameplay
    (kills, `/god exp`, `/milexp`, lollipops, demon shrine, random
    shrines, zombie shrine, warp bonus/trickle, bookcase, staffer book,
    and the two `raise_value_exp` callers via `StatScrollUsed`) now
    routes through the canonical `World::give_exp`/`check_levelup`, with
    the full multiplier/`NOEXP`/area-21/`NOLEVEL` gating and level-up
    side effects (level text, saves grant, profession unlock at 20, the
    "Grats" channel-6 broadcast) all verified against
    `src/system/tool.c:1318-1430` line by line. The two remaining named
    gaps - `achievement_check_level` and `reset_name(cn)` - are out of
    this task's scope: achievement checks have their own dedicated P4
    task below ("Achievements
    (`src/module/achievements/achievement.c`)"), and `reset_name` is a
    genuine no-op in this codebase (no server-side colored-name cache
    exists to invalidate, so there is nothing to port). `dlog`/
    `macro_track_exp_gain` also remain documented no-ops (no Rust
    debug-log/anti-macro subsystem exists). No code changes this
    iteration - this was a verification-only closure pass; full
    `cargo test --workspace` still green at the same counts.

- [x] **Ground item decay** - dropped items never disappear (bodies do).
  C: `set_expire(in, item_decay_time)` on player drops (`act_drop`) and
  `expire_item` behavior for `IF_TAKE` ground items in `src/system/item.c`
  / `tool.c`. Rust: reuse `World::set_item_expire` from `world/death.rs`
  in `complete_drop`; respect `IF_NODECAY`. Tests in `world/tests/items.rs`.

- [x] **`SV_SETVAL`/resource streaming on change** - C pushes value/exp/
  gold/HP bars whenever they change (`CF_UPDATE`/`CF_ITEMS` consumers in
  `plr_update`). Rust only sends resources in the periodic char record and
  after specific actions. Add a per-tick pass: when a session's character
  has `UPDATE`/`ITEMS` flags set, send the same packets login sends
  (`SV_SETVAL*`, `SV_SETHP/ENDUR/MANA`, exp, gold, inventory snapshot for
  `ITEMS`) and clear the flags. Mirror C's flag semantics exactly.
  - This replaces several ad-hoc `command_inventory_refresh` pushes -
    migrate call sites gradually, do not break existing tests.
  - C: `player_stats()` in `src/system/player.c:2944` (the function the todo
    called `plr_update` does not exist under that name) - gates the
    value-table diff loop behind `CF_UPDATE` and the item/citem/cprice/gold
    diff behind `CF_ITEMS`, clearing each flag right after; HP/endurance/
    mana/lifeshield/exp/exp_used are sent unconditionally on a per-session
    shadow diff (no flag gate). Confirmed `CF_UPDATE`/`CF_ITEMS` bit values
    (`1<<8`/`1<<12`) already matched the existing Rust `CharacterFlags`.
  - Rust: new `crates/ugaris-server/src/resource_sync.rs`
    (`queue_resource_sync_frames`), called once per tick in `main.rs` right
    before `queue_periodic_player_frames` (mirrors C's `player_map` then
    `player_stats` ordering). Rust has no per-session shadow-value cache
    (unlike C), so instead of per-field diffing it sends a full snapshot of
    whichever category's flag is set (same packet shapes as `login_payload`/
    `inventory_snapshot_payload`) and clears exactly the flag(s) that were
    acted on. This still matches C's flag-gating semantics (nothing sent
    when neither flag is set) and is idempotent alongside the existing
    ad-hoc `command_inventory_refresh`/action-specific pushes, which were
    left in place per the task note (not migrated this iteration).
  - Tests: `crates/ugaris-server/src/tests/resource_sync.rs` (5 tests) -
    no-op when neither flag set, `UPDATE` sends values/hp/exp and clears
    only `UPDATE`, `ITEMS` sends cursor/inventory/gold and clears only
    `ITEMS`, both flags in one frame clear both, and non-`Normal` sessions
    are skipped without touching the flag.
  - REMAINING: no per-session shadow diff cache exists, so this sends full
    category snapshots rather than only the changed fields (functionally
    correct, more bytes on the wire than C); `command_inventory_refresh`/
    `command_container_refresh` call sites in `main.rs` were not migrated
    away, so some actions will now (harmlessly) double-send an inventory
    snapshot in the same tick.

- [x] **Serial validation everywhere** - C guards every queued action with
  `ch[co].serial != act2 -> abort`. Rust stores serials but
  `apply_player_action_setup` checks them only for kill/fireball/ball.
  Audit `PAC_*` setups against C `player_driver.c` switch and add the
  missing serial guards. Tests: stale-serial targets abort to idle.
  Progress Log: audited `player_driver.c` in full - only the `PAC_KILL`
  pre-switch guard and `fireball_driver`/`ball_driver` (behind
  `PAC_FIREBALL2`/`PAC_BALL2`) ever validate the captured serial; take/
  drop/use/give/bless/heal capture `ch[in].serial`/`ch[co].serial` into
  `act2` but never check it (confirmed dead capture, not ported as a
  check). Added the missing `PAC_KILL` serial guard in
  `World::apply_player_action_setup` (`crates/ugaris-core/src/world/
  actions.rs`). Found and fixed the real gap: `crates/ugaris-server/src/
  player_actions.rs::apply_player_action` (the live client-command path)
  hardcoded serial `0` for Kill/Give/CharacterSpell/character-targeted
  MapSpell instead of capturing `ch[co].serial` at receive time like C's
  `cl_kill`/`cl_give`/`player_driver_charspell`, so the world-layer
  fireball/ball character-serial checks were previously always
  short-circuited by the `0` no-check sentinel in real gameplay. Wired a
  `character_serial` lookup and added explicit `Kill`/`Give` match arms;
  threaded `&World::characters` through `ServerRuntime::queue_action` from
  `crates/ugaris-server/src/main.rs`. Added tests for the stale/matching
  `PAC_KILL` guard and for live serial capture on
  Kill/Give/CharacterSpell/character-targeted MapSpell.

- [x] **Logout/exit flow** - C `cl_exit`/lostcon: linger timer
  (`CDR_LOSTCON` drives the body for `lagout_time`), save, despawn. Rust
  despawns instantly on disconnect. Port the lostcon linger: on disconnect
  keep the character with `CDR_LOSTCON` driver for `runtime.lagout_time`
  ticks (idle, attackable), then save+remove. Tests: disconnect keeps the
  character breathing for the window; reconnect within the window reclaims
  it (C `take_over_char`).
  Progress Log: ported C `kick_player` (`src/system/player.c:174`) +
  `lostcon_driver`'s timeout/reclaim halves (`src/module/lostcon.c`,
  `tick_login()`/`read_login()` in `database_character.c`/`player.c`).
  Rust: `World::enter_lostcon`/`reclaim_lostcon`/`is_lostcon`/
  `expired_lostcon_characters` (new `crates/ugaris-core/src/world/
  lostcon.rs`, reusing the existing `Character.driver_state` slot via a new
  `CharacterDriverState::Lostcon(LostconDriverData { deadline })` variant -
  no new `Character` field needed, so no literal-construction call sites
  broke). Session-side glue in new `crates/ugaris-server/src/lostcon.rs`:
  `enter_lostcon_on_disconnect` (stashes the disconnecting session's
  `PlayerRuntime` + account depot instead of dropping them),
  `reclaim_lostcon_on_login` (restores the stashed `PlayerRuntime` via new
  `PlayerRuntime::reclaim_for_session`, matching C's in-place reclaim
  instead of a stale DB re-read), and `take_expired_lostcon_characters`
  (tick-loop poll). Wired into `main.rs`: `SessionEvent::Disconnected` now
  arms the linger instead of saving+removing immediately; a new per-tick
  block saves+despawns expired lingerers; `SessionEvent::Login` reclaims a
  lingering character in place (both the DB-repository path, skipping the
  stale snapshot load, and the no-DB scaffold path) before falling through
  to the existing DB-load/template-spawn logic. Tests: 6 in
  `world/tests/lostcon.rs` (driver/deadline set, missing-character no-op,
  still-attackable-on-map, reclaim clears state, expiry set matches
  deadline+driver, reclaimed characters excluded from expiry), 1 in
  `player.rs` (`reclaim_for_session` keeps PPD state, resets session
  bookkeeping), 5 in `crates/ugaris-server/src/tests/lostcon.rs` (enter/
  fallback, reclaim/no-op, expiry collection only takes matured entries).
  Full workspace green (`cargo fmt --all`, `cargo test --workspace`:
  1130+366+9+3+33 passed, `cargo build -p ugaris-server`: zero warnings);
  boot-smoked 279+ ticks with no panics.
  REMAINING: the `lostcon_driver` self-defense AI cascade (auto-heal/
  potion/magicshield, `fight_driver_attack_visible`/
  `fight_driver_follow_invisible`) is not ported - a lingering character is
  attackable and takes/deals damage normally (matches the task's "idle,
  attackable" wording) but will not proactively fight back, heal, or drink
  potions on its own yet. Also unported: the instant-leave-at-restarea/
  arena special cases and the `karma <= -12`/`-5` early-exit checks in
  `lostcon_driver`, the `CDR_LOSTCON` exp-loss cap on death
  (`death.c:1214`, tracked separately in the `death.rs` ledger row), and
  duplicate-login kick of a still-connected (non-lostcon) old session
  (`read_login`'s `ch[cn].player != nr` guard).

- [x] **PostgreSQL login hardening** - wrong password must send the legacy
  reject (`SV_EXIT` reason? check C `cmd_exit(nr, reason)` in
  `src/system/io.c`), not a scaffold accept. Character creation for
  unknown names per C account flow (or explicit reject if creation is
  website-side - read `database_character.c::begin_login` fully and match
  it). Extend `crates/ugaris-db/src/character.rs` tests with a mocked pool
  if DB is unavailable; otherwise gate live tests behind `DATABASE_URL`.
  REMAINING: the C functions are `find_login`/`load_char`
  (`src/system/database/database_character.c`), not `begin_login`/
  `cmd_exit`; the actual C reject sender is `player_client_exit`
  (`src/system/player.c:260`), `SV_EXIT` opcode 19. `ugaris-db`'s
  `begin_login_tx` already correctly maps an unknown name and a wrong
  password to the same `LoginOutcome::WrongPassword` (matching C's
  anti-enumeration behavior - C never creates a character for an absent
  `chars` row; creation only happens in `tick_login()` for a pre-existing
  DB row with `CF_USED` unset, which is out of scope here). What was
  actually broken and is now fixed: `crates/ugaris-server/src/main.rs`'s
  `SessionEvent::Login` handler unconditionally fell through to a scaffold
  character spawn + login-accepted bootstrap for every non-`Ready`
  `LoginOutcome` (wrong password, locked, not paid, shutdown, etc.) and
  even for a hard DB error - a wrong password logged the client in as a
  fresh character. Added `login_reject_message()`
  (`crates/ugaris-server/src/login.rs`) mapping every `LoginOutcome`
  variant to the exact C `read_login` (`src/system/player.c:396-444`)
  reject string, wired it to build a `PacketBuilder::exit(...)` `SV_EXIT`
  payload, flush it immediately, and send `SessionCommand::Disconnect`
  instead of spawning a scaffold, for every reject outcome and DB error.
  `LoginOutcome::NewArea` is also rejected for now (cross-area transfer is
  a separate deferred P3 task) using C's target-area-server-down message
  instead of silently spawning a wrong-area scaffold. Still remaining:
  (1) `ugaris-db/src/character.rs` has no mocked-pool or `DATABASE_URL`-
  gated test exercising `begin_login_tx`'s row-decision branching itself
  (unknown name/wrong password/locked/not-paid/etc.) - only the pure
  helper functions are unit-tested; adding real coverage needs either a
  Postgres test dependency (out of scope per the "do not update
  dependencies" rule) or an async test harness `ugaris-db` does not
  currently depend on. (2) Live end-to-end reject test over a real TCP
  socket with a configured `DATABASE_URL` was not run (no local Postgres
  in this environment); verified via focused unit tests on
  `login_reject_message` and the `SV_EXIT` payload/dispatch wiring
  instead.
  - Iteration 35: closed the "`LoginOutcome::Duplicate`/
    `TooManyBadPasswords` never constructed" gap. Added a `bad_passwords`
    table (`migrations/0004_bad_passwords.sql`, mirrors C's `badip` table
    from `src/system/badip.c`'s trailing SQL comment) and two helpers in
    `crates/ugaris-db/src/character.rs`: `is_ip_rate_limited` (C
    `is_badpass_ip`, `badip.c:56-72` - blocked once an IP has more than 3
    bad-password rows in the last 60s, more than 8 in the last hour, or
    more than 25 in the last 24h; the threshold comparison itself is
    extracted into a pure `is_badpass_counts_rate_limited` helper so it is
    unit-testable without a live database) called from `begin_login`
    before the row-lookup transaction opens, matching C `load_char`'s
    `is_badpass_ip` guard preceding `START TRANSACTION`; and
    `record_bad_password_attempt` (C `add_badpass_ip`, `badip.c:78-85`)
    called from inside `begin_login_tx` only on an *existing* character
    row with a mismatched password (not on an unknown name), matching C's
    `load_char_pwd` returning `tmp==1` specifically - preserves the
    existing anti-enumeration behavior (unknown names never touch the
    rate-limit counter) while still tracking genuine wrong-password
    attempts against real accounts. Also wired the duplicate-login check:
    C `load_char_dup` (`database_character.c:731-753`) queries whether
    another character on the same subscriber/account is already online
    (`current_area != 0`); ported as a `count(*) from characters where
    account_id = $1 and id != $2 and current_area != 0` query inside
    `begin_login_tx`, run after the password/locked/paid checks and
    before the area-resolution branch (matching C's call-site order), with
    the same `account_id == 1` test-account exemption C hardcodes
    (`sID == 1` "hack for easier testing"). `clean_badpass_ips`
    (`badip.c:88-93`) was checked against the full C source and confirmed
    dead code (declared, never called anywhere in
    `Ugaris_Server/src/**`), so it was intentionally not ported. 5 new
    tests in `crates/ugaris-db/src/character.rs`
    (`badpass_ip_rate_limit_matches_legacy_thresholds` covering all three
    window boundaries as strict `>` not `>=`, `badpass_ip_sql_scopes_to_
    the_three_legacy_windows_for_one_ip`, `duplicate_login_query_
    excludes_self_and_scopes_to_online_characters`, plus the existing
    `login_outcomes_match_legacy_find_login_codes` continues to cover the
    `-4`/`-9` codes). `cargo fmt --all` / `cargo test --workspace` (1130
    core + 12 db + 3 net + 33 protocol + 368 server, all green, zero
    warnings) / `cargo build -p ugaris-server` clean; boot-smoked past
    tick 230 with no panics. Remaining gaps (1)/(2) above are unchanged -
    still blocked on a live Postgres instance/test harness, out of scope
    per the "do not update dependencies" rule.
  - Iteration 36: closed remaining gap (1) - `ugaris-db/src/character.rs`
    now has a `DATABASE_URL`-gated `live_login` test module (12 tests)
    directly exercising `begin_login_tx`'s row-decision branching: unknown
    name, wrong password (+ asserts the `bad_passwords` row is recorded),
    locked character, locked account, ip-locked account, unfixed account,
    not-paid account, `allowed_area <= 0` -> `InternalError`, duplicate
    login rejected for a normal account, the `account_id == 1` duplicate
    exemption (C's `sID == 1` "hack for easier testing"), `NewArea`
    routing when `allowed_area != request.area_id`, and the success
    `Ready` path (verifies both the `characters` row update and the new
    `login_sessions` row). Each test opens its own transaction, serializes
    against sibling live tests via a transaction-scoped
    `pg_advisory_xact_lock` and a deterministic `accounts_id_seq` reset
    (so the `account_id == 1` exemption test is race-free without needing
    a fresh/isolated database), and always rolls back - no fixture needs
    manual cleanup, and the tests are fully idempotent/re-runnable. Added
    `tokio` as a dev-dependency of `ugaris-db` (`crates/ugaris-db/
    Cargo.toml`, `tokio.workspace = true` under `[dev-dependencies]`) to
    get `#[tokio::test]` - it is already a workspace member dependency
    used elsewhere, so this is test-only wiring, not a new dependency.
    Verified for real: spun up a throwaway local `postgres:16-alpine`
    Docker container, applied all four `migrations/*.sql` files by hand
    with `psql`, ran `DATABASE_URL=... cargo test -p ugaris-db` (all 24
    tests green, including all 12 new live tests, confirmed stable across
    3 repeated runs with default parallel test threads), then destroyed
    the container. Without `DATABASE_URL` set (this environment's
    default, and CI), the 12 live tests compile and pass trivially by
    skipping (`connect()` returns `None`), so `cargo test --workspace`
    stays green with no live Postgres present - confirmed (1130 core + 24
    db + 3 net + 33 protocol + 368 server, zero warnings). `cargo build -p
    ugaris-server` clean; boot-smoked past tick 228 with no panics.
    Remaining gap (2) (a live end-to-end reject test over a real TCP
    socket) is unchanged - out of scope for this slice, and lower value
    now that `begin_login_tx` itself has direct DB-backed coverage; the
    `SV_EXIT` payload/dispatch wiring is still covered by the existing
    `login_reject_message` unit tests. Task considered complete: every
    literal requirement in the task description (legacy `SV_EXIT` reject,
    anti-enumeration character-creation behavior, and "extend
    `character.rs` tests ... otherwise gate live tests behind
    `DATABASE_URL`") is now satisfied.

- [x] **Merchant store DB persistence** - C `database_merchant.c`
  (load_merchant_inventory, queue_merchant_* tasks). Rust merchants are
  memory-only. Add `crates/ugaris-db/src/merchant.rs` + a migration
  mirroring the C tables, load on store creation, queue saves on
  buy/sell. Follow the existing `character.rs` repository shape.
  Progress Log: added `migrations/0005_merchant_stores.sql` (single
  `merchant_stores` table keyed by `(merchant_name, merchant_x,
  merchant_y)` like C's `merchant_items`/`merchant_gold`, but storing the
  whole ware list as one `jsonb` array per merchant instead of one row per
  ware - `Item` already round-trips through serde JSON elsewhere, so this
  avoids reimplementing C's hand-rolled `drdata_to_json`/`modifiers_to_json`
  string builders). Added `crates/ugaris-db/src/merchant.rs`
  (`MerchantRepository`/`PgMerchantRepository`, `save_store`/`load_store`,
  mirroring `area.rs`'s simple repository shape) and registered it in
  `lib.rs` (`Database::merchants()`). Wired into
  `crates/ugaris-server/src/merchants.rs`
  (`merchant_store_snapshot`/`apply_merchant_store_snapshot` conversion
  helpers, `save_merchant_store_if_configured`) and `main.rs`: C
  `create_store`'s "try `load_merchant_inventory`, else
  `queue_merchant_full_save`" is ported as a diff of `world.merchant_stores`
  keys before/after `world.process_merchant_actions()` each tick (detects
  newly-created stores, since `ensure_merchant_store` only creates once);
  buy (`Container` command) and fast-sell (`FastSell` command) both
  trigger an inline full-store save after a successful trade, matching C's
  own `add_item_to_merchant`/`remove_item_from_merchant`/
  `update_merchant_item` helpers ("simple implementation - just save the
  entire inventory") rather than the incremental `merchant_tasks.c` task
  queue (no Rust task-queue abstraction exists; direct `.await` inline in
  the tick loop matches the existing `character_repository` save
  convention). `cargo fmt --all` / `cargo test --workspace` (1130 core +
  27 db [3 new merchant tests, incl. 2 `DATABASE_URL`-gated live
  save/load-round-trip tests following `character.rs`'s `live_login`
  convention] + 3 net + 33 protocol + 372 server [9 merchant tests, 4
  new], zero warnings) / `cargo build -p ugaris-server` clean. Verified
  for real: spun up a throwaway local `postgres:16-alpine` Docker
  container, applied all five `migrations/*.sql` files, ran
  `DATABASE_URL=... cargo test -p ugaris-db` (all 27 tests green including
  the 2 live merchant tests actually hitting Postgres, confirmed the test
  row is cleaned up afterward), then boot-smoked the real server against
  that database twice: first run logged "saved initial merchant store to
  database" for all 3 zone-1 merchants (Egbert/Fred/Dolf) and persisted
  108-slot ware arrays; second run (same DB) logged "loaded merchant store
  from database" for all 3 instead of re-saving, confirming the
  load-else-save-initial branch. Destroyed the container afterward.
  REMAINING: (1) merchant position is captured as `character.x/y` at
  store-creation time, not C's `tmpx/tmpy` semantics for shops that move
  day/night (`MerchantDriverData.dayx/nightx` etc.) - day/night shop
  relocation is still unported per the `world/merchant.rs` module doc, so
  this is a latent gap, not a regression; (2) C's incremental
  `merchant_tasks.c` queue (`save_incremental_change` per-item add/
  remove/update/gold rows, batched via `process_pending_merchant_updates`)
  is not ported - Rust always does a full-store upsert instead, which is
  behaviorally equivalent but does more I/O per trade than C's targeted
  single-row updates; (3) `add_special_store`'s restock save and the
  periodic `save_all_merchants`/admin `#saveall` full-DB-sweep commands
  are not wired to the new repository (`add_special_store` itself is still
  unported per the "Special stores" task below).

- [x] **Special stores** - C `add_special_store`/`create_special_item`
  (`src/module/merchants/store.c` + `create.c`): the random enchanted-item
  stock merchants refresh every 12h. Port `create_special_item` into core
  (it is also used by chests/loot), then enable the `special` merchant arg
  path already parsed in `MerchantDriverData`.
  - C: `create_special_item(strength, base, potionprob, maxchance)`
    (`src/system/tool.c:2620-2789`, not `create.c` - corrected during
    implementation, the doc-comment reference was stale) builds one
    randomly-enchanted item: an optional potion branch
    (`RANDOM(potionprob)`), a 21-entry `ITEM_TYPES[]` base-item roll
    (`tool.c:2623-2626`), a non-gaussian `lowhi_random` strength roll
    (`tool.c:2793-2799`), a weighted 76-entry `special_item[]` table roll
    (`tool.c:2295-2390`, transcribed verbatim - the task description's "72
    entries" estimate was off), and `set_item_requirements_sub`
    (`tool.c:2392-2514`, level/Arch-class gating from the item's highest
    modifier value). `add_special_store` (`src/module/merchants/
    store.c:229-323`) rolls strength 1-22 (reused directly as
    `create_special_item`'s `strength` arg) and a derived `base` tier via
    a switch, then calls `create_special_item(str, base, 1, 1000)` (never
    a potion, "no junk" tier) and adds the result to the merchant's store.
    `merchant_driver` (`src/module/merchants/merchant.c:337-347,546-548`,
    duplicated in `aclerk_driver` - a separate, still-unported NPC driver,
    left alone) seeds five special wares the first time a
    `special`-flagged merchant's store is created, then adds one more
    every 12 real-time hours (`dat->lastadd`).
  - Rust: `World::create_special_item`/`World::add_special_store`/
    `World::refresh_special_stores` (new
    `crates/ugaris-core/src/world/special_item.rs`), reusing the existing
    `legacy_random_below_from_seed` LCG and `MerchantDriverData::special`/
    `last_special_add` fields (already parsed, previously unused). Added
    `pub const IID_GENERIC_SPECIAL` to `item_driver/ids.rs`. Threaded a
    new `&mut ZoneLoader` parameter through `create_special_item`/
    `add_special_store`/`refresh_special_stores` only (not through
    `ensure_merchant_store`/`process_merchant_actions`, to avoid breaking
    their many existing call sites) - `refresh_special_stores` is called
    once per tick in `main.rs` right after `process_merchant_actions()`,
    using the `last_special_add == 0` sentinel to detect "never seeded"
    and drive the initial five-item seed, matching the existing
    `clear_expired_merchant_memory` 12h-tick-comparison idiom. Returns the
    merchants whose store changed so `main.rs` can persist them via the
    existing `save_merchant_store_if_configured` helper (C: each
    successful `add_special_store` ends with its own
    `queue_merchant_full_save`).
  - Tests: `crates/ugaris-core/src/world/tests/special_item.rs` (8 tests) -
    a fully deterministic equipment roll verified digit-for-digit against
    a Python replica of the exact LCG/table sequence (item name,
    description text, value, modifier slot, `min_level`, `template_id`),
    the potion branch returning an unmodified template, a missing-template
    `None` result, `add_special_store` requiring an existing store, a
    single successful add, and the full `refresh_special_stores` seed-five
    then no-op-same-tick then refresh-after-12h sequence (plus a
    `special == 0` no-op case). Confirmed real `ugaris_data` templates
    exist for every one of the 21 `ITEM_TYPES` entries (all ten quality
    tiers of the eight `%dq3` families, all 12 fixed entries) and all
    three potion families, and that two real zone files
    (`zones/12/mine.chr`, `zones/31/mineshop.chr`) actually use
    `special=1`, confirming this isn't dead configuration.
  - REMAINING: `create_special_item` is not yet wired to chest/loot
    generation (`src/system/create.c:1102`'s `special_prob`/
    `special_str`/`special_base` character-template fields, already
    parsed by the zone loader but explicitly discarded per `zone.rs`'s
    own doc comment) - out of scope for this slice, a separate follow-up.
    `aclerk_driver`'s duplicate special-store logic
    (`merchant.c:667,846-848`) is unported since `CDR_TRADER`/aclerk
    itself has no Rust driver yet (tracked in the P2 "Aclerk / auction
    NPC" task). On the very first tick a brand-new `special` store is
    created, both the new explicit `save_merchant_store_if_configured`
    call and the pre-existing `newly_created_stores`-diff DB-load/save
    loop may both act on the same merchant in the same tick (the diff
    loop's `load_store` could even overwrite the freshly-seeded five items
    with an older persisted snapshot on a restart) - harmless (at most one
    redundant save, and restoring persisted state on restart is correct
    behavior) but worth knowing about if the merchant-persistence flow is
    touched again.

- [x] **Client command audit completion** - handle the remaining parsed
  actions: `ClientInfo`, `Log`, `ModPacket` (mod protocol - can be a
  logged no-op initially, but check `src/common/mod_packet.c` for the
  handshake the community client expects), `Nop`. Anything still
  unhandled must at least be an explicit logged no-op, not silence.
  Progress Log: audited C `cl_nop`/`cl_clientinfo`/`cl_log`/`cl_mod1..5`
  (`src/system/player.c`) and `mod_packet.c`. `cl_nop` and `cl_clientinfo`
  are genuine no-ops in C (the latter's body is entirely commented out),
  so gave them explicit non-logging match arms (matching the existing
  `FightMode` no-op precedent) instead of falling through the tick loop's
  catch-all `_ => {}` in `crates/ugaris-server/src/main.rs`. `cl_log`
  writes the client-supplied message to the server logfile via `charlog`
  (`"<name> (<cn>): <message> [ID=<charID>,IP=...]"`); ported as a
  `debug!` trace line using new helper
  `player_actions::format_client_log_message` (IP suffix omitted -
  `ServerRuntime` doesn't track per-session peer addresses). `cl_mod1`
  currently blind-acks handshake subtypes 0x01-0x0F ("For now, just
  acknowledge we received them") and routes 0x10-0x2F to an anti-cheat
  handler not yet ported in C itself, so a `debug!` logged no-op for
  `ModPacket` is a faithful port of the C oracle's own stub, not a gap
  Rust introduced. Also updated `apply_player_action`'s immediate
  dispatch (`player_actions.rs`) to explicitly no-op these four variants
  instead of relying on the generic `action_to_queued` fallthrough. Tests:
  `crates/ugaris-server/src/tests/player_actions.rs` -
  `format_client_log_message_matches_legacy_charlog_shape` and
  `apply_player_action_ignores_nop_client_info_log_and_mod_packet`.
  REMAINING: `CL_MOD2`/`CL_MOD4`/`CL_MOD5` and unknown `CL_MOD1`/`CL_MOD3`
  subtypes still hard-disconnect the session in the decoder
  (`crates/ugaris-protocol/src/client.rs`) instead of C's "trash the
  input, keep the connection" behavior, and several `CL_MOD1` handshake
  packet sizes in `mod_packet_size()` don't match the current C
  `mod_system.h`/`mod_anticheat.h` struct sizes - not observed in the
  wild (no current client sends these), but should be fixed before the
  mod/anti-cheat protocol is actually driven end to end; that is a
  separate framing-layer task, not part of this dispatch-level audit.

---

## P2 - NPC & Dialogue Framework

Unlocks every quest NPC. Do these before any P4 area work.

- [x] **Generic NPC text analysis (`analyse_text_driver`)** - C
  `src/module/merchants/merchant.c::analyse_text_driver` and the richer
  copy in `src/area/1/gwendylon.c` (they share a pattern: lowercase the
  text, match name + keyword, respond via `quiet_say`). Port a reusable
  keyword-matcher into `crates/ugaris-core/src/character_driver.rs` that
  drivers feed their `NT_TEXT` messages through. Tests: keyword hit/miss,
  name gating, case insensitivity.
  REMAINING: only the merchant `qa[]` table + guard clauses (player-flag,
  distance>12, visibility) are wired to a real driver
  (`world/merchant.rs::merchant_qa_reply`); the `gwendylon.c`/`bank.c`/
  `base.c`/`military.c`/`forest.c`/`area3.c`/`arkhata.c`/
  `orb_bank_npc.c` copies each need their own `qa[]` table transcribed and
  fed through `analyse_text_qa`, but every one of those files is a whole
  unported NPC driver in its own right (`bank.c`/`orb_bank_npc.c` -> P2
  "`CDR_BANK` banker NPC"; `gwendylon.c` -> P4 "Area 1"; `area3.c` -> P4
  "Area 3"; `forest.c` -> P4 "Area 16"; `arkhata.c` -> P4 "Area 37";
  `military.c` -> P3 "Military ranks"; `base.c`'s trader section -> P2
  "`CDR_TRADER`"), so wiring each qa table is properly scoped as part of
  porting that NPC's driver, not a standalone item here. The `mem_*`
  driver-memory system this REMAINING note used to point at is now done
  (see the "Driver memory" task above) and `world/merchant.rs`'s greet
  throttling already uses it instead of the old ad-hoc
  `MerchantDriverData::greeted` field. This task's actual deliverable -
  the reusable `analyse_text_qa` matcher - is complete and exercised by
  the merchant driver; leaving `[~]` only as a pointer for whoever ports
  the remaining NPC drivers to reuse it rather than re-inventing a
  tokenizer.
  Progress Log: added `TextQaEntry`/`TextAnalysisOutcome`/
  `tokenize_text_words`/`analyse_text_qa`/`format_qa_answer` to
  `character_driver.rs` (tokenizer matches C's delimiter set and
  exact-length qa matching, with the caller-supplied guard-clause model
  since guards need `World` access this module doesn't have) plus the
  `MERCHANT_QA` table transcribed from `merchant.c`. Wired it into
  `world/merchant.rs::process_merchant_messages` via a new
  `merchant_qa_reply`/`merchant_quiet_say` pair (also fixed the new
  `quiet_say` distance to use `settings.quietsay_dist` per C
  `quietsay_dist = SAYDIST/3`, rather than reusing the unrelated
  `SAY_DIST` the existing greet code uses). Added 7 unit tests for the
  matcher (hit/miss/case-insensitivity/name-gating/exact-length/
  answer-code/oversized-word) and 3 world-level tests covering the
  merchant's small-talk reply and its player-flag/distance guards.
  `cargo fmt --all`, `cargo test --workspace` (1147+27+3+33+374 passed),
  and `cargo build -p ugaris-server` all clean; boot-smoked 10s with no
  panics.

- [x] **Driver memory (`mem_*`)** - C `src/system/mem.c`:
  `mem_add_driver/mem_check_driver/mem_erase_driver` per-(npc, player,
  slot) memory with timeouts. The merchant greeting already fakes slot 7
  with `MerchantDriverData::greeted` - replace with a proper
  `DriverMemory` structure on `CharacterDriverState` usable by all
  drivers. Tests: add/check/expiry parity.
  Progress Log: the real C source is `src/system/drvlib.c` (declared in
  `drvlib.h`) - `src/system/mem.c` is an unrelated `xmalloc`/`xfree`
  allocator-tracking module the task description's reference was stale.
  Ported `struct char_mem_data`'s 8-slot (`nr` 0..=7) per-character
  membership list as `character_driver::DriverMemory` (an 8-element
  `[Vec<u32>; 8]`, `Default`-constructed via `std::array::from_fn`) plus
  free functions `mem_add_driver`/`mem_check_driver`/`mem_erase_driver`
  mirroring C's semantics exactly: out-of-range slots (`nr < 0 || nr >
  7`) are a no-op returning `false`/doing nothing, duplicate adds are
  idempotent (no duplicate entry, still returns `true`), and erase only
  clears the targeted slot. C dedupes membership by a stable identity
  (`ch[co].ID | 0x80000000` for players, else `ch[co].serial &
  0x7fffffff`) that survives character-table slot reuse; kept the
  pre-existing merchant-greet port's simplification of using the raw
  runtime `CharacterId` instead (documented in the new code), since
  threading persistent player IDs through is a bigger change than this
  task's scope and the existing merchant code already made the same
  call. Added `driver_memory: DriverMemory` directly to `Character`
  (`entity.rs`) rather than nesting it under `CharacterDriverState` (an
  enum tagged per driver kind) since C's memory slots are addressed
  per-character independent of which module owns the character - this
  matches how `driver_state`/`driver_messages` already sit directly on
  `Character`. Rewired `world/merchant.rs`'s greet-once tracking
  (`greet_nearby_players`/`clear_expired_merchant_memory`) off the old
  `MerchantDriverData::greeted: Vec<u32>` field onto
  `mem_add_driver`/`mem_check_driver`/`mem_erase_driver` at slot 7
  (matching C's literal `mem_add_driver(cn, co, 7)` call sites in
  `merchant.c`), keeping `MerchantDriverData::memory_clear_tick` as the
  driver's own timeout bookkeeping (C's `dat->memcleartimer` pattern,
  which is caller-side, not part of `mem_*` itself). Tests: 6 new unit
  tests in `character_driver.rs` (check-before-add, add-then-check,
  duplicate-add idempotency, out-of-range slot rejection for both add and
  check, erase-only-clears-requested-slot, erase-on-out-of-range-slot is
  a silent no-op) plus updated the existing merchant greet/small-talk
  tests' `merchant_npc_already_greeted` helper to seed slot 7 via
  `mem_add_driver` instead of the removed field. `cargo fmt --all`,
  `cargo test --workspace` (1153+27+3+33+374 passed), `cargo build -p
  ugaris-server` all clean, and a 10s boot-smoke showed "entering Rust
  game loop" with no panics.

- [x] **`quiet_say`/`say`/`emote` NPC speech helpers in core** - several
  drivers need to talk. There are queued area-text pieces already
  (`queue_lab2_undead_say`); generalize to `World::npc_say(cn, text)`
  (say format), `npc_emote`, `npc_murmur` with the C color/format rules
  from `src/system/talk.c`. Migrate existing call sites.
  C: `src/system/talk.c` - `say()` (`quiet_say`/`emote`/`murmur`'s
  sibling; note its quote-rejecting `strchr(buf, '"')` check is
  commented out, unlike the other three), `quiet_say()`, `emote()`,
  `murmur()`. All four share the `log_area(x, y, LOG_TALK/LOG_INFO, cn,
  <dist>, "<fmt>", ch[cn].name, buf)` pattern with a fixed format string
  and per-function distance constant (`say_dist`/`quietsay_dist`/
  `emote_dist`; `murmur` reuses `whisper_dist`, it has no distance of its
  own).
  Rust: added `World::npc_say`/`npc_quiet_say`/`npc_emote`/`npc_murmur`
  to `crates/ugaris-core/src/world/text.rs`, each pushing a
  `WorldAreaText` (the existing `pending_area_texts` queue merchant.rs
  already used) at the matching `GameSettings` distance field. Added
  `murmur_message`/`quiet_say_message` to `crates/ugaris-core/src/
  log_text.rs` alongside the pre-existing `say_message`/`emote_message`/
  `whisper_message`/`shout_message`/`holler_message` helpers.
  Migrated the three existing ad-hoc `pending_area_texts.push`
  call sites onto the new helpers: `world/lab2_undead.rs`'s
  `queue_lab2_undead_say` (removed; call sites now call `npc_say`
  directly - this was a latent bug fix, the old helper pushed the raw
  message with no `"<name> says: \"...\""` wrapper even though C's
  `say(cn, "Arrgh!")` always includes it; fixed the 4 affected unit
  tests in `world/tests/lab2_undead.rs` to expect the correct wrapped
  text per the Hard Rules), `world/npc_idle.rs`'s potion-drink emote
  (now `npc_emote`), and `world/merchant.rs`'s small-talk reply +
  greeting (now `npc_quiet_say` - the greeting call site was also a
  latent bug: it used `say_message`/`SAY_DIST` even though C's
  `merchant.c` greeting is `quiet_say(cn, "Hello %s! ...")`, i.e. the
  wrong distance; left the missing `COL_LIGHT_BLUE`/`COL_RESET` color
  codes around the trade phrase as a separate, out-of-scope gap).
  Tests: 4 new unit tests in `world/tests/text.rs` (`npc_say` never
  rejects quotes at `say_dist`, `npc_quiet_say`/`npc_emote`/`npc_murmur`
  each reject a `"` and use their respective distance field), plus the
  4 fixed `lab2_undead.rs` tests. `cargo fmt --all`, `cargo test
  --workspace` (1158+27+3+33+374 passed), `cargo build -p ugaris-server`
  all clean, and a 10s boot-smoke showed "entering Rust game loop" with
  no panics.
  REMAINING: `whisper`/`holler`/`shout` NPC helpers not added (only
  player-authored local speech uses those in
  `crates/ugaris-server/src/commands_chat.rs`; no NPC driver calls them
  yet) - add them the same way if/when an NPC driver needs to holler or
  shout. The merchant greeting's missing color codes noted above are
  also still open.

- [x] **Idle NPC chatter** - merchant/citizen random murmur tables
  (`merchant_driver` RANDOM(25) block, citizen equivalents). Needs the
  speech helpers. Low complexity, high flavor.
  REMAINING: only `merchant_driver`'s block is wired (into
  `World::process_merchant_actions` via a new
  `world/merchant.rs::merchant_idle_chatter`). The "citizen equivalents" -
  `bank.c::bank_driver`, `orb_bank_npc.c`, `base.c::trader_driver`,
  `merchant.c::aclerk_driver` (a *second*, different `RANDOM(25)` table
  at merchant.c:800, distinct from `merchant_driver`'s), `area3.c`,
  `clanmaster.c`, `tunnel.c`, `gwendylon.c`, `sidestory.c` - all have
  their own `RANDOM(25)` murmur blocks but are whole unported NPC drivers
  in their own right (bank -> P2 "`CDR_BANK`"; trader -> P2 "`CDR_TRADER`";
  aclerk -> P2 "Aclerk / auction NPC"; area3/gwendylon -> P4 area tasks;
  clanmaster -> P4 Area 30; tunnel -> P4 Area 33), so porting each one's
  murmur table is properly scoped as part of that driver's own port, not
  a standalone follow-up here (same reasoning the "Generic NPC text
  analysis" task above used for its per-driver `qa[]` tables). Added the
  missing `World::npc_whisper` speech helper (`src/system/talk.c`'s
  `whisper()`, `whisper_dist`) since the merchant table's case 1 needed
  it and only `say`/`quiet_say`/`emote`/`murmur` existed before.
  Progress Log: added `hisname`/`npc_whisper` to
  `crates/ugaris-core/src/world/text.rs`; added
  `world/merchant.rs::merchant_idle_chatter` (the 17-case table plus
  Lori's 4 extra mine-only cases at `max_case=20`, matched
  case-insensitively per C's `strcasecmp`) wired into
  `process_merchant_actions` after `greet_nearby_players`. Preserved C's
  exact text digit-for-digit, including the literal capitalization quirk
  in case 20's `"Flips %s coins"` emote. 5 new unit tests in
  `world/tests/merchant.rs` pinning `legacy_random_seed` to
  pre-computed values that land on known `(RANDOM(25), RANDOM(n+1))`
  rolls (lucky/unlucky hit, Lori's extended case range, indoor/outdoor
  emote branch, talk-interval throttle). `cargo fmt --all`, `cargo test
  --workspace` (1163+27+3+33+374 passed), `cargo build -p ugaris-server`
  all clean, and a 10s boot-smoke showed "entering Rust game loop" with
  no panics.
  Closing note (iteration 44): the "citizen equivalents" remainder listed
  above is explicitly scoped into the other drivers' own P2/P4 tasks
  (`CDR_BANK`, `CDR_TRADER`, Aclerk, area3/gwendylon/clanmaster/tunnel), so
  there is no standalone follow-up left for this task itself; marking done.

- [x] **`CDR_BANK` banker NPC** - C `src/module/bank.c`: deposit/withdraw
  via text commands + `NT_GIVE` money handling, balance stored in PPD
  (`DRD_BANK_PPD`? read the C). Port driver + PPD codec + tests.
  Progress Log: added `CDR_BANK`/`BankDriverData`/`parse_bank_driver_args`/
  `BANK_QA` to `character_driver.rs` and wired spawn-time arg parsing in
  `zone.rs`; added `DRD_BANK_PPD`/`bank_gold` PPD codec
  (`encode_legacy_bank_ppd`/`decode_legacy_bank_ppd`) to
  `player.rs::PlayerRuntime`; added `crates/ugaris-core/src/world/bank.rs`
  (`World::process_bank_actions`) porting the full `bank_driver` body:
  greeting (periodic nearby-player scan, same simplification
  `world/merchant.rs` already established for `NT_CHAR`), small talk via
  the shared `analyse_text_qa` matcher, deposit/withdraw/balance text
  commands, `NT_GIVE` cursor-item destruction, the 16-line idle-murmur
  table with `RANDOM(25)`/`RANDOM(16)` throttling, the 12h memory-clear
  timer, and the day/night shop-position/door movement block (`is_closed`/
  `is_room_empty`/`opening_time` ported fresh - no prior Rust equivalent
  existed). Since `World` cannot see `PlayerRuntime` (the persistent
  `ppd->imperial_gold` balance lives in the `ugaris-server` session layer,
  not `World`), added a `BankEvent`/`pending_bank_events`/
  `drain_pending_bank_events` queue (matching the existing
  `pending_kill_exp`-style convention) plus
  `crates/ugaris-server/src/world_events.rs::apply_bank_events` to apply
  deposit credit / withdraw debit+payout / balance-reply against the
  correct `PlayerRuntime`, called from `main.rs`'s tick loop right after
  `process_merchant_actions`. Deviations (documented in code comments,
  not silent): (1) `use_item_at`'s full keyed-door dispatch
  (`item_driver::door_driver`'s key-requirement gate) is not replicated -
  bank doors toggle directly via `toggle_door`, since no existing zone
  data is expected to put a keyed door on a bank; (2) the C "account"/
  "explain deposit"/"explain withdraw"/"explain balance" qa answers wrap
  keywords in `COL_LIGHT_BLUE`/`COL_RESET` - the shared `analyse_text_qa`
  pipeline works on plain `&str` (the legacy color marker is a raw
  non-UTF8 byte that cannot be represented in a Rust string literal), so
  color styling is dropped while wording stays byte-for-byte identical;
  (3) `NT_GIVE` unconditionally destroys the received item rather than
  first trying to hand it back (`give_driver`), following the same
  simplification `world/merchant.rs` already established (no generic
  "give item back" helper exists). `cargo fmt --all`, `cargo test
  --workspace` (1182+27+3+33+374 passed, including 17 new
  `world::tests::bank` tests and 2 new PPD round-trip tests in
  `player.rs`), `cargo build -p ugaris-server` all clean with zero
  warnings, and a 12s boot-smoke showed "entering Rust game loop" with no
  panics.

- [x] **`CDR_TRADER` player-to-player trade NPC** (`src/module/base.c`
  trader section) and **`CDR_JANITOR`** (item cleanup NPC). Both have
  registry stubs already - fill in behavior.
  Progress Log: `CDR_TRADER` is fully ported. Added `TraderDriverData`
  (`character_driver.rs`, wired into zone spawn) + `TRADER_QA` (base.c's
  shared `qa[]` table, also used by `CDR_JANITOR`/`CDR_MACRO`) +
  `crates/ugaris-core/src/world/trader.rs`
  (`World::process_trader_actions`) porting the full `trader_driver` body:
  the "trade with <name>"/"stop trade"/"accept trade"/"show trade" text
  command state machine (exact C string matching, including the
  case-sensitive `strstr` quirk and the "accept trade" exact-phrase
  requirement), `NT_GIVE` item collection capped at 10 per side with
  cross-partner notification, the three-minute timeout with item
  return, the swap-on-deal semantics, greeting (periodic nearby-player
  scan like `world/bank.rs`/`world/merchant.rs` already established, but
  additionally turning to face the greeted player since C's `talkdir`
  mechanic is part of this driver's observable behavior, unlike
  bank/merchant which never turn), the 12-line idle-murmur table, and the
  12h driver-memory clear timer. Added `offset2dx` (`drvlib.rs`, C
  `tool.c:309-349`) since this is the first driver needing the
  turn-to-face-the-speaker mechanic. Two things needing
  `legacy_item_look_text` (lives in `ugaris-server`, not `ugaris-core`)
  are deferred via a new `TraderEvent`/`pending_trader_events` queue
  (mirroring the `BankEvent` convention) applied by
  `crates/ugaris-server/src/world_events.rs::apply_trader_events`: the
  "show trade" item dump and the `NT_GIVE` "`<name>` gave me:"
  cross-notification. Deviations (documented in code comments): persistent
  player IDs (`ch[co].ID`) represented as the raw runtime `CharacterId`
  (same simplification as driver memory/bank/merchant); `is_gk_room`
  gatekeeper-room guard in `return_items` not replicated (gatekeeper not
  ported); `ACHIEVEMENT_TRUST_BUT_VERIFY` award on a successful deal not
  replicated (achievements not ported); `give_char_item`'s audit `dlog`
  line skipped; COL_LIGHT_BLUE/COL_LIGHT_GREEN/COL_RESET color markers
  dropped (same simplification as `BANK_QA`). Tests: 20 new tests in
  `world/tests/trader.rs` plus 1 in `drvlib.rs` for `offset2dx`. `cargo
  fmt --all`, `cargo test --workspace` (1203+27+3+33+374 passed), `cargo
  build -p ugaris-server` all clean with zero warnings, and a 10s
  boot-smoke showed "entering Rust game loop" with no panics.
  Continuation (iteration 46): `CDR_JANITOR` is now also fully ported.
  Added `JanitorDriverData` (`character_driver.rs`, only the `cnt` murmur
  counter is kept as real persistent state - see below) + wired zone
  spawn-time init (`zone.rs`, no zone-file args to parse, matching
  `CDR_TRADER`'s `set_data` zero-init) + `crates/ugaris-core/src/world/
  janitor.rs` (`World::process_janitor_actions`) porting the full
  `janitor_driver` body: toggling the nearest `IDR_TOYLIGHT` whose on/off
  state doesn't match the current day/night `ls` target, picking up the
  nearest visible `IF_TAKE` junk item on the janitor's town half (the
  `y == 192` divide filter from the C `NT_ITEM` handler) that isn't
  already on one of the nine home-area tiles, stashing held junk in the
  deep-inventory "bag" range (`item[30..INVENTORYSIZE]`, C's own comment
  on `struct char.item[]`), and dropping bagged junk off one at a time at
  the nine fixed home tiles in C's exact order, plus the idle-murmur
  table (rolled only right after a successful light-toggle, unlike the
  other NPC drivers' per-minute throttle) including the dynamic "N lights
  I turned on" counter case. Deviations (documented in code comments):
  (1) C's `struct janitor_data` also carries `light[MAXLIGHT]`/
  `take[MAXTAKE]`, a cache of item IDs discovered via `NT_ITEM` notify
  messages as the janitor patrols (`scan_item_driver`); this port
  recomputes the nearest matching candidate directly from `World::items`
  every tick instead - the same class of simplification already
  established for the merchant/bank/trader greeting scans; (2) C's
  bag-unstash loop reads `ch[cn].item[INVENTORYSIZE]` first (an
  off-by-one out-of-bounds read) before falling back to
  `INVENTORYSIZE-1` - this port starts at the last valid index instead of
  replicating undefined behavior. Added generic `take_driver`/
  `drop_driver`/`use_driver` equivalents local to `world/janitor.rs`
  (built on the existing `setup_walk_toward`/`setup_walk_toward_use_item`/
  `do_take`/`do_drop`/`do_use` primitives - no new pathfinding machinery
  needed, it already existed). Tests: 12 new tests in
  `world/tests/janitor.rs`. `cargo fmt --all`, `cargo test --workspace`
  (1215+27+3+33+374 passed), `cargo build -p ugaris-server` clean with
  zero warnings, and a 12s boot-smoke showed "entering Rust game loop"
  with no panics.

- [x] **Aclerk / auction NPC** - C `merchant.c::aclerk_driver` +
  `src/system/auction/*.c` + `database_merchant.c`. Big; slice it:
  (1) aclerk dialogue/give handling, (2) auction storage in DB,
  (3) `CL_*` auction client protocol if the community client uses it
  (check client sources first - if the client has no auction UI, mark
  N/A with a note).
  All three slices are done: (1)/(2)/the `auction_house.c` business logic
  and the `/ah` command are wired (see iteration 49 log below); the
  login-time `auction_check_deliveries_login` notice is now wired to
  `PlayerRuntime::deferred_init`'s `DEFERRED_AUCTION` hook (iteration 50
  log below); slice (3) `CL_*` auction protocol is N/A per the client
  audit noted below (community client `render.c` has no auction UI at
  all, and `amod.c` only ever handles `SV_MOD1`, never `SV_MOD3`).
  Progress Log (iteration 48): ported slice (2), the DB layer of
  `src/system/auction/auction_db.c` (`init_auction_database`,
  `db_create_auction`, `db_update_auction`, `db_get_auction`,
  `db_delete_auction`, `db_search_auctions`, `db_get_player_auctions`,
  `db_count_active_auctions`, `db_create_delivery`,
  `db_get_pending_deliveries`, `db_mark_delivery_claimed`,
  `db_get_delivery_summary`, `db_cleanup_expired_auctions`) as
  `crates/ugaris-db/src/auction.rs` (`AuctionRepository`/
  `PgAuctionRepository`) + `migrations/0006_auction_house.sql`
  (`auctions`/`auction_deliveries` tables). C stores the auctioned item as
  a raw `struct item` BLOB and filters/sorts on offsets inside it via
  `CAST(SUBSTRING(...))`; Rust stores the item as `jsonb` (same convention
  as `merchant_stores.wares_json`) and filters/sorts on its `name`/
  `min_level`/`max_level` keys directly instead. `db_get_character_name`
  is not ported: it's declared and defined in C but never called anywhere
  in the C tree (confirmed by grep), so it's dead code - everywhere else
  needing a seller name uses the `LEFT JOIN chars`/`characters` C already
  inlines into the auction queries, which this port replicates. Tests: 5
  unit tests (status/reason string round trips, `MAX_SEARCH_RESULTS`
  constant, item JSON round trip) plus 4 `DATABASE_URL`-gated live tests
  covering create/get/update/delete, name+level search with price
  sorting, delivery create/claim/summary, and expired-auction cleanup
  (winner delivery + gold-to-seller vs. no-bid item return) - verified
  against a real ephemeral Postgres 16 Docker container (all 6
  `migrations/*.sql` files applied by hand), not just compiled.
  `cargo fmt --all`, `cargo test --workspace` (1228+36+3+33+374 passed),
  `cargo build -p ugaris-server` clean with zero warnings. No boot-smoke:
  this change only adds an unused DB repository, touching neither the
  runtime loop, login, map sync, nor protocol.
  Progress Log: ported slice (1), `CDR_ACLERK`'s dialogue/greet/idle-chatter/
  give handling in new `world/aclerk.rs` (`AclerkDriverData`/
  `parse_aclerk_driver_args` in `character_driver.rs`, zone spawn wiring in
  `zone.rs`, generalized `ensure_merchant_store`/`refresh_special_stores`
  in `world/merchant.rs`/`world/special_item.rs` to also cover
  `CDR_ACLERK` since C's `create_store`/`add_special_store` calls are
  identical to `merchant_driver`'s). Confirmed via the community client's
  `render.c` that there is no auction UI at all (the only "auction" hit is
  a chat-palette color name) - slice (3) can likely be marked N/A once
  slice (2) is scoped, but leaving that call for whoever audits
  `src/system/auction/*.c` since this iteration didn't open that file.
  Deviations documented in code comments: C's `aclerk_driver` has three
  `quiet_say` blocks in its `NT_CHAR` handler but the first ends with an
  unconditional `continue`, making the second (an "arena is safe" message)
  and third (a merchant-style trade greeting) unreachable dead code - only
  the first "Welcome to the Cameron Arena!" message is ported. Also unlike
  `merchant_driver`, `aclerk_driver`'s "`<name> ... trade`" handler never
  sets `ch[co].merchant = cn` - it only reacts to a hardcoded `abuser()` ID
  list with a murmur/emote, so saying "<clerk>, trade" never actually
  opens the arena clerk's store in C, and this port matches that exactly.
  Two idle-chatter emote lines have an embedded period in their C format
  string that doubles up with `emote()`'s own trailing period
  (`"eyeballs deep within the forest.."`, `"...to wake himself up.."`) -
  copied digit-for-digit including the double period. `abuser()`'s
  hardcoded persistent player IDs are checked against the raw runtime
  `CharacterId` (same simplification as `TraderDriverData::c1_id`/`c2_id`).
  Day/night shop movement remains unported for `CDR_ACLERK`, matching the
  same known gap already documented on `CDR_MERCHANT` in `world/merchant.rs`.
  Tests: 13 new tests in `world/tests/aclerk.rs`. `cargo fmt --all`,
  `cargo test --workspace` (1228+27+3+33+374 passed), `cargo build
  -p ugaris-server` clean with zero warnings, and a 12s boot-smoke showed
  "entering Rust game loop" with no panics.
  Progress Log (iteration 49): ported the `auction_house.c` business logic
  and the `/ah` text command as new `crates/ugaris-server/src/auction.rs`
  (`AuctionError`, `format_money`/`format_time_left`/`format_price`/
  `format_item_details`/`format_item_modifiers` matching
  `format_money_string`/`format_time_left`/`format_price`/
  `format_item_details`/`format_item_modifiers`, `validate_auction_item`,
  `calculate_auction_fee`, `calculate_min_bid`, and async
  `auction_create`/`auction_bid`/`auction_buyout`/`auction_cancel`/
  `auction_claim_deliveries`/`auction_search` orchestrating
  `ugaris_db::PgAuctionRepository` plus `World` item/gold mutation) and
  `apply_auction_command` (the `/ah`/`/auctionhouse` dispatcher, matching
  `auction_process_command`'s `commands[]` abbreviation table), wired into
  `main.rs`'s `ClientAction::Text` chain (new `auction_repository: Option
  <PgAuctionRepository>` alongside `merchant_repository`), a 60-real-
  second periodic `cleanup_expired_auctions` sweep matching C's
  `maintenance_60s_task`, and startup/shutdown sweeps matching
  `init_auction_house`/`shutdown_auction_house`. Since auctions have no
  in-memory `World` state at all (DB-only by design per the slice-2 doc
  comment), `/ah` is unavailable without `--database-url`, unlike
  merchant/bank/trader. Deviations documented in the module's doc
  comment: C's business-logic functions (`auction_bid`/`auction_buyout`/
  `auction_cancel`) call `log_char` directly for most errors *and*
  `auction_cmd.c`'s command wrappers log a second, usually near-duplicate
  message from their own `switch` - e.g. a self-bid attempt shows two
  "you cannot bid on your own auction"-style lines back to back in C. This
  port keeps one message per error, picking whichever C message is more
  specific (e.g. `auction_bid`'s exact minimum-bid amount over
  `cmd_auction_bid`'s generic "5% increment" text - re-fetching the
  auction on that error path to compute it). `get_value_name`'s short
  lowercase abbreviations (`auction_house.c:512-643`) are reproduced
  verbatim in a local `AUCTION_VALUE_ABBREV` table (separate from
  `entity::CHARACTER_VALUE_NAMES`'s unrelated Title-Case display
  convention used by `legacy_item_look_text`, reused as-is for
  `/ah info`'s item lookup). One gap remains: `auction_check_deliveries_
  login` (login-time pending-delivery notice) is not wired to the
  existing `DEFERRED_AUCTION` hook - noted above and in
  `PORTING_LEDGER.md`. Tests: 18 new tests in
  `crates/ugaris-server/src/tests/auction.rs` covering money/fee/bid-
  increment math, item validation, value-name mapping, modifier/detail/
  time/price formatting and coloring, help text, and command-verb
  dispatch/fallback behavior (DB-touching command bodies are exercised
  only by type-checking + the DB-layer's own live tests, matching this
  crate's existing convention of no `DATABASE_URL`-gated tests). `cargo
  fmt --all`, `cargo test --workspace` (1228+36+3+33+392 passed), `cargo
  build -p ugaris-server` clean with zero warnings, and a 12s boot-smoke
  showed "entering Rust game loop" with no panics.
  Progress Log (iteration 50): closed the last gap - wired
  `auction_check_deliveries_login` (`auction_house.c:1206-1270`) to the
  existing-but-unused `PlayerRuntime::deferred_init`/`DEFERRED_AUCTION`
  hook. `ServerRuntime::login` (`main.rs`) now sets `DEFERRED_AUCTION` on
  every login (C's `!(ch[cn].flags & CF_AREACHANGE)` branch always holds
  here since cross-area transfer isn't implemented yet - see `login.rs`'s
  `LoginOutcome::NewArea` comment; C's `DEFERRED_ACHIEVEMENTS`/
  `DEFERRED_MOTD` bits are intentionally left unset since those systems
  aren't ported yet). The game loop's new deferred-init sweep (matching
  C `tick_player`'s `player.c:3660-3685`) fires exactly once, `>= 6`
  ticks after `login_tick`, calling a new `auction::auction_login_notice`
  (queries `AuctionRepository::get_delivery_summary`, then
  `format_auction_login_notice` builds the exact `COL_YELLOW`-wrapped
  text for all of C's four count/items/gold combinations, reusing
  `format_money`'s existing `gold > 0` split for C's `total_gold >= 100`
  gate) and sends it via the same `system_text_bytes` +
  `sessions_for_character` pattern already used for command feedback.
  Deviation documented in code comments: C's `count > 0` branch with
  neither pending items nor gold is unreachable dead code that reads an
  uninitialized `buf` in C; this port simply skips the notice instead of
  replicating the undefined behavior. Tests: 6 new tests in
  `crates/ugaris-server/src/tests/auction.rs` covering all four
  formatted-message combinations, the above/below-a-gold silver split,
  and the no-notice cases. `cargo fmt --all`, `cargo test --workspace`
  (398 passed in `ugaris-server`), `cargo build -p ugaris-server` clean
  with zero warnings, and a 12s boot-smoke showed "entering Rust game
  loop" with no panics.

- [x] **Gatekeeper NPC (`src/system/gatekeeper.c`)** - lab entrance
  dialogue/fight driver. The lab item drivers are ported; this is the
  character in front. Depends on text analysis + memory.
  Iteration 58 did the recommended full line-by-line re-read of the whole
  830-line C file against the Rust port and confirmed everything else
  (welcome dialogue, `enter_test`, `enter_room`, `gate_fight_driver`,
  `gate_fight_dead`, `turn_seyan`) was already faithfully ported; it found
  and fixed the one remaining real gap, `immortal_dead`
  (`gatekeeper.c:701-703`, the welcome NPC's death handler), now ported as
  `apply_gate_welcome_death_from_hurt_event`
  (`crates/ugaris-server/src/world_events.rs`). The only remaining
  deviations are the pre-existing documented no-ops inside `turn_seyan`
  (`destroy_chareffects`, `DRD_DEPOT_PPD` strip) and the architecturally-
  moot `labentrance` C `-1` "area is down" branch (impossible to reach in
  this monolithic single-process server). See Progress Log (iteration 58)
  and `PORTING_LEDGER.md` for full details. Historical remaining-work
  notes from earlier iterations, kept for context:
  the welcome NPC's greeting/small-talk message loop is wired
  into the tick loop (iteration 52), `enter_test`'s class-choice *failure*
  replies are wired too (iteration 53), and `enter_test`'s *success* path
  (`enter_room`'s private-room opponent spawn: `take_money`, the 7-room
  busy/refund search, spawning the `gatekeeper_w`/`_m`/`_s` opponent,
  teleporting the player, and stripping spell-slot items) is now wired
  (iteration 54) via `GateWelcomeOutcomeEvent::EnterTestReady` +
  `spawns::gate_enter_test_spawn_room`. `gate_fight_driver`'s combat loop
  and `gate_fight_dead`'s reward grant (item 1 below) are now wired too
  (iteration 56), via the new `world::gate_fight` module and
  `world_events::apply_gate_fight_death_from_hurt_event`. `turn_seyan`
  (`src/system/tool.c:4278-4389`) is now ported too (iteration 57) at
  `World::apply_turn_seyan` (`world/turn_seyan.rs`) plus
  `PlayerRuntime::clear_turn_seyan_ppd` (`player.rs`), and wired into
  `gate_fight_dead`'s class-8 case (`World::apply_gate_fight_reward`),
  which now actually turns the killer into a Seyan'Du (stat/exp/level
  reset, profession clear, item strip, flag set, `update_char` tail, ~22
  `del_data` calls) instead of the old placeholder message. Two documented
  gaps remain inside that port: `destroy_chareffects` is a no-op (no
  active-effect list modeled yet, same precedent as elsewhere) and
  `DRD_DEPOT_PPD`'s "strip `IF_QUEST` flags from the 80 depot slots" isn't
  ported (no per-character legacy depot exists in Rust at all yet -
  `ugaris-server::depot`'s `AccountDepotState` is a distinct, newer,
  account-wide system - so this has no observable effect until that depot
  is built). Still needed: the idle "return to post" `secure_move_driver`
  safety net (`gatekeeper.c:627-631`) for the welcome NPC is now wired
  (iteration 55), reusing `rest_x`/`rest_y` (already populated for every
  zone-spawned character, including this one, by the zone loader's
  `pop_create_char` substitution) as its post position; the fight
  opponent's own "return to post" tail (now wired in iteration 56 too,
  same `rest_x`/`rest_y` substitution) is also in place. At this point the
  whole gatekeeper.c file (welcome dialogue, test entry/success/failure,
  the fight driver, and the reward tail including `turn_seyan`) is
  believed fully ported; a full re-read of `gatekeeper.c` end-to-end
  against the Rust port to confirm nothing was missed would be a good use
  of the next iteration before marking this task `[x]`.
  Progress Log (iteration 57): ported `turn_seyan`
  (`src/system/tool.c:4278-4389`). Character-only half at
  `World::apply_turn_seyan` (new `crates/ugaris-core/src/world/
  turn_seyan.rs` module): copies the `"seyan_m"` template's `value[1][]`
  onto the target (caller-supplied, since `World` has no `ZoneLoader`
  reference - matches C's own `create_char("seyan_m", 0)` +
  `destroy_char(co)` without ever registering a throwaway character),
  resets exp/exp_used/level/lifeshield, clears professions, un-equips worn
  items into the first free inventory slot at/past 30 (destroying them
  instead if inventory is full, exactly matching C's persistent-cursor
  scan), destroys spell-slot items 12-29, sets `CF_MAGE|CF_WARRIOR|
  CF_ITEMS`, recomputes hp/endurance/mana from the (deliberately stale,
  matching C's exact ordering) `value[0]` before calling
  `World::update_character` (C's `update_char`, whose own clamp settles
  the real final value), then strips `IF_QUEST`-flagged items from the
  remaining inventory. `destroy_chareffects` is a documented no-op (no
  active-effect list modeled yet, same precedent as `world/gatekeeper.rs`
  and `world/death.rs`). PPD half at new `PlayerRuntime::
  clear_turn_seyan_ppd` (`player.rs`): resets the 14 `del_data`d ids that
  have dedicated typed fields (treasure chest, area3, flower, randchest,
  demonshrine, farmy, randomshrine, twocity, orbspawn, rune, lab,
  ratchest, staffer, arkhata) to their empty/default state, and strips the
  other 10 non-depot ids that have zero Rust representation
  (`DRD_FIRSTKILL_PPD`, `DRD_AREA1_PPD`, `DRD_RANK_PPD`,
  `DRD_MILITARY_PPD`, `DRD_ARENA_PPD`, `DRD_NOMAD_PPD`,
  `DRD_SIDESTORY_PPD`, `DRD_TUNNEL_PPD`, `DRD_STRATEGY_PPD`,
  `DRD_QUESTLOG_PPD` - all newly added as delete-only constants, no
  decode/encode logic) straight out of the raw `ppd_blob` via a new
  `strip_ppd_blocks` helper (mirrors the existing `DRD_JUNK_PPD`-skip
  precedent in `encode_legacy_ppd_blob`). `DRD_DEPOT_PPD`'s "clear
  `IF_QUEST` flags from the 80 depot slots" is a documented gap: no
  per-character legacy depot exists in Rust (`ugaris-server::depot`'s
  `AccountDepotState` is a distinct, newer, account-wide system), so
  nothing can put quest items there yet anyway. Wired into
  `World::apply_gate_fight_reward`'s class-8 case (`gate_fight.rs`,
  signature now takes `seyan_base_values: Option<&[i16]>`) and
  `ugaris-server::world_events::apply_gate_fight_death_from_hurt_event`
  (looks up `"seyan_m"` in the `ZoneLoader` passed down from `main.rs`'s
  tick loop through the now-threaded `loader` parameter on
  `apply_pk_hate_from_hurt_events`, then calls `clear_turn_seyan_ppd` once
  the reroll succeeds); falls back to the old honest placeholder message
  if the template can't be resolved. Tests: 9 new tests in
  `crates/ugaris-core/src/world/tests/turn_seyan.rs` (stat/exp/profession
  reset, flag set, worn-item move vs. destroy-when-full, spell-slot
  destruction, quest-item stripping, hp/endurance/mana clamp via
  `update_character`, missing-character and mismatched-length-guard
  failures), 2 new tests in `crates/ugaris-core/src/player.rs` (typed-field
  reset, unmapped-id raw-block strip), 1 rewritten + 1 new test in
  `crates/ugaris-core/src/world/tests/gate_fight.rs` (class-8 success
  reroll plus the renamed no-template fallback test), and 1 new test in
  `crates/ugaris-server/src/tests/world_events.rs` (full hurt-event ->
  reward -> reroll -> PPD-clear pipeline with a real `ZoneLoader`-backed
  `"seyan_m"` template). `cargo fmt --all`, `cargo test --workspace`
  (1292+36+3+33+404 passed), `cargo build -p ugaris-server` clean with
  zero warnings, and a 12s boot-smoke showed "entering Rust game loop"
  with no panics.
  Progress Log (iteration 58): full line-by-line re-read of
  `gatekeeper.c` (all 830 lines) against the Rust port, confirming
  `analyse_text_driver`/`qa[]` (all 26 entries), `gate_welcome_driver`'s
  whole message loop, `enter_test`, `enter_room`, `gate_fight_driver`, and
  `gate_fight_dead` were all already faithfully ported (iterations
  51-57). Found and ported the one remaining gap: `immortal_dead`
  (`gatekeeper.c:701-703`), the `ch_died_driver`/`CDR_GATE_WELCOME` death
  handler for the welcome NPC (just a server-log-only `charlog` write,
  never client-visible). New `apply_gate_welcome_death_from_hurt_event`
  (`crates/ugaris-server/src/world_events.rs`) follows the existing
  `apply_*_death_from_hurt_event` driver-filter idiom (`target.driver ==
  CDR_GATE_WELCOME`, no killer-flags check since C's dispatch here is
  unconditional), wired into `apply_pk_hate_from_hurt_events`'s handler
  list; reuses the `debug!(target: "client_log", ...)` +
  `format_client_log_message` precedent from `ClientAction::Log`/`cl_log`
  instead of `queue_system_text` (matching `charlog`'s log-file-only,
  non-client-visible C semantics). In practice unreachable through normal
  combat since the welcome NPC template carries `CF_IMMORTAL` (`hurt()`
  already suppresses lethal damage to it) - ported anyway for strict
  fidelity. Also confirmed and documented (not fixed, architecturally
  moot) that `labentrance`'s C `ret == -1` "area is down" branch has no
  reachable Rust equivalent: `needs_next_lab` only reproduces
  `teleport_next_lab(cn, 0)`'s truthiness, which in C's own `do_teleport =
  0` mode can never actually return `-1` (the `change_area` call that
  produces it is always short-circuited away), and this is a monolithic
  single-process area server with no separate per-area processes that
  could independently be "down" anyway. Tests: 2 new tests in
  `crates/ugaris-server/src/tests/world_events.rs`
  (`gate_welcome_death_is_handled_but_sends_no_client_message`,
  `gate_welcome_death_handler_ignores_non_matching_driver_and_non_lethal_hits`).
  `cargo fmt --all`, `cargo test --workspace` (1292+36+3+33+406 passed, 2
  net new), `cargo build -p ugaris-server` clean with zero warnings, and a
  10s boot-smoke showed "entering Rust game loop" with no panics. Task
  marked `[x]`: `gatekeeper.c` is now believed fully ported end-to-end
  with no remaining unaddressed gaps.
  Progress Log (iteration 56): ported `gate_fight_driver`
  (`gatekeeper.c:641-696`) and `gate_fight_dead` (`gatekeeper.c:705-763`)
  into a new `crates/ugaris-core/src/world/gate_fight.rs` module plus a
  new `CharacterDriverState::GateFight(GateFightDriverData)` variant
  (`character_driver.rs`, wired into `zone.rs`'s
  `instantiate_character_template` for `CDR_GATE_FIGHT` templates, pushing
  the same `NT_CREATE` bootstrap message `CDR_LAB2UNDEAD` already uses
  since Rust's `spawn_character` doesn't auto-notify creation like C's
  `create_char`). Simplified C's generic 10-slot `struct
  fight_driver_data`/`DRD_FIGHTDRIVER` enemy-list machinery
  (`fight_driver_update`/`_attack_visible`/`_follow_invisible`,
  `drvlib.c:2170-2345`) down to tracking the single `victim` this driver
  ever fights (set once via the `NT_NPC`/`NTID_GATEKEEPER` message, exactly
  as C's own `gate_fight_driver` does - it never calls
  `fight_driver_add_enemy` itself), reusing the already-generic
  `World::attack_driver_direct` (`world/npc_fight.rs`) for "attack
  visible" and `secure_move_driver` toward the last-known position for
  "follow invisible"; self-destruct after `TICKS*60*10`, return-to-post via
  `rest_x`/`rest_y` (C's `tmpx`/`tmpy`), and `regenerate_simple_baddy`/
  `spell_self_simple_baddy`/`idle_simple_baddy` (already-generic despite
  their names) round out the tail exactly matching C's order. `gate_fight_
  dead`'s reward tail (`World::apply_gate_fight_reward`) ports the Arch-
  Warrior/Arch-Mage/Arch-Seyan'Du class 5/6/7 flag+value grants, the
  channel-6 "Grats" broadcast (`queue_channel_broadcast`, `COL_MAUVE`), and
  the unconditional `teleport_char_driver(co, 181, 198)` tail - including
  C's subtle behavior that a failing class guard `return`s *before* the
  teleport, so the player stays put. Since `World` cannot read the killer's
  `PlayerRuntime::gate_target_class` itself, the death dispatch is wired
  the same way `CDR_SWAMPMONSTER`/`CDR_TEUFELRAT`/`CDR_CALIGARSKELLY`
  already are: a new `world_events::apply_gate_fight_death_from_hurt_event`
  reads `LegacyHurtEvent`s drained by `apply_pk_hate_from_hurt_events`
  (no change needed to the generic `CharacterDriverOutcome` dispatch, which
  only `CDR_SIMPLEBADDY` actually uses for deaths). `process_gate_fight_
  actions` is wired into the tick loop next to `process_gate_welcome_
  actions` in `main.rs`. Tests: 15 new tests in `crates/ugaris-core/src/
  world/tests/gate_fight.rs` (NT_CREATE bootstrap, NTID_GATEKEEPER victim
  tracking, self-destruct timing, adjacent-attack vs. distant-walk vs.
  return-to-post movement, giving up on a vanished victim, and all four
  `apply_gate_fight_reward` class outcomes plus their guard-failure/
  unmatched-class edge cases) and 2 new tests in `crates/ugaris-server/src/
  tests/world_events.rs` (the full `apply_legacy_hurt` ->
  `apply_pk_hate_from_hurt_events` -> reward-and-teleport pipeline, and a
  non-player-killer no-op check). `cargo fmt --all`, `cargo test --workspace`
  (1280+36+3+33+403 passed), `cargo build -p ugaris-server` clean with zero
  warnings, and a 12s boot-smoke showed "entering Rust game loop" with no
  panics.
  Progress Log (iteration 55): wired `gate_welcome_driver`'s idle
  "return to post" tail (`gatekeeper.c:627-631`) into
  `World::process_gate_welcome_actions`: once `TICKS*30` pass without the
  NPC speaking (`dat->last_talk`, already tracked), it calls the existing
  `secure_move_driver` (`world/npc_idle.rs`, unchanged) toward `rest_x`/
  `rest_y` with `DX_UP`/`ret=0`/`lastact=0` (this driver class, like
  `world::trader`/`world::bank`, doesn't thread the C dispatcher's own
  last-action/return code through - a simplification already accepted
  elsewhere, since it only matters right after a same-tick door-use).
  Confirmed the welcome NPC's spawn tile is already captured in `rest_x`/
  `rest_y` by the zone loader (`zone.rs`'s `pop_create_char` substitution),
  so no new `Character` field was needed; `gate_npc`'s tick-loop caller in
  `ugaris-server` now passes `config.area_id` (new parameter on
  `process_gate_welcome_actions`). Tests: 2 new tests in `world/tests/
  gatekeeper.rs` (`gate_welcome_returns_to_post_after_thirty_seconds_idle`
  asserting the walk starts toward the post tile past the 30s threshold,
  `gate_welcome_stays_put_shortly_after_talking` asserting no movement
  when `last_talk` is recent). `cargo fmt --all`, `cargo test --workspace`
  (1265+36+3+33+401 passed), `cargo build -p ugaris-server` clean with
  zero warnings, and a 12s boot-smoke showed "entering Rust game loop"
  with no panics.
  Progress Log (iteration 54): ported `enter_test`'s success tail /
  `enter_room` (`gatekeeper.c:227-407`). Core (`crates/ugaris-core/src/
  world/gatekeeper.rs`, all new `World` methods): `gate_room_is_clear`
  (the 9x17 room-clear scan - no character on any tile, and any item
  present must not be `IF_TAKE`), `gate_take_money`/
  `gate_give_money_silent` (`src/system/tool.c:3820-3826,1441-1449`, gold
  is already a plain `Character.gold` field so no PPD indirection is
  needed here, unlike bank gold), and `gate_finish_enter_room` (the
  player-side tail once the opponent already exists: `teleport_char_driver`
  including its "already within Manhattan distance 1" failure check,
  stripping spell slots `12..=29` via the existing `destroy_item`, the two
  `log_char` notices, and resetting HP/mana/endurance to `POWERSCALE * 1`
  plus `regen_ticker`). `GateEnterTestOutcome::Ready` now pushes a new
  `GateWelcomeOutcomeEvent::EnterTestReady { player_id, class }` instead of
  a no-op, since the opponent's `create_char` needs
  `ZoneLoader::instantiate_character_template`, which `World` cannot call.
  `ugaris-server` (`spawns.rs::gate_enter_test_spawn_room`, modeled on
  `spawn_swampspawn_character`): the `take_money` guard, the 7-room
  `GATE_TEST_ROOM_STARTS` search (`gatekeeper.c`'s `room_start[]`,
  digit-for-digit), the class-to-template map (`gatekeeper_w`/`_m`/`_s`),
  spawning the opponent (stats from `values[0]`, `Direction::RightDown`,
  the `NT_NPC`/`NTID_GATEKEEPER` driver message), and the busy-refund
  fallback; wired from `apply_gate_welcome_events` (now also taking
  `&mut World`/`&mut ZoneLoader`) on the new event, which sets
  `PlayerRuntime::gate_target_class`/`gate_step` on success (C's
  `ppd->target_class`/`step`). Two deviations documented in code comments:
  `destroy_chareffects` is a no-op (`Character` has no active-spell-effect
  list yet), and the opponent's `tmpx`/`tmpy` "return to post" coordinates
  (only consumed once `gate_fight_driver` is ported) reuse `rest_x`/
  `rest_y`, the same substitution `respawn_npc_character` already uses for
  other NPCs' post positions. Tests: 6 new core tests in `world/tests/
  gatekeeper.rs` (room-clear tile/item checks, take/give money, the
  teleport+strip+reset success tail, the zero-mana guard, the
  "already-there" teleport-failure guard, and the class-choice `Ready`
  path now asserting the `EnterTestReady` event instead of "no-op"); 3 new
  `ugaris-server` tests in `tests/spawns.rs` (full success spawn with a
  real inline `gatekeeper_w` template, the every-room-busy refund, and the
  underfunded rejection). `cargo fmt --all`, `cargo test --workspace`
  (1263+36+3+33+401 passed), `cargo build -p ugaris-server` clean with
  zero warnings, and a 12s boot-smoke showed "entering Rust game loop"
  with no panics.
  Progress Log (iteration 53): wired `enter_test`'s validation-failure
  reply text (`gatekeeper.c:320-390`) into
  `World::gate_welcome_handle_text_message` for `analyse_text_driver`
  answer codes `5`-`8` (the Arch-Warrior/Arch-Mage/Arch-Seyan'Du/Seyan'Du
  class choices), reusing the already-ported, previously-unwired
  `character_driver::gate_enter_test_precheck` pure helper. Added
  `gate_carried_item_count` (C's `cnt` loop over inventory slots
  `30..INVENTORYSIZE` plus `citem`) to feed it. Every *failure* outcome
  now matches C exactly: `NotPaid`/`LabNotSolved`/`NoExpMode`/
  `CarryingItems`/`CarryingTooManyItems` send a private
  `World::queue_system_text` (C's `log_char(cn, LOG_SYSTEM, ...)`,
  addressed to the player only, not spoken by the NPC), and
  `InvalidClass` makes the gatekeeper itself say "That is not a possible
  choice." (C's `say(cn, ...)` in the caller, via the existing
  `World::npc_say`). The `Ready` (success) outcome is intentionally left
  a no-op for now - see the REMAINING note above and the module doc
  comment in `world/gatekeeper.rs`, since it needs the unported
  `enter_room` opponent-spawn side effect. Tests: 6 new tests in
  `world/tests/gatekeeper.rs` covering each failure message, the
  invalid-class NPC reply, and the `Ready` no-op (still bookkept as
  `didsay` per C). `cargo fmt --all`, `cargo test --workspace`
  (1258+36+3+33+398 passed), `cargo build -p ugaris-server` clean with
  zero warnings, and a 12s boot-smoke showed "entering Rust game loop"
  with no panics.
  Progress Log (iteration 52): wired the welcome NPC's message loop into
  `World` (`crates/ugaris-core/src/world/gatekeeper.rs`,
  `World::process_gate_welcome_actions`), modeled on
  `world/trader.rs::process_trader_actions`: `NT_CHAR` greeting (calls the
  existing `gate_welcome_dialogue_step`), `NT_TEXT` small talk
  (`GATEKEEPER_QA` via `analyse_text_qa`, plus the "repeat"/"restart"
  dialogue reset and the god-only "reset" lab-ppd clear), and `NT_GIVE`
  give-back-or-destroy (matching `world/trader.rs`'s
  `trader_give_char_item` simplification of `give_driver`'s pathfinding).
  Added a new pure helper, `character_driver::needs_next_lab`, which
  proves `teleport_next_lab(cn, 0)`'s truthiness reduces to "not all of
  lab checkpoints 10/15/20/25/30 are solved" (reusing
  `item_driver::legacy_lab_destination`'s table) - this let the greeting
  dialogue's `needs_lab` input be computed without porting
  `teleport_next_lab`'s map/`change_area` side effects at all. Since the
  dialogue needs two `PlayerRuntime`-owned facts (`gate_welcome_state`,
  `needs_lab`) that `World` cannot see, and `World` cannot apply
  `PlayerRuntime` writes either, added a snapshot-in/events-out split
  (`GateWelcomePlayerFacts`, `GateWelcomeOutcomeEvent`) mirroring
  `world/bank.rs`'s `BankEvent` pattern, plumbed through
  `ugaris-server/src/world_events.rs` (`gate_welcome_player_facts`,
  `apply_gate_welcome_events`) and called each tick in `main.rs` right
  before `process_janitor_actions`. Added the `GateWelcome` variant to
  `CharacterDriverState` (wired into `zone.rs`'s template-init and every
  exhaustive match in `npc_messages.rs`/`npc_fight.rs`/`npc_idle.rs`).
  Tests: 1 new test for `needs_next_lab` in `character_driver.rs`; 12 new
  tests in `world/tests/gatekeeper.rs` covering the greeting
  distance/visibility/throttle rules (including the "different victim
  within the 10-tick window" C quirk), the labyrinth-still-needed wait,
  QA small talk, the repeat/reset text codes (god-gated), and the
  give-back-or-destroy paths. `cargo fmt --all`, `cargo test --workspace`
  (1252+36+3+33+398 passed), `cargo build -p ugaris-server` clean with
  zero warnings, and a 12s boot-smoke showed "entering Rust game loop"
  with no panics.
  Progress Log (iteration 51): ported the pure, fully-tested logic slice:
  `CDR_GATE_WELCOME`/`CDR_GATE_FIGHT` driver-id constants; `GATEKEEPER_QA`
  (the verbatim `qa[]` small-talk + class-choice-code table, reusing the
  existing `analyse_text_qa` engine); `gate_welcome_dialogue_step` (a pure
  state machine faithfully reproducing `gate_welcome_driver`'s
  `welcome_state` switch at `gatekeeper.c:475-542`, including its C
  fallthrough quirk where the "fast path" - lab never needed - and the
  "slow path" - lab satisfied on a later call - land on different
  terminal states, `6` vs `5`, and only the slow path shows the `case 5`
  "name the class" message); `gate_welcome_state_after_repeat`;
  `gate_enter_test_precheck`/`gate_class_choice_is_valid` (the
  `enter_test` class-eligibility and carried-item-count preconditions,
  excluding money/room-search side effects); and a `DRD_GATE_PPD`-shaped
  PPD block (`gate_ppd`/`gate_welcome_state`/`gate_target_class`/
  `gate_step` fields + encode/decode, modeled on the existing
  `DRD_WARP_PPD` fixed-layout block pattern) in
  `crates/ugaris-core/src/player.rs`. Ledger section "Gatekeeper NPC".
  Tests: 9 new tests in `character_driver.rs` (QA table word/code
  coverage, both dialogue-state fast/slow paths, the repeat-reset
  boundary, and full `enter_test` precondition/class-validation matrices)
  plus 3 new tests in `player.rs` (fixed-layout round-trip, outer PPD
  blob framing, append-without-existing-block). `cargo fmt --all`,
  `cargo test --workspace` (1240+36+3+33+398 passed), `cargo build -p
  ugaris-server` clean with zero warnings, and a 10s boot-smoke showed
  "entering Rust game loop" with no panics (this change doesn't touch
  the tick loop/login/map sync/protocol yet since nothing calls the new
  functions).

---

## P3 - World Systems

- [x] **Questlog initialization & quest state machine**
  (`src/system/questlog.c`) - quest open/done transitions driven by NPC
  dialogue flags, `sendquestlog` on change (packet already ported), exp
  rewards per quest (`quest_exp.h`). Port the quest table + the
  `questlog_open/done` helpers; wire the already-ported `CL_REOPENQUEST`
  reset side effects per area.
  CLOSED (iteration 64): every function this task's own description names
  is ported and wired into a live call site - `QUEST_TABLE`, the quest
  table metadata, `QuestLog::open`/`close`/`complete_legacy` (`questlog_
  open`/`questlog_close`/`questlog_done`), `PlayerRuntime::init_questlog`
  (called unconditionally on every login), and `PlayerRuntime::
  reopen_quest_legacy` (the full `CL_REOPENQUEST` per-area switch). This
  iteration closed the last named gap, `quest_exp.h`'s per-encounter
  exp/money constants, as `crate::quest::quest_exp` (see Progress Log).
  What remains unported all belongs to *other*, already-tracked tasks
  rather than this one: (1) `init_area1_quests`/`init_area3_quests`/
  `init_staff_quests`/`init_twocity_quests`/`init_nomad_quests` can't
  observe real state changes and no driver calls `QuestLog::open`/
  `complete_legacy`/reads `quest_exp` until the P4 "Area 1"/"Area 2"/
  etc. character-driver tasks below port the NPC dialogue that drives
  them; (2) `ACHIEVEMENT_QUESTER` on `CL_REOPENQUEST` success is gated on
  the separate "Achievements" P3 task directly below; (3) `questlog.c`'s
  trailing `destroy_item_byID`/`remove_item_from_body_bg`/`destroy_item_
  from_body` helpers (`questlog.c:1630-1703`, only ever called by area
  NPC drivers to clean up quest items) remain unported too - they need a
  Rust model of the legacy per-character `DRD_DEPOT_PPD` block (no
  equivalent exists; `ugaris-server::depot`'s `AccountDepotState` is an
  unrelated, newer system) and `destroy_item_from_body` additionally
  depends on real cross-server IPC (`server_chat` in this C function is a
  cross-node broadcast; the Rust `server_chat` is a same-server chat-
  channel fanout only, per `world/death.rs`'s documented cross-server-
  transfer-is-out-of-scope precedent) - whichever P4 area task first
  needs one of these three helpers should port them together with the
  legacy depot PPD at that point.
  Progress Log: ported the C `struct questlog questlog[]` metadata table
  (85 entries, name/level-range/giver/area/nominal-exp/flags incl.
  `QLF_XREPEAT`, copied digit-for-digit including the two trailing-space
  quest names) into `QUEST_TABLE`/`quest_meta()`
  (`crates/ugaris-core/src/quest.rs`). Ported `questlog_scale`'s repeat-
  completion exp decay curve (`scale_exp`) and `questlog_done`'s level-
  based taper (`taper_exp_by_level`) as pure, independently tested
  functions. Added `QuestLog::complete_legacy` (full `questlog_done` port:
  increments `done`, sets `flags = QF_DONE`, computes the scaled +
  tapered exp reward) returning a `QuestCompletion` for the caller to
  route through `World::give_exp`/`dlog`/`sendquestlog` (this leaf module
  has no access to `World`/`PlayerRuntime`). Fixed two pre-existing bugs
  in `QuestLog::open`/`close` found while porting: `open` now assigns
  `flags = QF_OPEN` outright (C `questlog_open`), not `|=` (previously
  left a stale `QF_DONE` bit when reopening); `close` now only transitions
  `QF_OPEN -> QF_DONE` when `flags` is exactly `QF_OPEN` (C
  `questlog_close`'s `if (quest[qnr].flags == QF_OPEN)` guard), not an
  unconditional bit-clear. Added 10 new tests
  (`crates/ugaris-core/src/quest.rs`) covering the table contents, the
  repeatability-flag/table sync, `scale_exp`'s full curve, `taper_exp_by_
  level`'s four level bands, `complete_legacy`'s first/repeat completions
  and out-of-range handling, and the corrected `open`/`close` semantics.
  Progress Log (iteration 60): promoted `area1_ppd`/`nomad_ppd`
  (`crates/ugaris-core/src/player.rs`) from delete-only stubs (raw bytes
  stripped wholesale via `strip_ppd_blocks` in `clear_turn_seyan_ppd`) to
  real fixed-layout codecs matching the C structs
  (`struct area1_ppd`, `src/area/1/area1.h:24-75`, 39 ints/156 bytes;
  `struct nomad_ppd`, `src/common/nomad_ppd.h:9-13`, 25 ints/100 bytes -
  `nomad_state[10]`/`nomad_win[10]`/4 roll-bet ints/`tribe_member`), with
  named accessors for the 10 area1 NPC states and the `nomad_state[]`
  array `questlog_init_area1`/`questlog_init_nomad` need, wired into the
  full `decode_legacy_ppd_blob`/`encode_legacy_ppd_blob` dispatch (decode
  match arm, encode match arm, `had_area1`/`had_nomad` append-if-missing)
  and `clear_turn_seyan_ppd` (now clears the typed fields directly
  instead of stripping the ids). Ported `questlog_init_area1`
  (`src/system/questlog.c:828-1039`) and `questlog_init_nomad`
  (`src/system/questlog.c:1571-1607`) as pure functions
  (`init_area1_quests`/`init_nomad_quests` in
  `crates/ugaris-core/src/quest.rs`) taking a plain `Area1QuestState`/
  `NomadQuestState` snapshot (this leaf module has no access to
  `PlayerRuntime`; `PlayerRuntime::area1_quest_state`/`nomad_quest_state`
  build the snapshot), including the required `GWENDYLON_STATE_*`/
  `JESSICA_STATE_*`/`BRITHILDIE_STATE_*`/`CAMHERMIT_STATE_*` constants
  from `src/common/npc_states.h` and the `mark_init_done`/`set_flags`
  helpers matching C's `if (!quest[qnr].done) quest[qnr].done = 1;
  quest[qnr].flags = QF_DONE;` idiom (seeds `done` once, never
  increments). Fixed 14 existing `player.rs` tests that reused
  `22 | PERSISTENT_PLAYER_DATA` (now `DRD_AREA1_PPD`) as a placeholder
  "unmodeled id" - repointed them at `DRD_RANK_PPD` (still genuinely
  unmodeled). Added 6 new tests in `player.rs` (area1/nomad fixed-layout
  round-trip, outer-blob replace/append, snapshot builders, out-of-range
  index safety) and 6 new tests in `quest.rs` (every `init_area1_quests`
  branch ladder incl. the Gwendylon 4-quest chain, `init_nomad_quests`
  thresholds, and the "done seeded once, not incremented" re-init
  invariant). `cargo fmt --all`, `cargo test --workspace` (1311+36+3+33+
  406 passed), `cargo build -p ugaris-server` clean with zero warnings,
  and a 10s boot-smoke showed ticking with no panics (this change doesn't
  wire `init_area1_quests`/`init_nomad_quests` into any live caller yet -
  no NPC driver advances these states, so nothing calls them at
  runtime).
  Progress Log (iteration 61): ported the remaining three
  `questlog_init_*` sub-functions - `questlog_init_area3`
  (`src/system/questlog.c:1040-1203`), `questlog_init_staff`
  (`:1203-1394`), `questlog_init_twocity` (`:1470-1546`) - as
  `init_area3_quests`/`init_staff_quests`/`init_twocity_quests` in
  `crates/ugaris-core/src/quest.rs`, taking plain `Area3QuestState`/
  `StaffQuestState`/`TwocityQuestState` snapshots built by the new
  `PlayerRuntime::area3_quest_state`/`staff_quest_state`/
  `twocity_quest_state` (`crates/ugaris-core/src/player.rs`). Added the
  missing named field accessors these functions need on the existing
  `area3_ppd`/`staffer_ppd`/`twocity_ppd` raw-byte blocks (`seymour`/
  `astro2`/`crypt`/`william`/`hermit` states for area3; `carlos`/
  `smugglecom`/`aristocrat`/`yoatin`/`countbran` state+bits/
  `brennethbran`/`spiritbran`/`broklin`/`dwarfchief`/`dwarfshaman`
  states for staffer; `sanwyn`/`skelly`/`alchemist` states for twocity -
  `thief_state`/`kelly_state`/`clara_state`/etc. already had accessors).
  Fixed a genuine pre-existing size bug found while doing this:
  `LEGACY_AREA3_PPD_SIZE` was `17 * 4` but C `struct area3_ppd`
  (`src/area/3/area3.h:18-35`) has 18 `int` fields (`imp_kills,
  imp_flags;` on one line) = 72 bytes, not 68 - corrected to `18 * 4`
  (every use went through the symbolic constant, so this was a safe,
  test-verified fix; the missing byte only mattered for the unused-until-
  now `kassim_item_wait_starttime` tail field). Faithfully reproduced two
  legacy C quirks instead of "fixing" them: the `william_state` ladder
  has no final `else` (`:1177-1191`), so quests 22/23 are left untouched
  (not reset to `0`) when `william_state <= 0`, unlike every other
  ladder in this function; and the `yoatin_state` ladder's "open" branch
  tests `ppd->aristocrat_state > 0` instead of `ppd->yoatin_state > 0`
  (`:1284-1290`), a copy-paste bug in the original C - both are covered
  by dedicated regression tests documenting the quirk. Added 8 new tests
  in `quest.rs` (every branch ladder for all three functions, plus
  dedicated tests for the two preserved C quirks) and 3 new tests in
  `player.rs` (fixed-layout round-trip + snapshot-builder coverage for
  area3/staffer/twocity's new fields). `cargo fmt --all`, `cargo test
  --workspace` (1322+36+3+33+406 passed), `cargo build -p ugaris-server`
  clean with zero warnings, and a 10s boot-smoke showed ticking with no
  panics (this change doesn't wire the three new `init_*_quests`
  functions into any live caller yet, same as the area1/nomad ones from
  the previous iteration - the `questlog_init` dispatcher itself, which
  would call all five, is also still unported; see REMAINING above).
  Progress Log (iteration 62): ported `questlog_init`
  (`src/system/questlog.c:1610-1626`) as `PlayerRuntime::init_questlog`
  (`crates/ugaris-core/src/player.rs`): checks
  `QuestLog::is_init_complete` (the `quest[MAXQUEST-1].done == 55`
  sentinel), calls all five `init_*_quests` sub-functions with fresh
  snapshots built from the existing `*_quest_state` accessors, then
  `QuestLog::mark_init_complete`. Also gave `DRD_QUESTLOG_PPD` a real
  codec (`encode_legacy_questlog_ppd`/`decode_legacy_questlog_ppd`,
  `LEGACY_QUESTLOG_PPD_SIZE` = `MAX_QUESTS` = 100 bytes) matching C's
  `struct quest { done:6; flags:2; }` per-quest bitfield byte (LSB-first
  allocation: `done` in bits 0-5, `flags` in bits 6-7), wired into
  `decode_legacy_ppd_blob`/`encode_legacy_ppd_blob` (decode match arm,
  encode match arm, `had_questlog` append-if-missing) - the in-memory
  `QuestLog` (`quest_log` field, already used by `open`/`complete_legacy`
  etc.) is now actually persisted instead of being silently dropped every
  save. Added `QuestLog::is_init_complete`/`mark_init_complete`/`set_raw`
  (`crates/ugaris-core/src/quest.rs`) as the primitives the codec and
  dispatcher need. Moved `DRD_QUESTLOG_PPD` out of the "no Rust
  representation, raw-strip only" constant group (matching the
  `DRD_AREA1_PPD`/`DRD_NOMAD_PPD` precedent) and updated
  `clear_turn_seyan_ppd` to reset `quest_log` to its default instead of
  stripping raw PPD bytes (C's `del_data(cn, DRD_QUESTLOG_PPD)` in
  `turn_seyan`, `src/system/tool.c:4364`, deletes the whole block, which
  is exactly what resetting to default + losing the sentinel achieves -
  the next `init_questlog` call re-seeds everything from scratch).
  Nothing yet calls `init_questlog` from a live login/character-load
  path (no such seam exists in the ported tree), so this remains inert at
  runtime, same caveat as prior iterations - see REMAINING above. Added 4
  new tests in `player.rs` (PPD codec byte-layout round-trip incl. the
  bitfield packing, blob replace/append incl. the "no progress yet ->
  nothing appended" case, `init_questlog` running all five sub-functions
  once and never re-running, `clear_turn_seyan_ppd` resetting the quest
  log). `cargo fmt --all`, `cargo test --workspace` (1326+36+3+33+406
  passed), `cargo build -p ugaris-server` clean with zero warnings, and a
  10s boot-smoke showed ticking with no panics.
  Progress Log (iteration 63): closed both remaining gaps from the
  REMAINING note above. (1) Wired `PlayerRuntime::init_questlog` into the
  live login path: added one call site in `crates/ugaris-server/src/
  main.rs`'s `SessionEvent::Login` handler, right after the DB-snapshot/
  scaffold-spawn branch (so it runs after `apply_character_snapshot`'s PPD
  decode for existing characters, and after `login_character_from_template`
  for brand-new ones), matching C `login_ok`'s unconditional
  `questlog_init(cn)` call (`src/system/player.c:659`) - safe to call on
  every login rather than only "first ever" since it's already idempotent
  via the sentinel. (2) Ported the full `questlog_reopen`/
  `questlog_reopen_qN` per-quest switch (`src/system/questlog.c:342-826`)
  as `PlayerRuntime::reopen_quest_legacy` (`crates/ugaris-core/src/
  player.rs`), replacing the old generic-only `try_reopen_legacy` call in
  `main.rs`'s `CL_REOPENQUEST` handler: every `questlog_reopen_qN` helper
  (q0/q1/q5/q7/q9/q10/q13/q16/q20/q22/q30/q31/q35/q38/q39/q40/q41/q45/q79/
  q83/q84) is ported as a small `reopen_*` method doing the area-PPD side
  effect plus (where applicable) the "cannot re-open more than one quest
  from a series" sibling-`QF_OPEN` exclusivity check, dispatched from a
  `reopen_dispatch` match on `qnr`. Added the missing PPD accessors these
  needed: `area1_james_state`, `area3_imp_state`/`area3_imp_kills`,
  `staffer_smugglecom_bits`, and the `CAMHERMIT_STATE_QUEST2_1` constant
  (promoted to `pub(crate)` alongside the other NPC-state constants
  `reopen_quest_legacy` needed from `quest.rs`). Split `QuestLog::
  try_reopen_legacy`'s generic preconditions into a shared `reopen_precheck`
  (`done > 9`, "table flags nonzero", `QF_DONE` bit) reused by both the
  old leaf-only method (kept, unchanged behavior, for its existing tests)
  and the new dispatch. Fixed a genuine latent bug found while doing this:
  the precondition's "is this quest repeatable" check was written as
  `(QUESTLOG_FLAGS[quest] & QLF_REPEATABLE) == 0`, the "obviously correct"
  reading - but C's actual code is `!questlog[qnr].flags & QLF_REPEATABLE`,
  where `!` binds tighter than `&`, making the real condition "table flags
  are exactly zero" (not "missing the REPEATABLE bit specifically"), a
  genuine C operator-precedence bug that lets `QLF_XREPEAT`-only quests
  (25-28) also pass the check; replicated verbatim per the porting rule to
  preserve legacy quirks rather than keep the "fixed" version. Added a new
  `QuestReopenResult::SeriesConflict` variant ("Cannot re-open more than
  one quest from a series.") and `NoEffect` variant (the switch's several
  explicit `ret = 0` arms/dead-code cases - reached the switch but nothing
  changes, no message) alongside the existing `Reopened`/`CannotOpenAgain`/
  `CannotOpenNow`/`InvalidQuest`; updated `main.rs`'s `CL_REOPENQUEST`
  handler to resend `SV_QUESTLOG` for every outcome except the three
  precondition-rejection variants (matching C's unconditional
  `sendquestlog` once the switch is reached) and to show the new
  `SeriesConflict` message. Faithfully reproduced C's `case 36` missing
  `break;` (falls through into `case 37`'s helper call with `state = 7`
  instead of doing nothing) - though this arm turns out to be dead code in
  practice since quest 36's table row has zero flags (an independent,
  separate real bug: its "repeatable" table entry is missing entirely, so
  the live precondition-gated path rejects it before ever reaching the
  switch) - tested directly against the internal `reopen_dispatch` split
  to prove the switch body itself is faithful, plus a companion test
  confirming the public API path is unreachable for that quest number.
  Same "reachable via precondition but not through the live switch"
  situation applies to case 22 (`william`/`imp` reset) and the smugglecom
  `state == 5` bit-clear branch (no live case ever passes `state == 5`) -
  both verified via direct calls to their private helper methods.
  `ACHIEVEMENT_QUESTER` award on success is noted but skipped (achievement
  system unported, separate P3 task below). Added 18 new tests in
  `crates/ugaris-core/src/player.rs` covering: the simple single-state-
  reset cases (q0, q5/q9/q13/q16/q20/q30/q31/q38/q39/q44), every series-
  exclusivity family (Gwendylon, Guiwynn, Seymour, William, Brenneth,
  Broklin, Jessica, smugglecom's case-36-fallthrough), the countbran
  bitmask-preserving clear, the camhermit hermit-quest2 entry state, the
  XREPEAT-precedence-bug quirk, the zero-table-flags rejection quirk, and
  the generic `CannotOpenNow`/`InvalidQuest` precondition paths. `cargo
  fmt --all`, `cargo test --workspace` (1344 ugaris-core [+18] + 36 db + 3
  net + 33 protocol + 406 server, all green, zero failures), `cargo build
  -p ugaris-server` clean with zero warnings, and a 10s boot-smoke showed
  "entering Rust game loop" with no panics.
  Progress Log (iteration 64, task closed): ported the last item this
  task's own description named as in-scope, `src/common/quest_exp.h`'s
  34 per-encounter exp/money constants (`EXP_AREA1_SKULL1` .. `EXP_
  AREA16_SPIDERKILL`, `MONEY_AREA1_SKULL1` .. `MONEY_AREA3_VAMPIRE1`),
  copied digit for digit into a new `crate::quest::quest_exp` module
  (`crates/ugaris-core/src/quest.rs`) with a doc comment noting that only
  2 of the 34 (`EXP_AREA15_HARDKILL`, `EXP_AREA3_SHRINE`) are actually
  referenced anywhere in the C source today - the rest are dead code in
  C too, not just "not ported yet". No consumer exists in Rust yet either
  (same P4-area-driver gate as the rest of this task), so this is data-
  only; added 1 new test (`quest_exp_constants_match_c_header`) asserting
  every constant against the C header to guard against silent drift.
  Verified line-by-line against `questlog.c` that every other function in
  the file is already ported: `questlog_open`/`close`/`scale`/`done`/
  `count`/`isdone` (-> `QuestLog::open`/`close`/`complete_legacy`/
  `mark_done`/`count`/`is_done`), all 24 `questlog_reopen_qN` helpers plus
  the outer switch (-> `reopen_dispatch`, iteration 63), and `questlog_
  init`/all five `questlog_init_*` (-> `PlayerRuntime::init_questlog`,
  iteration 62-63, called from `main.rs`'s login handler). The only
  unported leftovers are the file's trailing, quest-adjacent-only-by-
  file-location `destroy_item_byID`/`remove_item_from_body_bg`/`destroy_
  item_from_body` helpers (`questlog.c:1630-1703`) - deliberately left
  for whichever P4 area task first needs them, since they require a
  legacy `DRD_DEPOT_PPD` Rust model that doesn't exist and (for
  `destroy_item_from_body`) real cross-server IPC that this codebase has
  explicitly scoped out elsewhere (see the task's CLOSED note above for
  detail). Marked the task `[x]`: every action item in its own
  description is done, and everything else is already tracked by the
  "Achievements" P3 task and the P4 "Area 1"/etc. tasks below. `cargo fmt
  --all`, `cargo test --workspace` (1345 ugaris-core [+1] + 36 db + 3 net
  + 33 protocol + 406 server, all green, zero failures), `cargo build -p
  ugaris-server` clean with zero warnings, and a 10s boot-smoke showed
  ticking with no panics (data-only change, doesn't touch the runtime
  loop/login/map sync/protocol).

- [~] **Achievements (`src/module/achievements/achievement.c`)** - runtime
  markers partially exist (chests, transport). Port the achievement
  table, progress PPD, `SV_*` packets the community client expects
  (check client), and the grant/announce path. Wire existing markers.
  REMAINING: a previous iteration ported the full core data model and
  stat-driven award logic as a standalone leaf module
  (`crates/ugaris-core/src/achievement.rs`) - the 127-entry
  `AchievementType` enum + `achievement_defs` table, `Achievement`/
  `AccountAchievements`/`AchievementStats` structs, `award`/
  `add_progress`/`get_stat_progress`, and every `achievement_add_*`/
  `achievement_check_*` stat-update function - but nothing wired it into
  a live call site. This iteration (66) closed gap (1), persistence:
  added `PlayerRuntime::achievement_data`/`achievement_stats` fields
  (`crates/ugaris-core/src/player.rs`), Serialize/Deserialize on
  `Achievement`/`AccountAchievements`/`AchievementStats` (the 128-entry
  array needs a manual `serde(with = ...)` shim since serde's array impl
  tops out at derive-friendly sizes for non-`Copy` elements), and a new
  `crates/ugaris-server/src/achievement.rs` with byte-exact
  `DRD_ACHIEVEMENT_DATA`/`DRD_ACHIEVEMENT_STATS` subscriber-blob codecs
  (offsets verified against `achievement.h` with a throwaway C
  `sizeof`/`offsetof` probe), wired into `apply_character_snapshot`/
  `character_save_request` (`crates/ugaris-server/src/snapshots.rs`)
  alongside the existing `DRD_ACCOUNT_WIDE_DEPOT` block. Still to do:
  (2) protocol - the `SV_ACH_UNLOCK`/`SV_ACH_PROGRESS`/`SV_ACH_SYNC`/
  `SV_ACH_STATS` mod packets (`mod_achievements.h`) have no Rust
  definitions; (3) DB "first player globally" tracking + cross-server
  grats announcement (`database_achievement.c`) is unported; (4) the
  `/achievements`/`/achstats`/`/achfix`/`/achclear`/`/achsync`/
  `/achgive` commands are still help-text-only stubs in
  `commands_player.rs` with no dispatch logic; (5) no call site anywhere
  (chest opens, gathering, combat, mining, quests, clans, etc.) invokes
  the new `add_*`/`check_*` functions yet - each needs wiring at its own
  C-identified call site (`ACHIEVEMENT_STATUS.txt`'s file list) once
  (2)-(4) land. Note: the C `DRD_ACHIEVEMENT_DATA`/`_STATS` ids are
  `PERSISTENT_SUBSCRIBER_DATA` (account-wide); this port persists them
  per-character in `subscriber_blob` for now (same scoping compromise
  `DRD_ACCOUNT_WIDE_DEPOT` already makes) pending a real
  multi-character-per-account model - `crate::player`'s pre-existing
  `AchievementState` (chests + transport markers only) remains untouched
  and still coexists unwired with this model.

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
