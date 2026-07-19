import { expect, test } from "bun:test";

test("shell contains the approved controls and omits runtime internals", async () => {
  const html = await Bun.file(
    new URL("../../index.html", import.meta.url),
  ).text();
  for (const id of [
    "editor",
    "run-button",
    "stop-button",
    "output",
    "problems",
    "output-tab",
    "problems-tab",
    "cursor-position",
  ]) {
    expect(html).toContain(`id="${id}"`);
  }
  expect(html).not.toContain("Process running in isolated Web Worker");
  expect(html).not.toContain("10:00");
});
