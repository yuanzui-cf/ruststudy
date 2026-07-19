import { describe, expect, test } from "bun:test";

import { HOST_CONSTANTS, createHostFunctions } from "./host";

describe("host environment", () => {
  test("delegates numeric functions to Math", () => {
    const functions = createHostFunctions(() => {});
    expect(functions.sin(Math.PI / 2)).toBe(1);
    expect(functions.sqrt(9)).toBe(3);
  });

  test("println concatenates with calclang display semantics", () => {
    const lines: string[] = [];
    const functions = createHostFunctions((line) => lines.push(line));
    functions.println("answer=", 42, undefined);
    expect(lines).toEqual(["answer=42"]);
  });

  test("typeof understands primitives and function descriptors", () => {
    const functions = createHostFunctions(() => {});
    expect(functions.typeof(1)).toBe("float");
    expect(functions.typeof(undefined)).toBe("none");
    expect(functions.typeof({ __calclangType: "fn(x)" })).toBe("fn(x)");
  });

  test("typeof enforces one argument", () => {
    const functions = createHostFunctions(() => {});
    expect(() => functions.typeof()).toThrow(
      "Expect one arg in typeof, found 0",
    );
  });

  test("exports all requested Math constants", () => {
    expect(HOST_CONSTANTS).toEqual({
      PI: Math.PI,
      E: Math.E,
      SQRT2: Math.SQRT2,
      SQRT1_2: Math.SQRT1_2,
      LN2: Math.LN2,
      LN10: Math.LN10,
      LOG2E: Math.LOG2E,
      LOG10E: Math.LOG10E,
    });
  });
});
