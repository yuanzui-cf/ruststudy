# Calclang Terminal I/O Design

## Goal

Replace the line-oriented Output pane with an xterm-based Terminal and add
interactive `print`, `input`, and primitive conversion host functions without
changing Rust code.

## Scope

All implementation changes stay under `compile/calculator/web`. The Rust
interpreter, WASM exports, and generated WASM bindings remain unchanged.

The visible Output tab becomes Terminal. Problems remains a separate tool tab.
The Terminal keeps a 10,000-line scrollback buffer and uses xterm's own
selection, cursor, scrolling, and keyboard behavior.

## Host Functions

The JavaScript host environment adds:

```text
print(...values): none
input(...promptValues): string
float(value): float
string(value): string
bool(value): bool
```

`print` uses the existing Calclang display conversion, concatenates arguments
without separators, and writes without a newline. `println` does the same and
appends `\r\n` for terminal output.

`input` writes its optional concatenated prompt without a newline, flushes
pending terminal output, waits for one terminal line, and returns that line as
a JavaScript string. The WASM bridge already converts JavaScript strings into
Calclang string values.

The conversion functions delegate to JavaScript `Number`, `String`, and
`Boolean`. This preserves the intentionally permissive JavaScript-host
semantics already used by Math built-ins.

## Synchronous Input Bridge

The Rust evaluator invokes JavaScript host callbacks synchronously inside a
dedicated Worker. Browser dialogs are unavailable in Workers, and changing the
Rust evaluator to async is outside scope. Input therefore uses a per-run
`SharedArrayBuffer`:

1. The main thread creates the buffer and includes it in the run request.
2. When Calclang calls `input`, the Worker flushes output, marks the buffer as
   waiting, posts an input-request message, and calls `Atomics.wait`.
3. The main thread Terminal collects printable characters, supports Backspace,
   echoes input, and submits on Enter.
4. The main thread UTF-8 encodes the line into shared memory, stores its byte
   length and ready state, then calls `Atomics.notify`.
5. The Worker decodes the line, resets the buffer, and returns the string to
   WASM.

Stop, timeout, or a replacement Run terminates the blocked Worker as it does
for an infinite loop. The Terminal discards any pending input session. Stale
Worker input requests are filtered by the existing `runId` mechanism.

`SharedArrayBuffer` requires cross-origin isolation. Vite development and
preview responses set `Cross-Origin-Opener-Policy: same-origin` and
`Cross-Origin-Embedder-Policy: require-corp`. A production host must preserve
those headers. If shared memory is unavailable, programs that do not call
`input` still run; calling `input` produces a clear RuntimeError.

## Terminal Output Flow

Worker output batches remain arrays of strings, but each string is now a raw
terminal chunk rather than a DOM row. The 100-chunk batching rule remains.
Prompts flush immediately before the Worker waits, so users never wait on an
invisible prompt.

Normal final values, manual Stop, and timeout messages append `\r\n`. Problems
continue to render in the Problems DOM panel and automatically select that tab.
Starting a run resets Terminal and Problems. Editing source still does not stop
an active run.

## Monaco Support

Completion adds `print`, `input`, `float`, `string`, and `bool`, while retaining
all existing host, Math, and prelude names. No grammar or Rust parser changes
are required because these are ordinary injected identifiers.

## Testing

Bun tests cover:

- `print` versus `println` terminal chunks;
- `input` prompt emission and string return;
- JavaScript primitive conversions;
- shared-buffer UTF-8 round trips and capacity errors;
- protocol validation and stale input-request filtering;
- terminal line editing for printable text, Backspace, and Enter;
- Terminal shell IDs and labels;
- the expanded Monaco completion inventory.

Browser verification covers xterm rendering, `print`/`println`, an interactive
`input` round trip, conversion functions, Stop while waiting for input, and the
absence of a visible Output label. The existing Rust and WASM tests remain
unchanged because no Rust code is modified.
