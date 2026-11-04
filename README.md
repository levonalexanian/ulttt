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
| Rules in Lean | `proofs/` | The three variants, the game value, and proofs: games end, symmetries preserve value, certificate checker is sound |
| Engine | `backend/crates/game` | Bitboard rules, symmetries, notation, perft |
| Solver | `backend/crates/solver` | Alpha-beta with a lock-free transposition table, draw detection, move ordering, history heuristic, parallel search |
| Certificates | `backend/crates/certificate` | Strategy certificates for solved positions, checked by `ulttt-check` (Lean) |
| Storage | `backend/crates/db` | SQLite: benchmark suites, runs, solved positions |
| CLI | `backend/crates/cli` | `ulttt perft`, `solve`, `certify`, `suite-create`, `bench`, `runs` |
| Differential tests | `backend/crates/difftest` | Rust rules and solver values against the Lean definitions on random games |
| Frontend | `frontend/` | React app: play against the solver, analysis; the engine runs in the browser as WebAssembly |

## How the pieces check each other

- The Lean definitions in `proofs/Ulttt/Rules.lean` are the specification.
- `ulttt-difftest` plays thousands of random games and checks that the Rust
  engine agrees with the Lean rules on every position, and that the solver's
  values equal Lean's `value` on random endgames.
- `ulttt certify` writes a certificate (a strategy) for a solved position, and
  `ulttt-check` checks it with `Ulttt.check`, which Lean proves sound
  (`check_sound`). The solver itself need not be trusted.

## Development

Everything runs in the devcontainer (`.devcontainer/`): open the repo in VS Code
with Dev Containers, or use `make image` and `make shell` with plain Docker.

```sh
cd proofs && lake build                    # Lean proofs and executables
cd backend && cargo test --release         # Rust
backend/target/release/ulttt solve -v open "<position>"
scripts/create-suites.sh                   # benchmark suites in data/ulttt.db
backend/target/release/ulttt bench classic-p42 --solver search
make frontend                              # http://localhost:5173
```
