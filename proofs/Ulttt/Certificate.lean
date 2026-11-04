import Ulttt.Fast
import Ulttt.MayWin

/-! Checking solver certificates.

A certificate claims that player `p` can force a result worth at least `r` to
`p` (`r = 1`: a win, `r = 0`: at least a draw). It is `p`'s strategy, written
as the list of `p`'s moves in depth-first order: at a position where `p` is to
move, the next number is `p`'s move; where the opponent is to move, every legal
reply is explored, in increasing order. Finished games must be worth at least
`r` to `p`. When `r ≤ 0` and the opponent can no longer win (`mayWin`), the
claim holds without looking further, and the certificate stops there.

`check` consumes a certificate and returns what is left of it, or `none` if
the certificate is wrong. `check_sound` proves that a certificate accepted by
`check` really establishes the claimed bound on `value`, so the solver that
produced it need not be trusted. -/

namespace Ulttt

/-- Checks that `cert` is a strategy for `p` worth at least `r` from `s`.
Returns the unused rest of `cert` on success. Uses `compact` and
`legalMovesFast` for speed; both are proved equal to the plain definitions. -/
def check (v : Variant) (p : Player) (r : Int) (s : State) (cert : List Nat) :
    Option (List Nat) :=
  if s.terminal v then
    if r ≤ (s.outcome v).scoreFor p then some cert else none
  else if decide (r ≤ 0) && !s.mayWin v p.other then some cert
  else if s.toMove = p then
    match cert with
    | [] => none
    | n :: rest =>
      if h : n < 81 then
        if s.legal v ⟨n, h⟩ then check v p r (s.play ⟨n, h⟩).compact rest else none
      else none
  else
    (s.legalMovesFast v).attach.foldl
      (fun acc ⟨m, _⟩ => acc.bind fun c => check v p r (s.play m).compact c) (some cert)
termination_by s.empties
decreasing_by
  · simp only [State.compact_eq]
    exact State.empties_play_lt (State.cells_isNone_of_legal ‹_›)
  · simp only [State.compact_eq]
    rename_i hm
    rw [State.legalMovesFast_eq] at hm
    exact State.empties_play_lt (State.cells_isNone_of_legal (State.mem_legalMoves.mp hm))

/-! ### Soundness -/

theorem valueFor_other (v : Variant) (p : Player) (s : State) :
    valueFor v p.other s = -valueFor v p s := by
  cases p <;> simp [valueFor, Player.other]

/-- **Soundness**: if `check` accepts a certificate, player `p` can force a
result worth at least `r` (for `r ≤ 1`), i.e. `r ≤ valueFor v p s`. -/
theorem check_sound (v : Variant) (p : Player) (r : Int) (hr : r ≤ 1) (s : State)
    (cert rest : List Nat) (h : check v p r s cert = some rest) : r ≤ valueFor v p s := by
  rw [check.eq_def] at h
  by_cases ht : s.terminal v = true
  · simp only [ht, ↓reduceIte] at h
    split at h
    · rename_i hge
      cases p <;> simp only [valueFor, Outcome.scoreFor] at hge ⊢ <;>
        rw [value_unfold v s] <;> simp only [ht, ↓reduceIte] <;> omega
    · cases h
  · simp only [ht, Bool.false_eq_true, ↓reduceIte] at h
    have hnt : s.terminal v = false := by simpa using ht
    split at h
    · -- The opponent can no longer win, so `p` gets at least a draw.
      rename_i hshort
      simp only [Bool.and_eq_true, decide_eq_true_eq, Bool.not_eq_true'] at hshort
      have := valueFor_le_zero_of_mayWin_false v p.other s hshort.2
      rw [valueFor_other] at this
      omega
    split at h
    · rename_i hp
      split at h
      · cases h
      · rename_i n rest'
        split at h
        · rename_i hn
          split at h
          · rename_i hl
            have := check_sound v p r hr _ _ _ h
            rw [State.compact_eq] at this
            exact Int.le_trans this (valueFor_play_le hnt hp hl)
          · cases h
        · cases h
    · rename_i hp
      apply valueFor_ge_of_all hr hnt hp
      intro m hm
      have hmem : m ∈ s.legalMovesFast v := by
        rw [State.legalMovesFast_eq]
        exact State.mem_legalMoves.mpr hm
      obtain ⟨c₁, c₂, hc⟩ := foldl_bind_some _
        (fun (x : {x // x ∈ s.legalMovesFast v}) c => check v p r (s.play x.val).compact c)
        _ _ h ⟨m, hmem⟩ (List.mem_attach _ _)
      have := check_sound v p r hr _ _ _ hc
      rwa [State.compact_eq] at this
termination_by s.empties
decreasing_by
  · simp only [State.compact_eq]
    exact State.empties_play_lt (State.cells_isNone_of_legal ‹_›)
  · simp only [State.compact_eq]
    exact State.empties_play_lt (State.cells_isNone_of_legal hm)

end Ulttt
