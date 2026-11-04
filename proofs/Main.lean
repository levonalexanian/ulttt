import Ulttt

/-! `ulttt-ref`: a reference implementation of the rules, run directly from the
Lean definitions, for differential testing against the Rust engine.

Each input line is `<variant> <move> <move> ...` with moves as indices 0-80.
For the start position and after each move, one output line is printed:
`<legal moves>|<owners>|<result>` where legal moves are comma-separated
indices, owners are 9 of `x`/`o`/`.`, and result is `x`, `o`, `d` (draw) or
`-` (not over). Moves are played without checking legality.

A line `value <variant> <move> ...` instead prints the game value (`1`, `0`
or `-1`, from X's point of view) of the position after the moves, computed by
`Ulttt.value`. That is a full minimax, so only use it near the end of a game. -/

open Ulttt

def parseVariant : String → Option Variant
  | "classic" => some .classic
  | "open" => some .«open»
  | "majority" => some .majority
  | _ => none

def symbol : Option Player → Char
  | some .X => 'x'
  | some .O => 'o'
  | none => '.'

def describe (v : Variant) (s : State) : String :=
  let legal := (s.legalMoves v).map fun m => toString m.val
  let owners := String.ofList ((List.finRange 9).map fun b => symbol (s.owner b))
  let result :=
    if s.terminal v then
      match s.outcome v with
      | .win .X => "x"
      | .win .O => "o"
      | .draw => "d"
    else "-"
  s!"{",".intercalate legal}|{owners}|{result}"

/-- Splits a line into words separated by spaces, ignoring line endings. -/
def words (line : String) : List String :=
  let rec go : List Char → List Char → List String
    | [], cur => if cur.isEmpty then [] else [String.ofList cur.reverse]
    | c :: cs, cur =>
      if c == ' ' || c == '\n' || c == '\r' then
        if cur.isEmpty then go cs [] else String.ofList cur.reverse :: go cs []
      else go cs (c :: cur)
  go line.toList []

/-- The position after the moves, or an error. -/
def playMoves (ms : List String) : Except String State := do
  let mut s := State.init
  for m in ms do
    let some n := m.toNat? | throw s!"bad move {m}"
    if h : n < 81 then
      s := (s.play ⟨n, h⟩).compact
    else throw s!"bad move {m}"
  return s

def runLine (line : String) : Except String (List String) := do
  match words line with
  | [] => throw "empty line"
  | "value" :: vs :: ms =>
    let some v := parseVariant vs | throw s!"unknown variant {vs}"
    let s ← playMoves ms
    return [toString (value v s)]
  | vs :: ms =>
    let some v := parseVariant vs | throw s!"unknown variant {vs}"
    let mut s := State.init
    let mut out := [describe v s]
    for m in ms do
      let some n := m.toNat? | throw s!"bad move {m}"
      if h : n < 81 then
        s := (s.play ⟨n, h⟩).compact
        out := describe v s :: out
      else throw s!"bad move {m}"
    return out.reverse

partial def loop (stdin : IO.FS.Stream) (stdout : IO.FS.Stream) : IO Unit := do
  let line ← stdin.getLine
  if line.isEmpty then return
  match runLine line with
  | .ok out => for l in out do stdout.putStrLn l
  | .error e => stdout.putStrLn s!"error: {e}"
  stdout.flush
  loop stdin stdout

def main : IO Unit := do
  loop (← IO.getStdin) (← IO.getStdout)
