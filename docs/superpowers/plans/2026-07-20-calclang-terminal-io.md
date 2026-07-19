# Calclang Terminal I/O Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the Output pane with an xterm Terminal and add synchronous `print`, `input`, `float`, `string`, and `bool` JavaScript host functions without modifying Rust.

**Architecture:** Worker output becomes raw terminal chunks. A per-run `SharedArrayBuffer` lets the synchronous Worker host callback block with `Atomics.wait` while the main-thread xterm collects and echoes one input line, then wakes the Worker. Pure helpers own shared-memory encoding and line editing; RunController only routes typed messages and lifecycle events.

**Tech Stack:** Bun, TypeScript, Vite, Monaco Editor, `@xterm/xterm`, `@xterm/addon-fit`, Web Workers, SharedArrayBuffer, Atomics

---

## File Map

- Modify: `compile/calculator/web/package.json` and `bun.lock` - xterm dependencies.
- Modify: `compile/calculator/web/src/calclang/host.ts` and tests - terminal I/O and conversion built-ins.
- Modify: `compile/calculator/web/src/calclang/language-data.ts` and tests - completion inventory.
- Create: `compile/calculator/web/src/runtime/input-buffer.ts` and tests - shared-memory state and UTF-8 encoding.
- Modify: `compile/calculator/web/src/runtime/protocol.ts` and tests - input request messages.
- Modify: `compile/calculator/web/src/runtime/run-controller.ts` and tests - route input and include per-run buffer.
- Create: `compile/calculator/web/src/ui/terminal-input.ts` and tests - pure terminal line editing.
- Create: `compile/calculator/web/src/ui/terminal-window.ts` - xterm ownership and input sessions.
- Modify: `compile/calculator/web/src/runtime/calclang.worker.ts` - synchronous input bridge.
- Modify: `compile/calculator/web/index.html`, `src/main.ts`, `src/styles.css`, and shell tests - Terminal UI.
- Modify: `compile/calculator/web/vite.config.ts` - cross-origin isolation headers.
- Modify: `README.md` - production header requirement.

### Task 1: Add Terminal Host Functions

**Files:**

- Modify: `compile/calculator/web/src/calclang/host.test.ts`
- Modify: `compile/calculator/web/src/calclang/host.ts`
- Modify: `compile/calculator/web/src/calclang/language-data.test.ts`
- Modify: `compile/calculator/web/src/calclang/language-data.ts`

- [ ] **Step 1: Write failing host tests**

Add tests asserting:

```ts
test("print and println emit terminal chunks", () => {
  const chunks: string[] = [];
  const functions = createHostFunctions((chunk) => chunks.push(chunk), () => "");
  functions.print("answer=", 42);
  functions.println(" done");
  expect(chunks).toEqual(["answer=42", " done\r\n"]);
});

test("input emits a prompt and returns a string", () => {
  const chunks: string[] = [];
  const functions = createHostFunctions((chunk) => chunks.push(chunk), () => "Leo");
  expect(functions.input("Name: ")).toBe("Leo");
  expect(chunks).toEqual(["Name: "]);
});

test("exports JavaScript primitive conversions", () => {
  const functions = createHostFunctions(() => {}, () => "");
  expect(functions.float("3.5")).toBe(3.5);
  expect(functions.string(42)).toBe("42");
  expect(functions.bool(0)).toBe(false);
});
```

Extend the language-data test to require `print`, `input`, `float`, `string`, and `bool` in `COMPLETION_NAMES`.

- [ ] **Step 2: Run tests and verify RED**

Run:

```sh
cd compile/calculator/web
bun test src/calclang/host.test.ts src/calclang/language-data.test.ts
```

Expected: failures because the five functions and completions do not exist and `println` lacks `\r\n`.

- [ ] **Step 3: Implement the host API**

Change the factory signature and add the functions:

```ts
export function createHostFunctions(
  emit: (chunk: string) => void,
  readLine: () => string,
) {
  return {
    print: (...values: unknown[]) => {
      emit(values.map(display).join(""));
      return undefined;
    },
    println: (...values: unknown[]) => {
      emit(values.map(display).join("") + "\r\n");
      return undefined;
    },
    input: (...promptValues: unknown[]) => {
      const prompt = promptValues.map(display).join("");
      if (prompt !== "") emit(prompt);
      return readLine();
    },
    float: Number,
    string: String,
    bool: Boolean,
    // keep typeof and every Math function unchanged
  };
}
```

Add the five names to the `hostFunctions` completion array.

- [ ] **Step 4: Verify GREEN and commit**

Run the focused tests, then:

```sh
git add compile/calculator/web/src/calclang/host.ts \
  compile/calculator/web/src/calclang/host.test.ts \
  compile/calculator/web/src/calclang/language-data.ts \
  compile/calculator/web/src/calclang/language-data.test.ts
git commit -m "feat(calculator): Add terminal host functions"
```

### Task 2: Build The Shared Input Buffer

**Files:**

- Create: `compile/calculator/web/src/runtime/input-buffer.test.ts`
- Create: `compile/calculator/web/src/runtime/input-buffer.ts`

- [ ] **Step 1: Write failing shared-buffer tests**

Test UTF-8 round trips, state transitions, and capacity:

```ts
test("resolves and reads a UTF-8 input line", () => {
  const buffer = createInputBuffer(32);
  const state = new Int32Array(buffer, 0, 2);
  Atomics.store(state, INPUT_STATE_INDEX, INPUT_WAITING);
  expect(resolveInput(buffer, "你好")).toBe(true);
  expect(readResolvedInput(buffer)).toBe("你好");
  expect(Atomics.load(state, INPUT_STATE_INDEX)).toBe(INPUT_IDLE);
});

test("rejects a line larger than the shared capacity", () => {
  const buffer = createInputBuffer(2);
  const state = new Int32Array(buffer, 0, 2);
  Atomics.store(state, INPUT_STATE_INDEX, INPUT_WAITING);
  expect(() => resolveInput(buffer, "abc")).toThrow("Input exceeds terminal buffer capacity");
});
```

- [ ] **Step 2: Run and verify RED**

Run `bun test src/runtime/input-buffer.test.ts` and expect a missing-module failure.

- [ ] **Step 3: Implement the buffer API**

Use two `Int32` header slots followed by UTF-8 bytes:

```ts
export const INPUT_STATE_INDEX = 0;
export const INPUT_LENGTH_INDEX = 1;
export const INPUT_IDLE = 0;
export const INPUT_WAITING = 1;
export const INPUT_READY = 2;
const HEADER_BYTES = Int32Array.BYTES_PER_ELEMENT * 2;

export function createInputBuffer(capacity = 64 * 1024): SharedArrayBuffer;
export function resolveInput(buffer: SharedArrayBuffer, value: string): boolean;
export function readResolvedInput(buffer: SharedArrayBuffer): string;
export function waitForInput(
  buffer: SharedArrayBuffer,
  requestInput: () => void,
): string;
```

`waitForInput` stores `INPUT_WAITING`, invokes `requestInput`, loops on
`Atomics.wait`, and returns `readResolvedInput` when state becomes ready.
`resolveInput` returns false unless state is waiting, writes bytes and length,
stores ready, and calls `Atomics.notify`.

- [ ] **Step 4: Verify GREEN and commit**

Run the focused test and commit the two files with:

```sh
git commit -m "feat(calculator): Add shared terminal input"
```

### Task 3: Route Input Through The Worker Controller

**Files:**

- Modify: `compile/calculator/web/src/runtime/protocol.test.ts`
- Modify: `compile/calculator/web/src/runtime/protocol.ts`
- Modify: `compile/calculator/web/src/runtime/run-controller.test.ts`
- Modify: `compile/calculator/web/src/runtime/run-controller.ts`

- [ ] **Step 1: Write failing protocol and controller tests**

Add an input response contract:

```ts
const buffer = new SharedArrayBuffer(16);
expect(isWorkerResponse({ type: "input", runId: 4, buffer })).toBe(true);
```

Add a controller test that calls:

```ts
controller.run("input()", false, buffer);
expect(worker.messages[0]).toMatchObject({ inputBuffer: buffer });
worker.emit({ type: "input", runId: 1, buffer });
expect(requestedInputs).toEqual([buffer]);
```

Also assert a stale input response is ignored.

- [ ] **Step 2: Run and verify RED**

Run `bun test src/runtime/protocol.test.ts src/runtime/run-controller.test.ts`.

- [ ] **Step 3: Extend typed messages and controller options**

Add `inputBuffer?: SharedArrayBuffer` to `RunRequest`, add this response:

```ts
{ type: "input"; runId: number; buffer: SharedArrayBuffer }
```

Add `onInput: (buffer: SharedArrayBuffer) => void` to controller options. Change
`run` to `run(source, preserveEnvironment = false, inputBuffer?)`, include the
buffer in the request only when defined, and route current-run input messages
without clearing the active run or timer.

Terminal-facing controller messages append newlines:

```ts
this.#options.onOutput([TIMEOUT_MESSAGE + "\r\n"]);
this.#options.onOutput(["Execution stopped by user.\r\n"]);
this.#options.onOutput([value.result + "\r\n"]);
```

- [ ] **Step 4: Verify GREEN and commit**

Run the two focused tests and commit the four files with:

```sh
git commit -m "feat(calculator): Route terminal input"
```

### Task 4: Add The Xterm Terminal Component

**Files:**

- Modify: `compile/calculator/web/package.json`
- Modify: `compile/calculator/web/bun.lock`
- Create: `compile/calculator/web/src/ui/terminal-input.test.ts`
- Create: `compile/calculator/web/src/ui/terminal-input.ts`
- Create: `compile/calculator/web/src/ui/terminal-window.ts`

- [ ] **Step 1: Install xterm packages with Bun**

Run:

```sh
cd compile/calculator/web
bun add @xterm/xterm @xterm/addon-fit
```

- [ ] **Step 2: Write failing line-editor tests**

Define a pure API:

```ts
const edit = applyTerminalInput("ab", "\x7fc\r");
expect(edit).toEqual({ value: "ac", echo: "\b \bc\r\n", submitted: "ac" });
```

Also test ordinary pasted text and ignored control sequences.

- [ ] **Step 3: Run and verify RED**

Run `bun test src/ui/terminal-input.test.ts` and expect a missing-module failure.

- [ ] **Step 4: Implement line editing and xterm ownership**

`applyTerminalInput(value, data)` returns the next value, terminal echo, and an
optional submitted line. It accepts printable characters, handles Backspace,
submits on CR/LF, and ignores other C0 controls.

`TerminalWindow` owns `Terminal`, `FitAddon`, and `ResizeObserver`:

```ts
new Terminal({
  cursorBlink: true,
  scrollback: 10_000,
  fontFamily: "Consolas, 'Courier New', monospace",
  fontSize: 13,
  theme: { background: "#1e1e1e", foreground: "#d8d8d8" },
});
```

Expose `reset`, `write(chunks)`, `requestInput(buffer)`, `cancelInput`, `fit`,
and `dispose`. On Enter, call `resolveInput`; on failure ring the terminal bell
and keep the input session active.

- [ ] **Step 5: Verify GREEN and commit**

Run the focused test, `bunx tsc --noEmit`, and commit package, lock, and the
three UI files with:

```sh
git commit -m "feat(calculator): Add xterm terminal"
```

### Task 5: Integrate Terminal I/O End To End

**Files:**

- Modify: `compile/calculator/web/src/runtime/calclang.worker.ts`
- Modify: `compile/calculator/web/index.html`
- Modify: `compile/calculator/web/src/main.ts`
- Modify: `compile/calculator/web/src/styles.css`
- Modify: `compile/calculator/web/src/ui/shell.test.ts`
- Modify: `compile/calculator/web/vite.config.ts`
- Modify: `README.md`

- [ ] **Step 1: Write failing shell contracts**

Require `terminal`, `terminal-tab`, and visible `Terminal`; reject `output`,
`output-tab`, and visible `Output` labels in `index.html`.

- [ ] **Step 2: Run and verify RED**

Run `bun test src/ui/shell.test.ts` and expect failures for the old Output IDs.

- [ ] **Step 3: Wire Worker input and prompt flushing**

Pass a `readLine` callback into `createHostFunctions`. It must throw
`Error("input() requires cross-origin isolation")` when `inputBuffer` is absent.
Otherwise it flushes `OutputBatcher` and calls:

```ts
waitForInput(request.inputBuffer, () => {
  post({ type: "input", runId: request.runId, buffer: request.inputBuffer! });
});
```

Use the same callback when refreshing host functions in a preserved environment.

- [ ] **Step 4: Replace the Output pane with Terminal**

Change HTML IDs and labels to `terminal-tab` and `terminal`. In `main.ts`, import
`@xterm/xterm/css/xterm.css`, create `TerminalWindow`, keep `OutputWindow` only
for Problems, and use tool names `terminal | problems`.

On Run, create a shared buffer when `SharedArrayBuffer` exists and call:

```ts
controller.run(editor.getValue(), false, inputBuffer);
```

Route output chunks to `terminal.write`, input requests to
`terminal.requestInput`, reset to `terminal.reset`, and lifecycle completion or
replacement to `terminal.cancelInput`. Dispose xterm before unload.

- [ ] **Step 5: Add isolation headers and production documentation**

Use the same headers for Vite dev and preview:

```ts
const isolationHeaders = {
  "Cross-Origin-Opener-Policy": "same-origin",
  "Cross-Origin-Embedder-Policy": "require-corp",
};
```

Document that production hosting must serve these two headers for `input()`.

- [ ] **Step 6: Style the terminal pane**

Remove DOM-output row styles from the active terminal pane. Give `#terminal`
zero outer padding, a stable flex height, and let `.xterm` fill the panel. Keep
Problems scrollable and preserve the existing desktop/mobile split.

- [ ] **Step 7: Run complete frontend verification**

Run:

```sh
cd compile/calculator/web
bun test
bunx tsc --noEmit
bun run build
```

Expected: every Bun test passes, TypeScript exits zero, and Vite emits the
Terminal UI, Worker, Monaco, and WASM without errors.

- [ ] **Step 8: Browser smoke test**

Verify through ChromeDriver:

```calc
print("Name: ");
let name = input();
println("Hello, " + name);
println(string(float("3.5") + 1));
bool(0)
```

Enter `Leo`. Expected terminal text includes `Name: Leo`, `Hello, Leo`, `4.5`,
and final `false`. Also start `input()` and press Stop; the Worker terminates and
the Terminal prints `Execution stopped by user.`. Confirm the UI contains no
visible Output tab.

- [ ] **Step 9: Commit and push**

Commit the integration files with:

```sh
git commit -m "feat(calculator): Add interactive terminal"
git push
```

Do not add or modify files under `compile/calculator/src`, `compile/calculator/tests`,
or any other Rust source path.
