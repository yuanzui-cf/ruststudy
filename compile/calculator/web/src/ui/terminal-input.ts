export interface TerminalInputResult {
  echo: string;
  submitted?: string;
}

type EscapeState = "none" | "escape" | "csi" | "string";

const graphemeSegmenter = new Intl.Segmenter(undefined, {
  granularity: "grapheme",
});
const extendedPictographic = /\p{Extended_Pictographic}/u;
const mark = /\p{Mark}/u;

export class TerminalInputEditor {
  private line: string;
  private escapeState: EscapeState = "none";
  private stringAllowsBell = false;
  private stringEscape = false;

  constructor(value = "") {
    this.line = value;
  }

  get value(): string {
    return this.line;
  }

  apply(data: string): TerminalInputResult {
    let echo = "";

    for (const character of data) {
      const codePoint = character.codePointAt(0) ?? 0;

      if (codePoint === 0x18 || codePoint === 0x1a) {
        this.cancelSequence();
        continue;
      }
      if (this.escapeState === "string") {
        this.applyControlString(character, codePoint);
        continue;
      }
      if (this.escapeState === "escape") {
        this.applyEscape(character, codePoint);
        continue;
      }
      if (this.escapeState === "csi") {
        if (character === "\x1b") {
          this.escapeState = "escape";
          continue;
        }
        if (isCsiFinalByte(codePoint)) {
          this.escapeState = "none";
        }
        continue;
      }

      if (character === "\x1b") {
        this.escapeState = "escape";
        continue;
      }
      if (codePoint === 0x9b) {
        this.escapeState = "csi";
        continue;
      }
      if (codePoint === 0x9d) {
        this.startControlString(true);
        continue;
      }
      if (isC1ControlString(codePoint)) {
        this.startControlString(false);
        continue;
      }
      if (character === "\x7f") {
        echo += this.eraseLastGrapheme();
        continue;
      }
      if (character === "\r" || character === "\n") {
        return { echo, submitted: this.line };
      }
      if (codePoint < 0x20 || (codePoint >= 0x80 && codePoint <= 0x9f)) {
        continue;
      }

      this.line += character;
      echo += character;
    }

    return { echo };
  }

  private applyControlString(character: string, codePoint: number): void {
    if (codePoint === 0x9c) {
      this.escapeState = "none";
      this.stringEscape = false;
      return;
    }
    if (this.stringEscape) {
      if (character === "\\") {
        this.escapeState = "none";
        this.stringEscape = false;
      } else if (character !== "\x1b") {
        this.stringEscape = false;
      }
      return;
    }
    if (character === "\x1b") {
      this.stringEscape = true;
    } else if (this.stringAllowsBell && character === "\x07") {
      this.escapeState = "none";
    }
  }

  private applyEscape(character: string, codePoint: number): void {
    if (character === "[") {
      this.escapeState = "csi";
      return;
    }
    if (character === "]") {
      this.startControlString(true);
      return;
    }
    if (
      character === "P" ||
      character === "X" ||
      character === "^" ||
      character === "_"
    ) {
      this.startControlString(false);
      return;
    }
    if (isEscapeFinalByte(codePoint)) {
      this.escapeState = "none";
    }
  }

  private startControlString(allowsBell: boolean): void {
    this.escapeState = "string";
    this.stringAllowsBell = allowsBell;
    this.stringEscape = false;
  }

  private cancelSequence(): void {
    this.escapeState = "none";
    this.stringAllowsBell = false;
    this.stringEscape = false;
  }

  private eraseLastGrapheme(): string {
    const segments = Array.from(graphemeSegmenter.segment(this.line));
    const last = segments.at(-1);
    if (last === undefined) {
      return "";
    }

    this.line = this.line.slice(0, last.index);
    return "\b \b".repeat(getCellWidth(last.segment));
  }
}

function isCsiFinalByte(codePoint: number): boolean {
  return codePoint >= 0x40 && codePoint <= 0x7e;
}

function isEscapeFinalByte(codePoint: number): boolean {
  return codePoint >= 0x30 && codePoint <= 0x7e;
}

function isC1ControlString(codePoint: number): boolean {
  return (
    codePoint === 0x90 ||
    codePoint === 0x98 ||
    codePoint === 0x9e ||
    codePoint === 0x9f
  );
}

function getCellWidth(grapheme: string): number {
  if (extendedPictographic.test(grapheme)) {
    return 2;
  }

  let width = 0;
  for (const character of grapheme) {
    const codePoint = character.codePointAt(0) ?? 0;
    if (mark.test(character) || codePoint === 0x200d) {
      continue;
    }
    width = Math.max(width, isWideCodePoint(codePoint) ? 2 : 1);
  }
  return width;
}

function isWideCodePoint(codePoint: number): boolean {
  return (
    codePoint >= 0x1100 &&
    (codePoint <= 0x115f ||
      codePoint === 0x2329 ||
      codePoint === 0x232a ||
      (codePoint >= 0x2e80 && codePoint <= 0x303e) ||
      (codePoint >= 0x3040 && codePoint <= 0xa4cf) ||
      (codePoint >= 0xac00 && codePoint <= 0xd7a3) ||
      (codePoint >= 0xf900 && codePoint <= 0xfaff) ||
      (codePoint >= 0xfe10 && codePoint <= 0xfe19) ||
      (codePoint >= 0xfe30 && codePoint <= 0xfe6f) ||
      (codePoint >= 0xff00 && codePoint <= 0xff60) ||
      (codePoint >= 0xffe0 && codePoint <= 0xffe6) ||
      (codePoint >= 0x1f1e6 && codePoint <= 0x1f1ff) ||
      (codePoint >= 0x1f300 && codePoint <= 0x1faff) ||
      (codePoint >= 0x20000 && codePoint <= 0x3fffd))
  );
}
