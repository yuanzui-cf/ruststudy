import { expect, test } from "bun:test";

import { TerminalWindow } from "./terminal-window";

class FakeTerminal {
  readonly writes: string[] = [];
  focusCount = 0;
  disposed = false;
  subscriptionDisposed = false;
  private listener?: (data: string) => void;

  open(_container: HTMLElement): void {}

  onData(listener: (data: string) => void): { dispose(): void } {
    this.listener = listener;
    return {
      dispose: () => {
        this.subscriptionDisposed = true;
        this.listener = undefined;
      },
    };
  }

  write(data: string): void {
    this.writes.push(data);
  }

  focus(): void {
    this.focusCount += 1;
  }

  dispose(): void {
    this.disposed = true;
  }

  emit(data: string): void {
    this.listener?.(data);
  }
}

class FakeResizeObserver {
  observed?: Element;
  disconnected = false;

  constructor(readonly callback: ResizeObserverCallback) {}

  observe(target: Element): void {
    this.observed = target;
  }

  disconnect(): void {
    this.disconnected = true;
  }
}

function setup(
  resolver: (buffer: SharedArrayBuffer, input: string) => boolean,
): {
  window: TerminalWindow;
  terminal: FakeTerminal;
  observer: FakeResizeObserver;
  fitCalls: () => number;
} {
  const terminal = new FakeTerminal();
  let fitCount = 0;
  let observer: FakeResizeObserver | undefined;
  const container = {} as HTMLElement;
  const window = new TerminalWindow(container, {
    terminal,
    fitAddon: { fit: () => (fitCount += 1) },
    createResizeObserver: (callback) => {
      observer = new FakeResizeObserver(callback);
      return observer;
    },
    resolveInput: resolver,
  });
  if (observer === undefined) {
    throw new Error("TerminalWindow did not create a ResizeObserver");
  }
  expect(observer.observed).toBe(container);
  return { window, terminal, observer, fitCalls: () => fitCount };
}

test("writes CRLF only after input resolves successfully", () => {
  const calls: Array<[SharedArrayBuffer, string]> = [];
  const state = setup((buffer, input) => {
    calls.push([buffer, input]);
    return true;
  });
  const buffer = new SharedArrayBuffer(16);

  state.window.requestInput(buffer);
  state.terminal.emit("abc\r");

  expect(calls).toEqual([[buffer, "abc"]]);
  expect(state.terminal.writes).toEqual(["abc", "\r\n"]);
  state.terminal.emit("ignored");
  expect(state.terminal.writes).toEqual(["abc", "\r\n"]);
});

test("bells without a newline and retains input when resolve returns false", () => {
  const inputs: string[] = [];
  const state = setup((_buffer, input) => {
    inputs.push(input);
    return false;
  });

  state.window.requestInput(new SharedArrayBuffer(16));
  state.terminal.emit("abc\r");
  state.terminal.emit("\x7f\r");

  expect(inputs).toEqual(["abc", "ab"]);
  expect(state.terminal.writes).toEqual([
    "abc",
    "\x07",
    "\b \b",
    "\x07",
  ]);
});

test("bells without throwing or losing input when resolve throws", () => {
  const inputs: string[] = [];
  const state = setup((_buffer, input) => {
    inputs.push(input);
    throw new Error("invalid internal buffer");
  });

  state.window.requestInput(new SharedArrayBuffer(16));
  expect(() => state.terminal.emit("abc\r")).not.toThrow();
  expect(() => state.terminal.emit("\x7f\r")).not.toThrow();

  expect(inputs).toEqual(["abc", "ab"]);
  expect(state.terminal.writes).toEqual([
    "abc",
    "\x07",
    "\b \b",
    "\x07",
  ]);
});

test("queues reset between existing and subsequent writes", () => {
  const state = setup(() => true);
  state.window.requestInput(new SharedArrayBuffer(16));

  state.window.write(["old-1", "old-2"]);
  state.window.reset();
  state.window.write(["new"]);
  state.terminal.emit("ignored");

  expect(state.terminal.writes).toEqual(["old-1", "old-2", "\x1bc", "new"]);
});

test("replaces an old request with a fresh editor", () => {
  const calls: Array<[SharedArrayBuffer, string]> = [];
  const state = setup((buffer, input) => {
    calls.push([buffer, input]);
    return true;
  });
  const oldBuffer = new SharedArrayBuffer(16);
  const newBuffer = new SharedArrayBuffer(16);

  state.window.requestInput(oldBuffer);
  state.terminal.emit("old");
  state.window.requestInput(newBuffer);
  state.terminal.emit("new\r");

  expect(calls).toEqual([[newBuffer, "new"]]);
});

test("cancel and dispose detach pending input and owned resources", () => {
  const state = setup(() => true);
  state.window.requestInput(new SharedArrayBuffer(16));
  expect(state.terminal.focusCount).toBe(1);
  expect(state.fitCalls()).toBe(1);

  state.window.cancelInput();
  state.terminal.emit("ignored");
  expect(state.terminal.writes).toEqual([]);

  state.window.dispose();
  expect(state.observer.disconnected).toBe(true);
  expect(state.terminal.subscriptionDisposed).toBe(true);
  expect(state.terminal.disposed).toBe(true);
});
