import { FitAddon } from "@xterm/addon-fit";
import { Terminal } from "@xterm/xterm";
import "@xterm/xterm/css/xterm.css";

import { resolveInput } from "../runtime/input-buffer";
import { TerminalInputEditor } from "./terminal-input";

interface DisposablePort {
  dispose(): void;
}

interface TerminalPort {
  open(container: HTMLElement): void;
  onData(listener: (data: string) => void): DisposablePort;
  write(data: string): void;
  focus(): void;
  dispose(): void;
}

interface FitAddonPort {
  fit(): void;
}

interface ResizeObserverPort {
  observe(target: Element): void;
  disconnect(): void;
}

interface TerminalWindowDependencies {
  terminal: TerminalPort;
  fitAddon: FitAddonPort;
  createResizeObserver(callback: ResizeObserverCallback): ResizeObserverPort;
  resolveInput(buffer: SharedArrayBuffer, input: string): boolean;
}

interface PendingInput {
  buffer: SharedArrayBuffer;
  editor: TerminalInputEditor;
}

export class TerminalWindow {
  private readonly terminal: TerminalPort;
  private readonly fitAddon: FitAddonPort;
  private readonly resizeObserver: ResizeObserverPort;
  private readonly inputSubscription: DisposablePort;
  private readonly resolveInput: TerminalWindowDependencies["resolveInput"];
  private pendingInput?: PendingInput;

  constructor(
    container: HTMLElement,
    dependencies: TerminalWindowDependencies = createDefaultDependencies(),
  ) {
    this.terminal = dependencies.terminal;
    this.fitAddon = dependencies.fitAddon;
    this.resolveInput = dependencies.resolveInput;
    this.terminal.open(container);

    this.inputSubscription = this.terminal.onData((data) => {
      this.handleData(data);
    });
    this.resizeObserver = dependencies.createResizeObserver(() => {
      this.fit();
    });
    this.resizeObserver.observe(container);
    this.fit();
  }

  reset(): void {
    this.cancelInput();
    this.terminal.write("\x1bc");
  }

  write(chunks: readonly string[]): void {
    for (const chunk of chunks) {
      this.terminal.write(chunk);
    }
  }

  requestInput(buffer: SharedArrayBuffer): void {
    this.cancelInput();
    this.pendingInput = { buffer, editor: new TerminalInputEditor() };
    this.focus();
  }

  cancelInput(): void {
    this.pendingInput = undefined;
  }

  fit(): void {
    this.fitAddon.fit();
  }

  focus(): void {
    this.terminal.focus();
  }

  dispose(): void {
    this.cancelInput();
    this.resizeObserver.disconnect();
    this.inputSubscription.dispose();
    this.terminal.dispose();
  }

  private handleData(data: string): void {
    const pending = this.pendingInput;
    if (pending === undefined) {
      return;
    }

    const result = pending.editor.apply(data);
    if (result.echo !== "") {
      this.terminal.write(result.echo);
    }
    if (result.submitted === undefined) {
      return;
    }

    try {
      if (!this.resolveInput(pending.buffer, result.submitted)) {
        this.terminal.write("\x07");
        return;
      }
    } catch {
      this.terminal.write("\x07");
      return;
    }

    this.terminal.write("\r\n");
    if (this.pendingInput === pending) {
      this.pendingInput = undefined;
    }
  }
}

function createDefaultDependencies(): TerminalWindowDependencies {
  const terminal = new Terminal({
    scrollback: 10_000,
    cursorBlink: true,
    fontFamily: "Consolas, 'Courier New', monospace",
    fontSize: 13,
    theme: {
      background: "#1e1e1e",
      foreground: "#d4d4d4",
      cursor: "#d4d4d4",
      selectionBackground: "#264f78",
    },
  });
  const fitAddon = new FitAddon();
  terminal.loadAddon(fitAddon);
  return {
    terminal,
    fitAddon,
    createResizeObserver: (callback) => new ResizeObserver(callback),
    resolveInput,
  };
}
