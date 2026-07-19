import { expect, test } from "bun:test";

import {
  bindEditorDirtyIndicator,
  createRunControllerCallbacks,
  runEditor,
} from "./playground-integration";
import { RunController, type WorkerLike } from "../runtime/run-controller";
import type { WorkerResponse } from "../runtime/protocol";

class FakeWorker implements WorkerLike {
  onmessage: ((event: MessageEvent<WorkerResponse>) => void) | null = null;
  readonly messages: unknown[] = [];
  terminated = false;

  postMessage(message: unknown): void {
    this.messages.push(message);
  }

  terminate(): void {
    this.terminated = true;
  }

  emit(message: WorkerResponse): void {
    this.onmessage?.({ data: message } as MessageEvent<WorkerResponse>);
  }
}

test("controller callbacks route terminal output, input, reset, and lifecycle", () => {
  const workers: FakeWorker[] = [];
  const terminal = {
    resets: 0,
    writes: [] as string[][],
    inputs: [] as SharedArrayBuffer[],
    cancels: 0,
    reset() {
      this.resets += 1;
    },
    write(entries: readonly string[]) {
      this.writes.push([...entries]);
    },
    requestInput(buffer: SharedArrayBuffer) {
      this.inputs.push(buffer);
    },
    cancelInput() {
      this.cancels += 1;
    },
  };
  const problems = {
    clears: 0,
    entries: [] as string[],
    clear() {
      this.clears += 1;
    },
    append(entries: readonly string[]) {
      this.entries.push(...entries);
    },
  };
  const selected: string[] = [];
  const callbacks = createRunControllerCallbacks({
    terminal,
    problems,
    setProblemCount: () => {},
    selectTool: (tool) => selected.push(tool),
    setStopDisabled: () => {},
  });
  const controller = new RunController({
    createWorker: () => {
      const worker = new FakeWorker();
      workers.push(worker);
      return worker;
    },
    timeoutMs: 600_000,
    ...callbacks,
  });
  const firstBuffer = new SharedArrayBuffer(32);

  controller.run("input()", false, firstBuffer);
  expect(terminal.resets).toBe(1);
  expect(terminal.cancels).toBe(1);
  workers[0]!.emit({ type: "output", runId: 1, entries: ["prompt"] });
  workers[0]!.emit({ type: "input", runId: 1, buffer: firstBuffer });
  expect(terminal.writes).toEqual([["prompt"]]);
  expect(terminal.inputs).toEqual([firstBuffer]);

  controller.run("bad", false, new SharedArrayBuffer(32));
  expect(terminal.cancels).toBe(2);
  expect(terminal.resets).toBe(2);
  expect(problems.clears).toBe(2);
  workers[1]!.emit({
    type: "problem",
    runId: 2,
    category: "RuntimeError",
    message: "bad",
  });
  expect(problems.entries).toEqual(["RuntimeError: bad"]);
  expect(selected).toEqual(["terminal", "terminal", "problems"]);
  expect(terminal.cancels).toBe(3);

  controller.run("1 + 1", false, new SharedArrayBuffer(32));
  expect(terminal.cancels).toBe(4);
  workers[1]!.emit({ type: "complete", runId: 3, result: "2" });
  expect(terminal.cancels).toBe(5);
  expect(terminal.writes).toEqual([["prompt"], ["2\r\n"]]);
  controller.dispose();
});

test("runEditor creates a fresh shared buffer for each run", () => {
  const requests: unknown[] = [];
  const controller = {
    run: (...args: unknown[]) => requests.push(args),
  };
  const editor = { getValue: () => "input()" };
  const buffers = [new SharedArrayBuffer(32), new SharedArrayBuffer(32)];
  let index = 0;

  runEditor(controller, editor, () => buffers[index++]!);
  runEditor(controller, editor, () => buffers[index++]!);

  expect(requests).toEqual([
    ["input()", false, buffers[0]],
    ["input()", false, buffers[1]],
  ]);
  expect(buffers[0]).not.toBe(buffers[1]);
});

test("editor changes mark the document dirty without stopping execution", () => {
  let listener: (() => void) | undefined;
  let dirty = 0;
  let stopped = 0;
  const editor = {
    onDidChangeModelContent: (callback: () => void) => {
      listener = callback;
    },
    stop: () => stopped++,
  };
  bindEditorDirtyIndicator(editor, { markDirty: () => dirty++ });

  listener?.();
  expect(dirty).toBe(1);
  expect(stopped).toBe(0);
});
