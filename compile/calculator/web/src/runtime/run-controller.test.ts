import { describe, expect, test } from "bun:test";

import { RunController, type WorkerLike } from "./run-controller";
import type { WorkerResponse } from "./protocol";

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

function setup(timeoutMs = 600_000, withInputHandler = true) {
  const workers: FakeWorker[] = [];
  const output: string[] = [];
  const problems: string[] = [];
  const running: boolean[] = [];
  const requestedInputs: SharedArrayBuffer[] = [];
  const controller = new RunController({
    createWorker: () => {
      const worker = new FakeWorker();
      workers.push(worker);
      return worker;
    },
    timeoutMs,
    onReset: () => {
      output.length = 0;
      problems.length = 0;
    },
    onOutput: (entries) => output.push(...entries),
    onProblem: (category, message) =>
      problems.push(`${category}: ${message}`),
    ...(withInputHandler
      ? { onInput: (buffer: SharedArrayBuffer) => requestedInputs.push(buffer) }
      : {}),
    onRunningChange: (value) => running.push(value),
  });
  return { controller, workers, output, problems, requestedInputs, running };
}

describe("RunController", () => {
  test("a second run replaces an active worker", () => {
    const state = setup();
    state.controller.run("loop {}", false);
    state.controller.run("1 + 1", false);
    expect(state.workers[0]?.terminated).toBe(true);
    expect(state.workers).toHaveLength(2);
    state.controller.dispose();
  });

  test("manual stop terminates and reports to Output", () => {
    const state = setup();
    state.controller.run("loop {}", false);
    state.controller.stop();
    expect(state.workers[0]?.terminated).toBe(true);
    expect(state.output).toEqual(["Execution stopped by user.\r\n"]);
  });

  test("timeout terminates and reports the required RuntimeError", async () => {
    const state = setup(5);
    state.controller.run("loop {}", false);
    await Bun.sleep(15);
    expect(state.workers[0]?.terminated).toBe(true);
    expect(state.output).toEqual([
      "RuntimeError: Loop execution exceeded the 10-minute time limit.\r\n",
    ]);
  });

  test("forwards an input buffer in run requests", () => {
    const state = setup();
    const inputBuffer = new SharedArrayBuffer(16);
    state.controller.run("input()", false, inputBuffer);
    expect(state.workers[0]?.messages[0]).toEqual({
      type: "run",
      runId: 1,
      source: "input()",
      preserveEnvironment: false,
      inputBuffer,
    });
    state.controller.dispose();
  });

  test("routes current input requests without ending the run", async () => {
    const state = setup(5);
    state.controller.run("input()", false, new SharedArrayBuffer(16));
    const worker = state.workers[0]!;
    const inputBuffer = new SharedArrayBuffer(16);
    worker.emit({ type: "input", runId: 1, buffer: inputBuffer });
    expect(state.requestedInputs).toEqual([inputBuffer]);
    expect(state.running).toEqual([true]);
    expect(worker.terminated).toBe(false);
    await Bun.sleep(15);
    expect(worker.terminated).toBe(true);
    expect(state.running).toEqual([true, false]);
  });

  test("fails fast when no terminal input handler is available", () => {
    const state = setup(600_000, false);
    state.controller.run("input()", false, new SharedArrayBuffer(16));
    const worker = state.workers[0]!;
    worker.emit({
      type: "input",
      runId: 1,
      buffer: new SharedArrayBuffer(16),
    });
    expect(worker.terminated).toBe(true);
    expect(state.problems).toEqual([
      "RuntimeError: Terminal input handler is unavailable",
    ]);
    expect(state.running).toEqual([true, false]);
  });

  test("ignores input requests that arrive after stop", () => {
    const state = setup();
    state.controller.run("input()", false, new SharedArrayBuffer(16));
    const worker = state.workers[0]!;
    state.controller.stop();
    worker.emit({
      type: "input",
      runId: 1,
      buffer: new SharedArrayBuffer(16),
    });
    expect(state.requestedInputs).toEqual([]);
    expect(state.running).toEqual([true, false]);
  });

  test("ignores stale input requests", () => {
    const state = setup();
    state.controller.run("input()", false, new SharedArrayBuffer(16));
    const first = state.workers[0]!;
    state.controller.run("input()", false, new SharedArrayBuffer(16));
    first.emit({
      type: "input",
      runId: 1,
      buffer: new SharedArrayBuffer(16),
    });
    expect(state.requestedInputs).toEqual([]);
    state.controller.dispose();
  });

  test("ignores stale run messages", () => {
    const state = setup();
    state.controller.run("1", false);
    const first = state.workers[0]!;
    state.controller.run("2", false);
    first.emit({ type: "output", runId: 1, entries: ["stale"] });
    expect(state.output).toEqual([]);
    state.controller.dispose();
  });

  test("reuses an idle worker and forwards preserveEnvironment", () => {
    const state = setup();
    state.controller.run("let x = 1;", false);
    const worker = state.workers[0]!;
    worker.emit({ type: "complete", runId: 1 });

    state.controller.run("x", true);

    expect(state.workers).toHaveLength(1);
    expect(worker.messages[1]).toEqual({
      type: "run",
      runId: 2,
      source: "x",
      preserveEnvironment: true,
    });
    state.controller.dispose();
  });

  test("routes problems and final results", () => {
    const state = setup();
    state.controller.run("bad", false);
    const worker = state.workers[0]!;
    worker.emit({
      type: "problem",
      runId: 1,
      category: "NameError",
      message: "missing",
    });
    expect(state.problems).toEqual(["NameError: missing"]);

    state.controller.run("40 + 2", false);
    worker.emit({ type: "complete", runId: 2, result: "42" });
    expect(state.output).toEqual(["42\r\n"]);
    state.controller.dispose();
  });
});
