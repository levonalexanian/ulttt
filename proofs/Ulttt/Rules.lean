import Ulttt.State

/-! The rules of the three supported variants, following `RULES.md`. -/

namespace Ulttt

/-- The supported rule variants. See `RULES.md`. -/
inductive Variant where
  /-- Won or full local boards are closed. -/
  | classic
  /-- Won local boards stay playable until full (Bertholon et al., 2020). -/
  | «open»
  /-- `classic`, but without a global line the player with more local boards wins. -/
  | majority
  deriving DecidableEq, Repr

namespace State

variable (v : Variant) (s : State)

/-- Whether local board `b` can no longer be played in. -/
def closed (b : Fin 9) : Bool :=
  match v with
  | .«open» => s.full b
  | .classic | .majority => (s.owner b).isSome || s.full b

/-- Whether some local board is still playable. -/
def anyOpen : Bool :=
  (List.finRange 9).any fun b => !s.closed v b

/-- Whether the game is over: someone has a global line or nothing is playable. -/
def terminal : Bool :=
  s.globalLine .X || s.globalLine .O || !s.anyOpen v

/-- Whether the player to move may play on local board `b`: it must be open,
and must be the board they were sent to unless that board is closed. -/
def allowedBoard (b : Fin 9) : Bool :=
  !s.closed v b &&
    match s.sent with
    | none => true
    | some t => s.closed v t || decide (b = t)

/-- Whether `m` is a legal move. -/
def legal (m : Move) : Bool :=
  !s.terminal v && (s.cells m).isNone && s.allowedBoard v m.board

/-- All legal moves, in increasing order. -/
def legalMoves : List Move :=
  (List.finRange 81).filter (s.legal v)

/-- The position after the player to move plays `m`. The same in every variant;
only which moves are legal differs. Assumes `m` is legal. -/
def play (m : Move) : State :=
  let p := s.toMove
  let cells' : Move → Option Player := fun i => if i = m then some p else s.cells i
  { cells := cells'
    owner := fun b =>
      if b = m.board && (s.owner b).isNone && hasLine (fun c => cells' (Move.mk b c)) (some p)
      then some p else s.owner b
    toMove := p.other
    sent := some m.cell }

/-- The result of a finished game. Only meaningful when `s.terminal v`. -/
def outcome : Outcome :=
  if s.globalLine .X then .win .X
  else if s.globalLine .O then .win .O
  else match v with
    | .majority =>
      if s.wonCount .O < s.wonCount .X then .win .X
      else if s.wonCount .X < s.wonCount .O then .win .O
      else .draw
    | .classic | .«open» => .draw

end State

end Ulttt
