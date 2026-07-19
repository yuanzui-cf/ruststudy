import { expect, test } from "bun:test";

import { TerminalInputEditor } from "./terminal-input";

test("appends and echoes printable text", () => {
  const editor = new TerminalInputEditor();

  expect(editor.apply("hello 世界")).toEqual({ echo: "hello 世界" });
  expect(editor.value).toBe("hello 世界");
});

test("handles pasted printable text as one edit", () => {
  const editor = new TerminalInputEditor("ab");

  expect(editor.apply("cd ef")).toEqual({ echo: "cd ef" });
  expect(editor.value).toBe("abcd ef");
});

test("backspace erases one ASCII character", () => {
  const editor = new TerminalInputEditor("abc");

  expect(editor.apply("\x7f")).toEqual({ echo: "\b \b" });
  expect(editor.value).toBe("ab");
});

test("enter submits without echoing CRLF or clearing the line", () => {
  for (const enter of ["\r", "\n"]) {
    const editor = new TerminalInputEditor("ready");
    expect(editor.apply(enter)).toEqual({ echo: "", submitted: "ready" });
    expect(editor.value).toBe("ready");
  }
});

test("ignores C0 and ANSI control sequences", () => {
  const editor = new TerminalInputEditor("ab");

  expect(editor.apply("\x01\x1b[2D\x1b[31m")).toEqual({ echo: "" });
  expect(editor.value).toBe("ab");
});

test("ends a single-byte ESC sequence before printable input", () => {
  const editor = new TerminalInputEditor();

  expect(editor.apply("\x1b7X")).toEqual({ echo: "X" });
  expect(editor.value).toBe("X");
});

test("ignores OSC control strings terminated by BEL", () => {
  const editor = new TerminalInputEditor("ab");

  expect(editor.apply("\x1b]0;title\x07")).toEqual({ echo: "" });
  expect(editor.value).toBe("ab");
});

test("ignores DCS control strings terminated by ST", () => {
  const editor = new TerminalInputEditor("ab");

  expect(editor.apply("\x1bPpayload\x1b\\")).toEqual({ echo: "" });
  expect(editor.value).toBe("ab");
});

test("ends C1 control strings at an eight-bit ST", () => {
  const editor = new TerminalInputEditor("ab");

  expect(editor.apply("\u009dtitle\u009cZ")).toEqual({ echo: "Z" });
  expect(editor.value).toBe("abZ");
});

test("keeps CSI state across data chunks", () => {
  const editor = new TerminalInputEditor();

  expect(editor.apply("\x1b[")).toEqual({ echo: "" });
  expect(editor.apply("31mX")).toEqual({ echo: "X" });
  expect(editor.value).toBe("X");
});

test("keeps OSC state across data chunks", () => {
  const editor = new TerminalInputEditor();

  expect(editor.apply("\x1b]0;ti")).toEqual({ echo: "" });
  expect(editor.apply("tle\x07X")).toEqual({ echo: "X" });
  expect(editor.value).toBe("X");
});

test("keeps DCS state across data chunks", () => {
  const editor = new TerminalInputEditor();

  expect(editor.apply("\x1bPpay")).toEqual({ echo: "" });
  expect(editor.apply("load\x1b\\X")).toEqual({ echo: "X" });
  expect(editor.value).toBe("X");
});

test("keeps a split ST escape across data chunks", () => {
  const editor = new TerminalInputEditor();

  expect(editor.apply("\x1b]title\x1b")).toEqual({ echo: "" });
  expect(editor.apply("\\X")).toEqual({ echo: "X" });
  expect(editor.value).toBe("X");
});

test("processes backspace, text, and enter in one data event", () => {
  const editor = new TerminalInputEditor("cat");

  expect(editor.apply("\x7fdog\r")).toEqual({
    echo: "\b \bdog",
    submitted: "cadog",
  });
  expect(editor.value).toBe("cadog");
});

test("backspace erases a grapheme using its terminal cell width", () => {
  for (const [grapheme, width] of [
    ["界", 2],
    ["😀", 2],
    ["e\u0301", 1],
  ] as const) {
    const editor = new TerminalInputEditor(grapheme);
    expect(editor.apply("\x7f")).toEqual({
      echo: "\b \b".repeat(width),
    });
    expect(editor.value).toBe("");
  }
});
