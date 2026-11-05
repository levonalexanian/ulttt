const assert = require("node:assert/strict");
const { EventEmitter } = require("node:events");
const { readFileSync } = require("node:fs");
const { test } = require("node:test");
const vm = require("node:vm");
const ts = require("typescript");

// Run the actual hook with controlled PeerJS events and a deterministic clock.
// No signaling server, browser network, or additional test dependencies needed.
const source = ts.transpileModule(readFileSync("src/online.ts", "utf8"), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
}).outputText;

function setup() {
  class Connection extends EventEmitter {
    open = false;
    closed = false;
    close() {
      this.closed = true;
      // Match PeerJS: close before open does not emit a close event.
      if (this.open) {
        this.open = false;
        this.emit("close");
      }
    }
    send() {}
    establish() {
      this.open = true;
      this.emit("open");
    }
  }
  const peers = [];
  class Peer extends EventEmitter {
    destroyed = false;
    connections = [];
    constructor() {
      super();
      peers.push(this);
    }
    connect() {
      const c = new Connection();
      this.connections.push(c);
      return c;
    }
    destroy() {
      this.destroyed = true;
    }
  }
  const states = [];
  const cleanups = [];
  const timers = new Map();
  let now = 0;
  let timerId = 0;
  const clock = {
    setTimeout(fn, delay) {
      timers.set(++timerId, { fn, at: now + delay });
      return timerId;
    },
    clearTimeout(id) { timers.delete(id); },
  };
  const react = {
    useState(initial) {
      const slot = states.push(initial) - 1;
      return [initial, (next) => { states[slot] = typeof next === "function" ? next(states[slot]) : next; }];
    },
    useRef: (current) => ({ current }),
    useEffect: (effect) => cleanups.push(effect()),
  };
  const exports = {};
  vm.runInNewContext(source, {
    exports,
    require(name) {
      if (name === "react") return react;
      if (name === "peerjs") return { default: Peer };
      throw new Error(`Unexpected import: ${name}`);
    },
    crypto: require("node:crypto").webcrypto,
    window: clock,
    ...clock,
  });
  return {
    online: exports.useOnline(), peers, Connection,
    get status() { return states[0]; },
    get inbox() { return states[1]; },
    get timers() { return timers.size; },
    cleanup() { cleanups.forEach((fn) => fn?.()); },
    advance(ms) {
      now += ms;
      for (const [id, timer] of [...timers]) {
        if (timer.at <= now && timers.delete(id)) timer.fn();
      }
    },
  };
}

test("a negotiation error ends a guest attempt even without a close event", () => {
  const h = setup();
  h.online.join("abcdef");
  const p = h.peers[0];
  p.emit("open");
  const c = p.connections[0];
  c.emit("error", { type: "negotiation-failed" });
  assert.equal(h.status.kind, "closed");
  assert.match(h.status.reason, /try again/);
  assert.equal(c.closed, true);
  assert.equal(p.destroyed, true);
});

test("a host accepts a new guest after negotiation fails", () => {
  const h = setup();
  h.online.host();
  const p = h.peers[0];
  p.emit("open");
  const first = new h.Connection();
  p.emit("connection", first);
  first.emit("error", { type: "negotiation-failed" });
  assert.equal(h.status.kind, "waiting");
  assert.equal(p.destroyed, false);
  const retry = new h.Connection();
  p.emit("connection", retry);
  retry.establish();
  assert.equal(h.status.kind, "connected");
  assert.equal(h.status.host, true);
  assert.equal(retry.closed, false);
});

test("late events from an old connection cannot corrupt a retry", () => {
  const h = setup();
  h.online.join("first");
  const oldPeer = h.peers[0];
  oldPeer.emit("open");
  const oldConn = oldPeer.connections[0];
  h.online.join("second");
  oldPeer.emit("open");
  oldPeer.emit("error", { type: "network" });
  oldConn.establish();
  oldConn.emit("data", { type: "rematch" });
  oldConn.emit("error", { type: "negotiation-failed" });
  oldConn.emit("close");
  assert.equal(h.status.kind, "connecting");
  assert.equal(h.status.code, "second");
  assert.equal(h.inbox.length, 0);
  assert.equal(oldPeer.connections.length, 1);
  h.peers[1].emit("open");
  h.peers[1].connections[0].establish();
  assert.equal(h.status.kind, "connected");
});

test("normal departure preserves the host's game code", () => {
  const h = setup();
  h.online.host();
  const p = h.peers[0];
  p.emit("open");
  const code = h.status.code;
  const c = new h.Connection();
  p.emit("connection", c);
  c.establish();
  c.close();
  assert.equal(h.status.kind, "waiting");
  assert.equal(h.status.code, code);
});

test("signaling errors destroy the failed peer and allow another attempt", () => {
  const h = setup();
  h.online.join("missing");
  h.peers[0].emit("error", { type: "peer-unavailable" });
  assert.equal(h.status.reason, "No game with that code.");
  assert.equal(h.peers[0].destroyed, true);
  h.online.join("retry");
  assert.equal(h.status.kind, "connecting");
});

test("leaving an open connection stays idle despite synchronous close events", () => {
  const h = setup();
  h.online.join("abcdef");
  h.peers[0].emit("open");
  h.peers[0].connections[0].establish();
  h.online.leave();
  assert.equal(h.status.kind, "idle");
  assert.equal(h.peers[0].destroyed, true);
});

test("the guest deadline includes signaling and negotiation in one ten-second budget", () => {
  const h = setup();
  h.online.join("abcdef");
  h.advance(9_000);
  h.peers[0].emit("open");
  h.advance(999);
  assert.equal(h.status.kind, "connecting");
  h.advance(1);
  assert.equal(h.status.kind, "closed");
  assert.match(h.status.reason, /10 seconds/);
  assert.equal(h.peers[0].destroyed, true);
  assert.equal(h.peers[0].connections[0].closed, true);
  assert.equal(h.timers, 0);
});

test("a guest can time out before the signaling server responds", () => {
  const h = setup();
  h.online.join("abcdef");
  h.advance(10_000);
  h.peers[0].emit("open");
  assert.equal(h.status.kind, "closed");
  assert.equal(h.peers[0].connections.length, 0);
});

test("a stalled incoming attempt times out and the host accepts a retry", () => {
  const h = setup();
  h.online.host();
  const p = h.peers[0];
  p.emit("open");
  const first = new h.Connection();
  p.emit("connection", first);
  h.advance(10_000);
  assert.equal(first.closed, true);
  assert.equal(h.status.kind, "waiting");
  const retry = new h.Connection();
  p.emit("connection", retry);
  retry.establish();
  h.advance(10_000);
  assert.equal(h.status.kind, "connected");
  assert.equal(h.timers, 0);
});

test("an established connection survives past the deadline", () => {
  const h = setup();
  h.online.join("abcdef");
  h.peers[0].emit("open");
  h.peers[0].connections[0].establish();
  h.advance(60_000);
  assert.equal(h.status.kind, "connected");
  assert.equal(h.peers[0].destroyed, false);
  assert.equal(h.timers, 0);
});

test("leaving, retrying, and unmounting cancel obsolete deadlines", () => {
  const h = setup();
  h.online.join("first");
  h.advance(5_000);
  h.online.join("second");
  h.advance(5_000);
  assert.equal(h.status.kind, "connecting");
  assert.equal(h.status.code, "second");
  assert.equal(h.timers, 1);
  h.online.leave();
  h.advance(20_000);
  assert.equal(h.status.kind, "idle");
  assert.equal(h.timers, 0);
  h.online.join("third");
  h.cleanup();
  assert.equal(h.timers, 0);
  assert.equal(h.peers[2].destroyed, true);
});
