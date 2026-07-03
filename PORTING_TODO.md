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

- [ ] **Skill raising (`CL_RAISE`)** - parsed but ignored; players cannot
  spend experience.
  - C: `cl_raise` in `src/system/player.c` -> `raise_value` /
    `raise_value_exp` in `src/system/skill.c`. The scroll path is already
    ported in `crates/ugaris-core/src/item_driver/scrolls.rs`
    (`raise_value_exp`, `raise_cost`, `skill_start`, `skillmax`) - reuse
    those helpers, do not duplicate the math.
  - Rust: handle `ClientAction::Raise { value }` in the `main.rs` action
    match; apply through a small `World`/character helper; send updated
    `SV_SETVAL0/1` + exp packets after success (see how login sends them in
    `crates/ugaris-server/src/login.rs`).
  - Tests: `tests/commands_player.rs` or a new `tests/raise.rs` - success
    spends exp and bumps bare value, blocked cases (cost too high, at max,
    `/noexp` flag) match C feedback text.

- [ ] **Speed mode (`CL_SPEED`) and fight mode (`CL_FIGHTMODE`)** - both
  parsed, both ignored.
  - C: `cl_speed`, `cl_fightmode` in `src/system/player.c` (speed sets
    `ch[cn].speed_mode` 0/1/2 = stealth/normal/fast, fight mode stores
    `ch[cn].gethit_bonus`-style fields; read the actual C body).
  - Rust: `Character.speed_mode` already exists and drives
    `speed_ticks`; just wire the actions in `main.rs`/`player_actions.rs`.
  - Tests: setting speed changes action durations (`do_walk` duration);
    fight mode round-trips its C side effects.

- [ ] **Player death saves** - `die_character` never consults `saves`.
  - C: `god_save_char` in `src/system/death.c` (lines ~850): if
    `ch[cn].saves > 0` and death is not PK, decrement saves, HP = 1
    POWERSCALE, remove poison (`remove_all_poison` - already ported),
    extinguish burn effects, send the two Ishtar messages, transfer to rest
    area, and skip the normal death consequences.
  - Rust: extend `World::die_character` in
    `crates/ugaris-core/src/world/death.rs` before the exp-loss branch.
    `save_number` text helper already exists server-side in
    `crates/ugaris-server/src/area_apply.rs` - move or mirror it in core.
  - Tests: extend `world/tests/death.rs` - save consumed, exp kept, items
    kept, feedback text, saves=0 falls through to normal death.

- [ ] **Game clock advancement** - `world.date` never moves; it is always
  the same hour, so daylight/nightlight logic is frozen.
  - C: `tick_date()`/time advancement in `src/system/date.c` (game time
    advances some minutes per real tick; find the exact ratio in the C
    main loop) plus the periodic `compute_dlight_*` refresh when `hour`
    changes.
  - Rust: `crates/ugaris-core/src/game_time.rs` already has `GameDate`
    math. Advance it in the `main.rs` tick branch (or `World::advance`),
    and when `daylight` changes, mark light-dirty sectors / recompute
    indoor dlight (see `world/light.rs` helpers).
  - Tests: date advances at the C rate; daylight value changes across a
    sunrise boundary.
  - Gotcha: nightlight item timers already fire on a daylight threshold -
    verify they light up when night falls in a live boot.

- [ ] **Look at character (`CL_LOOK_CHAR`)** - parsed, ignored.
  - C: `cl_look_char` -> `look_char` in `src/system/player.c` /
    `src/system/act.c` (sends `SV_LOOK*` packets with sprite, name,
    description, equipment worn sprites for players; text description for
    NPCs). Check the exact packet in the C client `sv_look`.
  - Rust: protocol builder in `crates/ugaris-protocol/src/packet.rs` +
    handler in `main.rs`; follow `legacy_item_look_text` in
    `crates/ugaris-server/src/inventory.rs` for the text-side conventions.
  - Tests: packet layout against C client expectations; NPC vs player
    variants.

- [ ] **Look at map item (`CL_LOOK_ITEM`)** - parsed, ignored. Reuse
  `legacy_item_look_text`; gate by `char_see_item` and distance like C
  `cl_look_item`. Tests in `tests/inventory.rs`.

- [ ] **Junk item (`CL_JUNK_ITEM`)** - C `cl_junk_item` destroys the cursor
  item (with `IF_QUEST` guard). Small task: handler + test.

- [ ] **Ping (`CL_PING`)** - C echoes `SV_PING`/`SV_LPING` with the client
  timestamp (see client `sv_ping`, `svl_ping`). Wire it so client RTT
  display works. Trivial: builder + handler + test.

- [ ] **Fast sell (`CL_FASTSELL`)** - C `cl_fastsell` sells an inventory
  slot directly to the active merchant (`player_store`-adjacent path).
  Extend `crates/ugaris-server/src/merchants.rs`; reuse
  `merchant_store_sell` semantics but from an inventory slot. Tests in
  `tests/commands_chat.rs`... no - `tests/merchants.rs` (create it).

- [ ] **NPC sighting messages (`NT_CHAR` emission)** - NPCs only "see"
  players through ad-hoc scans (merchant greeting, simple-baddy attack
  scan). C emits `NT_CHAR` notify messages from character movement so
  *every* driver reacts through its message queue.
  - C: `notify_area(x, y, NT_CHAR, cn, 0, 0)` call sites in
    `src/system/act.c` (walk completion) and `src/system/create.c`
    (spawn). Sector-based: only characters that can currently see the
    mover get the message (`char_see_char` gate inside `notify_area` -
    check the real C filter).
  - Rust: emit in `World::complete_walk` and `World::spawn_character`
    through the existing `notify_area` (add the see-gate). Then simplify
    the merchant greeting scan to consume `NT_CHAR` like C
    `merchant_driver` (keep the scan fallback if you must, but prefer the
    message path).
  - Tests: walking near an NPC queues `NT_CHAR` exactly once per sighting
    with the C dedup behavior.

---

## P1 - Core Framework

Systems every later port depends on. Order within the section is a
suggestion; dependencies are noted.

- [ ] **`update_char` stat recomputation** - the big one. C
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

- [ ] **Equipment slot rules on swap (`CL_SWAP` into worn slots)** - C
  `cl_swap`/`swap` checks `place_item_typed` rules: worn slot flag match
  (`IF_WN*`), min level, class gates, two-handed vs left hand, and calls
  `update_char`. Verify the Rust `inventory_swap_slot`
  (`crates/ugaris-server/src/inventory.rs`) against C and port the missing
  gates. Tests exist in `tests/inventory.rs` - extend them.

- [ ] **Experience/level-up side effects** - C `give_exp` ->
  `check_levelup` in `src/system/skill.c`/`tool.c`: level recompute from
  exp, `SV_TEXT` "You have reached level N!", HP/end/mana refill on level,
  `update_char`, achievements hook. Rust has exp modifiers server-side but
  no level recompute. Port `exp2level`/`level2exp` into core (variants
  already exist in `crates/ugaris-server/src/spawns.rs` - consolidate into
  `ugaris-core` and re-export) and apply on every exp grant (kill exp path
  in `world/death.rs` + admin/quest grants).

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
