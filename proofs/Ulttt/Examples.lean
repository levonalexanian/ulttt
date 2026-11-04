import Ulttt.Rules

/-! Concrete sanity checks of the rules, checked by evaluation. -/

namespace Ulttt

open State

/-- Play a list of moves, given as `(board, cell)` pairs, from the start. -/
def playAll (moves : List (Fin 9 × Fin 9)) : State :=
  moves.foldl (fun s (b, c) => s.play (Move.mk b c)) State.init

/-- Whether every move in the list is legal when played in order. -/
def allLegal (v : Variant) (moves : List (Fin 9 × Fin 9)) : Bool :=
  (moves.foldl (fun (s, ok) (b, c) =>
    (s.play (Move.mk b c), ok && s.legal v (Move.mk b c))) (State.init, true)).2

/-- The legal moves as `(board, cell)` pairs. -/
def legalPairs (v : Variant) (s : State) : List (Fin 9 × Fin 9) :=
  (s.legalMoves v).map fun m => (m.board, m.cell)

-- Every cell is legal at the start.
example : (State.init.legalMoves .classic).length = 81 := by decide
example : (State.init.legalMoves .«open»).length = 81 := by decide
example : (State.init.legalMoves .majority).length = 81 := by decide

-- After X plays cell 4 of board 0, O is sent to board 4.
example : legalPairs .classic (playAll [(0, 4)]) = (List.finRange 9).map ((4, ·)) := by
  decide

/-- X wins local board 0 on its top row, then O is sent to board 0. -/
def xWinsBoard0 : List (Fin 9 × Fin 9) := [(0, 1), (1, 0), (0, 2), (2, 0), (0, 0)]

example : allLegal .classic xWinsBoard0 = true := by decide
example : allLegal .«open» xWinsBoard0 = true := by decide
example : (playAll xWinsBoard0).owner 0 = some .X := by decide
example : (playAll xWinsBoard0).sent = some 0 := by decide

-- `classic`: board 0 is closed, so O may play any empty cell of boards 1-8.
example : (legalPairs .classic (playAll xWinsBoard0)).length = 72 - 2 := by decide
-- `open`: board 0 is still playable, so O must play one of its 6 empty cells.
example : legalPairs .«open» (playAll xWinsBoard0) =
    [(0, 3), (0, 4), (0, 5), (0, 6), (0, 7), (0, 8)] := by decide

/-- In `open`, O later completes the middle row of board 0 without taking it over. -/
def oLineOnWonBoard : List (Fin 9 × Fin 9) :=
  xWinsBoard0 ++ [(0, 3), (3, 0), (0, 4), (4, 0), (0, 5)]

example : allLegal .«open» oLineOnWonBoard = true := by decide
example : (playAll oLineOnWonBoard).owner 0 = some .X := by decide
-- In `classic` the same moves are illegal: board 0 closed when X won it.
example : allLegal .classic oLineOnWonBoard = false := by decide

end Ulttt
