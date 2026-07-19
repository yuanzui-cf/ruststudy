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

test("shell uses only the Calclang product name", async () => {
  const html = await Bun.file(
    new URL("../../index.html", import.meta.url),
  ).text();
  expect(html).toContain("Calclang Playground");
  expect(html).not.toContain("Microsoft");
  expect(html).not.toContain("Visual Studio");
});

test("main loads Monaco's complete editor contributions", async () => {
  const main = await Bun.file(new URL("../main.ts", import.meta.url)).text();
  expect(main).toContain('from "monaco-editor";');
  expect(main).not.toContain("editor.api.js");
});
