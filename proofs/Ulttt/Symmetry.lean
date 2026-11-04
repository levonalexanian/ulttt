import Ulttt.Basic

/-! The 8 symmetries of the 3×3 grid (rotations and reflections). Ultimate
Tic-Tac-Toe is invariant under applying the same symmetry to the global board
and to every local board at once. -/

namespace Ulttt

/-- The symmetries of the 3×3 grid. -/
inductive Sym where
  | id
  | rot90
  | rot180
  | rot270
  | flipH
  | flipV
  | transpose
  | antitranspose
  deriving DecidableEq, Repr

namespace Sym

/-- All 8 symmetries. -/
def all : List Sym := [id, rot90, rot180, rot270, flipH, flipV, transpose, antitranspose]

theorem mem_all (g : Sym) : g ∈ all := by cases g <;> decide

/-- The image of each index `0..8` under the symmetry (rotations are clockwise). -/
def table : Sym → List (Fin 9)
  | id => [0, 1, 2, 3, 4, 5, 6, 7, 8]
  | rot90 => [2, 5, 8, 1, 4, 7, 0, 3, 6]
  | rot180 => [8, 7, 6, 5, 4, 3, 2, 1, 0]
  | rot270 => [6, 3, 0, 7, 4, 1, 8, 5, 2]
  | flipH => [2, 1, 0, 5, 4, 3, 8, 7, 6]
  | flipV => [6, 7, 8, 3, 4, 5, 0, 1, 2]
  | transpose => [0, 3, 6, 1, 4, 7, 2, 5, 8]
  | antitranspose => [8, 5, 2, 7, 4, 1, 6, 3, 0]

/-- Where the symmetry sends grid index `i`. -/
def apply (g : Sym) (i : Fin 9) : Fin 9 := g.table.getD i 0

/-- The inverse symmetry. -/
def inv : Sym → Sym
  | rot90 => rot270
  | rot270 => rot90
  | g => g

@[simp] theorem inv_apply_apply (g : Sym) (i : Fin 9) : g.inv.apply (g.apply i) = i := by
  revert i; cases g <;> decide

@[simp] theorem apply_inv_apply (g : Sym) (i : Fin 9) : g.apply (g.inv.apply i) = i := by
  revert i; cases g <;> decide

@[simp] theorem inv_inv (g : Sym) : g.inv.inv = g := by cases g <;> rfl

theorem apply_injective (g : Sym) {i j : Fin 9} (h : g.apply i = g.apply j) : i = j := by
  have := congrArg g.inv.apply h
  simpa using this

@[simp] theorem apply_eq_apply_iff (g : Sym) {i j : Fin 9} : g.apply i = g.apply j ↔ i = j :=
  ⟨g.apply_injective, fun h => h ▸ rfl⟩

theorem apply_eq_iff_eq_inv (g : Sym) {i j : Fin 9} : g.apply i = j ↔ i = g.inv.apply j := by
  constructor
  · rintro rfl; simp
  · rintro rfl; simp

/-- Every line is sent onto a line: each index of `l'` is the image of an index of `l`. -/
def SendsLine (g : Sym) (l l' : Fin 9 × Fin 9 × Fin 9) : Prop :=
  let img := [g.apply l.1, g.apply l.2.1, g.apply l.2.2]
  l'.1 ∈ img ∧ l'.2.1 ∈ img ∧ l'.2.2 ∈ img

instance (g : Sym) (l l' : Fin 9 × Fin 9 × Fin 9) : Decidable (g.SendsLine l l') := by
  unfold SendsLine; infer_instance

theorem sendsLine_exists (g : Sym) : ∀ l ∈ lines, ∃ l' ∈ lines, g.SendsLine l l' := by
  cases g <;> decide

end Sym

/-- If `f ∘ g` has a line of `a`s then so does `f`. -/
theorem hasLine_of_hasLine_comp {α : Type} [DecidableEq α] (g : Sym) (f : Fin 9 → α) (a : α)
    (h : hasLine (fun i => f (g.apply i)) a = true) : hasLine f a = true := by
  simp only [hasLine, List.any_eq_true, Bool.and_eq_true, decide_eq_true_eq] at h ⊢
  obtain ⟨⟨i, j, k⟩, hl, ⟨hi, hj⟩, hk⟩ := h
  obtain ⟨⟨i', j', k'⟩, hl', h1, h2, h3⟩ := g.sendsLine_exists _ hl
  refine ⟨(i', j', k'), hl', ?_⟩
  have key : ∀ x ∈ [g.apply i, g.apply j, g.apply k], f x = a := by
    simp only [List.mem_cons, List.not_mem_nil, or_false]
    rintro x (rfl | rfl | rfl) <;> assumption
  exact ⟨⟨key _ h1, key _ h2⟩, key _ h3⟩

/-- Lines are preserved by every symmetry. -/
@[simp] theorem hasLine_comp {α : Type} [DecidableEq α] (g : Sym) (f : Fin 9 → α) (a : α) :
    hasLine (fun i => f (g.apply i)) a = hasLine f a := by
  apply Bool.eq_iff_iff.mpr
  refine ⟨hasLine_of_hasLine_comp g f a, fun h => ?_⟩
  apply hasLine_of_hasLine_comp g.inv
  simpa using h

/-- Reindexing all 9 grid indices by a symmetry permutes them. -/
theorem Sym.map_finRange_perm (g : Sym) :
    List.Perm ((List.finRange 9).map g.apply) (List.finRange 9) := by
  rw [List.perm_ext_iff_of_nodup]
  · intro i
    simp only [List.mem_map, List.mem_finRange, true_and, iff_true]
    exact ⟨g.inv.apply i, by simp⟩
  · exact (List.nodup_finRange 9).map g.apply fun _ _ h h' => h (g.apply_injective h')
  · exact List.nodup_finRange 9

theorem Sym.all_comp (g : Sym) (p : Fin 9 → Bool) :
    (List.finRange 9).all (fun i => p (g.apply i)) = (List.finRange 9).all p := by
  change (List.finRange 9).all (p ∘ g.apply) = _
  rw [← List.all_map]
  exact (g.map_finRange_perm).all_eq

theorem Sym.any_comp (g : Sym) (p : Fin 9 → Bool) :
    (List.finRange 9).any (fun i => p (g.apply i)) = (List.finRange 9).any p := by
  change (List.finRange 9).any (p ∘ g.apply) = _
  rw [← List.any_map]
  exact (g.map_finRange_perm).any_eq

theorem Sym.countP_comp (g : Sym) (p : Fin 9 → Bool) :
    (List.finRange 9).countP (fun i => p (g.apply i)) = (List.finRange 9).countP p := by
  change (List.finRange 9).countP (p ∘ g.apply) = _
  rw [← List.countP_map]
  exact (g.map_finRange_perm).countP_eq p

end Ulttt
