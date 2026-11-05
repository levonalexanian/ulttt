import { useEffect, useRef, useState } from "react";
import { Board } from "./Board";
import { Thinking, randomWord } from "./Thinking";
import { engine, type Result, type Value, VARIANTS, type Variant } from "./engine";
import { codeFromUrl, joinLink, useOnline } from "./online";
import { boardOf, cellOf, glyph, parsePosition } from "./position";

type Mode = "two-players" | "play-x" | "play-o" | "online";

const MODES: { id: Mode; label: string }[] = [
  { id: "play-x", label: `You play ${glyph("x")}` },
  { id: "play-o", label: `You play ${glyph("o")}` },
  { id: "two-players", label: "Two players" },
  { id: "online", label: "Online" },
];

const STRENGTHS = [
  { nodes: 10_000, label: "Easy" },
  { nodes: 200_000, label: "Medium" },
  { nodes: 2_000_000, label: "Hard" },
  { nodes: 20_000_000, label: "Strongest" },
];

const START = `${".".repeat(81)}/${".".repeat(9)}/x/-`;

const other = (p: "x" | "o") => (p === "x" ? "o" : "x");
const markOf = glyph;

/** The positions of a game: the start and after each move. */
async function replay(variant: Variant, moves: number[]): Promise<string[]> {
  const history = [START];
  for (const m of moves) history.push(await engine.play(variant, history[history.length - 1], m));
  return history;
}

function describeValue(v: Value | null): string {
  switch (v) {
    case 1:
      return `${glyph("x")} wins with perfect play`;
    case -1:
      return `${glyph("o")} wins with perfect play`;
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
  // A link with `?join=` opens straight into that online game.
  const joinCode = useRef(codeFromUrl());
  const [mode, setMode] = useState<Mode>(joinCode.current ? "online" : "play-x");
  const [strength, setStrength] = useState(1);
  const [analysis, setAnalysis] = useState(false);
  const [history, setHistory] = useState<string[]>([START]);
  const [moves, setMoves] = useState<number[]>([]);
  const [legal, setLegal] = useState<Set<number>>(new Set());
  const [result, setResult] = useState<Result>("");
  const [values, setValues] = useState<Map<number, Value> | null>(null);
  const [posValue, setPosValue] = useState<Value | null>(null);
  const [line, setLine] = useState<number[]>([]);
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
  const solverTurn =
    result === "" && ((mode === "play-x" && pos.toMove === "o") || (mode === "play-o" && pos.toMove === "x"));
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
  function rematch() {
    if (!isHost) {
      online.send({ type: "rematch" });
      return;
    }
    const swapped = other(me);
    setMe(swapped);
    newGame();
    sendStart(variant, swapped, []);
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
      if (msg.type === "rematch" && isHost) rematch();
      done();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [online.inbox, moves, legal]);

  // Rules: legal moves and the result of the current position.
  useEffect(() => {
    const gen = generation.current;
    setValues(null);
    setPosValue(null);
    setLine([]);
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
    const wait = new Promise((resolve) => {
      timer = window.setTimeout(resolve, 3000 + 3000 * Math.random() ** 1.5);
    });
    Promise.all([engine.bestMove(variant, current, nodeLimit), wait]).then(([m]) => {
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

  // Analysis: the value of the position and of each legal move.
  useEffect(() => {
    if (!analysis || result !== "" || solverTurn || mode === "online") return;
    const gen = generation.current;
    engine.solve(variant, current, nodeLimit).then((v) => {
      if (gen !== generation.current) return;
      setPosValue(v);
      // A solved position also gets a line of optimal play.
      if (v !== 2) {
        engine.principalVariation(variant, current, nodeLimit).then((line) => {
          if (gen === generation.current) setLine(line);
        });
      }
    });
    engine.analyze(variant, current, nodeLimit).then((vals) => {
      if (gen === generation.current) setValues(vals);
    });
  }, [analysis, variant, current, nodeLimit, result, solverTurn, mode]);

  const status =
    result === "draw"
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
      </header>
      <main>
        <Board
          pos={pos}
          legal={legal}
          lastMove={moves.length ? moves[moves.length - 1] : null}
          values={analysisShown ? values : null}
          onMove={(m) => playMove(m)}
          disabled={solverTurn || thinking !== null || waitingForPeer}
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
                disabled={mode === "online" && !isHost}
                onChange={(e) => {
                  const v = e.target.value as Variant;
                  setVariant(v);
                  newGame();
                  if (mode === "online") sendStart(v, me, []);
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
              </>
            )}
            <div className="buttons">
              {mode === "online" ? (
                <button onClick={rematch} disabled={!connected}>
                  Rematch
                </button>
              ) : (
                <>
                  <button onClick={newGame}>New game</button>
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
              <p>{describeValue(posValue)}</p>
              {line.length ? (
                <p className="line">
                  Best line:{" "}
                  {line
                    .map((m, i) => `${glyph((pos.toMove === "x") === (i % 2 === 0) ? "x" : "o")}${boardOf(m) + 1}.${cellOf(m) + 1}`)
                    .join(" ")}
                </p>
              ) : null}
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
