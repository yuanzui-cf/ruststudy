import * as monaco from "monaco-editor";
import EditorWorker from "monaco-editor/esm/vs/editor/editor.worker?worker";

import sampleSource from "./sample.calc?raw";
import { registerCalclang } from "./calclang/language";
import "@xterm/xterm/css/xterm.css";
import { createInputBuffer } from "./runtime/input-buffer";
import { RunController } from "./runtime/run-controller";
import { OutputWindow } from "./ui/output-window";
import {
  bindEditorDirtyIndicator,
  createRunControllerCallbacks,
  runEditor,
  type ToolName,
} from "./ui/playground-integration";
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
  ...createRunControllerCallbacks({
    terminal,
    problems,
    setProblemCount,
    selectTool,
    setStopDisabled: (disabled) => {
      stopButton.disabled = disabled;
    },
  }),
});

runButton.addEventListener("click", () => {
  runEditor(controller, editor, () =>
    typeof SharedArrayBuffer === "undefined" ? undefined : createInputBuffer(),
  );
});
stopButton.addEventListener("click", () => controller.stop());
terminalTab.addEventListener("click", () => selectTool("terminal"));
problemsTab.addEventListener("click", () => selectTool("problems"));

bindEditorDirtyIndicator(editor, {
  markDirty: () => dirtyIndicator.classList.add("visible"),
});
editor.onDidChangeCursorPosition(({ position }) => {
  cursorPosition.textContent = `Ln ${position.lineNumber}, Col ${position.column}`;
});

window.addEventListener("beforeunload", () => {
  controller.dispose();
  terminal.dispose();
  editor.dispose();
});
