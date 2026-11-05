// Peer-to-peer games between two browsers over WebRTC. PeerJS's public
// server only introduces the two browsers; moves then go directly between them.
import Peer, { type DataConnection } from "peerjs";
import { useEffect, useRef, useState } from "react";
import type { Variant } from "./engine";

/** What the two players send each other. */
export type Message =
  /** From the host: a game starts (or restarts) with these moves played. */
  | { type: "start"; variant: Variant; you: "x" | "o"; moves: number[] }
  /** The sender's move, as move number `ply` of the game (0-based). */
  | { type: "move"; ply: number; move: number }
  /** The guest asks the host for a new game with sides swapped, under these
   * rules (the current ones if absent). */
  | { type: "rematch"; variant?: Variant }
  /** The guest asks the host to restart the game, not yet begun, under other rules. */
  | { type: "rules"; variant: Variant };

export type Status =
  | { kind: "idle" }
  | { kind: "waiting"; code: string }
  | { kind: "connecting"; code: string }
  | { kind: "connected"; host: boolean }
  | { kind: "closed"; reason: string };

const PREFIX = "ulttt-game-";
// Unambiguous characters, so a code read aloud is easy to type.
const ALPHABET = "abcdefghjkmnpqrstuvwxyz23456789";

function newCode(): string {
  const bytes = crypto.getRandomValues(new Uint8Array(6));
  return Array.from(bytes, (b) => ALPHABET[b % ALPHABET.length]).join("");
}

/** The link that joins the game with this code. */
export function joinLink(code: string): string {
  const url = new URL(location.href);
  url.search = `?join=${code}`;
  url.hash = "";
  return url.toString();
}

/** The code in the page's `?join=` parameter, if any. */
export function codeFromUrl(): string | null {
  return new URLSearchParams(location.search).get("join");
}

/**
 * A connection to one other player. `host()` waits for a guest under a fresh
 * code; `join(code)` connects to a host. Received messages are queued in
 * `inbox` and removed with `take()`, so none are lost between renders.
 */
export function useOnline() {
  const [status, setStatus] = useState<Status>({ kind: "idle" });
  const [inbox, setInbox] = useState<Message[]>([]);
  const peer = useRef<Peer | null>(null);
  const conn = useRef<DataConnection | null>(null);
  const code = useRef<string | null>(null);
  // Set when the host's guest connects, so the app can send the first state.
  const [guestArrived, setGuestArrived] = useState(0);

  function close() {
    conn.current?.close();
    peer.current?.destroy();
    conn.current = null;
    peer.current = null;
    code.current = null;
  }

  function attach(c: DataConnection, host: boolean) {
    conn.current = c;
    c.on("open", () => {
      setStatus({ kind: "connected", host });
      if (host) setGuestArrived((n) => n + 1);
    });
    c.on("data", (data) => setInbox((q) => [...q, data as Message]));
    c.on("close", () => {
      if (conn.current !== c) return;
      conn.current = null;
      // The host keeps the game and its code, so the guest can come back.
      if (host && code.current) setStatus({ kind: "waiting", code: code.current });
      else setStatus({ kind: "closed", reason: "Your opponent left." });
    });
  }

  function host() {
    close();
    setInbox([]);
    const c = newCode();
    code.current = c;
    const p = new Peer(PREFIX + c);
    peer.current = p;
    p.on("open", () => setStatus({ kind: "waiting", code: c }));
    p.on("connection", (c) => {
      // One opponent per game.
      if (conn.current) {
        c.on("open", () => c.close());
        return;
      }
      attach(c, true);
    });
    p.on("error", (e) => {
      if (e.type === "unavailable-id") host();
      else if (peer.current === p) setStatus({ kind: "closed", reason: `Connection error: ${e.type}` });
    });
  }

  function join(joinCode: string) {
    close();
    setInbox([]);
    const c = joinCode.trim().toLowerCase();
    setStatus({ kind: "connecting", code: c });
    const p = new Peer();
    peer.current = p;
    p.on("open", () => attach(p.connect(PREFIX + c, { reliable: true }), false));
    p.on("error", (e) => {
      if (peer.current !== p) return;
      setStatus({
        kind: "closed",
        reason: e.type === "peer-unavailable" ? "No game with that code." : `Connection error: ${e.type}`,
      });
    });
  }

  function leave() {
    close();
    setInbox([]);
    setStatus({ kind: "idle" });
  }

  function send(m: Message) {
    conn.current?.send(m);
  }

  function take() {
    setInbox((q) => q.slice(1));
  }

  useEffect(() => close, []);

  return { status, inbox, take, host, join, leave, send, guestArrived };
}
