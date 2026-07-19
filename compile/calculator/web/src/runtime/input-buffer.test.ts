import { expect, test } from "bun:test";

import {
  INPUT_IDLE,
  INPUT_LENGTH_INDEX,
  INPUT_READY,
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

test("rejects input whose UTF-8 bytes exceed buffer capacity", () => {
  const buffer = createInputBuffer(2);
  const header = new Int32Array(buffer, 0, 2);
  Atomics.store(header, INPUT_STATE_INDEX, INPUT_WAITING);

  expect(() => resolveInput(buffer, "abc")).toThrow(
    "Input exceeds terminal buffer capacity",
  );
});

test("does not resolve input unless the buffer is waiting", () => {
  const buffer = createInputBuffer(32);

  expect(resolveInput(buffer, "ignored")).toBe(false);
});

test("requires a positive integer capacity", () => {
  expect(() => createInputBuffer(0)).toThrow(
    "Terminal input buffer capacity must be a positive integer",
  );
  expect(() => createInputBuffer(1.5)).toThrow(
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

test("requests input before waiting and returns the resolved value", () => {
  const buffer = createInputBuffer(32);

  expect(
    waitForInput(buffer, () => {
      expect(resolveInput(buffer, "ready")).toBe(true);
    }),
  ).toBe("ready");
});
