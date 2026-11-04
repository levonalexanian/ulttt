import Ulttt.Rules

/-! Faster versions of rule functions, each proved equal to its specification.
Programs such as the certificate checker run these; proofs rewrite them back
to the specification. -/

namespace Ulttt

namespace State

/-- The same position with cells and owners stored in arrays. A position built
by many `play`s is a deep chain of closures, so looking up a cell gets slower
with every move; after `compact` lookups take constant time. -/
def compact (s : State) : State :=
  let cells := Array.ofFn s.cells
  let owner := Array.ofFn s.owner
  { s with
    cells := fun m => cells[m.val]'(by simp [cells])
    owner := fun b => owner[b.val]'(by simp [owner]) }

@[simp] theorem compact_eq (s : State) : s.compact = s := by
  cases s
  simp only [compact, State.mk.injEq, and_true]
  constructor <;> funext i <;> simp only [Array.getElem_ofFn]

/-- `legalMoves`, deciding whether the game is over and which boards are
allowed once per position instead of once per cell. -/
def legalMovesFast (v : Variant) (s : State) : List Move :=
  if s.terminal v then []
  else
    let allowed := Array.ofFn (s.allowedBoard v)
    (List.finRange 81).filter fun m =>
      (s.cells m).isNone && allowed[m.board.val]'(by simp [allowed])

@[simp] theorem legalMovesFast_eq (v : Variant) (s : State) :
    s.legalMovesFast v = s.legalMoves v := by
  unfold legalMovesFast legalMoves legal
  by_cases ht : s.terminal v = true
  · simp [ht]
  · simp only [ht, Bool.false_eq_true, ↓reduceIte, Bool.not_false, Bool.true_and]
    apply List.filter_congr
    intro m _
    simp

end State

end Ulttt
