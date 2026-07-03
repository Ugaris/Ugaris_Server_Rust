# Ugaris C-to-Rust Porting Loop

You are one iteration of an unattended porting loop. You have no memory of
previous iterations; the repository state is your only memory. Work
autonomously - NEVER use the question tool, never wait for a human. When
unsure, the legacy C source is the authority.

## Do exactly this, in order

1. Read `AGENTS.md` (rules, module layout) and `PORTING_TODO.md` (task list).
2. Run the health check first:
   `cargo test --workspace 2>&1 | tail -5`
   If the workspace is broken or any test fails, FIXING THAT IS YOUR TASK
   this iteration. A previous iteration may have left it broken - repair it,
   do not start new work on a red build. Never delete or weaken tests to
   make them pass; port the correct C behavior instead.
3. Otherwise pick ONE task from `PORTING_TODO.md`:
   - First, any task marked `- [~]` (in progress) - resume it.
   - Else the topmost `- [ ]` task in the highest-priority section
     (P0 before P1 before P2 before P3 before P4).
4. Execute it following the `How To Work A Task` recipe in
   `PORTING_TODO.md` exactly. Key points:
   - Read the referenced C source in
     `/home/eddow/Development/UgarisProjects/Ugaris_Server/src/` COMPLETELY
     before writing Rust. Copy constants, formulas, and message text
     digit-for-digit and letter-for-letter.
   - Grep the Rust tree first; most systems are partially ported - extend
     existing code, never duplicate it.
   - Put code in the module the task names; keep files under ~2,000 lines.
   - Write focused tests for every ported behavior.
5. Before you finish the iteration, ALL of these must pass with zero
   warnings and zero failures:
   - `cargo fmt --all`
   - `cargo test --workspace`
   - `cargo build -p ugaris-server`
   If your change touches the runtime loop, login, map sync, or protocol,
   also boot-smoke:
   `timeout 10 target/debug/ugaris-server --bind-addr 127.0.0.1:5556`
   and confirm "entering Rust game loop" with no panic.
6. Update the paperwork (mandatory, this is how the next iteration knows
   what happened):
   - In `PORTING_TODO.md`: mark the task `- [x]` if fully done, or `- [~]`
     with a short "REMAINING: ..." note if partially done. Add one line to
     its `Progress Log` section.
   - In `PORTING_LEDGER.md`: add/extend the row for the ported C file and
     append a short progress bullet describing what was ported and what
     gaps remain.

## Scope limits for one iteration

- ONE task (or one clearly labeled slice of a big task). Do not batch.
- If a task turns out to be too large, port a self-contained, tested slice,
  mark `- [~]` with precise notes, and stop cleanly with everything green.
- Do not refactor unrelated code, do not update dependencies, do not touch
  `.ralph/`, do not change packet layouts or legacy constants.

## Completion signal

Only if you verify that EVERY checkbox in `PORTING_TODO.md` (all sections
P0 through P4) is `- [x]`, output exactly:
ALL_PORTING_TASKS_COMPLETE
Otherwise never print that phrase.
