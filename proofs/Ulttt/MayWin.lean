import Ulttt.Theorems
import Ulttt.ValueFor

/-! When can a player no longer win?

`mayWin v s p` over-approximates whether player `p` can still win: a local
board is *winnable* for `p` if `p` owns it, or nobody owns it, it is still
playable, and some line on it holds no opponent mark. If no line of winnable
boards exists (and, in `majority`, `p` cannot reach more boards than the
opponent already owns), `p` can do no better than a draw.

`valueFor_le_zero_of_mayWin_false` proves this, so solvers may prune with it
and certificates may stop early with it. The Rust engine's `may_win` computes
the same predicate (checked by `ulttt-difftest`). -/

namespace Ulttt

namespace State

/-- Whether cell `c` of local board `b` holds a mark of `q`. -/
def marks (s : State) (q : Player) (b c : Fin 9) : Bool :=
  decide (s.cells (Move.mk b c) = some q)

/-- Whether some line of local board `b` holds no mark of `q`. -/
def lineFree (s : State) (q : Player) (b : Fin 9) : Bool :=
  lines.any fun (i, j, k) => !s.marks q b i && !s.marks q b j && !s.marks q b k

/-- Whether `p` owns local board `b` or might still win it. -/
def winnable (v : Variant) (s : State) (p : Player) (b : Fin 9) : Bool :=
  decide (s.owner b = some p) ||
    (decide (s.owner b = none) && !s.closed v b && s.lineFree p.other b)

/-- Whether `p` might still win the game. `false` means `p` can at best draw. -/
def mayWin (v : Variant) (s : State) (p : Player) : Bool :=
  hasLine (s.winnable v p) true ||
    (decide (v = .majority) &&
      decide (s.wonCount p.other < (List.finRange 9).countP (s.winnable v p)))

end State

/-! ### Lines and monotonicity -/

theorem hasLine_mono {f g : Fin 9 → Bool} (h : ∀ i, f i = true → g i = true)
    (hf : hasLine f true = true) : hasLine g true = true := by
  simp only [hasLine, List.any_eq_true, Bool.and_eq_true, decide_eq_true_eq] at hf ⊢
  obtain ⟨⟨i, j, k⟩, hl, ⟨hi, hj⟩, hk⟩ := hf
  exact ⟨(i, j, k), hl, ⟨h _ hi, h _ hj⟩, h _ hk⟩

theorem hasLine_decide {α : Type} [DecidableEq α] (f : Fin 9 → α) (a : α) :
    hasLine f a = hasLine (fun i => decide (f i = a)) true := by
  simp [hasLine]

namespace State

variable {v : Variant} {s : State} {m : Move}

theorem owner_play (b : Fin 9) : (s.play m).owner b =
    if b = m.board && (s.owner b).isNone &&
        hasLine (fun c => (s.play m).cells (Move.mk b c)) (some s.toMove)
    then some s.toMove else s.owner b := rfl

/-- A cell `p` holds after a move on an empty cell was not the opponent's before. -/
theorem not_other_of_play {p : Player} {i : Move} (hm : (s.cells m).isNone = true)
    (h : (s.play m).cells i = some p) : s.cells i ≠ some p.other := by
  rw [cells_play] at h
  intro h'
  split at h
  · rename_i he
    subst he
    rw [h'] at hm
    cases hm
  · rw [h] at h'
    exact Player.other_ne p (Option.some.inj h').symm

theorem owner_none_of_play {b : Fin 9} (h : (s.play m).owner b = none) : s.owner b = none := by
  cases ho : s.owner b with
  | none => rfl
  | some p => rw [owner_play_of_owner ho] at h; cases h

theorem full_play {b : Fin 9} (h : s.full b = true) : (s.play m).full b = true := by
  simp only [full, List.all_eq_true, List.mem_finRange, true_implies, localBoard,
    cells_play] at h ⊢
  intro c
  split <;> simp_all

theorem closed_play {b : Fin 9} (h : s.closed v b = true) : (s.play m).closed v b = true := by
  cases v <;> simp only [closed, Bool.or_eq_true] at h ⊢
  · rcases h with h | h
    · left
      obtain ⟨p, hp⟩ := Option.isSome_iff_exists.mp h
      simp [owner_play_of_owner hp]
    · exact Or.inr (full_play h)
  · exact full_play h
  · rcases h with h | h
    · left
      obtain ⟨p, hp⟩ := Option.isSome_iff_exists.mp h
      simp [owner_play_of_owner hp]
    · exact Or.inr (full_play h)

/-- Marks never disappear when an empty cell is played. -/
theorem marks_play {q : Player} {b c : Fin 9} (hm : (s.cells m).isNone = true)
    (h : s.marks q b c = true) : (s.play m).marks q b c = true := by
  simp only [marks, decide_eq_true_eq, cells_play] at h ⊢
  split
  · rename_i he
    rw [he] at h
    simp [h] at hm
  · simpa using h

/-- A mark after a move was there before, or is the move itself. -/
theorem marks_of_play {q : Player} {b c : Fin 9} (h : (s.play m).marks q b c = true) :
    s.marks q b c = true ∨ (Move.mk b c = m ∧ s.toMove = q) := by
  simp only [marks, decide_eq_true_eq, cells_play] at h ⊢
  split at h
  · rename_i he
    exact Or.inr ⟨he, Option.some.inj (of_decide_eq_true h)⟩
  · exact Or.inl (of_decide_eq_true h)

theorem lineFree_of_play {q : Player} {b : Fin 9} (hm : (s.cells m).isNone = true)
    (h : (s.play m).lineFree q b = true) : s.lineFree q b = true := by
  simp only [lineFree, List.any_eq_true, Bool.and_eq_true, Bool.not_eq_true'] at h ⊢
  obtain ⟨l, hl, ⟨hi, hj⟩, hk⟩ := h
  refine ⟨l, hl, ⟨?_, ?_⟩, ?_⟩ <;>
    · cases hx : s.marks q b _
      · rfl
      · first
          | (rw [marks_play hm hx] at hi; cases hi)
          | (rw [marks_play hm hx] at hj; cases hj)
          | (rw [marks_play hm hx] at hk; cases hk)

/-- Winnable boards only become fewer. -/
theorem winnable_of_play {p : Player} {b : Fin 9} (hl : s.legal v m = true)
    (h : (s.play m).winnable v p b = true) : s.winnable v p b = true := by
  have hm := cells_isNone_of_legal hl
  simp only [winnable, Bool.or_eq_true, Bool.and_eq_true, decide_eq_true_eq,
    Bool.not_eq_true'] at h ⊢
  rcases h with h | ⟨⟨hn, hc⟩, hf⟩
  · -- `p` owns `b` after the move: either before too, or by this move.
    rw [owner_play] at h
    split at h
    · rename_i hnew
      simp only [Bool.and_eq_true, decide_eq_true_eq, Option.isNone_iff_eq_none] at hnew
      obtain ⟨⟨hb, hn⟩, hline⟩ := hnew
      have hp : s.toMove = p := Option.some.inj h
      right
      subst hb
      have hallowed : s.allowedBoard v m.board = true := by
        simp only [legal, Bool.and_eq_true] at hl
        exact hl.2
      refine ⟨⟨hn, ?_⟩, ?_⟩
      · simp only [allowedBoard, Bool.and_eq_true, Bool.not_eq_true'] at hallowed
        exact hallowed.1
      · -- The completed line holds only `p`'s marks, so none of the opponent's.
        simp only [hasLine, List.any_eq_true, Bool.and_eq_true, decide_eq_true_eq] at hline
        obtain ⟨⟨i, j, k⟩, hl', ⟨hi, hj⟩, hk⟩ := hline
        simp only at hi hj hk
        rw [hp] at hi hj hk
        simp only [lineFree, List.any_eq_true, Bool.and_eq_true, Bool.not_eq_true', marks,
          decide_eq_false_iff_not]
        exact ⟨(i, j, k), hl', ⟨not_other_of_play hm hi, not_other_of_play hm hj⟩,
          not_other_of_play hm hk⟩
    · exact Or.inl h
  · right
    refine ⟨⟨owner_none_of_play hn, ?_⟩, lineFree_of_play hm hf⟩
    cases hcs : s.closed v b
    · rfl
    · rw [closed_play hcs] at hc; cases hc

theorem wonCount_le_play (p : Player) : s.wonCount p ≤ (s.play m).wonCount p := by
  apply List.countP_mono_left
  intro b _ h
  simp only [decide_eq_true_eq] at h ⊢
  exact owner_play_of_owner h

/-- A player ruled out stays ruled out. -/
theorem mayWin_of_play {p : Player} (hl : s.legal v m = true)
    (h : (s.play m).mayWin v p = true) : s.mayWin v p = true := by
  simp only [mayWin, Bool.or_eq_true, Bool.and_eq_true, decide_eq_true_eq] at h ⊢
  rcases h with h | ⟨hv, hc⟩
  · exact Or.inl (hasLine_mono (fun b hb => winnable_of_play hl hb) h)
  · right
    refine ⟨hv, ?_⟩
    have h1 := wonCount_le_play (s := s) (m := m) p.other
    have h2 : (List.finRange 9).countP ((s.play m).winnable v p) ≤
        (List.finRange 9).countP (s.winnable v p) :=
      List.countP_mono_left fun b _ hb => winnable_of_play hl hb
    omega

/-- A ruled-out player has no global line. -/
theorem globalLine_false_of_mayWin_false {p : Player} (h : s.mayWin v p = false) :
    s.globalLine p = false := by
  simp only [mayWin, Bool.or_eq_false_iff] at h
  cases hg : s.globalLine p
  · rfl
  · have : hasLine (s.winnable v p) true = true := by
      apply hasLine_mono _ (by rw [globalLine, hasLine_decide] at hg; exact hg)
      intro b hb
      simp only [winnable, Bool.or_eq_true, decide_eq_true_eq] at hb ⊢
      exact Or.inl hb
    rw [h.1] at this; cases this

/-- A ruled-out player owns no more boards than the opponent, in `majority`. -/
theorem wonCount_le_of_mayWin_false {p : Player} (h : s.mayWin .majority p = false) :
    s.wonCount p ≤ s.wonCount p.other := by
  simp only [mayWin, Bool.or_eq_false_iff, Bool.and_eq_false_iff, decide_eq_false_iff_not,
    Nat.not_lt] at h
  have hle : s.wonCount p ≤ (List.finRange 9).countP (s.winnable .majority p) := by
    apply List.countP_mono_left
    intro b _ hb
    simp only [winnable, Bool.or_eq_true, decide_eq_true_eq] at hb ⊢
    exact Or.inl hb
  rcases h.2 with h | h
  · simp at h
  · omega

/-- A finished game is worth at most a draw to a ruled-out player. -/
theorem scoreFor_le_zero_of_mayWin_false {p : Player} (h : s.mayWin v p = false) :
    (s.outcome v).scoreFor p ≤ 0 := by
  have hg := globalLine_false_of_mayWin_false h
  cases p with
  | X =>
    simp only [outcome, hg, Bool.false_eq_true, ↓reduceIte]
    split
    · simp [Outcome.scoreFor, Outcome.score]
    · cases v
      · simp [Outcome.scoreFor, Outcome.score]
      · simp [Outcome.scoreFor, Outcome.score]
      · have := wonCount_le_of_mayWin_false h
        simp only [Player.other] at this
        simp only [show ¬ s.wonCount .O < s.wonCount .X by omega, ↓reduceIte]
        split <;> simp [Outcome.scoreFor, Outcome.score]
  | O =>
    simp only [outcome]
    split
    · simp [Outcome.scoreFor, Outcome.score]
    · simp only [hg, Bool.false_eq_true, ↓reduceIte]
      cases v
      · simp [Outcome.scoreFor, Outcome.score]
      · simp [Outcome.scoreFor, Outcome.score]
      · have := wonCount_le_of_mayWin_false h
        simp only [Player.other] at this
        simp only [show ¬ s.wonCount .X < s.wonCount .O by omega, ↓reduceIte]
        split <;> simp [Outcome.scoreFor, Outcome.score]

end State

/-- **Soundness of `mayWin`**: a player it rules out can at best draw. -/
theorem valueFor_le_zero_of_mayWin_false (v : Variant) (p : Player) (s : State)
    (h : s.mayWin v p = false) : valueFor v p s ≤ 0 := by
  by_cases ht : s.terminal v = true
  · have := State.scoreFor_le_zero_of_mayWin_false (s := s) h
    cases p <;> simp only [valueFor, Outcome.scoreFor] at this ⊢ <;>
      rw [value_unfold v s] <;> simp only [ht, ↓reduceIte] <;> omega
  · have hnt : s.terminal v = false := by simpa using ht
    have hchild : ∀ m, s.legal v m = true → valueFor v p (s.play m) ≤ 0 := by
      intro m hm
      have hc : (s.play m).mayWin v p = false := by
        cases hc : (s.play m).mayWin v p
        · rfl
        · rw [State.mayWin_of_play hm hc] at h; cases h
      exact valueFor_le_zero_of_mayWin_false v p (s.play m) hc
    by_cases hp : s.toMove = p
    · exact valueFor_le_of_all (by omega) hnt hp hchild
    · obtain ⟨m, hm⟩ := State.exists_legal_iff.mpr hnt
      exact valueFor_le_of_exists hnt hp hm (hchild m hm)
termination_by s.empties
decreasing_by exact State.empties_play_lt (State.cells_isNone_of_legal hm)

end Ulttt
