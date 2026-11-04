import Ulttt.Basic

/-! Positions: the 81 cells, local board ownership, side to move and the board
the side to move was sent to. -/

namespace Ulttt

/-- A move is one of the 81 cells. Move `m` is cell `m % 9` of local board `m / 9`. -/
abbrev Move := Fin 81

/-- The move at cell `c` of local board `b`. -/
def Move.mk (b c : Fin 9) : Move := ⟨9 * b.val + c.val, by omega⟩

/-- The local board a move is played on. -/
def Move.board (m : Move) : Fin 9 := ⟨m.val / 9, by omega⟩

/-- The cell of its local board a move is played on; the opponent is sent to
the local board with this index. -/
def Move.cell (m : Move) : Fin 9 := ⟨m.val % 9, by omega⟩

@[simp] theorem Move.board_mk (b c : Fin 9) : (Move.mk b c).board = b := by
  ext; simp only [Move.board, Move.mk]; omega

@[simp] theorem Move.cell_mk (b c : Fin 9) : (Move.mk b c).cell = c := by
  ext; simp only [Move.cell, Move.mk]; omega

@[simp] theorem Move.mk_board_cell (m : Move) : Move.mk m.board m.cell = m := by
  ext; simp only [Move.board, Move.cell, Move.mk]; omega

/-- A position.

Ownership of local boards is stored rather than derived from the cells: in the
`open` variant a won board stays playable, and its owner is whoever completed a
line on it first, even if the other player later completes one too. -/
@[ext] structure State where
  /-- Contents of each of the 81 cells. -/
  cells : Move → Option Player
  /-- Owner of each local board, set the first time a player completes a line on it. -/
  owner : Fin 9 → Option Player
  /-- The player to move. -/
  toMove : Player
  /-- The local board the player to move was sent to; `none` before the first move. -/
  sent : Option (Fin 9)

/-- The empty starting position, X to move. -/
def State.init : State where
  cells _ := none
  owner _ := none
  toMove := .X
  sent := none

/-- Local board `b` as a 3×3 grid. -/
def State.localBoard (s : State) (b : Fin 9) : Fin 9 → Option Player :=
  fun c => s.cells (Move.mk b c)

/-- Whether every cell of local board `b` is occupied. -/
def State.full (s : State) (b : Fin 9) : Bool :=
  (List.finRange 9).all fun c => (s.localBoard b c).isSome

/-- Whether player `p` owns three local boards in a line. -/
def State.globalLine (s : State) (p : Player) : Bool :=
  hasLine s.owner (some p)

/-- Number of local boards owned by `p`. -/
def State.wonCount (s : State) (p : Player) : Nat :=
  (List.finRange 9).countP fun b => decide (s.owner b = some p)

/-- Number of empty cells. Every move fills one, so this bounds the game length. -/
def State.empties (s : State) : Nat :=
  (List.finRange 81).countP fun m => (s.cells m).isNone

end Ulttt
