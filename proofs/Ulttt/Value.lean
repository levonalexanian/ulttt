import Ulttt.Rules

/-! Every legal move fills an empty cell, so every game ends within 81 moves.
That makes the game value under perfect play a well-defined function, which is
the specification a solver's answers are measured against. -/

namespace Ulttt

/-- If `q` implies `p` on `l`, and some element of `l` satisfies `p` but not
`q`, then fewer elements satisfy `q` than `p`. -/
theorem List.countP_lt_of_imp {α : Type} (l : List α) (p q : α → Bool)
    (himp : ∀ x ∈ l, q x = true → p x = true) (x : α) (hx : x ∈ l)
    (hp : p x = true) (hq : q x = false) : l.countP q < l.countP p := by
  induction l with
  | nil => simp at hx
  | cons y ys ih =>
    have himp' : ∀ z ∈ ys, q z = true → p z = true :=
      fun z hz => himp z (List.mem_cons_of_mem _ hz)
    have hle : ys.countP q ≤ ys.countP p := List.countP_mono_left himp'
    simp only [List.countP_cons]
    rcases List.mem_cons.mp hx with rfl | hx'
    · simp [hp, hq]; omega
    · have := ih himp' hx'
      by_cases hqy : q y = true
      · have hpy := himp y (by simp) hqy
        simp [hqy, hpy]; omega
      · simp only [Bool.not_eq_true] at hqy
        cases p y <;> simp [hqy] <;> omega

namespace State

variable {v : Variant} {s : State}

theorem cells_isNone_of_legal {m : Move} (h : s.legal v m = true) :
    (s.cells m).isNone = true := by
  simp only [legal, Bool.and_eq_true] at h
  exact h.1.2

theorem not_terminal_of_legal {m : Move} (h : s.legal v m = true) :
    s.terminal v = false := by
  simp only [legal, Bool.and_eq_true, Bool.not_eq_true'] at h
  exact h.1.1

/-- Playing on an empty cell strictly decreases the number of empty cells. -/
theorem empties_play_lt {m : Move} (h : (s.cells m).isNone = true) :
    (s.play m).empties < s.empties := by
  unfold empties
  apply List.countP_lt_of_imp _ _ _ _ m (List.mem_finRange m) h
  · simp [play]
  · intro i _ hi
    simp only [play] at hi
    split at hi
    · simp at hi
    · exact hi

theorem empties_le (s : State) : s.empties ≤ 81 := by
  have := List.countP_le_length (l := List.finRange 81) (p := fun m => (s.cells m).isNone)
  simpa [empties] using this

theorem mem_legalMoves {m : Move} : m ∈ s.legalMoves v ↔ s.legal v m = true := by
  simp [legalMoves]

end State

/-- The score of a finished game from X's point of view. -/
def Outcome.score : Outcome → Int
  | .win .X => 1
  | .win .O => -1
  | .draw => 0

/-- The value of a position under perfect play by both sides, from X's point of
view: `1` if X can force a win, `-1` if O can, `0` if both can force at least a
draw. X maximizes and O minimizes. -/
def value (v : Variant) (s : State) : Int :=
  if s.terminal v then (s.outcome v).score
  else
    let vals := (s.legalMoves v).attach.map fun ⟨m, _⟩ => value v (s.play m)
    match s.toMove with
    | .X => vals.foldl max (-1)
    | .O => vals.foldl min 1
termination_by s.empties
decreasing_by
  exact State.empties_play_lt (State.cells_isNone_of_legal (State.mem_legalMoves.mp ‹_›))

end Ulttt
