# Calclang Web Playground Design

## Goal

Build a browser playground for calclang that runs the Rust interpreter through WebAssembly, exposes JavaScript host functions and constants, edits source with Monaco, and safely terminates a long-running program without freezing the page.

## Scope

The first version provides a single editor, Run and Stop controls, streaming output, structured problems, a fresh environment per run, JavaScript-backed built-ins, a small calclang prelude, and EBNF-based Monaco highlighting and completion.

Environment persistence is represented in the runtime API by a `preserveEnvironment` flag, but the UI always sends `false` in this version. Source persistence, multiple files, debugging, and user-defined host functions are outside this version.

## Toolchain and Location

The frontend lives beside the interpreter under `compile/calculator/web/`.

- Bun installs dependencies, runs scripts, and runs TypeScript tests.
- Vite serves and builds the browser application through Bun.
- `wasm-pack` builds the `wasm32-unknown-unknown` library and generates browser bindings.
- Monaco is consumed from the `monaco-editor` package.
- The repository commits `bun.lock` and does not use npm, pnpm, Yarn, or a Node-based command in its documented workflow.

The development commands are:

```text
bun install
bun run wasm
bun run dev
bun test
bun run build
```

## WebAssembly Boundary

The current 367-byte raw artifact exports only `memory`, `__data_end`, and `__heap_base`, so it cannot evaluate source from JavaScript. The calculator crate will add `wasm-bindgen` and expose a browser-facing interpreter API.

The exported interpreter owns an `Environment` and supports these operations:

- create or reset an environment;
- define a primitive constant;
- define a JavaScript host function;
- evaluate calclang source;
- return a final primitive value or a structured calclang error.

`Value::BuiltIn` will evolve from a plain function pointer to a cloneable callback wrapper so a built-in can capture a JavaScript `Function`. Native built-ins remain supported through the same wrapper. Debug output and equality for built-ins use their stable identity rather than attempting to compare closures.

Primitive values cross the host bridge as JavaScript numbers, booleans, strings, and `undefined` for `none`. Calclang functions cross as an opaque branded descriptor containing their calclang type name. JavaScript return values of number, boolean, string, `null`, or `undefined` convert back to calclang values; an unsupported object return produces a calclang `TypeError`. A JavaScript exception becomes a calclang `RuntimeError`.

## Host Environment

Each fresh environment receives these JavaScript `Math` constants before the prelude is evaluated:

```text
PI E SQRT2 SQRT1_2 LN2 LN10 LOG2E LOG10E
```

It also receives these JavaScript host functions:

```text
println typeof
sin cos tan asin acos atan atan2
sinh cosh tanh
abs sqrt cbrt
exp log log2 log10 pow
floor ceil round trunc sign
random
```

Math functions delegate to the corresponding `Math` function. This preserves the calculator's intentionally permissive argument behavior: JavaScript handles missing or extra arguments, and floating-point results such as `NaN` and infinity remain valid `f64` values.

`println(...values)` renders values using calclang display semantics, concatenates them without an inserted separator to match the native CLI, and emits one output entry. `typeof(value)` returns calclang names such as `float`, `bool`, `string`, `none`, `fn`, or `fn(arg1,arg2)` by recognizing primitive values and the opaque function descriptor.

The frontend imports a calclang prelude as source and evaluates it after host injection and before user source:

```calc
fn min(a, b) {
    if a < b { a } else { b }
}

fn max(a, b) {
    if a > b { a } else { b }
}
```

## Execution Model

The Monaco editor and UI run on the main browser thread. Calclang evaluation runs in a dedicated module Web Worker so a tight WASM loop cannot block the page.

The main thread sends a run request containing:

```ts
interface RunRequest {
  type: "run";
  runId: number;
  source: string;
  preserveEnvironment: false;
}
```

The flag remains in the contract for later environment persistence. An idle Worker can eventually reuse its interpreter when the flag becomes `true`. The first version always resets the environment, injects host values, evaluates the prelude, and then evaluates the user snapshot.

Editing does not stop or modify an active run. Clicking Run while a program is active terminates that Worker, creates a replacement, and starts the new source snapshot. Clicking Stop terminates the active Worker. A monotonically increasing `runId` lets the controller ignore messages from an obsolete run.

The ten-minute safety timer lives on the main thread because an executing WASM loop can block the Worker event loop. It starts with each run and is cleared on completion, ordinary failure, manual stop, or replacement. Expiration terminates the Worker and appends this entry to Output:

```text
RuntimeError: Loop execution exceeded the 10-minute time limit.
```

The UI does not display elapsed time, the time limit, or Worker implementation details.

## Output Flow

The injected `println` callback runs inside the Worker and streams output to the main thread. It groups each 100 entries into one `postMessage` batch; a partial final batch is flushed when evaluation returns or throws. This reduces message overhead without suppressing output.

The Output pane is a rolling DOM window containing the newest 10,000 entries. New batches are appended through a `DocumentFragment`, and the oldest nodes are removed when the count exceeds 10,000. No truncation notice is shown and the program continues unchanged.

On each Run, the prior Output and Problems are cleared. A normal final value is appended after pending `println` output unless it is `none`. Manual Stop appends:

```text
Execution stopped by user.
```

Lexer, parser, type, name, JavaScript-host, and ordinary runtime failures populate the Problems pane and select its tab. The ten-minute termination remains in Output because it is generated by the main-thread run controller after the Worker has been destroyed.

## Interface Design

The shell follows the dense Visual Studio 2022 dark industrial style, not the VS Code activity-bar layout:

- dark title, menu, and command bars;
- Run and Stop controls in the command bar;
- a document tab above the editor;
- a fixed desktop split with Monaco on the left and a docked Output/Problems tool window on the right;
- a purple Visual Studio-style status bar;
- a responsive narrow layout that moves the tool window below the editor.

The browser page does not customize Monaco's internal CSS. Monaco uses its built-in `vs-dark` theme and its normal editor chrome. The surrounding shell supplies the Visual Studio styling.

The status bar may show the fresh-environment mode, WASM readiness, cursor position, indentation, and encoding. It does not show run duration or a message such as “Process running in isolated Web Worker.” Run remains available so it can replace an active execution; Stop is enabled only while a run is active.

## Monaco Language Support

The frontend registers a `calclang` language and a Monarch tokenizer derived from `EBNF.txt`.

The tokenizer recognizes:

- identifiers and the keywords `let`, `if`, `else`, `loop`, `break`, `continue`, `fn`, and `return`;
- `true`, `false`, and `none` literals;
- decimal forms accepted by the lexer, including `.5`, `1.`, and scientific notation;
- quoted strings and `\\t`, `\\n`, `\\r`, `\\"`, `\\'`, and `\\\\` escapes;
- `//` line comments and `/* ... */` block comments;
- assignment, arithmetic, comparison, logical, punctuation, and bracket tokens.

The language configuration supplies matching brackets, auto-closing pairs, surrounding pairs, comment syntax, and indentation behavior for braces.

Completion contains:

- every keyword and literal keyword;
- snippets for `let`, `if`, `if/else`, `loop`, named `fn`, anonymous `fn`, and `return`;
- signatures and short descriptions for every JavaScript host function;
- all injected Math constants;
- `min` and `max` from the prelude.

Completion is syntax-aware only at the normal Monaco word level. Live parsing, live diagnostics, semantic analysis, and formatting are outside this version.

## Errors and Recovery

WASM initialization or prelude failure prevents a run and appears in Problems. A host-function exception is converted to a calclang `RuntimeError` with the JavaScript message. A host return type that cannot become a calclang primitive becomes a `TypeError`.

After Stop, timeout, or replacement, the terminated Worker is discarded. The controller lazily creates a clean Worker for the next run. Run, Stop, completion, and error paths all clear their timer handles and active-run references so an old timeout cannot terminate a later execution.

## Testing and Verification

Rust tests cover existing lexer, parser, environment, and evaluator behavior plus the callback-backed built-in representation. Browser-targeted WASM tests cover primitive conversion, host function calls, JavaScript exceptions, and unsupported return values.

Bun tests cover pure TypeScript modules:

- EBNF-derived token patterns and the complete completion inventory;
- Run replacing an active Worker;
- source edits leaving an active Worker untouched;
- manual Stop;
- ten-minute timeout behavior using an injectable shorter test timeout;
- stale `runId` messages being ignored;
- output batches preserving order;
- rolling DOM output retaining the newest 10,000 entries.

The completion gate runs:

```text
cargo test -p calculator
bun test
bun run wasm
bun run build
```

The sample program verifies a JavaScript Math host call, a prelude function, `println`, and the final expression in one end-to-end browser run.
