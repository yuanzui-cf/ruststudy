import { FitAddon } from "@xterm/addon-fit";
import { Terminal } from "@xterm/xterm";
import "@xterm/xterm/css/xterm.css";

import { resolveInput } from "../runtime/input-buffer";
import { applyTerminalInput } from "./terminal-input";

interface PendingInput {
  buffer: SharedArrayBuffer;
  value: string;
}

const CAPACITY_ERROR = "Input exceeds terminal buffer capacity";

export class TerminalWindow {
  private readonly terminal: Terminal;
  private readonly fitAddon = new FitAddon();
  private readonly resizeObserver: ResizeObserver;
  private readonly inputSubscription: { dispose(): void };
  private pendingInput?: PendingInput;

  constructor(container: HTMLElement) {
    this.terminal = new Terminal({
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
    this.terminal.loadAddon(this.fitAddon);
    this.terminal.open(container);

    this.inputSubscription = this.terminal.onData((data) => {
      this.handleData(data);
    });
    this.resizeObserver = new ResizeObserver(() => {
      this.fit();
    });
    this.resizeObserver.observe(container);
    this.fit();
  }

  reset(): void {
    this.cancelInput();
    this.terminal.reset();
    this.terminal.clear();
  }

  write(chunks: readonly string[]): void {
    for (const chunk of chunks) {
      this.terminal.write(chunk);
    }
  }

  requestInput(buffer: SharedArrayBuffer): void {
    this.cancelInput();
    this.pendingInput = { buffer, value: "" };
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

    const result = applyTerminalInput(pending.value, data);
    if (result.echo !== "") {
      this.terminal.write(result.echo);
    }
    if (result.submitted === undefined) {
      pending.value = result.value;
      return;
    }

    try {
      if (resolveInput(pending.buffer, result.submitted)) {
        this.pendingInput = undefined;
      } else {
        pending.value = result.submitted;
      }
    } catch (error) {
      if (error instanceof Error && error.message === CAPACITY_ERROR) {
        pending.value = result.submitted;
        this.terminal.write("\x07");
        return;
      }
      throw error;
    }
  }
}
