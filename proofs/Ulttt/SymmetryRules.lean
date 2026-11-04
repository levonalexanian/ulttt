import Ulttt.Symmetry
import Ulttt.Value

/-! Applying a grid symmetry to the global board and every local board at once
preserves the rules: legality, the effect of moves, results and the game value.
This justifies symmetry reduction in the solver. -/

namespace Ulttt

namespace Sym

/-- A symmetry acting on moves: the same symmetry on the board and the cell. -/
def move (g : Sym) (m : Move) : Move := Move.mk (g.apply m.board) (g.apply m.cell)

@[simp] theorem move_board (g : Sym) (m : Move) : (g.move m).board = g.apply m.board := by
  simp [move]

@[simp] theorem move_cell (g : Sym) (m : Move) : (g.move m).cell = g.apply m.cell := by
  simp [move]

@[simp] theorem move_mk (g : Sym) (b c : Fin 9) :
    g.move (Move.mk b c) = Move.mk (g.apply b) (g.apply c) := by
  simp [move]

theorem Move.ext_board_cell {m n : Move} (hb : m.board = n.board) (hc : m.cell = n.cell) :
    m = n := by
  rw [← Move.mk_board_cell m, ← Move.mk_board_cell n, hb, hc]

@[simp] theorem inv_move_move (g : Sym) (m : Move) : g.inv.move (g.move m) = m :=
  Move.ext_board_cell (by simp) (by simp)

@[simp] theorem move_inv_move (g : Sym) (m : Move) : g.move (g.inv.move m) = m :=
  Move.ext_board_cell (by simp) (by simp)

theorem move_eq_iff (g : Sym) {m n : Move} : g.move m = n ↔ m = g.inv.move n := by
  constructor
  · rintro rfl; simp
  · rintro rfl; simp

end Sym

namespace State

/-- The position with symmetry `g` applied to the global board and every local board. -/
def map (g : Sym) (s : State) : State where
  cells m := s.cells (g.inv.move m)
  owner b := s.owner (g.inv.apply b)
  toMove := s.toMove
  sent := s.sent.map g.apply

variable (v : Variant) (g : Sym) (s : State)

@[simp] theorem map_cells (m : Move) : (s.map g).cells m = s.cells (g.inv.move m) := rfl
@[simp] theorem map_owner (b : Fin 9) : (s.map g).owner b = s.owner (g.inv.apply b) := rfl
@[simp] theorem map_toMove : (s.map g).toMove = s.toMove := rfl
@[simp] theorem map_sent : (s.map g).sent = s.sent.map g.apply := rfl

theorem map_localBoard (b c : Fin 9) :
    (s.map g).localBoard b c = s.localBoard (g.inv.apply b) (g.inv.apply c) := by
  simp [localBoard]

theorem map_full (b : Fin 9) : (s.map g).full b = s.full (g.inv.apply b) := by
  simp only [full, map_localBoard]
  exact g.inv.all_comp (fun c => (s.localBoard (g.inv.apply b) c).isSome)

theorem map_closed (b : Fin 9) : (s.map g).closed v b = s.closed v (g.inv.apply b) := by
  cases v <;> simp [closed, map_full]

theorem map_globalLine (p : Player) : (s.map g).globalLine p = s.globalLine p := by
  simp only [globalLine]
  exact hasLine_comp g.inv s.owner (some p)

theorem map_wonCount (p : Player) : (s.map g).wonCount p = s.wonCount p := by
  simp only [wonCount, map_owner]
  exact g.inv.countP_comp (fun b => decide (s.owner b = some p))

theorem map_anyOpen : (s.map g).anyOpen v = s.anyOpen v := by
  simp only [anyOpen, map_closed]
  exact g.inv.any_comp (fun b => !s.closed v b)

theorem map_terminal : (s.map g).terminal v = s.terminal v := by
  simp [terminal, map_globalLine, map_anyOpen]

theorem map_outcome : (s.map g).outcome v = s.outcome v := by
  simp [outcome, map_globalLine, map_wonCount]

theorem map_allowedBoard (b : Fin 9) :
    (s.map g).allowedBoard v (g.apply b) = s.allowedBoard v b := by
  simp only [allowedBoard, map_closed, Sym.inv_apply_apply, map_sent]
  cases s.sent with
  | none => rfl
  | some t => simp

/-- A move is legal exactly when its image is legal in the image position. -/
theorem map_legal (m : Move) : (s.map g).legal v (g.move m) = s.legal v m := by
  simp [legal, map_terminal, map_allowedBoard]

/-- Playing commutes with applying a symmetry. -/
theorem map_play (m : Move) : (s.map g).play (g.move m) = (s.play m).map g := by
  -- The cells after the move, before applying the symmetry.
  let F : Move → Option Player := fun j => if j = m then some s.toMove else s.cells j
  have hcells : ∀ i, (if i = g.move m then some s.toMove else s.cells (g.inv.move i)) =
      F (g.inv.move i) := by
    intro i
    by_cases h : i = g.move m
    · subst h; simp [F]
    · have h' : g.inv.move i ≠ m := fun h' => h (by rw [← h']; simp)
      simp [F, h, h']
  have hboard : ∀ b, (b = g.apply m.board) ↔ (g.inv.apply b = m.board) := by
    intro b
    constructor
    · rintro rfl; simp
    · intro h; rw [← h]; simp
  ext1
  · funext i
    exact hcells i
  · funext b
    simp only [play, map]
    simp only [hcells]
    simp only [Sym.move_board, Sym.move_mk]
    have hline := hasLine_comp g.inv (fun c => F (Move.mk (g.inv.apply b) c)) (some s.toMove)
    rw [hline, decide_eq_decide.mpr (hboard b)]
  · rfl
  · simp [play, map]

/-- Legal moves of the image position are the images of the legal moves, up to order. -/
theorem map_legalMoves_perm :
    List.Perm ((s.map g).legalMoves v) ((s.legalMoves v).map g.move) := by
  have hperm : List.Perm ((List.finRange 81).map g.move) (List.finRange 81) := by
    rw [List.perm_ext_iff_of_nodup]
    · intro i
      simp only [List.mem_map, List.mem_finRange, true_and, iff_true]
      exact ⟨g.inv.move i, by simp⟩
    · exact (List.nodup_finRange 81).map g.move fun _ _ h h' =>
        h (by simpa using congrArg g.inv.move h')
    · exact List.nodup_finRange 81
  have hlegal : (s.map g).legal v = fun i => s.legal v (g.inv.move i) := by
    funext i
    rw [← map_legal v g s (g.inv.move i), Sym.move_inv_move]
  unfold legalMoves
  rw [hlegal]
  have hmap : ((List.finRange 81).filter (s.legal v)).map g.move =
      ((List.finRange 81).map g.move).filter (fun i => s.legal v (g.inv.move i)) := by
    rw [List.filter_map]
    congr 2
    funext i
    simp
  rw [hmap]
  exact (hperm.filter _).symm

end State

end Ulttt
