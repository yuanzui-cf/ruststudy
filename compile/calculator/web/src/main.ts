import * as monaco from "monaco-editor";
import EditorWorker from "monaco-editor/esm/vs/editor/editor.worker?worker";

import sampleSource from "./sample.calc?raw";
import { registerCalclang } from "./calclang/language";
import "@xterm/xterm/css/xterm.css";
import { createInputBuffer } from "./runtime/input-buffer";
import { RunController } from "./runtime/run-controller";
import { OutputWindow } from "./ui/output-window";
import { TerminalWindow } from "./ui/terminal-window";
import "./styles.css";

self.MonacoEnvironment = {
  getWorker: () => new EditorWorker(),
};

function required<T extends HTMLElement>(id: string): T {
  const element = document.getElementById(id);
  if (!(element instanceof HTMLElement)) {
    throw new Error(`Missing required playground element: #${id}`);
  }
  return element as T;
}

type ToolName = "terminal" | "problems";

const editorContainer = required<HTMLDivElement>("editor");
const runButton = required<HTMLButtonElement>("run-button");
const stopButton = required<HTMLButtonElement>("stop-button");
const terminalTab = required<HTMLButtonElement>("terminal-tab");
const problemsTab = required<HTMLButtonElement>("problems-tab");
const terminalPanel = required<HTMLDivElement>("terminal");
const problemsPanel = required<HTMLDivElement>("problems");
const problemCount = required<HTMLSpanElement>("problem-count");
const dirtyIndicator = required<HTMLSpanElement>("dirty-indicator");
const cursorPosition = required<HTMLSpanElement>("cursor-position");

function setProblemCount(count: number): void {
  problemCount.textContent = String(count);
}

function selectTool(selected: ToolName): void {
  const terminalSelected = selected === "terminal";
  terminalTab.setAttribute("aria-selected", String(terminalSelected));
  problemsTab.setAttribute("aria-selected", String(!terminalSelected));
  terminalPanel.hidden = !terminalSelected;
  problemsPanel.hidden = terminalSelected;
}

registerCalclang(monaco);
const editor = monaco.editor.create(editorContainer, {
  value: sampleSource,
  language: "calclang",
  theme: "vs-dark",
  automaticLayout: true,
  minimap: { enabled: false },
  fontFamily: "Consolas, 'Courier New', monospace",
});

const terminal = new TerminalWindow(terminalPanel);
const problems = new OutputWindow(problemsPanel);

const controller = new RunController({
  createWorker: () =>
    new Worker(new URL("./runtime/calclang.worker.ts", import.meta.url), {
      type: "module",
    }),
  onReset: () => {
    terminal.reset();
    problems.clear();
    setProblemCount(0);
    selectTool("terminal");
  },
  onOutput: (entries) => terminal.write(entries),
  onInput: (buffer) => terminal.requestInput(buffer),
  onProblem: (category, message) => {
    problems.append([`${category}: ${message}`], "problem-entry");
    setProblemCount(1);
    selectTool("problems");
  },
  onRunningChange: (running) => {
    stopButton.disabled = !running;
    if (!running) {
      terminal.cancelInput();
    }
  },
});

runButton.addEventListener("click", () => {
  const inputBuffer =
    typeof SharedArrayBuffer === "undefined" ? undefined : createInputBuffer();
  controller.run(editor.getValue(), false, inputBuffer);
});
stopButton.addEventListener("click", () => controller.stop());
terminalTab.addEventListener("click", () => selectTool("terminal"));
problemsTab.addEventListener("click", () => selectTool("problems"));

editor.onDidChangeModelContent(() => {
  dirtyIndicator.classList.add("visible");
});
editor.onDidChangeCursorPosition(({ position }) => {
  cursorPosition.textContent = `Ln ${position.lineNumber}, Col ${position.column}`;
});

window.addEventListener("beforeunload", () => {
  controller.dispose();
  terminal.dispose();
  editor.dispose();
});
