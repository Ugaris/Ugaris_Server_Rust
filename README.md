# Ugaris Rust Server

This repository is the standalone Rust rewrite track for the existing C/C++ Ugaris area server. The current C server remains authoritative while this workspace grows feature parity module by module.

The rewrite was split out of `Ugaris_Server/rust_server` so it can have independent history, CI, and release flow without mixing Rust rewrite work into the legacy C server repository.

## Compatibility Contract

- Existing clients must keep using the same TCP protocol from `src/common/client.h`.
- Server-to-client traffic keeps the legacy tick batching envelope from `src/system/io.c`.
- Client-to-server commands keep the legacy byte-oriented command stream parsed by `src/system/player.c`.
- The game loop remains 24 ticks per second.
- Area transfer semantics keep `SV_SERVER` compatibility for the gateway/client.
- Persistent storage targets PostgreSQL instead of MariaDB.

## Workspace Layout

- `crates/ugaris-protocol`: Wire constants, command sizing, legacy frame encoding, and binary helpers.
- `crates/ugaris-core`: Engine-agnostic MMO domain types, tick scheduler, IDs, world state, and system traits.
- `crates/ugaris-net`: Tokio TCP listener/session runtime that maps sockets to protocol commands and outgoing tick frames.
- `crates/ugaris-db`: PostgreSQL pool and repository scaffolding for accounts, characters, areas, and game persistence.
- `crates/ugaris-server`: Executable that wires config, database, world state, networking, and the tick loop.
- `ugaris_data`: Local symlink to `/home/eddow/Development/UgarisProjects/Ugaris_Data` containing runtime data, zones, quests, loot tables, and MOTD. It is gitignored and not committed.
- `PORTING_LEDGER.md`: Current porting coverage, gaps, and continuation handoff.
- `DRIVER_PORTING_PLAN.md`: Driver architecture and next driver priorities.

## Setup

Create the local data symlink:

```bash
ln -s /home/eddow/Development/UgarisProjects/Ugaris_Data ugaris_data
```

Build and test:

```bash
cargo fmt --all
cargo test --workspace
cargo build -p ugaris-server
```

Run the server locally:

```bash
target/debug/ugaris-server --bind-addr 0.0.0.0:5556
```

Run the legacy client from the sibling client repository:

```bash
cd /home/eddow/Development/UgarisProjects/astonia_community_client
bin/moac -u Godmode -p test123 -d localhost -t 5556 -o 3141 -c 8000 -k 60 -m 8 -n 0
```

## Porting Order

1. Lock protocol parity with tests against the C constants and packet sizes.
2. Port immutable data definitions: flags, values, map tiles, item/character structs.
3. Port networking/login flow while temporarily using compatibility repositories.
4. Port persistence from `src/system/database/*` to PostgreSQL repositories.
5. Port core simulation: date, timers, map, player actions, effects, skills, combat, death, respawn.
6. Port driver system with typed Rust traits and a compatibility registry for all `CDR_*` and `IDR_*` drivers.
7. Port modules and areas file by file, preserving behavior before refactoring.
8. Run client compatibility playtests for every area and migration checkpoint.

## Current Status

The server can load real area 1 data, accept a legacy client login, send initial map/inventory bootstrap data, and execute a growing subset of movement and item-driver behavior. It is not yet a playable replacement for the C server.

See `PORTING_LEDGER.md`, especially `Continuation Handoff`, for the latest verified state and next steps.
