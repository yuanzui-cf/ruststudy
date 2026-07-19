import { expect, test } from "bun:test";

import { createInputReader, refreshHostFunctions } from "./input-bridge";

test("input reader rejects requests without cross-origin isolation", () => {
  const reader = createInputReader(
    { runId: 4 },
    {
      flush: () => {},
      post: () => {},
      waitForInput: () => "unused",
    },
  );

  expect(() => reader()).toThrow("input() requires cross-origin isolation");
});

test("input reader flushes before waiting and posts the exact input request", () => {
  const buffer = new SharedArrayBuffer(32);
  const events: string[] = [];
  const reader = createInputReader(
    { runId: 7, inputBuffer: buffer },
    {
      flush: () => events.push("flush"),
      post: (message) => {
        expect(message).toEqual({ type: "input", runId: 7, buffer });
        events.push("post");
      },
      waitForInput: (received, requestInput) => {
        expect(received).toBe(buffer);
        events.push("wait");
        requestInput();
        return "resolved";
      },
    },
  );

  expect(reader()).toBe("resolved");
  expect(events).toEqual(["flush", "wait", "post"]);
});

test("refreshed host bindings use each run's input reader", () => {
  const firstBuffer = new SharedArrayBuffer(32);
  const secondBuffer = new SharedArrayBuffer(32);
  const received: SharedArrayBuffer[] = [];
  const dependencies = {
    flush: () => {},
    post: () => {},
    waitForInput: (buffer: SharedArrayBuffer) => {
      received.push(buffer);
      return "line";
    },
  };

  const functions = new Map<string, (...values: unknown[]) => unknown>();
  const runtime = {
    define_host_function(
      name: string,
      callback: (...values: unknown[]) => unknown,
    ) {
      functions.set(name, callback);
    },
  };
  refreshHostFunctions(
    runtime,
    () => {},
    createInputReader({ runId: 1, inputBuffer: firstBuffer }, dependencies),
  );
  expect(functions.get("input")?.()).toBe("line");

  refreshHostFunctions(
    runtime,
    () => {},
    createInputReader({ runId: 2, inputBuffer: secondBuffer }, dependencies),
  );
  expect(functions.get("input")?.()).toBe("line");
  expect(received).toEqual([firstBuffer, secondBuffer]);
});
