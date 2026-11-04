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
| Playing engine | `backend/crates/engine` | For positions too deep to solve: iterative-deepening alpha-beta (transposition table, PVS, killer and history ordering, quiescence) over a heuristic evaluation tuned per variant; also a Monte Carlo tree search engine. Drives the frontend's moves |
| Certificates | `backend/crates/certificate` | Strategy certificates for solved positions, checked by `ulttt-check` (Lean) |
| Storage | `backend/crates/db` | SQLite: benchmark suites, runs, solved and Lean-verified positions |
| CLI | `backend/crates/cli` | `ulttt perft`, `solve [--pv]`, `certify`, `verify`, `suite-create`, `bench`, `runs`, `compare`, `match`, `matches`, `engine-bench` |
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

Every position of the p46 and p42 suites in all three variants, and of
classic-p38 (140 positions), is Lean-verified with `ulttt verify`: their
certificates were accepted by `ulttt-check`.

The empty board is still out of reach: on `open`, df-pn's proof number for
"X wins" keeps growing with the search.

### Playing engine

Positions too deep to solve are played by `ulttt-engine`. Changes are kept
only when a self-play match (`ulttt match`, recorded in the database) shows
they help: each opening (4 random moves) is played twice with colors swapped,
at 20k nodes per move. The margins are 95% intervals.

Altogether, against the engine as it was before this tuning (no quiescence,
untuned weights, exact solver at the root): +83 ±19 Elo in classic, +42 ±22
in open and +105 ±23 in majority (800 games each).

| Change | Result |
| --- | --- |
| Quiescence search over board-winning moves (6 plies) | +38 ±18 Elo (classic, 800 games), +65 ±22 (majority) |
| Evaluation weights tuned on self-play games (`ulttt texel`), per variant | +37 ±18 Elo (classic, 800 games), +38 ±22 (majority); open keeps the untuned weights (+10 ±22) |
| Win-in-one at the horizon, PVS | +20, +14 ±26 Elo |
| Killer moves, two-entry table buckets, late move reductions, aspiration windows | no significant gain; off or kept neutral |
| Not calling the exact solver at the root (its share of the budget goes to the search) | +21 ±18 Elo (classic), +29 ±25 (classic, 200k nodes), +15 and +14 ±22 (majority, open) |
| Monte Carlo tree search instead of alpha-beta | -127 to -207 Elo at 100 ms per move |

`ulttt engine-accuracy` checks the engine against known values: on the
benchmark suites' solved positions, its move at 20k nodes, with the exact
solver turned off, keeps the position's value in 167 of 179 positions (all
nine p46, p42 and p38 suites; between 17/20 on open-p42 and 20/20 on
classic-p46, majority-p46 and majority-p38).

`ulttt engine-bench` measures speed (about 5M positions per second on one
core) and checksums the chosen moves, so pure speed changes can be checked to
play the same moves.

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
