import type { RunControllerOptions } from "../runtime/run-controller";

export type ToolName = "terminal" | "problems";

interface TerminalSink {
  reset(): void;
  write(entries: readonly string[]): void;
  requestInput(buffer: SharedArrayBuffer): void;
  cancelInput(): void;
}

interface ProblemsSink {
  clear(): void;
  append(entries: readonly string[], className?: string): void;
}

interface RunControllerUi {
  terminal: TerminalSink;
  problems: ProblemsSink;
  setProblemCount(count: number): void;
  selectTool(tool: ToolName): void;
  setStopDisabled(disabled: boolean): void;
}

export function createRunControllerCallbacks(
  ui: RunControllerUi,
): Pick<
  RunControllerOptions,
  "onReset" | "onOutput" | "onProblem" | "onInput" | "onRunningChange"
> {
  return {
    onReset: () => {
      ui.terminal.reset();
      ui.terminal.cancelInput();
      ui.problems.clear();
      ui.setProblemCount(0);
      ui.selectTool("terminal");
    },
    onOutput: (entries) => ui.terminal.write(entries),
    onInput: (buffer) => ui.terminal.requestInput(buffer),
    onProblem: (category, message) => {
      ui.problems.append([`${category}: ${message}`], "problem-entry");
      ui.setProblemCount(1);
      ui.selectTool("problems");
    },
    onRunningChange: (running) => {
      ui.setStopDisabled(!running);
      if (!running) {
        ui.terminal.cancelInput();
      }
    },
  };
}

interface RunnableController {
  run(
    source: string,
    preserveEnvironment: boolean,
    inputBuffer?: SharedArrayBuffer,
  ): void;
}

interface EditorValueSource {
  getValue(): string;
}

export function runEditor(
  controller: RunnableController,
  editor: EditorValueSource,
  createInputBuffer: () => SharedArrayBuffer | undefined,
): void {
  controller.run(editor.getValue(), false, createInputBuffer());
}

interface EditorChangeSource {
  onDidChangeModelContent(listener: () => void): unknown;
}

interface DirtyIndicator {
  markDirty(): void;
}

export function bindEditorDirtyIndicator(
  editor: EditorChangeSource,
  dirtyIndicator: DirtyIndicator,
): void {
  editor.onDidChangeModelContent(() => dirtyIndicator.markDirty());
}
