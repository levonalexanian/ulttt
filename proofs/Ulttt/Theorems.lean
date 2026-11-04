import Ulttt.Value

/-! Basic properties of the rules that hold in every variant, and how the
variants relate to each other. -/

namespace Ulttt

namespace State

variable {v : Variant} {s : State}

/-- A full local board is closed in every variant. -/
theorem closed_of_full {b : Fin 9} (h : s.full b = true) : s.closed v b = true := by
  cases v <;> simp [closed, h]

/-- An open local board has an empty cell. -/
theorem exists_empty_of_not_closed {b : Fin 9} (h : s.closed v b = false) :
    ∃ c, (s.localBoard b c).isNone = true := by
  have hfull : s.full b = false := by
    cases hf : s.full b
    · rfl
    · simp [closed_of_full hf] at h
  simp only [full, List.all_eq_false, List.mem_finRange, true_and] at hfull
  obtain ⟨c, hc⟩ := hfull
  exact ⟨c, by simpa using hc⟩

/-- The player to move has a legal move exactly when the game is not over. -/
theorem exists_legal_iff : (∃ m, s.legal v m = true) ↔ s.terminal v = false := by
  constructor
  · rintro ⟨m, hm⟩
    exact not_terminal_of_legal hm
  · intro ht
    have hopen : s.anyOpen v = true := by
      simp only [terminal, Bool.or_eq_false_iff, Bool.not_eq_false'] at ht
      exact ht.2
    simp only [anyOpen, List.any_eq_true, List.mem_finRange, true_and,
      Bool.not_eq_true'] at hopen
    obtain ⟨b₀, hb₀⟩ := hopen
    -- Play on the board we were sent to if it is open, otherwise on `b₀`.
    obtain ⟨b, hb, hallowed⟩ : ∃ b, s.closed v b = false ∧ s.allowedBoard v b = true := by
      cases hsent : s.sent with
      | none => exact ⟨b₀, hb₀, by simp [allowedBoard, hb₀, hsent]⟩
      | some t =>
        cases hct : s.closed v t with
        | true => exact ⟨b₀, hb₀, by simp [allowedBoard, hb₀, hsent, hct]⟩
        | false => exact ⟨t, hct, by simp [allowedBoard, hsent, hct]⟩
    obtain ⟨c, hc⟩ := exists_empty_of_not_closed hb
    refine ⟨Move.mk b c, ?_⟩
    simp only [legal, ht, Bool.not_false, Move.board_mk, hallowed, Bool.and_true,
      Bool.true_and]
    exact hc

/-- Some legal move exists exactly when the game is not over. -/
theorem legalMoves_ne_nil_iff : s.legalMoves v ≠ [] ↔ s.terminal v = false := by
  rw [← exists_legal_iff]
  simp [legalMoves, List.filter_eq_nil_iff]

/-- Ownership of a local board never changes once set. -/
theorem owner_play_of_owner {b : Fin 9} {p : Player} {m : Move}
    (h : s.owner b = some p) : (s.play m).owner b = some p := by
  simp [play, h]

/-- A move fills exactly the cell it is played on. -/
theorem cells_play (m i : Move) :
    (s.play m).cells i = if i = m then some s.toMove else s.cells i := rfl

/-- Players alternate. -/
@[simp] theorem toMove_play (m : Move) : (s.play m).toMove = s.toMove.other := rfl

/-- The opponent is sent to the board matching the cell just played. -/
@[simp] theorem sent_play (m : Move) : (s.play m).sent = some m.cell := rfl

/-! ### Relations between the variants -/

/-- `classic` and `majority` have exactly the same legal moves. -/
theorem legal_classic_eq_majority : s.legal .classic = s.legal .majority := rfl

/-- `classic` and `majority` agree on which positions are terminal. -/
theorem terminal_classic_eq_majority : s.terminal .classic = s.terminal .majority := rfl

/-- `classic` and `majority` only disagree on games that end without a global line. -/
theorem outcome_classic_eq_majority_of_globalLine
    (h : s.globalLine .X = true ∨ s.globalLine .O = true) :
    s.outcome .classic = s.outcome .majority := by
  rcases h with h | h <;> simp [outcome, h]

/-- Without a global line, `majority` is decided by who owns more local boards. -/
theorem outcome_majority_of_no_globalLine
    (hX : s.globalLine .X = false) (hO : s.globalLine .O = false) :
    s.outcome .majority =
      if s.wonCount .O < s.wonCount .X then .win .X
      else if s.wonCount .X < s.wonCount .O then .win .O
      else .draw := by
  simp [outcome, hX, hO]

/-- Every board closed in `open` is closed in `classic`. -/
theorem closed_classic_of_closed_open {b : Fin 9} (h : s.closed .«open» b = true) :
    s.closed .classic b = true := by
  simp only [closed] at h ⊢
  simp [h]

end State

end Ulttt
