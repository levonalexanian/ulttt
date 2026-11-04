import Ulttt

/-! `ulttt-check`: checks solver certificates with `Ulttt.check`, whose
soundness is proved by `Ulttt.check_sound`.

Usage: `ulttt-check <certificate file>...`. Prints one line per file and exits
with status 1 if any certificate is rejected. The format is documented in
`backend/crates/certificate/src/lib.rs`. -/

open Ulttt

def parseVariant : String → Option Variant
  | "classic" => some .classic
  | "open" => some .«open»
  | "majority" => some .majority
  | _ => none

def parsePlayer : String → Option Player
  | "x" => some .X
  | "o" => some .O
  | _ => none

def parseCell : Char → Option (Option Player)
  | 'x' => some (some .X)
  | 'o' => some (some .O)
  | '.' => some none
  | _ => none

/-- Parses the position notation `<81 cells>/<9 owners>/<to move>/<sent>`. -/
def parseState (str : String) : Option State := do
  let [cells, owners, toMove, sent] := str.splitOn "/" | none
  let cells ← cells.toList.mapM parseCell
  let owners ← owners.toList.mapM parseCell
  if cells.length = 81 ∧ owners.length = 9 then
    let toMove ← parsePlayer toMove
    let sent ← match sent with
      | "-" => some none
      | s => s.toNat?.bind fun n => if h : n < 9 then some (some ⟨n, h⟩) else none
    let cellArr := cells.toArray
    let ownerArr := owners.toArray
    some {
      cells := fun m => cellArr[m.val]!
      owner := fun b => ownerArr[b.val]!
      toMove
      sent }
  else none

structure Certificate where
  variant : Variant
  player : Player
  atLeast : Int
  state : State
  moves : List Nat

/-- Splits on spaces, dropping empty words. -/
def words (s : String) : List String :=
  (s.splitOn " ").filter (· ≠ "")

def parseCertificate (text : String) : Except String Certificate := do
  let lines := (text.splitOn "\n").map fun l => (l.splitOn "\r").head!
  let some header := lines.head? | throw "empty file"
  if header ≠ "ulttt-certificate 1" then throw "missing header"
  let field (key : String) : Except String String :=
    match lines.find? (fun l => l == key || l.startsWith (key ++ " ")) with
    | some l => pure ((l.drop (key.length + 1)).toString)
    | none => throw s!"missing {key}"
  let some variant := parseVariant (← field "variant") | throw "bad variant"
  let some player := parsePlayer (← field "player") | throw "bad player"
  let atLeast ← match (← field "at-least") with
    | "0" => pure 0
    | "1" => pure 1
    | _ => throw "bad at-least"
  let some state := parseState (← field "position") | throw "bad position"
  let moves ← (words (← field "moves")).mapM fun w =>
    match w.toNat? with
    | some n => pure n
    | none => throw s!"bad move {w}"
  return { variant, player, atLeast, state, moves }

def claim (c : Certificate) : String :=
  let who := match c.player with
    | .X => "x"
    | .O => "o"
  let what := if c.atLeast = 1 then "a win" else "at least a draw"
  s!"{who} can force {what} ({repr c.variant})"

def checkFile (path : String) : IO Bool := do
  let text ← IO.FS.readFile path
  match parseCertificate text with
  | .error e =>
    IO.println s!"{path}: unreadable: {e}"
    return false
  | .ok c =>
    match check c.variant c.player c.atLeast c.state c.moves with
    | some [] =>
      IO.println s!"{path}: ok: {claim c}"
      return true
    | some _ =>
      IO.println s!"{path}: rejected: unused moves at the end"
      return false
    | none =>
      IO.println s!"{path}: rejected"
      return false

def main (args : List String) : IO UInt32 := do
  let mut ok := true
  for path in args do
    ok := (← checkFile path) && ok
  return if ok then 0 else 1
