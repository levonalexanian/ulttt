import { useEffect, useRef, useState } from "react";
import { Board } from "./Board";
import { engine, type Result, type Value, VARIANTS, type Variant } from "./engine";
import { boardOf, cellOf, parsePosition } from "./position";

type Mode = "two-players" | "play-x" | "play-o";

const MODES: { id: Mode; label: string }[] = [
  { id: "play-x", label: "You play ×" },
  { id: "play-o", label: "You play ○" },
  { id: "two-players", label: "Two players" },
];

const STRENGTHS = [
  { nodes: 10_000, label: "Easy" },
  { nodes: 200_000, label: "Medium" },
  { nodes: 2_000_000, label: "Hard" },
  { nodes: 20_000_000, label: "Strongest" },
];

const START = `${".".repeat(81)}/${".".repeat(9)}/x/-`;

function describeValue(v: Value | null): string {
  switch (v) {
    case 1:
      return "× wins with perfect play";
    case -1:
      return "○ wins with perfect play";
    case 0:
      return "Draw with perfect play";
    case 2:
      return "Too deep to solve at this strength";
    default:
      return "…";
  }
}

export function App() {
  const [variant, setVariant] = useState<Variant>("classic");
  const [mode, setMode] = useState<Mode>("play-x");
  const [strength, setStrength] = useState(1);
  const [analysis, setAnalysis] = useState(false);
  const [history, setHistory] = useState<string[]>([START]);
  const [moves, setMoves] = useState<number[]>([]);
  const [legal, setLegal] = useState<Set<number>>(new Set());
  const [result, setResult] = useState<Result>("");
  const [values, setValues] = useState<Map<number, Value> | null>(null);
  const [posValue, setPosValue] = useState<Value | null>(null);
  const [thinking, setThinking] = useState(false);
  const [loadText, setLoadText] = useState("");
  const [loadError, setLoadError] = useState("");
  // Bumped on every change, so answers about an older position are ignored.
  const generation = useRef(0);

  const current = history[history.length - 1];
  const pos = parsePosition(current);
  const nodeLimit = STRENGTHS[strength].nodes;
  const solverTurn =
    result === "" && ((mode === "play-x" && pos.toMove === "o") || (mode === "play-o" && pos.toMove === "x"));

  function newGame() {
    generation.current++;
    setHistory([START]);
    setMoves([]);
  }

  function undo() {
    generation.current++;
    // Against the solver, take back both its reply and your move.
    const steps = mode === "two-players" ? 1 : moves.length >= 2 ? 2 : 1;
    if (moves.length === 0) return;
    setHistory((h) => h.slice(0, Math.max(1, h.length - steps)));
    setMoves((m) => m.slice(0, Math.max(0, m.length - steps)));
  }

  /** Starts from a position given in the engine's notation, e.g. one from the CLI. */
  async function loadPosition(text: string) {
    const s = text.trim();
    try {
      await engine.legalMoves(variant, s);
    } catch {
      setLoadError("Not a valid position");
      return;
    }
    setLoadError("");
    generation.current++;
    setHistory([s]);
    setMoves([]);
  }

  async function playMove(move: number) {
    const gen = generation.current;
    const next = await engine.play(variant, current, move);
    if (gen !== generation.current) return;
    generation.current++;
    setHistory((h) => [...h, next]);
    setMoves((m) => [...m, move]);
  }

  // Rules: legal moves and the result of the current position.
  useEffect(() => {
    const gen = generation.current;
    setValues(null);
    setPosValue(null);
    Promise.all([engine.legalMoves(variant, current), engine.result(variant, current)]).then(([l, r]) => {
      if (gen !== generation.current) return;
      setLegal(new Set(l));
      setResult(r);
    });
  }, [variant, current]);

  // The solver's move.
  useEffect(() => {
    if (!solverTurn) return;
    const gen = generation.current;
    setThinking(true);
    engine.bestMove(variant, current, nodeLimit).then((m) => {
      setThinking(false);
      if (gen === generation.current && m >= 0) playMove(m);
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [solverTurn, variant, current, nodeLimit]);

  // Analysis: the value of the position and of each legal move.
  useEffect(() => {
    if (!analysis || result !== "" || solverTurn) return;
    const gen = generation.current;
    engine.solve(variant, current, nodeLimit).then((v) => {
      if (gen === generation.current) setPosValue(v);
    });
    engine.analyze(variant, current, nodeLimit).then((vals) => {
      if (gen === generation.current) setValues(vals);
    });
  }, [analysis, variant, current, nodeLimit, result, solverTurn]);

  const status =
    result === "draw"
      ? "Draw"
      : result
        ? `${result === "x" ? "×" : "○"} wins`
        : thinking
          ? "Solver is thinking…"
          : `${pos.toMove === "x" ? "×" : "○"} to move`;

  return (
    <div className="app">
      <header>
        <h1>ulttt</h1>
        <p className="tagline">Ultimate tic-tac-toe with an exact solver</p>
      </header>
      <main>
        <Board
          pos={pos}
          legal={legal}
          lastMove={moves.length ? moves[moves.length - 1] : null}
          values={analysis ? values : null}
          onMove={playMove}
          disabled={solverTurn || thinking}
        />
        <aside>
          <div className={`status${result ? " over" : ""}`} aria-live="polite">
            {status}
          </div>
          <div className="controls">
            <label>
              Rules
              <select
                value={variant}
                onChange={(e) => {
                  setVariant(e.target.value as Variant);
                  newGame();
                }}
              >
                {VARIANTS.map((v) => (
                  <option key={v} value={v}>
                    {v}
                  </option>
                ))}
              </select>
            </label>
            <label>
              Mode
              <select
                value={mode}
                onChange={(e) => {
                  setMode(e.target.value as Mode);
                  newGame();
                }}
              >
                {MODES.map((m) => (
                  <option key={m.id} value={m.id}>
                    {m.label}
                  </option>
                ))}
              </select>
            </label>
            <label>
              Strength
              <select value={strength} onChange={(e) => setStrength(Number(e.target.value))}>
                {STRENGTHS.map((s, i) => (
                  <option key={s.label} value={i}>
                    {s.label}
                  </option>
                ))}
              </select>
            </label>
            <label className="toggle">
              <input type="checkbox" checked={analysis} onChange={(e) => setAnalysis(e.target.checked)} />
              Show analysis
            </label>
            <div className="buttons">
              <button onClick={newGame}>New game</button>
              <button onClick={undo} disabled={moves.length === 0}>
                Undo
              </button>
            </div>
          </div>
          {analysis && result === "" ? (
            <div className="analysis">
              <p>{describeValue(posValue)}</p>
              <p className="legend">
                <span className="dot v-win" /> wins <span className="dot v-draw" /> draws{" "}
                <span className="dot v-loss" /> loses <span className="dot v-unknown" /> unknown
              </p>
            </div>
          ) : null}
          <ol className="moves">
            {moves.map((m, i) => (
              <li key={i}>
                {i % 2 === 0 ? "×" : "○"} board {boardOf(m) + 1}, cell {cellOf(m) + 1}
              </li>
            ))}
          </ol>
          <details>
            <summary>Position</summary>
            <p className="notation">
              <code>{current}</code>
              <button onClick={() => navigator.clipboard?.writeText(current)}>Copy</button>
            </p>
            <form
              className="load"
              onSubmit={(e) => {
                e.preventDefault();
                loadPosition(loadText);
              }}
            >
              <input
                value={loadText}
                onChange={(e) => setLoadText(e.target.value)}
                placeholder="Paste a position, e.g. from `ulttt random`"
                aria-label="Position to load"
              />
              <button type="submit">Load</button>
            </form>
            {loadError ? <p className="error">{loadError}</p> : null}
          </details>
          <details>
            <summary>Rules</summary>
            <p>
              Play on the small board matching the cell your opponent just played. Win three small boards in a row to
              win. <b>classic</b>: won or full boards are closed, and if you are sent to one you may play anywhere.{" "}
              <b>open</b>: won boards stay playable until full. <b>majority</b>: classic, but without three in a row
              the player with more small boards wins.
            </p>
          </details>
        </aside>
      </main>
    </div>
  );
}
