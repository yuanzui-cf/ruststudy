import { expect, test } from "bun:test";

test("shell contains the approved controls and omits runtime internals", async () => {
  const html = await Bun.file(
    new URL("../../index.html", import.meta.url),
  ).text();
  for (const id of [
    "editor",
    "run-button",
    "stop-button",
    "terminal",
    "problems",
    "terminal-tab",
    "problems-tab",
    "cursor-position",
  ]) {
    expect(html).toContain(`id="${id}"`);
  }
  expect(html).toMatch(/>\s*Terminal\s*<\/button>/);
  expect(html).not.toContain('id="output"');
  expect(html).not.toContain('id="output-tab"');
  expect(html).not.toMatch(/>\s*Output\s*<\/button>/);
  expect(html).not.toContain("Process running in isolated Web Worker");
  expect(html).not.toContain("10:00");
});

test("Vite serves cross-origin isolation headers", async () => {
  const config = (await import("../../vite.config")).default as {
    server?: { headers?: Record<string, string> };
    preview?: { headers?: Record<string, string> };
  };
  const headers = {
    "Cross-Origin-Opener-Policy": "same-origin",
    "Cross-Origin-Embedder-Policy": "require-corp",
  };
  expect(config.server?.headers).toEqual(headers);
  expect(config.preview?.headers).toEqual(headers);
});

test("README documents production cross-origin isolation", async () => {
  const readme = await Bun.file(
    new URL("../../../../../README.md", import.meta.url),
  ).text();
  expect(readme).toContain("Cross-Origin-Opener-Policy: same-origin");
  expect(readme).toContain("Cross-Origin-Embedder-Policy: require-corp");
  expect(readme).toContain("input()");
});

test("terminal panel uses stable flex sizing without outer padding", async () => {
  const styles = await Bun.file(
    new URL("../styles.css", import.meta.url),
  ).text();
  expect(styles).toMatch(/#terminal\s*\{[^}]*padding:\s*0/s);
  expect(styles).toMatch(/#terminal\s*\{[^}]*flex:\s*1/s);
  expect(styles).toMatch(/\.xterm\s*\{[^}]*width:\s*100%[^}]*height:\s*100%/s);
  expect(styles).toMatch(/#problems\s*\{[^}]*overflow:\s*auto/s);
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
