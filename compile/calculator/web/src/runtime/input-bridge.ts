import { createHostFunctions } from "../calclang/host";
import type { RunRequest, WorkerResponse } from "./protocol";

export interface InputBridgeDependencies {
  flush: () => void;
  post: (message: WorkerResponse) => void;
  waitForInput: (
    buffer: SharedArrayBuffer,
    requestInput: () => void,
  ) => string;
}

export interface HostFunctionRuntime {
  define_host_function(
    name: string,
    callback: (...values: unknown[]) => unknown,
  ): void;
}

export function createInputReader(
  request: Pick<RunRequest, "runId" | "inputBuffer">,
  dependencies: InputBridgeDependencies,
): () => string {
  return () => {
    const buffer = request.inputBuffer;
    if (buffer === undefined) {
      throw new Error("input() requires cross-origin isolation");
    }
    dependencies.flush();
    return dependencies.waitForInput(buffer, () =>
      dependencies.post({ type: "input", runId: request.runId, buffer }),
    );
  };
}

export function refreshHostFunctions(
  runtime: HostFunctionRuntime,
  emit: (chunk: string) => void,
  readLine: () => string,
): void {
  for (const [name, callback] of Object.entries(
    createHostFunctions(emit, readLine),
  )) {
    runtime.define_host_function(
      name,
      callback as (...values: unknown[]) => unknown,
    );
  }
}
