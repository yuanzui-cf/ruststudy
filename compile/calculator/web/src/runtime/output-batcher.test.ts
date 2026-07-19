import { expect, test } from "bun:test";

import { OutputBatcher } from "./output-batcher";

test("posts every one hundred entries in order", () => {
  const batches: string[][] = [];
  const batcher = new OutputBatcher((entries) => batches.push(entries));

  for (let index = 0; index < 205; index += 1) {
    batcher.push(String(index));
  }
  batcher.flush();

  expect(batches.map((batch) => batch.length)).toEqual([100, 100, 5]);
  expect(batches.flat()).toEqual(
    Array.from({ length: 205 }, (_, index) => String(index)),
  );
});

test("does not emit an empty batch", () => {
  const batches: string[][] = [];
  const batcher = new OutputBatcher((entries) => batches.push(entries));
  batcher.flush();
  expect(batches).toEqual([]);
});
