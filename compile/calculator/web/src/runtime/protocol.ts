export interface RunRequest {
  type: "run";
  runId: number;
  source: string;
  preserveEnvironment: boolean;
  inputBuffer?: SharedArrayBuffer;
}

export type WorkerRequest = RunRequest;

export type WorkerResponse =
  | { type: "output"; runId: number; entries: string[] }
  | { type: "input"; runId: number; buffer: SharedArrayBuffer }
  | { type: "complete"; runId: number; result?: string }
  | {
      type: "problem";
      runId: number;
      category: string;
      message: string;
    };

export function isWorkerResponse(value: unknown): value is WorkerResponse {
  if (!isRecord(value) || typeof value.runId !== "number") {
    return false;
  }

  switch (value.type) {
    case "output":
      return (
        Array.isArray(value.entries) &&
        value.entries.every((entry) => typeof entry === "string")
      );
    case "input":
      return isSharedArrayBuffer(value.buffer);
    case "complete":
      return value.result === undefined || typeof value.result === "string";
    case "problem":
      return (
        typeof value.category === "string" && typeof value.message === "string"
      );
    default:
      return false;
  }
}

function isRecord(value: unknown): value is Record<PropertyKey, unknown> {
  return typeof value === "object" && value !== null;
}

function isSharedArrayBuffer(value: unknown): value is SharedArrayBuffer {
  return (
    typeof SharedArrayBuffer !== "undefined" &&
    value instanceof SharedArrayBuffer
  );
}
