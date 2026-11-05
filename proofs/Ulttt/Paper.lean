import Ulttt.Closed

/-! The winning strategy for X in `open` of Bertholon, Géraud-Stewart,
Kugelmann, Lenoir and Naccache, "At Most 43 Moves, At Least 29: Optimal
Strategies and Bounds for Ultimate Tic-Tac-Toe" (2020), and its proof.

The paper's strategy, as a function of the position alone:

- **Opening**: play the centre of the centre board, then the centre of every
  board O sends X to. O is sent back to the centre board until it is full. The
  board O sends X to last is `f` (the only board whose centre X lacks), and
  `g = 8 - f` is the board opposite it.
- **Middlegame**: play cell `f` of `f`. When O plays cell `k` of `f`, answer
  cell `f` of `k`, sending O back to `f`; when O plays cell 4 or `g`, answer
  cell `f` of `g`, or cell `g` of `g` once that is taken.
- **Endgame**: on the board `k` X is sent to, play cell `f` if free, else `g`.

The paper proves that it wins by an invariant argument. Here instead the
positions it can reach are enumerated (64k of them), and `closedSetOk`
checks that they form a closed set: X's move is always legal and stays in the
set, every reply of O stays in the set, and every finished game is won by X.
`paper_wins` turns that check into `1 ≤ valueFor .open .X State.init`. -/

namespace Ulttt

/-- The paper's move for X; `none` where it gives none. -/
def paperMove (s : State) : Option Move :=
  if s.toMove ≠ .X then none
  else if s.empties = 81 then some (Move.mk 4 4)
  else if !s.full 4 then s.sent.map fun k => Move.mk k 4
  else
    match (List.finRange 9).filter fun b => b ≠ 4 && s.cells (Move.mk b 4) ≠ some .X with
    | [f] =>
      let g : Fin 9 := ⟨8 - f.val, by omega⟩
      let free (m : Move) : Bool := (s.cells m).isNone
      -- Sent to the full centre board, to `g`, or (never, in fact) anywhere else.
      let toG := if free (Move.mk g f) then Move.mk g f else Move.mk g g
      match s.sent with
      | some k =>
        if k = f then some (Move.mk f f)
        else if k ≠ 4 && k ≠ g && !s.full k then
          some (if free (Move.mk k f) then Move.mk k f else Move.mk k g)
        else some toG
      | none => some toG
    | _ => none

/-! ### The positions the strategy reaches

This enumeration is not trusted: `closedSetOk` checks whatever it returns. -/

/-- Every position reachable from `s` with X following `paperMove`, added to
`acc` (whose keys are in `seen`), by depth-first search. -/
partial def paperReach (s : State) (seen : Std.HashSet (List Nat)) (acc : Array State) :
    Std.HashSet (List Nat) × Array State :=
  let s := s.compact
  let k := s.key
  if seen.contains k then (seen, acc)
  else
    let seen := seen.insert k
    let acc := acc.push s
    let v := Variant.«open»
    if s.terminal v then (seen, acc)
    else if s.toMove = .X then
      match paperMove s with
      | some m => paperReach (s.play m) seen acc
      | none => (seen, acc)
    else
      (s.legalMovesFast v).foldl (fun (seen, acc) m => paperReach (s.play m) seen acc) (seen, acc)

/-- The positions reached by the strategy, starting with the empty board. -/
def paperPositions : List State :=
  State.init :: ((paperReach State.init {} #[]).2.toList)

/-- Whether the reached positions form a closed set in which X wins. -/
def paperCheck : Bool :=
  closedSetOk .«open» .X 1 paperMove paperPositions

/-- **The paper's strategy wins `open` for X**, given that `paperCheck`
evaluates to `true` (which `ulttt-paper` runs). -/
theorem paper_wins (h : paperCheck = true) : 1 ≤ valueFor .«open» .X State.init :=
  closedSetOk_sound .«open» .X 1 (by decide) paperMove paperPositions h State.init
    (List.mem_cons_self ..)

end Ulttt
