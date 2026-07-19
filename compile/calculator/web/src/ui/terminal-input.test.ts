import { expect, test } from "bun:test";

import { applyTerminalInput } from "./terminal-input";

test("appends printable text and echoes it", () => {
  expect(applyTerminalInput("", "hello 世界")).toEqual({
    value: "hello 世界",
    echo: "hello 世界",
  });
});

test("handles pasted printable text as one edit", () => {
  expect(applyTerminalInput("ab", "cd ef")).toEqual({
    value: "abcd ef",
    echo: "cd ef",
  });
});

test("backspace erases one character and echoes terminal erase", () => {
  expect(applyTerminalInput("abc", "\x7f")).toEqual({
    value: "ab",
    echo: "\b \b",
  });
});

test("enter submits the current line with CRLF echo", () => {
  expect(applyTerminalInput("ready", "\r")).toEqual({
    value: "",
    echo: "\r\n",
    submitted: "ready",
  });
  expect(applyTerminalInput("ready", "\n")).toEqual({
    value: "",
    echo: "\r\n",
    submitted: "ready",
  });
});

test("ignores C0 and ANSI control sequences", () => {
  expect(applyTerminalInput("ab", "\x01\x1b[2D\x1b[31m")).toEqual({
    value: "ab",
    echo: "",
  });
});

test("ignores OSC control strings terminated by BEL", () => {
  expect(applyTerminalInput("ab", "\x1b]0;title\x07")).toEqual({
    value: "ab",
    echo: "",
  });
});

test("ignores DCS control strings terminated by ST", () => {
  expect(applyTerminalInput("ab", "\x1bPpayload\x1b\\")).toEqual({
    value: "ab",
    echo: "",
  });
});

test("ends C1 control strings at an eight-bit ST", () => {
  expect(applyTerminalInput("ab", "\u009dtitle\u009cZ")).toEqual({
    value: "abZ",
    echo: "Z",
  });
});

test("processes backspace, text, and enter in one data event", () => {
  expect(applyTerminalInput("cat", "\x7fdog\r")).toEqual({
    value: "",
    echo: "\b \bdog\r\n",
    submitted: "cadog",
  });
});

test("backspace removes a full Unicode code point", () => {
  expect(applyTerminalInput("A😀", "\x7f")).toEqual({
    value: "A",
    echo: "\b \b",
  });
});
