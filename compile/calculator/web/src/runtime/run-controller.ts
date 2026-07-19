import { isWorkerResponse, type RunRequest, type WorkerResponse } from "./protocol";

export interface WorkerLike {
  onmessage: ((event: MessageEvent<WorkerResponse>) => void) | null;
  postMessage(message: unknown): void;
  terminate(): void;
}

export interface RunControllerOptions {
  createWorker: () => WorkerLike;
  timeoutMs?: number;
  onReset: () => void;
  onOutput: (entries: string[]) => void;
  onProblem: (category: string, message: string) => void;
  onRunningChange: (running: boolean) => void;
}

const TIMEOUT_MESSAGE =
  "RuntimeError: Loop execution exceeded the 10-minute time limit.";

export class RunController {
  readonly #options: Required<RunControllerOptions>;
  #worker: WorkerLike | undefined;
  #activeRunId: number | undefined;
  #nextRunId = 1;
  #timer: ReturnType<typeof setTimeout> | undefined;

  constructor(options: RunControllerOptions) {
    this.#options = {
      ...options,
      timeoutMs: options.timeoutMs ?? 600_000,
    };
  }

  run(source: string, preserveEnvironment = false): void {
    if (this.#activeRunId !== undefined) {
      this.#discardWorker();
    }
    this.#options.onReset();

    const worker = this.#worker ?? this.#createWorker();
    const runId = this.#nextRunId++;
    this.#activeRunId = runId;
    this.#options.onRunningChange(true);
    this.#timer = setTimeout(() => {
      if (this.#activeRunId !== runId) {
        return;
      }
      this.#discardWorker();
      this.#options.onOutput([TIMEOUT_MESSAGE]);
      this.#options.onRunningChange(false);
    }, this.#options.timeoutMs);

    const request: RunRequest = {
      type: "run",
      runId,
      source,
      preserveEnvironment,
    };
    worker.postMessage(request);
  }

  stop(): void {
    if (this.#activeRunId === undefined) {
      return;
    }
    this.#discardWorker();
    this.#options.onOutput(["Execution stopped by user."]);
    this.#options.onRunningChange(false);
  }

  dispose(): void {
    this.#discardWorker();
  }

  #createWorker(): WorkerLike {
    const worker = this.#options.createWorker();
    worker.onmessage = (event) => this.#handleMessage(event.data);
    this.#worker = worker;
    return worker;
  }

  #handleMessage(value: unknown): void {
    if (!isWorkerResponse(value) || value.runId !== this.#activeRunId) {
      return;
    }
    if (value.type === "output") {
      this.#options.onOutput(value.entries);
      return;
    }

    this.#clearActiveRun();
    this.#options.onRunningChange(false);
    if (value.type === "problem") {
      this.#options.onProblem(value.category, value.message);
    } else if (value.result !== undefined && value.result !== "") {
      this.#options.onOutput([value.result]);
    }
  }

  #clearActiveRun(): void {
    if (this.#timer !== undefined) {
      clearTimeout(this.#timer);
      this.#timer = undefined;
    }
    this.#activeRunId = undefined;
  }

  #discardWorker(): void {
    this.#clearActiveRun();
    this.#worker?.terminate();
    this.#worker = undefined;
  }
}
