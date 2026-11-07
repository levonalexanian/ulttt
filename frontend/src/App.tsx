import { useEffect, useRef, useState } from "react";
import { Board } from "./Board";
import { EvalBar, type Evaluation, fromScore } from "./EvalBar";
import { Thinking, randomWord } from "./Thinking";
import { analyst, engine, type Result, type Thought, UNKNOWN, type Value, VARIANTS, type Variant } from "./engine";
import { codeFromUrl, joinLink, useOnline } from "./online";
import { boardOf, cellOf, glyph, parsePosition } from "./position";

type Mode = "two-players" | "play-x" | "play-o" | "online";

const MODES: { id: Mode; label: string }[] = [
  { id: "play-x", label: `You play ${glyph("x")}` },
  { id: "play-o", label: `You play ${glyph("o")}` },
  { id: "two-players", label: "Two players" },
  { id: "online", label: "Online" },
];

/** Engine strengths: positions searched per move, or `null` to think for the
 * whole of the move's wait (searching about 1-3M positions a second). */
const STRENGTHS: { nodes: number | null; label: string }[] = [
  { nodes: 10_000, label: "Easy" },
  { nodes: 200_000, label: "Medium" },
  { nodes: 2_000_000, label: "Hard" },
  { nodes: null, label: "Strongest" },
];

/** Analysis budget, the same at every strength: positions for the solver's
 * value of the position, and in all for the values of its moves. */
const ANALYSIS_NODES = 1_000_000;
const ANALYSIS_MOVES_NODES = 2_000_000;
/** The engine's passes over a position for the evaluation bar, in
 * milliseconds: a quick first look, then deeper ones while it stays shown. */
const EVAL_PASSES_MS = [250, 1500, 6000];

const START = `${".".repeat(81)}/${".".repeat(9)}/x/-`;

const other = (p: "x" | "o") => (p === "x" ? "o" : "x");
const markOf = glyph;

/** The positions of a game: the start and after each move. */
async function replay(variant: Variant, moves: number[]): Promise<string[]> {
  const history = [START];
  for (const m of moves) history.push(await engine.play(variant, history[history.length - 1], m));
  return history;
}

export function App() {
  const [variant, setVariant] = useState<Variant>("classic");
  // A link with `?join=` opens straight into that online game.
  const joinCode = useRef(codeFromUrl());
  const [mode, setMode] = useState<Mode>(joinCode.current ? "online" : "play-x");
  const [strength, setStrength] = useState(1);
  const [analysis, setAnalysis] = useState(false);
  // Against the engine or on one device, settings are chosen first and the
  // game begins with "Start game"; they are locked while it is played.
  const [started, setStarted] = useState(false);
  const [history, setHistory] = useState<string[]>([START]);
  const [moves, setMoves] = useState<number[]>([]);
  const [legal, setLegal] = useState<Set<number>>(new Set());
  const [result, setResult] = useState<Result>("");
  const [values, setValues] = useState<Map<number, Value> | null>(null);
  const [evaluation, setEvaluation] = useState<Evaluation | null>(null);
  // While the engine thinks, the word it shows (e.g. "Pondering").
  const [thinking, setThinking] = useState<string | null>(null);
  const [loadText, setLoadText] = useState("");
  const [loadError, setLoadError] = useState("");
  // Bumped on every change, so answers about an older position are ignored.
  const generation = useRef(0);

  const online = useOnline();
  const [joinText, setJoinText] = useState("");
  // Online: the side this browser plays. The host starts as X.
  const [me, setMe] = useState<"x" | "o">("x");
  // Online: set while a received message is being applied.
  const busy = useRef(false);

  const current = history[history.length - 1];
  const pos = parsePosition(current);
  const nodeLimit = STRENGTHS[strength].nodes;
  const inGame = mode === "online" || started;
  const solverTurn =
    started &&
    result === "" &&
    ((mode === "play-x" && pos.toMove === "o") || (mode === "play-o" && pos.toMove === "x"));
  const status_ = online.status;
  const connected = status_.kind === "connected";
  const isHost = status_.kind === "waiting" || (status_.kind === "connected" && status_.host);
  const waitingForPeer = mode === "online" && (!connected || pos.toMove !== me);

  function newGame() {
    generation.current++;
    setHistory([START]);
    setMoves([]);
  }

  /** Online, as host: tells the guest the game, with its moves so far. */
  function sendStart(v: Variant, host: "x" | "o", ms: number[]) {
    online.send({ type: "start", variant: v, you: other(host), moves: ms });
  }

  /** Online: a new game with sides swapped. The guest asks the host for it. */
  function rematch(v: Variant = variant) {
    if (!isHost) {
      online.send({ type: "rematch", variant: v });
      return;
    }
    const swapped = other(me);
    setMe(swapped);
    setVariant(v);
    newGame();
    sendStart(v, swapped, []);
  }

  /** Online: other rules for a game nobody has moved in yet, same sides. */
  function restartWithRules(v: Variant) {
    if (!isHost) {
      online.send({ type: "rules", variant: v });
      return;
    }
    setVariant(v);
    newGame();
    sendStart(v, me, []);
  }

  function undo() {
    generation.current++;
    // Against the solver, take back both its reply and your move.
    const steps = mode === "two-players" ? 1 : moves.length >= 2 ? 2 : 1;
    if (moves.length === 0 || mode === "online") return;
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
    if (mode !== "online") setStarted(true);
    setHistory([s]);
    setMoves([]);
  }

  async function playMove(move: number, fromPeer = false) {
    const gen = generation.current;
    const next = await engine.play(variant, current, move);
    if (gen !== generation.current) return;
    generation.current++;
    if (mode === "online" && !fromPeer) online.send({ type: "move", ply: moves.length, move });
    setHistory((h) => [...h, next]);
    setMoves((m) => [...m, move]);
  }

  // Online: a link with `?join=` joins that game.
  useEffect(() => {
    const code = joinCode.current;
    if (code) {
      online.join(code);
      // A reload should not join again behind the player's back.
      window.history.replaceState(null, "", location.pathname);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Online, as host: a guest who arrives (or comes back) gets the game so far.
  useEffect(() => {
    if (online.guestArrived) sendStart(variant, me, moves);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [online.guestArrived]);

  // Online: apply the other player's messages one at a time, in order.
  useEffect(() => {
    const msg = online.inbox[0];
    if (!msg || busy.current) return;
    busy.current = true;
    const done = () => {
      busy.current = false;
      online.take();
    };
    if (msg.type === "start" && !isHost) {
      generation.current++;
      const gen = generation.current;
      replay(msg.variant, msg.moves)
        .then((h) => {
          if (gen !== generation.current) return;
          generation.current++;
          setVariant(msg.variant);
          setMe(msg.you);
          setHistory(h);
          setMoves(msg.moves);
        })
        .catch(() => {})
        .finally(done);
    } else if (msg.type === "move" && msg.ply === moves.length && pos.toMove !== me && legal.has(msg.move)) {
      playMove(msg.move, true).finally(done);
    } else {
      const known = (v: unknown): v is Variant => VARIANTS.includes(v as Variant);
      if (msg.type === "rematch" && isHost) rematch(known(msg.variant) ? msg.variant : variant);
      if (msg.type === "rules" && isHost && moves.length === 0 && known(msg.variant)) restartWithRules(msg.variant);
      done();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [online.inbox, moves, legal]);

  // Rules: legal moves and the result of the current position.
  useEffect(() => {
    const gen = generation.current;
    setValues(null);
    setEvaluation(null);
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
    let live = true;
    let timer = 0;
    setThinking(randomWord());
    // The engine answers in well under a second; it takes 3-6 seconds to look
    // like it is thinking, so the game does not feel rushed. Raising the
    // random number to the power 1.5 slightly favors short waits: 48% under
    // 4 s and 76% under 5 s, against 33% and 67% for an even spread.
    const waitMs = 3000 + 3000 * Math.random() ** 1.5;
    const wait = new Promise((resolve) => {
      timer = window.setTimeout(resolve, waitMs);
    });
    // The strongest setting thinks for the whole wait, less a margin for
    // finishing the last search.
    const search =
      nodeLimit === null
        ? engine.think(variant, current, waitMs - 300).then((t) => t.move)
        : engine.bestMove(variant, current, nodeLimit);
    Promise.all([search, wait]).then(([m]) => {
      if (!live) return;
      setThinking(null);
      if (gen === generation.current && m >= 0) playMove(m);
    });
    return () => {
      live = false;
      clearTimeout(timer);
      setThinking(null);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [solverTurn, variant, current, nodeLimit]);

  // Analysis: the evaluation of the position, and the value of each legal move.
  useEffect(() => {
    if (!analysis || !inGame || result !== "" || mode === "online" || legal.size === 0) return;
    const gen = generation.current;
    const fresh = () => gen === generation.current;
    const ignore = () => {};
    const toMove = pos.toMove;
    // A forced win found by the engine says how far away it is, so it is kept
    // over the solver's bare value; either is kept over a heuristic score.
    const solved = (value: Value) => {
      if (value === UNKNOWN || !fresh()) return;
      setEvaluation((e) => (e?.kind === "forced" ? e : { kind: "solved", value: value as -1 | 0 | 1 }));
    };
    const thought = (t: Thought) => {
      if (!fresh()) return;
      const e = fromScore(t.score, t.depth, toMove);
      if (e.kind === "forced") setEvaluation(e);
      else if (t.value !== UNKNOWN) solved(t.value);
      else setEvaluation((prev) => (prev && prev.kind !== "score" ? prev : e));
    };
    // The worker answers in order: a quick look for the bar, the solver, the
    // moves' values for the dots, then deeper looks for the bar.
    const [quick, ...deeper] = EVAL_PASSES_MS;
    analyst.think(variant, current, quick).then(thought, ignore);
    analyst.solve(variant, current, ANALYSIS_NODES).then(solved, ignore);
    const perMove = Math.max(20_000, Math.floor(ANALYSIS_MOVES_NODES / legal.size));
    analyst.analyze(variant, current, perMove).then((vals) => {
      if (gen === generation.current) setValues(vals);
    }, ignore);
    for (const ms of deeper) analyst.think(variant, current, ms).then(thought, ignore);
    // Work for a position that is no longer shown is abandoned at once.
    return () => {
      if (analyst.busy) analyst.restart();
    };
  }, [analysis, inGame, variant, current, result, mode, legal]);

  const status = !inGame
    ? "Pick your settings, then start"
    : result === "draw"
      ? "Draw"
      : result
        ? `${markOf(result)} wins`
        : thinking
          ? <Thinking word={thinking} />
          : mode === "online" && connected
            ? pos.toMove === me
              ? `Your move (${markOf(me)})`
              : "Opponent's move"
            : `${markOf(pos.toMove)} to move`;

  const analysisShown = analysis && mode !== "online";

  return (
    <div className="app">
      <header>
        <h1>ulttt</h1>
        <a className="repo" href="https://github.com/levonalexanian/ulttt" aria-label="Source code on GitHub">
          <svg viewBox="0 0 16 16" width="32" height="32" aria-hidden="true">
            <path
              fill="currentColor"
              d="M8 0c4.42 0 8 3.58 8 8a8.013 8.013 0 0 1-5.45 7.59c-.4.08-.55-.17-.55-.38 0-.27.01-1.13.01-2.2 0-.75-.25-1.23-.54-1.48 1.78-.2 3.65-.88 3.65-3.95 0-.88-.31-1.59-.82-2.15.08-.2.36-1.02-.08-2.12 0 0-.67-.22-2.2.82-.64-.18-1.32-.27-2-.27-.68 0-1.36.09-2 .27-1.53-1.03-2.2-.82-2.2-.82-.44 1.1-.16 1.92-.08 2.12-.51.56-.82 1.28-.82 2.15 0 3.06 1.86 3.75 3.64 3.95-.23.2-.44.55-.51 1.07-.46.21-1.61.55-2.33-.66-.15-.24-.6-.83-1.23-.82-.67.01-.27.38.01.53.34.19.73.9.82 1.13.16.45.68 1.31 2.69.94 0 .67.01 1.3.01 1.49 0 .21-.15.45-.55.38A7.995 7.995 0 0 1 0 8c0-4.42 3.58-8 8-8Z"
            />
          </svg>
        </a>
      </header>
      <main>
        <div className="board-area">
          {analysisShown ? <EvalBar evaluation={evaluation} result={result} variant={variant} /> : null}
          <Board
            pos={pos}
            legal={legal}
            lastMove={moves.length ? moves[moves.length - 1] : null}
            values={analysisShown ? values : null}
            onMove={(m) => playMove(m)}
            disabled={!inGame || solverTurn || thinking !== null || waitingForPeer}
          />
        </div>
        <aside>
          <div className={`status${result ? " over" : ""}`} aria-live="polite">
            {status}
          </div>
          <div className="controls">
            <label>
              Rules
              <select
                value={variant}
                // Online, either player may change the rules before the first
                // move or once the game is over (which starts the rematch).
                disabled={mode === "online" ? connected && moves.length > 0 && result === "" : started}
                onChange={(e) => {
                  const v = e.target.value as Variant;
                  if (mode === "online" && connected) {
                    if (result !== "") rematch(v);
                    else restartWithRules(v);
                    return;
                  }
                  setVariant(v);
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
                disabled={started}
                onChange={(e) => {
                  if (mode === "online") online.leave();
                  setMode(e.target.value as Mode);
                  setMe("x");
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
            {mode === "online" ? null : (
              <>
                <label>
                  Strength
                  <select
                    value={strength}
                    disabled={started || mode === "two-players"}
                    onChange={(e) => setStrength(Number(e.target.value))}
                  >
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
              </>
            )}
            <div className="buttons">
              {mode === "online" ? (
                <button className={result ? "primary" : ""} onClick={() => rematch()} disabled={!connected}>
                  Rematch
                </button>
              ) : !started ? (
                <button
                  className="primary"
                  onClick={() => {
                    newGame();
                    setStarted(true);
                  }}
                >
                  Start game
                </button>
              ) : (
                <>
                  <button
                    onClick={() => {
                      newGame();
                      setStarted(false);
                    }}
                  >
                    New game
                  </button>
                  <button onClick={undo} disabled={moves.length === 0}>
                    Undo
                  </button>
                </>
              )}
            </div>
          </div>
          {mode === "online" ? (
            <div className="online">
              {status_.kind === "waiting" ? (
                <>
                  <p>Send this link to your opponent:</p>
                  <p className="notation">
                    <code>{joinLink(status_.code)}</code>
                    <button onClick={() => navigator.clipboard?.writeText(joinLink(status_.code))}>Copy</button>
                  </p>
                  <p>
                    Or tell them the code <b>{status_.code}</b>. Waiting for them to join…
                  </p>
                </>
              ) : status_.kind === "connecting" ? (
                <p>Connecting to game {status_.code}…</p>
              ) : status_.kind === "connected" ? (
                <p>
                  Connected. You play {markOf(me)}. <button onClick={online.leave}>Leave</button>
                </p>
              ) : (
                <>
                  {status_.kind === "closed" ? <p className="error">{status_.reason}</p> : null}
                  <div className="buttons">
                    <button
                      onClick={() => {
                        setMe("x");
                        newGame();
                        online.host();
                      }}
                    >
                      Create game
                    </button>
                  </div>
                  <form
                    className="load"
                    onSubmit={(e) => {
                      e.preventDefault();
                      // Accept a pasted link as well as a bare code.
                      const code = joinText.trim().split("join=").pop() ?? "";
                      if (code) online.join(code);
                    }}
                  >
                    <input
                      value={joinText}
                      onChange={(e) => setJoinText(e.target.value)}
                      placeholder="Game code or link"
                      aria-label="Game code or link"
                    />
                    <button type="submit">Join</button>
                  </form>
                </>
              )}
            </div>
          ) : null}
          {analysisShown && result === "" ? (
            <div className="analysis">
              <p className="legend">
                <span className="dot v-win" /> wins <span className="dot v-draw" /> draws{" "}
                <span className="dot v-loss" /> loses <span className="dot v-unknown" /> unknown
              </p>
            </div>
          ) : null}
          <ol className="moves">
            {moves.map((m, i) => (
              <li key={i}>
                {glyph(i % 2 === 0 ? "x" : "o")} board {boardOf(m) + 1}, cell {cellOf(m) + 1}
              </li>
            ))}
          </ol>
          {mode === "online" ? null : (
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
          )}
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
