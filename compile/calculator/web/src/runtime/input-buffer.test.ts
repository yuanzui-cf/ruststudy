import { expect, test } from "bun:test";

import {
  INPUT_IDLE,
  INPUT_LENGTH_INDEX,
  INPUT_READY,
  INPUT_RESOLVING,
  INPUT_STATE_INDEX,
  INPUT_WAITING,
  createInputBuffer,
  readResolvedInput,
  resolveInput,
  waitForInput,
} from "./input-buffer";

test("resolves UTF-8 input and resets the buffer after reading", () => {
  const buffer = createInputBuffer(32);
  const header = new Int32Array(buffer, 0, 2);
  Atomics.store(header, INPUT_STATE_INDEX, INPUT_WAITING);

  expect(resolveInput(buffer, "你好")).toBe(true);
  expect(Atomics.load(header, INPUT_STATE_INDEX)).toBe(INPUT_READY);
  expect(Atomics.load(header, INPUT_LENGTH_INDEX)).toBe(6);
  expect(readResolvedInput(buffer)).toBe("你好");
  expect(Atomics.load(header, INPUT_STATE_INDEX)).toBe(INPUT_IDLE);
  expect(Atomics.load(header, INPUT_LENGTH_INDEX)).toBe(0);
});

test("copies resolved input out of shared memory before decoding", () => {
  const originalDecode = TextDecoder.prototype.decode;
  TextDecoder.prototype.decode = function (...args) {
    const [input] = args;
    if (
      input !== undefined &&
      ArrayBuffer.isView(input) &&
      input.buffer instanceof SharedArrayBuffer
    ) {
      throw new TypeError(
        "The provided ArrayBufferView value must not be shared",
      );
    }
    return originalDecode.apply(this, args);
  };

  try {
    const buffer = createInputBuffer(32);
    const header = new Int32Array(buffer, 0, 2);
    Atomics.store(header, INPUT_STATE_INDEX, INPUT_WAITING);

    expect(resolveInput(buffer, "Chrome 你好")).toBe(true);
    expect(readResolvedInput(buffer)).toBe("Chrome 你好");
  } finally {
    TextDecoder.prototype.decode = originalDecode;
  }
});

test("accepts input at the exact UTF-8 byte boundary and empty input", () => {
  const buffer = createInputBuffer(6);
  const header = new Int32Array(buffer, 0, 2);
  Atomics.store(header, INPUT_STATE_INDEX, INPUT_WAITING);

  expect(resolveInput(buffer, "你好")).toBe(true);
  expect(Atomics.load(header, INPUT_LENGTH_INDEX)).toBe(6);
  expect(readResolvedInput(buffer)).toBe("你好");

  Atomics.store(header, INPUT_STATE_INDEX, INPUT_WAITING);
  expect(resolveInput(buffer, "")).toBe(true);
  expect(Atomics.load(header, INPUT_LENGTH_INDEX)).toBe(0);
  expect(readResolvedInput(buffer)).toBe("");
});

test("rejects input whose UTF-8 bytes exceed buffer capacity", () => {
  const buffer = createInputBuffer(2);
  const header = new Int32Array(buffer, 0, 2);
  Atomics.store(header, INPUT_LENGTH_INDEX, 2);
  Atomics.store(header, INPUT_STATE_INDEX, INPUT_WAITING);

  expect(() => resolveInput(buffer, "abc")).toThrow(
    "Input exceeds terminal buffer capacity",
  );
  expect(Atomics.load(header, INPUT_STATE_INDEX)).toBe(INPUT_WAITING);
  expect(Atomics.load(header, INPUT_LENGTH_INDEX)).toBe(0);
});

test("does not resolve input unless the buffer is waiting", () => {
  const buffer = createInputBuffer(32);

  expect(resolveInput(buffer, "ignored")).toBe(false);
});

test("does not resolve input after another resolver claims the wait", () => {
  const buffer = createInputBuffer(32);
  const header = new Int32Array(buffer, 0, 2);
  Atomics.store(header, INPUT_STATE_INDEX, INPUT_RESOLVING);

  expect(resolveInput(buffer, "stale")).toBe(false);
});

test("requires a positive integer capacity", () => {
  expect(() => createInputBuffer(0)).toThrow(
    "Terminal input buffer capacity must be a positive integer",
  );
  expect(() => createInputBuffer(1.5)).toThrow(
    "Terminal input buffer capacity must be a positive integer",
  );
  expect(() => createInputBuffer(0x80000000)).toThrow(
    "Terminal input buffer capacity must be a positive integer",
  );
});

test("rejects a buffer without payload capacity", () => {
  expect(() => resolveInput(new SharedArrayBuffer(8), "input")).toThrow(
    "Invalid terminal input buffer layout",
  );
});

test("rejects reads before input is ready", () => {
  expect(() => readResolvedInput(createInputBuffer(32))).toThrow(
    "Terminal input buffer is not ready",
  );
});

test("cleans up an invalid resolved length", () => {
  const buffer = createInputBuffer(4);
  const header = new Int32Array(buffer, 0, 2);
  Atomics.store(header, INPUT_LENGTH_INDEX, 5);
  Atomics.store(header, INPUT_STATE_INDEX, INPUT_READY);

  expect(() => readResolvedInput(buffer)).toThrow(
    "Invalid terminal input length",
  );
  expect(Atomics.load(header, INPUT_STATE_INDEX)).toBe(INPUT_IDLE);
  expect(Atomics.load(header, INPUT_LENGTH_INDEX)).toBe(0);
});

test("cleans up malformed UTF-8 after a decoder error", () => {
  const buffer = createInputBuffer(1);
  const header = new Int32Array(buffer, 0, 2);
  new Uint8Array(buffer, 8).set([0xff]);
  Atomics.store(header, INPUT_LENGTH_INDEX, 1);
  Atomics.store(header, INPUT_STATE_INDEX, INPUT_READY);

  expect(() => readResolvedInput(buffer)).toThrow();
  expect(Atomics.load(header, INPUT_STATE_INDEX)).toBe(INPUT_IDLE);
  expect(Atomics.load(header, INPUT_LENGTH_INDEX)).toBe(0);
});

test("cleans up when the input request callback throws", () => {
  const buffer = createInputBuffer(32);
  const header = new Int32Array(buffer, 0, 2);

  expect(() =>
    waitForInput(buffer, () => {
      throw new Error("request failed");
    }),
  ).toThrow("request failed");
  expect(Atomics.load(header, INPUT_STATE_INDEX)).toBe(INPUT_IDLE);
  expect(Atomics.load(header, INPUT_LENGTH_INDEX)).toBe(0);
});

test("requests input before waiting and returns the resolved value", () => {
  const buffer = createInputBuffer(32);

  expect(
    waitForInput(buffer, () => {
      expect(resolveInput(buffer, "ready")).toBe(true);
    }),
  ).toBe("ready");
});

test("waits in a Worker and wakes after the main thread resolves input", async () => {
  const moduleUrl = new URL("./input-buffer.ts", import.meta.url).href;
  const source = `
    import { waitForInput } from ${JSON.stringify(moduleUrl)};
    self.onmessage = (event) => {
      const value = waitForInput(event.data, () => self.postMessage({ type: "request" }));
      self.postMessage({ type: "result", value });
    };
  `;
  const url = URL.createObjectURL(new Blob([source], { type: "text/javascript" }));
  const worker = new Worker(url);
  const buffer = createInputBuffer(32);

  try {
    const result = new Promise<string>((resolve, reject) => {
      worker.onmessage = (event) => {
        if (event.data.type === "request") {
          expect(resolveInput(buffer, "from worker")).toBe(true);
        } else {
          resolve(event.data.value);
        }
      };
      worker.onerror = (event) => reject(event.error ?? new Error(event.message));
    });
    worker.postMessage(buffer);
    expect(await result).toBe("from worker");
  } finally {
    worker.terminate();
    URL.revokeObjectURL(url);
  }
});

test("only one concurrent Worker resolver wins the CAS", async () => {
  const moduleUrl = new URL("./input-buffer.ts", import.meta.url).href;
  const source = `
    import { resolveInput } from ${JSON.stringify(moduleUrl)};
    self.onmessage = (event) => self.postMessage(resolveInput(event.data.buffer, event.data.input));
  `;
  const url = URL.createObjectURL(new Blob([source], { type: "text/javascript" }));
  const first = new Worker(url);
  const second = new Worker(url);
  const buffer = createInputBuffer(32);
  const header = new Int32Array(buffer, 0, 2);
  Atomics.store(header, INPUT_STATE_INDEX, INPUT_WAITING);

  try {
    const resolveWorker = (worker: Worker, input: string) =>
      new Promise<boolean>((resolve, reject) => {
        worker.onmessage = (event) => resolve(event.data);
        worker.onerror = (event) => reject(event.error ?? new Error(event.message));
        worker.postMessage({ buffer, input });
      });
    const results = await Promise.all([
      resolveWorker(first, "first"),
      resolveWorker(second, "second"),
    ]);

    expect(results.sort()).toEqual([false, true]);
    expect(["first", "second"]).toContain(readResolvedInput(buffer));
  } finally {
    first.terminate();
    second.terminate();
    URL.revokeObjectURL(url);
  }
});
