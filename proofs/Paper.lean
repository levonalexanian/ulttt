import Ulttt

/-! `ulttt-paper`: checks that the winning strategy for X in `open` of
Bertholon et al. (2020), `Ulttt.paperMove`, wins against every reply. When it
prints `ok`, `Ulttt.paper_wins` gives `1 ≤ valueFor .open .X State.init`: X
can force a win from the empty board. -/

open Ulttt

def main : IO UInt32 := do
  IO.println s!"{paperPositions.length} positions reached by the strategy"
  if paperCheck then
    IO.println "ok: the strategy wins open for X against every reply"
    return 0
  else
    IO.println "rejected"
    return 1
