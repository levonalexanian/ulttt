// Promise-based access to the engine running in a Web Worker.
import type { Request, Response } from "./engine.worker";

export type Variant = "classic" | "open" | "majority";
export const VARIANTS: Variant[] = ["classic", "open", "majority"];

/** A value from X's point of view: 1 X wins, 0 draw, -1 O wins, 2 unknown. */
export type Value = -1 | 0 | 1 | 2;
export const UNKNOWN: Value = 2;

export type Result = "x" | "o" | "draw" | "";

type Pending = { resolve: (v: unknown) => void; reject: (e: Error) => void };

// Distributes `Omit` over the union of request types.
type Call = Request extends infer R ? (R extends Request ? Omit<R, "id"> : never) : never;

class Engine {
  private worker = new Worker(new URL("./engine.worker.ts", import.meta.url), { type: "module" });
  private pending = new Map<number, Pending>();
  private nextId = 0;

  constructor() {
    this.worker.onmessage = (event: MessageEvent<Response>) => {
      const res = event.data;
      const p = this.pending.get(res.id);
      if (!p) return;
      this.pending.delete(res.id);
      if (res.ok) p.resolve(res.value);
      else p.reject(new Error(res.error));
    };
  }

  private call<T>(req: Call): Promise<T> {
    const id = this.nextId++;
    return new Promise<T>((resolve, reject) => {
      this.pending.set(id, { resolve: resolve as (v: unknown) => void, reject });
      this.worker.postMessage({ ...req, id });
    });
  }

  legalMoves(variant: Variant, pos: string) {
    return this.call<number[]>({ fn: "legalMoves", variant, pos });
  }
  play(variant: Variant, pos: string, move: number) {
    return this.call<string>({ fn: "play", variant, pos, move });
  }
  result(variant: Variant, pos: string) {
    return this.call<Result>({ fn: "result", variant, pos });
  }
  solve(variant: Variant, pos: string, nodeLimit: number) {
    return this.call<Value>({ fn: "solve", variant, pos, nodeLimit });
  }
  /** The value of each legal move, as a map from move to value. */
  async analyze(variant: Variant, pos: string, nodeLimit: number) {
    const flat = await this.call<number[]>({ fn: "analyze", variant, pos, nodeLimit });
    const values = new Map<number, Value>();
    for (let i = 0; i < flat.length; i += 2) values.set(flat[i], flat[i + 1] as Value);
    return values;
  }
  /** A line of optimal play; empty if the position could not be solved. */
  principalVariation(variant: Variant, pos: string, nodeLimit: number) {
    return this.call<number[]>({ fn: "principalVariation", variant, pos, nodeLimit });
  }
  bestMove(variant: Variant, pos: string, nodeLimit: number) {
    return this.call<number>({ fn: "bestMove", variant, pos, nodeLimit });
  }
}

export const engine = new Engine();
