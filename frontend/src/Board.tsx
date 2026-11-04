import type { Value } from "./engine";
import { type Position, boardOf } from "./position";

interface Props {
  pos: Position;
  legal: Set<number>;
  lastMove: number | null;
  /** Values of legal moves from X's point of view, when analysis is on. */
  values: Map<number, Value> | null;
  onMove: (move: number) => void;
  disabled: boolean;
}

/** How good a move's value is for the player to move. */
function verdict(value: Value, toMove: "x" | "o"): "win" | "draw" | "loss" | "unknown" {
  if (value === 2) return "unknown";
  if (value === 0) return "draw";
  return (value === 1) === (toMove === "x") ? "win" : "loss";
}

const VERDICT_LABEL = { win: "wins", draw: "draws", loss: "loses", unknown: "unknown" };

export function Board({ pos, legal, lastMove, values, onMove, disabled }: Props) {
  const playable = new Set([...legal].map(boardOf));
  return (
    <div className="global" role="grid" aria-label="Ultimate tic-tac-toe board">
      {Array.from({ length: 9 }, (_, b) => {
        const owner = pos.owners[b];
        return (
          <div
            key={b}
            className={`local${playable.has(b) ? " playable" : ""}${owner ? ` won won-${owner}` : ""}`}
          >
            {Array.from({ length: 9 }, (_, c) => {
              const m = 9 * b + c;
              const mark = pos.cells[m];
              const isLegal = legal.has(m) && !disabled;
              const value = values?.get(m);
              const v = value === undefined ? null : verdict(value, pos.toMove);
              return (
                <button
                  key={c}
                  className={`cell${m === lastMove ? " last" : ""}${v ? ` v-${v}` : ""}`}
                  disabled={!isLegal}
                  onClick={() => onMove(m)}
                  aria-label={`board ${b + 1} cell ${c + 1}${mark ? `: ${mark}` : ""}${v ? `, ${VERDICT_LABEL[v]}` : ""}`}
                >
                  {mark ? <span className={`mark mark-${mark}`}>{mark === "x" ? "×" : "○"}</span> : null}
                </button>
              );
            })}
            {owner ? <div className={`owner owner-${owner}`}>{owner === "x" ? "×" : "○"}</div> : null}
          </div>
        );
      })}
    </div>
  );
}
