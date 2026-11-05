// Reading the position notation `<81 cells>/<9 owners>/<to move>/<sent>`
// produced by the engine. Moves are numbered 9 * board + cell.

export type Mark = "x" | "o" | null;

export interface Position {
  cells: Mark[];
  owners: Mark[];
  toMove: "x" | "o";
  sent: number | null;
}

const mark = (c: string): Mark => (c === "x" || c === "o" ? c : null);

export function parsePosition(s: string): Position {
  const [cells, owners, toMove, sent] = s.split("/");
  return {
    cells: [...cells].map(mark),
    owners: [...owners].map(mark),
    toMove: toMove === "o" ? "o" : "x",
    sent: sent === "-" ? null : Number(sent),
  };
}

export const boardOf = (move: number) => Math.floor(move / 9);
export const cellOf = (move: number) => move % 9;

/** How a player's mark is written. */
export const glyph = (p: "x" | "o") => (p === "x" ? "X" : "O");
