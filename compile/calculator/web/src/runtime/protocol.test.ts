import { expect, test } from "bun:test";

import { isWorkerResponse } from "./protocol";

test("accepts complete typed worker responses", () => {
  expect(
    isWorkerResponse({ type: "output", runId: 1, entries: ["hello"] }),
  ).toBe(true);
  expect(isWorkerResponse({ type: "complete", runId: 1, result: "42" })).toBe(
    true,
  );
  expect(
    isWorkerResponse({
      type: "problem",
      runId: 1,
      category: "TypeError",
      message: "bad value",
    }),
  ).toBe(true);
  expect(
    isWorkerResponse({
      type: "input",
      runId: 4,
      buffer: new SharedArrayBuffer(16),
    }),
  ).toBe(true);
});

test("rejects malformed worker responses", () => {
  expect(isWorkerResponse(null)).toBe(false);
  expect(isWorkerResponse({ type: "output", runId: "one", entries: [] })).toBe(
    false,
  );
  expect(isWorkerResponse({ type: "complete" })).toBe(false);
  expect(isWorkerResponse({ type: "unknown", runId: 1 })).toBe(false);
  expect(
    isWorkerResponse({
      type: "input",
      runId: 4,
      buffer: new ArrayBuffer(16),
    }),
  ).toBe(false);
});
