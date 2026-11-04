import Ulttt.Fast
import Ulttt.Value

/-! Checking solver certificates.

A certificate claims that player `p` can force a result worth at least `r` to
`p` (`r = 1`: a win, `r = 0`: at least a draw). It is `p`'s strategy, written
as the list of `p`'s moves in depth-first order: at a position where `p` is to
move, the next number is `p`'s move; where the opponent is to move, every legal
reply is explored, in increasing order. Finished games must be worth at least
`r` to `p`.

`check` consumes a certificate and returns what is left of it, or `none` if
the certificate is wrong. `check_sound` proves that a certificate accepted by
`check` really establishes the claimed bound on `value`, so the solver that
produced it need not be trusted. -/

namespace Ulttt

/-- The value of a position from player `p`'s point of view. -/
def valueFor (v : Variant) (p : Player) (s : State) : Int :=
  match p with
  | .X => value v s
  | .O => -value v s

/-- The score of an outcome from player `p`'s point of view. -/
def Outcome.scoreFor (o : Outcome) : Player → Int
  | .X => o.score
  | .O => -o.score

/-- Checks that `cert` is a strategy for `p` worth at least `r` from `s`.
Returns the unused rest of `cert` on success. Uses `compact` and
`legalMovesFast` for speed; both are proved equal to the plain definitions. -/
def check (v : Variant) (p : Player) (r : Int) (s : State) (cert : List Nat) :
    Option (List Nat) :=
  if s.terminal v then
    if r ≤ (s.outcome v).scoreFor p then some cert else none
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

theorem foldl_max_ge_init (l : List Int) (a : Int) : a ≤ l.foldl max a := by
  induction l generalizing a with
  | nil => simp
  | cons x xs ih => exact Int.le_trans (Int.le_max_left a x) (ih _)

theorem foldl_max_ge_mem (l : List Int) (a x : Int) (h : x ∈ l) : x ≤ l.foldl max a := by
  induction l generalizing a with
  | nil => simp at h
  | cons y ys ih =>
    simp only [List.foldl_cons]
    rcases List.mem_cons.mp h with rfl | h
    · exact Int.le_trans (Int.le_max_right a x) (foldl_max_ge_init ys _)
    · exact ih _ h

theorem foldl_min_le_mem (l : List Int) (a x : Int) (h : x ∈ l) : l.foldl min a ≤ x := by
  induction l generalizing a with
  | nil => simp at h
  | cons y ys ih =>
    simp only [List.foldl_cons]
    rcases List.mem_cons.mp h with rfl | h
    · have : ∀ (l : List Int) (b : Int), l.foldl min b ≤ b := by
        intro l
        induction l with
        | nil => intro b; simp
        | cons z zs ih' => intro b; exact Int.le_trans (ih' _) (Int.min_le_left b z)
      exact Int.le_trans (this ys _) (Int.min_le_right a x)
    · exact ih _ h

theorem le_foldl_min (l : List Int) (a r : Int) (ha : r ≤ a) (h : ∀ x ∈ l, r ≤ x) :
    r ≤ l.foldl min a := by
  induction l generalizing a with
  | nil => simpa using ha
  | cons y ys ih =>
    simp only [List.foldl_cons]
    apply ih
    · exact Int.le_min.mpr ⟨ha, h y List.mem_cons_self⟩
    · exact fun x hx => h x (List.mem_cons_of_mem _ hx)

theorem ge_foldl_max (l : List Int) (a r : Int) (ha : a ≤ r) (h : ∀ x ∈ l, x ≤ r) :
    l.foldl max a ≤ r := by
  induction l generalizing a with
  | nil => simpa using ha
  | cons y ys ih =>
    simp only [List.foldl_cons]
    apply ih
    · exact Int.max_le.mpr ⟨ha, h y List.mem_cons_self⟩
    · exact fun x hx => h x (List.mem_cons_of_mem _ hx)

/-- Unfolding `value` one step, without the membership proofs. -/
theorem value_unfold (v : Variant) (s : State) : value v s =
    if s.terminal v then (s.outcome v).score
    else match s.toMove with
      | .X => ((s.legalMoves v).map fun m => value v (s.play m)).foldl max (-1)
      | .O => ((s.legalMoves v).map fun m => value v (s.play m)).foldl min 1 := by
  have h := List.attach_map_val (l := s.legalMoves v) (f := fun m => value v (s.play m))
  rw [value, h]
  rfl

/-- A folded check succeeds only if every step succeeds. -/
theorem foldl_bind_some {α : Type} (l : List α) (f : α → List Nat → Option (List Nat))
    (c c' : List Nat) (h : l.foldl (fun acc a => acc.bind (f a)) (some c) = some c') :
    ∀ a ∈ l, ∃ c₁ c₂, f a c₁ = some c₂ := by
  induction l generalizing c with
  | nil => simp
  | cons x xs ih =>
    intro a ha
    simp only [List.foldl_cons, Option.bind_some] at h
    cases hx : f x c with
    | none =>
      rw [hx] at h
      have : ∀ (l : List α), l.foldl (fun acc a => acc.bind (f a)) none = none := by
        intro l; induction l with
        | nil => rfl
        | cons _ _ ih' => simpa using ih'
      rw [this] at h
      cases h
    | some c₁ =>
      rw [hx] at h
      rcases List.mem_cons.mp ha with rfl | ha
      · exact ⟨c, c₁, hx⟩
      · exact ih c₁ h a ha

theorem valueFor_play_le {v : Variant} {p : Player} {s : State} {m : Move}
    (hnt : s.terminal v = false) (hp : s.toMove = p) (hm : s.legal v m = true) :
    valueFor v p (s.play m) ≤ valueFor v p s := by
  have hmem : value v (s.play m) ∈ (s.legalMoves v).map fun m => value v (s.play m) :=
    List.mem_map.mpr ⟨m, State.mem_legalMoves.mpr hm, rfl⟩
  cases p with
  | X =>
    simp only [valueFor]
    rw [value_unfold v s]
    simp only [hnt, Bool.false_eq_true, ↓reduceIte, hp]
    exact foldl_max_ge_mem _ _ _ hmem
  | O =>
    simp only [valueFor]
    rw [value_unfold v s]
    simp only [hnt, Bool.false_eq_true, ↓reduceIte, hp]
    exact Int.neg_le_neg (foldl_min_le_mem _ _ _ hmem)

theorem valueFor_ge_of_all {v : Variant} {p : Player} {s : State} {r : Int}
    (hr : r ≤ 1) (hnt : s.terminal v = false) (hp : s.toMove ≠ p)
    (h : ∀ m, s.legal v m = true → r ≤ valueFor v p (s.play m)) :
    r ≤ valueFor v p s := by
  have hall : ∀ x ∈ (s.legalMoves v).map (fun m => value v (s.play m)),
      ∃ m, s.legal v m = true ∧ x = value v (s.play m) := by
    intro x hx
    obtain ⟨m, hm, rfl⟩ := List.mem_map.mp hx
    exact ⟨m, State.mem_legalMoves.mp hm, rfl⟩
  cases p with
  | X =>
    have hO : s.toMove = .O := by cases h' : s.toMove <;> simp_all
    simp only [valueFor] at h ⊢
    rw [value_unfold v s]
    simp only [hnt, Bool.false_eq_true, ↓reduceIte, hO]
    apply le_foldl_min _ _ _ hr
    intro x hx
    obtain ⟨m, hm, rfl⟩ := hall x hx
    exact h m hm
  | O =>
    have hX : s.toMove = .X := by cases h' : s.toMove <;> simp_all
    simp only [valueFor] at h ⊢
    rw [value_unfold v s]
    simp only [hnt, Bool.false_eq_true, ↓reduceIte, hX]
    have : ((s.legalMoves v).map fun m => value v (s.play m)).foldl max (-1) ≤ -r := by
      apply ge_foldl_max _ _ _ (by omega)
      intro x hx
      obtain ⟨m, hm, rfl⟩ := hall x hx
      have := h m hm
      omega
    omega

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
