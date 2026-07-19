# Learning Rust

This is the rust learning repository for [yuanzui-cf](https://github.com/yuanzui-cf). It contains codes that writing for learning rust.

## Calclang web playground

The browser playground lives in `compile/calculator/web`. Install
[`wasm-pack`](https://rustwasm.github.io/wasm-pack/installer/) first, then use
Bun for the frontend workflow:

```sh
cd compile/calculator/web
bun install
bun run dev
```

`bun run dev` builds the release WebAssembly bindings before starting Vite.
Each click on **Run** evaluates a source snapshot in a fresh calclang
environment. Editing the source does not interrupt active code; use **Stop** to
terminate it, or click **Run** again to replace it with a new execution.

Create production assets with:

```sh
bun run build
```

The generated site is written to `compile/calculator/web/dist`.
