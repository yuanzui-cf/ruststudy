import * as monaco from "monaco-editor";
import EditorWorker from "monaco-editor/esm/vs/editor/editor.worker?worker";

import sampleSource from "./sample.calc?raw";
import { registerCalclang } from "./calclang/language";
import { RunController } from "./runtime/run-controller";
import { OutputWindow } from "./ui/output-window";
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

type ToolName = "output" | "problems";

const editorContainer = required<HTMLDivElement>("editor");
const runButton = required<HTMLButtonElement>("run-button");
const stopButton = required<HTMLButtonElement>("stop-button");
const outputTab = required<HTMLButtonElement>("output-tab");
const problemsTab = required<HTMLButtonElement>("problems-tab");
const outputPanel = required<HTMLDivElement>("output");
const problemsPanel = required<HTMLDivElement>("problems");
const problemCount = required<HTMLSpanElement>("problem-count");
const dirtyIndicator = required<HTMLSpanElement>("dirty-indicator");
const cursorPosition = required<HTMLSpanElement>("cursor-position");

function setProblemCount(count: number): void {
  problemCount.textContent = String(count);
}

function selectTool(selected: ToolName): void {
  const outputSelected = selected === "output";
  outputTab.setAttribute("aria-selected", String(outputSelected));
  problemsTab.setAttribute("aria-selected", String(!outputSelected));
  outputPanel.hidden = !outputSelected;
  problemsPanel.hidden = outputSelected;
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

const output = new OutputWindow(outputPanel);
const problems = new OutputWindow(problemsPanel);

const controller = new RunController({
  createWorker: () =>
    new Worker(new URL("./runtime/calclang.worker.ts", import.meta.url), {
      type: "module",
    }),
  onReset: () => {
    output.clear();
    problems.clear();
    setProblemCount(0);
    selectTool("output");
  },
  onOutput: (entries) => output.append(entries),
  onProblem: (category, message) => {
    problems.append([`${category}: ${message}`], "problem-entry");
    setProblemCount(1);
    selectTool("problems");
  },
  onRunningChange: (running) => {
    stopButton.disabled = !running;
  },
});

runButton.addEventListener("click", () => {
  controller.run(editor.getValue(), false);
});
stopButton.addEventListener("click", () => controller.stop());
outputTab.addEventListener("click", () => selectTool("output"));
problemsTab.addEventListener("click", () => selectTool("problems"));

editor.onDidChangeModelContent(() => {
  dirtyIndicator.classList.add("visible");
});
editor.onDidChangeCursorPosition(({ position }) => {
  cursorPosition.textContent = `Ln ${position.lineNumber}, Col ${position.column}`;
});

window.addEventListener("beforeunload", () => {
  controller.dispose();
  editor.dispose();
});
