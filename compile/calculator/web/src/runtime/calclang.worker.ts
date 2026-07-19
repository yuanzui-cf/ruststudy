/// <reference lib="webworker" />

import init, { WasmInterpreter } from "../generated/calclang/calclang";
import { HOST_CONSTANTS } from "../calclang/host";
import prelude from "../calclang/prelude.calc?raw";
import { createInputReader, refreshHostFunctions } from "./input-bridge";
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
    refreshHostFunctions(runtime, (line) => batcher.push(line), readLine);
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
  const readLine = createInputReader(request, {
    flush: () => batcher.flush(),
    post,
    waitForInput,
  });

  try {
    await wasmReady;
    if (!request.preserveEnvironment || interpreter === undefined) {
      interpreter?.free();
      interpreter = createInterpreter(batcher, readLine);
    } else {
      refreshHostFunctions(interpreter, (line) => batcher.push(line), readLine);
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
