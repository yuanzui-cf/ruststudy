export interface TerminalInputResult {
  value: string;
  echo: string;
  submitted?: string;
}

/** Applies one xterm data event to a line buffer and returns its terminal echo. */
export function applyTerminalInput(
  value: string,
  data: string,
): TerminalInputResult {
  let nextValue = value;
  let echo = "";
  let escapeState: "none" | "escape" | "csi" | "string" = "none";
  let stringAllowsBell = false;
  let stringEscape = false;

  for (const character of data) {
    const codePoint = character.codePointAt(0) ?? 0;

    if (escapeState === "string") {
      if (stringEscape) {
        if (character === "\\") {
          escapeState = "none";
          stringEscape = false;
        } else if (character !== "\x1b") {
          stringEscape = false;
        }
        continue;
      }
      if (character === "\x1b") {
        stringEscape = true;
      } else if (stringAllowsBell && character === "\x07") {
        escapeState = "none";
      }
      continue;
    }

    if (escapeState === "escape") {
      if (character === "[") {
        escapeState = "csi";
        continue;
      }
      if (character === "]") {
        escapeState = "string";
        stringAllowsBell = true;
        continue;
      }
      if (character === "P" || character === "X" || character === "^" || character === "_") {
        escapeState = "string";
        stringAllowsBell = false;
        continue;
      }
      if (codePoint >= 0x40 && codePoint <= 0x7e) {
        escapeState = "none";
      }
      continue;
    }
    if (escapeState === "csi") {
      if (codePoint >= 0x40 && codePoint <= 0x7e) {
        escapeState = "none";
      }
      continue;
    }

    if (character === "\x1b") {
      escapeState = "escape";
      continue;
    }
    if (codePoint === 0x9b) {
      escapeState = "csi";
      continue;
    }
    if (codePoint === 0x9d) {
      escapeState = "string";
      stringAllowsBell = true;
      continue;
    }
    if (codePoint === 0x90 || codePoint === 0x98 || codePoint === 0x9e || codePoint === 0x9f) {
      escapeState = "string";
      stringAllowsBell = false;
      continue;
    }
    if (character === "\x7f") {
      const codePoints = Array.from(nextValue);
      if (codePoints.length > 0) {
        codePoints.pop();
        nextValue = codePoints.join("");
        echo += "\b \b";
      }
      continue;
    }
    if (character === "\r" || character === "\n") {
      return { value: "", echo: `${echo}\r\n`, submitted: nextValue };
    }
    if (codePoint < 0x20 || (codePoint >= 0x80 && codePoint <= 0x9f)) {
      continue;
    }

    nextValue += character;
    echo += character;
  }

  return { value: nextValue, echo };
}
