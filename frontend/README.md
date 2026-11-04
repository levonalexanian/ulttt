# frontend

React + TypeScript (Vite) interface: play against the solver in any of the
three variants, or analyze positions. The Rust engine (`backend/crates/wasm`)
is compiled to WebAssembly and runs in a Web Worker, so everything runs in the
browser; there is no server.

```sh
npm install
npm run dev     # builds the engine to src/wasm/, then serves on port 5173
npm run build   # production build in dist/
```

Building the engine needs the `wasm32-unknown-unknown` Rust target and
`wasm-bindgen-cli` 0.2.130, both in the devcontainer.
