# ulttt

A high-performance Ultimate Tic-Tac-Toe solver in Rust, with the rules
formalized in Lean 4 and solver results checked by a Lean-verified certificate
checker.

Three rule variants are supported everywhere; see [`RULES.md`](RULES.md):

- `classic`: won or full boards are closed.
- `open`: won boards stay playable until full (proved a first-player win by
  Bertholon et al., 2020).
- `majority`: `classic`, but without three in a row, more boards wins.

## What is here

| Part | Where | What |
| --- | --- | --- |
| Rules in Lean | `proofs/` | The three variants, the game value, and proofs: games end, symmetries preserve value, `mayWin` pruning is sound, the certificate checker is sound |
| Engine | `backend/crates/game` | Bitboard rules, symmetries, notation, perft |
| Solvers | `backend/crates/solver` | `dfpn`: depth-first proof-number search (the default). `search`: alpha-beta with a lock-free transposition table, draw detection, move ordering, history heuristic and parallel search. `alphabeta`: the plain baseline |
| Certificates | `backend/crates/certificate` | Strategy certificates for solved positions, checked by `ulttt-check` (Lean) |
| Storage | `backend/crates/db` | SQLite: benchmark suites, runs, solved and Lean-verified positions |
| CLI | `backend/crates/cli` | `ulttt perft`, `solve [--pv]`, `certify`, `verify`, `suite-create`, `bench`, `runs`, `compare` |
| Differential tests | `backend/crates/difftest` | Rust rules, `may_win` and every solver's values against the Lean definitions on random games |
| Frontend | `frontend/` | React app: play against the solver, analysis with the best line; the engine runs in the browser as WebAssembly |

## How the pieces check each other

- The Lean definitions in `proofs/Ulttt/Rules.lean` are the specification.
- `ulttt-difftest` plays thousands of random games and checks that the Rust
  engine agrees with the Lean rules on every position, and that every solver's
  values equal Lean's `value` on random endgames.
- The solvers' main pruning rule, "this player can no longer win", is proved
  sound in Lean (`valueFor_le_zero_of_mayWin_false`).
- `ulttt certify` writes a certificate (a strategy) for a solved position, and
  `ulttt-check` checks it with `Ulttt.check`, which Lean proves sound
  (`check_sound`). The solver itself need not be trusted. `ulttt verify`
  does this for a whole suite and records the results in the database.

## Results

Benchmark suites hold 20 positions reached by random play: `<variant>-pN`
after N random moves (see `scripts/create-suites.sh`); fewer moves means a
harder position. Times are single-threaded on an 8-vCPU machine, with a limit
of 300M nodes per position where noted; the `alphabeta` baseline ran with a
200M limit.

| Suite | `alphabeta` | `search` | `dfpn` |
| --- | --- | --- | --- |
| classic-p46 | 13/20, 1.7G nodes | 20/20, 3.0M nodes, 2.3s | |
| open-p46 | 10/20, 2.8G nodes | 20/20, 171M nodes, 28s | 20/20, 2.7M nodes, 1.6s |
| majority-p46 | 15/20, 1.5G nodes | 20/20, 1.8M nodes, 1.3s | |
| classic-p42 | | 20/20, 11M nodes, 5.3s | 20/20, 1.0M nodes, 1.5s |
| open-p42 | | | 20/20, 41M nodes, 31s |
| classic-p38 | | 20/20, 759M nodes, 197s | 20/20, 48M nodes, 60s |
| majority-p38 | | 18/20 | 20/20, 280M nodes, 373s |
| open-p38 | | | 19/20, 875M nodes |

All 20 positions of `classic-p46` and `open-p46` are Lean-verified: their
certificates were accepted by `ulttt-check`.

The empty board is still out of reach: on `open`, df-pn's proof number for
"X wins" keeps growing with the search.

## Development

Everything runs in the devcontainer (`.devcontainer/`): open the repo in VS Code
with Dev Containers, or use `make image` and `make shell` with plain Docker.

```sh
cd proofs && lake build                    # Lean proofs and executables
cd backend && cargo test --release         # Rust
backend/target/release/ulttt solve -v open --pv "<position>"
scripts/create-suites.sh                   # benchmark suites in data/ulttt.db
backend/target/release/ulttt bench classic-p42 --jobs 4
backend/target/release/ulttt verify classic-p46
make frontend                              # http://localhost:5173
```
