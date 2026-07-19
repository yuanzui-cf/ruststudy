import { describe, expect, test } from "bun:test";

import {
  COMPLETION_NAMES,
  FLOAT_LITERAL,
  KEYWORDS,
  WORD_OPERATORS,
} from "./language-data";

describe("calclang language data", () => {
  test("matches every EBNF float form", () => {
    for (const value of ["0", "12", "1.", ".5", "1.25", "1e3", ".5E-2"]) {
      expect(new RegExp(`^(?:${FLOAT_LITERAL.source})$`).test(value)).toBe(
        true,
      );
    }
  });

  test("rejects incomplete float forms", () => {
    for (const value of [".", "1e", "1e+", "1.2.3"]) {
      expect(new RegExp(`^(?:${FLOAT_LITERAL.source})$`).test(value)).toBe(
        false,
      );
    }
  });

  test("contains every grammar keyword and word operator", () => {
    expect(KEYWORDS).toEqual([
      "let",
      "if",
      "else",
      "loop",
      "break",
      "continue",
      "fn",
      "return",
      "true",
      "false",
      "none",
    ]);
    expect(WORD_OPERATORS).toEqual(["and", "or"]);
  });

  test("completes host and prelude names", () => {
    for (const name of [
      "println",
      "typeof",
      "PI",
      "sin",
      "random",
      "min",
      "max",
    ]) {
      expect(COMPLETION_NAMES).toContain(name);
    }
  });
});
