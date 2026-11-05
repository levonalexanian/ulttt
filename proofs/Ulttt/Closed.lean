import Std.Data.HashSet.Lemmas
import Ulttt.Fast
import Ulttt.ValueFor

/-! Proving a strategy with a closed set of positions.

A strategy tree, as in `Ulttt.Certificate`, repeats every position once for
each order of moves reaching it, which for a strategy that lasts the whole
game is far too many. Instead, a set of positions is **closed** for a strategy
`σ` of player `p` when every position in it is:

- finished, with a result worth at least `r` to `p`;
- `p`'s turn, where `σ` gives a legal move leading back into the set;
- the opponent's turn, where every legal move leads back into the set.

`closed_sound` proves that from every position of a closed set, `p` can force
a result worth at least `r`. `closedSetOk` checks that a list of positions is
closed, looking positions up by `State.key` in a hash set. -/

namespace Ulttt

/-- Whether position `s` satisfies its part of being closed: `mem` says which
positions are in the set. -/
def closedStep (v : Variant) (p : Player) (r : Int) (σ : State → Option Move)
    (mem : State → Bool) (s : State) : Bool :=
  if s.terminal v then decide (r ≤ (s.outcome v).scoreFor p)
  else if s.toMove = p then
    match σ s with
    | some m => s.legal v m && mem (s.play m)
    | none => false
  else (s.legalMovesFast v).all fun m => mem (s.play m)

/-- **Soundness of closed sets**: if every position in the set `mem` satisfies
`closedStep`, then from each of them `p` can force at least `r`. -/
theorem closed_sound (v : Variant) (p : Player) (r : Int) (hr : r ≤ 1)
    (σ : State → Option Move) (mem : State → Bool)
    (hclosed : ∀ t, mem t = true → closedStep v p r σ mem t = true)
    (s : State) (hs : mem s = true) : r ≤ valueFor v p s := by
  have h := hclosed s hs
  rw [closedStep] at h
  by_cases ht : s.terminal v = true
  · simp only [ht, ↓reduceIte, decide_eq_true_eq] at h
    cases p <;> simp only [valueFor, Outcome.scoreFor] at h ⊢ <;>
      rw [value_unfold v s] <;> simp only [ht, ↓reduceIte] <;> omega
  · have hnt : s.terminal v = false := by simpa using ht
    simp only [hnt, Bool.false_eq_true, ↓reduceIte] at h
    by_cases hp : s.toMove = p
    · simp only [hp, ↓reduceIte] at h
      split at h
      · rename_i m _
        have hl : s.legal v m = true := (Bool.and_eq_true _ _ ▸ h).1
        have hm : mem (s.play m) = true := (Bool.and_eq_true _ _ ▸ h).2
        have := closed_sound v p r hr σ mem hclosed (s.play m) hm
        exact Int.le_trans this (valueFor_play_le hnt hp hl)
      · cases h
    · simp only [hp, ↓reduceIte] at h
      apply valueFor_ge_of_all hr hnt hp
      intro m hm
      have hfast : m ∈ s.legalMovesFast v := by
        rw [State.legalMovesFast_eq]
        exact State.mem_legalMoves.mpr hm
      exact closed_sound v p r hr σ mem hclosed (s.play m) (List.all_eq_true.mp h m hfast)
termination_by s.empties
decreasing_by
  · exact State.empties_play_lt (State.cells_isNone_of_legal hl)
  · exact State.empties_play_lt (State.cells_isNone_of_legal hm)

/-! ### Keys: positions as lists of numbers, for hashing -/

/-- A cell or board owner as a number. -/
def cellCode : Option Player → Nat
  | none => 0
  | some .X => 1
  | some .O => 2

theorem cellCode_inj {a b : Option Player} (h : cellCode a = cellCode b) : a = b := by
  cases a with
  | none => cases b with
    | none => rfl
    | some q => cases q <;> simp [cellCode] at h
  | some p => cases b with
    | none => cases p <;> simp [cellCode] at h
    | some q => cases p <;> cases q <;> simp_all [cellCode]

/-- A position as a list of numbers: its cells, its board owners, the player
to move and the board they were sent to (9 for none). -/
def State.key (s : State) : List Nat :=
  List.ofFn (fun m => cellCode (s.cells m)) ++ List.ofFn (fun b => cellCode (s.owner b)) ++
    [cellCode (some s.toMove), match s.sent with | none => 9 | some b => b.val]

theorem ofFn_congr_apply {n : Nat} {f g : Fin n → Nat} (h : List.ofFn f = List.ofFn g)
    (i : Fin n) : f i = g i := by
  have := congrArg (fun l => l[i.val]?) h
  simpa using this

/-- Different positions have different keys. -/
theorem State.key_inj {s t : State} (h : s.key = t.key) : s = t := by
  simp only [State.key, List.append_assoc] at h
  obtain ⟨hc, h⟩ := List.append_inj h (by simp)
  obtain ⟨ho, h⟩ := List.append_inj h (by simp)
  simp only [List.cons.injEq, and_true] at h
  obtain ⟨hp, hsent⟩ := h
  apply State.ext
  · funext m; exact cellCode_inj (ofFn_congr_apply hc m)
  · funext b; exact cellCode_inj (ofFn_congr_apply ho b)
  · have := cellCode_inj hp
    simpa using this
  · cases hs : s.sent <;> cases ht : t.sent <;> simp only [hs, ht] at hsent
    · rfl
    · rename_i b; have := b.isLt; omega
    · rename_i b; have := b.isLt; omega
    · rename_i a b; exact congrArg some (Fin.ext hsent)

/-! ### Checking a list of positions -/

/-- Whether the positions of `L` form a closed set for `σ`. -/
def closedSetOk (v : Variant) (p : Player) (r : Int) (σ : State → Option Move)
    (L : List State) : Bool :=
  let keys : Std.HashSet (List Nat) := Std.HashSet.ofList (L.map State.key)
  L.all (closedStep v p r σ fun t => keys.contains t.key)

/-- If `closedSetOk` accepts `L`, then from every position of `L`, `p` can
force at least `r`. -/
theorem closedSetOk_sound (v : Variant) (p : Player) (r : Int) (hr : r ≤ 1)
    (σ : State → Option Move) (L : List State) (h : closedSetOk v p r σ L = true)
    (s : State) (hs : s ∈ L) : r ≤ valueFor v p s := by
  let keys : Std.HashSet (List Nat) := Std.HashSet.ofList (L.map State.key)
  have hmem : ∀ t, keys.contains t.key = true → t ∈ L := by
    intro t ht
    simp only [keys, Std.HashSet.contains_ofList, List.contains_iff_mem, List.mem_map] at ht
    obtain ⟨u, hu, hk⟩ := ht
    exact State.key_inj hk ▸ hu
  apply closed_sound v p r hr σ (fun t => keys.contains t.key)
  · intro t ht
    exact List.all_eq_true.mp h t (hmem t ht)
  · simp only [keys, Std.HashSet.contains_ofList, List.contains_iff_mem, List.mem_map]
    exact ⟨s, hs, rfl⟩

end Ulttt
