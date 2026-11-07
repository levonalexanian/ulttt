import type { Result, Variant } from "./engine";
import { glyph } from "./position";

/** What the analysis knows about a position, from X's point of view. */
export type Evaluation =
  /** The engine's heuristic score for X, after searching `depth` plies. */
  | { kind: "score"; score: number; depth: number }
  /** A forced win for `winner`, `plies` moves (by both sides) away. */
  | { kind: "forced"; winner: "x" | "o"; plies: number }
  /** The exact value, proved by the solver: 1 X wins, 0 draw, -1 O wins. */
  | { kind: "solved"; value: -1 | 0 | 1 };

/** Engine scores at least this large are forced results (`WIN - plies`). */
const WIN = 1_000_000;
const FORCED = WIN - 1000;

/** The logistic scale that turns an engine score into X's expected result:
 * `σ(k · score)`, fitted per variant on self-play games (`ulttt texel`). */
const K: Record<Variant, number> = {
  classic: 2.6e-4,
  open: 2.2e-4,
  majority: 3.1e-4,
};

/** Chess engines show "pawns", where +1 is about a 59% expected score
 * (lichess's 2/(1+e^(-0.00368·cp)) - 1). Scores here use the same scale, so
 * the numbers feel familiar: +1 is a small edge, +5 close to winning. */
const PAWN = 0.368;

/** An evaluation from an engine search's score for the player to move. */
export function fromScore(score: number, depth: number, toMove: "x" | "o"): Evaluation {
  const forX = toMove === "x" ? score : -score;
  if (Math.abs(forX) >= FORCED) {
    return { kind: "forced", winner: forX > 0 ? "x" : "o", plies: WIN - Math.abs(forX) };
  }
  return { kind: "score", score: forX, depth };
}

/** X's share of the bar (0-1) and the label shown on the side ahead. */
function describe(e: Evaluation | null, result: Result, variant: Variant) {
  if (result === "draw") return { share: 0.5, label: "½", title: "Draw" };
  if (result) return { share: result === "x" ? 1 : 0, label: "Win", title: `${glyph(result)} won` };
  if (!e) return { share: 0.5, label: "", title: "Analysing…" };
  switch (e.kind) {
    case "solved":
      if (e.value === 0) return { share: 0.5, label: "=", title: "Draw with perfect play" };
      return {
        share: e.value === 1 ? 1 : 0,
        label: "Win",
        title: `${glyph(e.value === 1 ? "x" : "o")} wins with perfect play`,
      };
    case "forced": {
      const moves = Math.ceil(e.plies / 2);
      return {
        share: e.winner === "x" ? 1 : 0,
        label: `M${moves}`,
        title: `${glyph(e.winner)} wins in ${moves} move${moves === 1 ? "" : "s"}`,
      };
    }
    case "score": {
      const x = K[variant] * e.score;
      const share = 1 / (1 + Math.exp(-x));
      const pawns = Math.abs(x / PAWN);
      const ahead = e.score >= 0 ? "x" : "o";
      return {
        share,
        label: pawns >= 10 ? pawns.toFixed(0) : pawns.toFixed(1),
        title: `${glyph(ahead)} ahead: ${glyph("x")} scores ${Math.round(share * 100)}% (depth ${e.depth})`,
      };
    }
  }
}

interface Props {
  evaluation: Evaluation | null;
  result: Result;
  variant: Variant;
}

/** A vertical bar beside the board: X's part grows from the bottom, O's from
 * the top, in proportion to their expected result. */
export function EvalBar({ evaluation, result, variant }: Props) {
  const { share, label, title } = describe(evaluation, result, variant);
  // The label sits at the end of the side that is ahead (X at an even score).
  const xAhead = share >= 0.5;
  return (
    <div className="evalbar" role="img" aria-label={title} title={title}>
      <div className="evalbar-x" style={{ height: `${share * 100}%` }} />
      {label ? <span className={`evalbar-label ${xAhead ? "on-x" : "on-o"}`}>{label}</span> : null}
    </div>
  );
}
