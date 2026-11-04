/-! Ultimate Tic-Tac-Toe: basic definitions shared by every rule variant. -/

namespace Ulttt

/-- The two players. X moves first. -/
inductive Player where
  | X
  | O
  deriving DecidableEq, Repr, Inhabited

/-- The opponent of a player. -/
def Player.other : Player → Player
  | .X => .O
  | .O => .X

@[simp] theorem Player.other_other (p : Player) : p.other.other = p := by
  cases p <;> rfl

@[simp] theorem Player.other_ne (p : Player) : p.other ≠ p := by
  cases p <;> decide

/-- The 8 lines (rows, columns, diagonals) of a 3×3 grid numbered row-major:
```
0 1 2
3 4 5
6 7 8
```
-/
def lines : List (Fin 9 × Fin 9 × Fin 9) :=
  [(0, 1, 2), (3, 4, 5), (6, 7, 8),
   (0, 3, 6), (1, 4, 7), (2, 5, 8),
   (0, 4, 8), (2, 4, 6)]

/-- Whether the 3×3 grid `f` has a line whose three entries all equal `a`. -/
def hasLine {α : Type} [DecidableEq α] (f : Fin 9 → α) (a : α) : Bool :=
  lines.any fun (i, j, k) => decide (f i = a) && decide (f j = a) && decide (f k = a)

/-- The result of a finished game. -/
inductive Outcome where
  | win (p : Player)
  | draw
  deriving DecidableEq, Repr

end Ulttt
