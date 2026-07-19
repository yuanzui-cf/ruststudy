/// <reference lib="webworker" />

import init, { WasmInterpreter } from "../generated/calclang/calclang";
import { HOST_CONSTANTS, createHostFunctions } from "../calclang/host";
import prelude from "../calclang/prelude.calc?raw";
import { waitForInput } from "./input-buffer";
import { OutputBatcher } from "./output-batcher";
import type { RunRequest, WorkerResponse } from "./protocol";

const scope = self as DedicatedWorkerGlobalScope;
const wasmReady = init();
let interpreter: WasmInterpreter | undefined;

function post(message: WorkerResponse): void {
  scope.postMessage(message);
}

function createInterpreter(
  batcher: OutputBatcher,
  readLine: () => string,
): WasmInterpreter {
  const runtime = new WasmInterpreter();
  try {
    for (const [name, value] of Object.entries(HOST_CONSTANTS)) {
      runtime.define_value(name, value);
    }
    for (const [name, callback] of Object.entries(
      createHostFunctions((line) => batcher.push(line), readLine),
    )) {
      runtime.define_host_function(name, callback);
    }
    runtime.evaluate(prelude);
    return runtime;
  } catch (error) {
    runtime.free();
    throw error;
  }
}

scope.onmessage = async (event: MessageEvent<RunRequest>) => {
  const request = event.data;
  if (request.type !== "run") {
    return;
  }

  const batcher = new OutputBatcher((entries) => {
    post({ type: "output", runId: request.runId, entries });
  });
  const readLine = (): string => {
    if (request.inputBuffer === undefined) {
      throw new Error("input() requires cross-origin isolation");
    }
    batcher.flush();
    return waitForInput(request.inputBuffer, () =>
      post({ type: "input", runId: request.runId, buffer: request.inputBuffer! }),
    );
  };

  try {
    await wasmReady;
    if (!request.preserveEnvironment || interpreter === undefined) {
      interpreter?.free();
      interpreter = createInterpreter(batcher, readLine);
    } else {
      for (const [name, callback] of Object.entries(
        createHostFunctions((line) => batcher.push(line), readLine),
      )) {
        interpreter.define_host_function(name, callback);
      }
    }

    const result: unknown = interpreter.evaluate(request.source);
    batcher.flush();
    const resultText =
      result === undefined || typeof result === "object"
        ? undefined
        : String(result);
    post({
      type: "complete",
      runId: request.runId,
      result: resultText,
    });
  } catch (value) {
    batcher.flush();
    const error = isErrorPayload(value) ? value : undefined;
    post({
      type: "problem",
      runId: request.runId,
      category: error?.category ?? "RuntimeError",
      message: error?.message ?? "Unknown calclang runtime failure",
    });
  }
};

function isErrorPayload(
  value: unknown,
): value is { category: string; message: string } {
  return (
    typeof value === "object" &&
    value !== null &&
    "category" in value &&
    typeof value.category === "string" &&
    "message" in value &&
    typeof value.message === "string"
  );
}
