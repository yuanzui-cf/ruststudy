import { expect, test } from "bun:test";
import { Window } from "happy-dom";

import { OutputWindow } from "./output-window";

test("keeps the newest ten thousand DOM entries", () => {
  const window = new Window();
  Object.defineProperty(globalThis, "document", {
    configurable: true,
    value: window.document,
  });
  const container = window.document.createElement("div");
  const output = new OutputWindow(container as unknown as HTMLElement);

  output.append(
    Array.from({ length: 10_005 }, (_, index) => `line-${index}`),
  );

  expect(container.childElementCount).toBe(10_000);
  expect(container.firstElementChild?.textContent).toBe("line-5");
  expect(container.lastElementChild?.textContent).toBe("line-10004");
});

test("clear removes every output entry", () => {
  const window = new Window();
  Object.defineProperty(globalThis, "document", {
    configurable: true,
    value: window.document,
  });
  const container = window.document.createElement("div");
  const output = new OutputWindow(container as unknown as HTMLElement);
  output.append(["one", "two"]);
  output.clear();
  expect(container.childElementCount).toBe(0);
});
