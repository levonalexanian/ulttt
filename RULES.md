# Rules

Ultimate Tic-Tac-Toe has many house rules. This project supports three
variants. Every component (Rust engine, Lean formalization, frontend) must
implement all three exactly as written here, and must name the variant in any
result it reports.

## Common definitions

- The **global board** is a 3×3 grid of **local boards**; each local board is a
  3×3 grid of **cells**. There are 81 cells.
- Boards and cells are numbered 0–8 in row-major order:

  ```
  0 1 2
  3 4 5
  6 7 8
  ```

- A **move** is a pair `(b, c)` of a local board `b` and a cell `c`. Its index
  is `9*b + c` (0–80).
- **X** moves first. Players alternate.
- A **line** is one of the 8 rows, columns, or diagonals of a 3×3 grid.
- A local board is **won** by a player the first time that player occupies all
  three cells of a line on it. Once won, its owner never changes.
- A local board is **full** when all 9 of its cells are occupied.
- The player owning all three local boards of a line on the global board has a
  **global line**, and wins the game immediately.
- **Sending:** after a move `(b, c)`, the opponent is *sent* to local board `c`.
  The first move of the game may be any cell.

The variants differ only in which local boards are *playable* and in how a game
without a global line is scored.

## Variant `classic`

The most widely played rule set.

- A local board is **closed** once it is won or full. Closed boards are not
  playable.
- If the board you are sent to is open, you must play in it. If it is closed,
  you may play in any open board.
- The game ends when a player has a global line (that player wins) or when no
  open board remains (draw).
- A full local board that nobody won counts for nobody.

## Variant `open`

The rule set used by Bertholon et al., *"At Most 43 Moves, At Least 29: Optimal
Strategies and Bounds for Ultimate Tic-Tac-Toe"* (2020), who prove it is a
**first-player (X) win**. That known result is the main correctness target for
the solver.

- A won local board stays playable until it is full. Moves in it have no effect
  on its ownership.
- If the board you are sent to is not full, you must play in it. If it is full,
  you may play in any board that is not full.
- The game ends when a player has a global line (that player wins) or when all
  81 cells are occupied (draw).

## Variant `majority`

`classic` with a tiebreak, as used by several bot competitions.

- Playability, sending, and global lines are exactly as in `classic`.
- If the game ends with no open board and no global line, the player who won
  **more local boards** wins. Equal counts are a draw.

## Notes for implementers

- A global line is checked after every move. A move that wins a local board can
  also win the game.
- In `classic`, once neither player can still complete a global line the game
  value is a draw; solvers may stop there. This shortcut is **not** valid in
  `majority`, where later local boards still change the result.
- "Value" always means the result with perfect play by both sides, from the
  side of the player to move unless stated otherwise.
