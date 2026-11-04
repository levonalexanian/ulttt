/-! Ultimate Tic-Tac-Toe: basic definitions. -/

namespace Ulttt

/-- The two players. X moves first. -/
inductive Player where
  | X
  | O
  deriving DecidableEq, Repr, Inhabited

end Ulttt
