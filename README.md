# ulttt

A high-performance Ultimate Tic-Tac-Toe solver in Rust, with the game rules and
certificate checking formalized in Lean 4.

Work in progress.

## Layout

- `docs/` — rules spec (`RULES.md`) and design notes
- `backend/` — Rust workspace
- `proofs/` — Lean 4 formalization
- `frontend/` — React + TypeScript UI
- `scripts/` — dev and benchmark scripts
- `data/` — local search artifacts and databases (gitignored)

## Development

Everything runs in the devcontainer (`.devcontainer/`). Open the repo in VS Code
with Dev Containers, or use `make image` and `make shell` with plain Docker.
