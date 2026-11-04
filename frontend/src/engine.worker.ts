// Runs the Rust engine (compiled to WebAssembly) off the main thread, so long
// solver searches do not freeze the page.
import init, * as wasm from "./wasm/ulttt_wasm.js";

export type Request =
  | { id: number; fn: "legalMoves"; variant: string; pos: string }
  | { id: number; fn: "play"; variant: string; pos: string; move: number }
  | { id: number; fn: "result"; variant: string; pos: string }
  | { id: number; fn: "solve"; variant: string; pos: string; nodeLimit: number }
  | { id: number; fn: "analyze"; variant: string; pos: string; nodeLimit: number }
  | { id: number; fn: "bestMove"; variant: string; pos: string; nodeLimit: number }
  | { id: number; fn: "think"; variant: string; pos: string; ms: number }
  | { id: number; fn: "principalVariation"; variant: string; pos: string; nodeLimit: number };

export type Response = { id: number; ok: true; value: unknown } | { id: number; ok: false; error: string };

const ready = init();

function run(req: Request): unknown {
  switch (req.fn) {
    case "legalMoves":
      return Array.from(wasm.legalMoves(req.variant, req.pos));
    case "play":
      return wasm.play(req.variant, req.pos, req.move);
    case "result":
      return wasm.result(req.variant, req.pos);
    case "solve":
      return wasm.solve(req.variant, req.pos, req.nodeLimit);
    case "analyze":
      return Array.from(wasm.analyze(req.variant, req.pos, req.nodeLimit));
    case "bestMove":
      return wasm.bestMove(req.variant, req.pos, req.nodeLimit);
    case "think":
      return Array.from(wasm.think(req.variant, req.pos, req.ms));
    case "principalVariation":
      return Array.from(wasm.principalVariation(req.variant, req.pos, req.nodeLimit));
  }
}

self.onmessage = async (event: MessageEvent<Request>) => {
  await ready;
  const req = event.data;
  let res: Response;
  try {
    res = { id: req.id, ok: true, value: run(req) };
  } catch (e) {
    res = { id: req.id, ok: false, error: String(e) };
  }
  self.postMessage(res);
};
