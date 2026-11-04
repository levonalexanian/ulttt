# proofs

Lean 4 formalization of the rules in `RULES.md` and proofs about them. No
Mathlib; only Lean core. Every proof is checked by Lean's kernel (`decide`,
never `native_decide`).

Build with `lake build` (CI runs it with `--wfail`, so warnings fail too).

## Files

| File | Contents |
| --- | --- |
| `Ulttt/Basic.lean` | Players, the 8 lines of a 3×3 grid, outcomes |
| `Ulttt/State.lean` | Positions: cells, board owners, side to move, board sent to |
| `Ulttt/Rules.lean` | The `classic`, `open` and `majority` variants: legal moves, playing a move, game end, results |
| `Ulttt/Examples.lean` | Concrete games checked by evaluation |
| `Ulttt/Value.lean` | Every game ends; the game value under perfect play |
| `Ulttt/Theorems.lean` | Properties of the rules and how the variants relate |
| `Ulttt/Symmetry.lean` | The 8 rotations and reflections of the grid |
| `Ulttt/SymmetryRules.lean` | Symmetries preserve the rules and the game value |

## Main results

- `empties_play_lt`: every legal move fills an empty cell, so games last at
  most 81 moves and `value` (minimax over all legal moves) is well defined.
- `exists_legal_iff`: a legal move exists exactly when the game is not over.
- `owner_play_of_owner`: once a local board is won, its owner never changes.
- `legal_classic_eq_majority`, `outcome_classic_eq_majority_of_globalLine`:
  `classic` and `majority` differ only in scoring games without a global line.
- `map_legal`, `map_play`, `map_outcome`: applying a symmetry to a position
  preserves which moves are legal, commutes with playing, and keeps the result.
- `value_map`: symmetric positions have the same value. This is what makes
  symmetry reduction in the solver sound.
