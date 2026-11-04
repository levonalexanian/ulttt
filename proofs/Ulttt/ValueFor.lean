import Ulttt.Value

/-! Values from one player's point of view, and lemmas relating a position's value
to its children's. Used by the certificate checker and the `mayWin` proofs. -/

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

theorem valueFor_le_of_all {v : Variant} {p : Player} {s : State} {r : Int}
    (hr : -1 ≤ r) (hnt : s.terminal v = false) (hp : s.toMove = p)
    (h : ∀ m, s.legal v m = true → valueFor v p (s.play m) ≤ r) : valueFor v p s ≤ r := by
  have hall : ∀ x ∈ (s.legalMoves v).map (fun m => value v (s.play m)),
      ∃ m, s.legal v m = true ∧ x = value v (s.play m) := by
    intro x hx
    obtain ⟨m, hm, rfl⟩ := List.mem_map.mp hx
    exact ⟨m, State.mem_legalMoves.mp hm, rfl⟩
  cases p with
  | X =>
    simp only [valueFor] at h ⊢
    rw [value_unfold v s]
    simp only [hnt, Bool.false_eq_true, ↓reduceIte, hp]
    apply ge_foldl_max _ _ _ hr
    intro x hx
    obtain ⟨m, hm, rfl⟩ := hall x hx
    exact h m hm
  | O =>
    simp only [valueFor] at h ⊢
    rw [value_unfold v s]
    simp only [hnt, Bool.false_eq_true, ↓reduceIte, hp]
    have : -r ≤ ((s.legalMoves v).map fun m => value v (s.play m)).foldl min 1 := by
      apply le_foldl_min _ _ _ (by omega)
      intro x hx
      obtain ⟨m, hm, rfl⟩ := hall x hx
      have := h m hm
      omega
    omega

theorem valueFor_le_of_exists {v : Variant} {p : Player} {s : State} {r : Int}
    (hnt : s.terminal v = false) (hp : s.toMove ≠ p) {m : Move} (hm : s.legal v m = true)
    (h : valueFor v p (s.play m) ≤ r) : valueFor v p s ≤ r := by
  have hmem : value v (s.play m) ∈ (s.legalMoves v).map fun m => value v (s.play m) :=
    List.mem_map.mpr ⟨m, State.mem_legalMoves.mpr hm, rfl⟩
  cases p with
  | X =>
    have hO : s.toMove = .O := by cases h' : s.toMove <;> simp_all
    simp only [valueFor] at h ⊢
    rw [value_unfold v s]
    simp only [hnt, Bool.false_eq_true, ↓reduceIte, hO]
    exact Int.le_trans (foldl_min_le_mem _ _ _ hmem) h
  | O =>
    have hX : s.toMove = .X := by cases h' : s.toMove <;> simp_all
    simp only [valueFor] at h ⊢
    rw [value_unfold v s]
    simp only [hnt, Bool.false_eq_true, ↓reduceIte, hX]
    have := foldl_max_ge_mem _ (-1) _ hmem
    omega

end Ulttt
