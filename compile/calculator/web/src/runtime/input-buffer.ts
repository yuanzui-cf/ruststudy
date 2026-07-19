export const INPUT_STATE_INDEX = 0;
export const INPUT_LENGTH_INDEX = 1;

export const INPUT_IDLE = 0;
export const INPUT_WAITING = 1;
export const INPUT_RESOLVING = 2;
export const INPUT_READY = 3;

const HEADER_LENGTH = 2;
const HEADER_BYTE_LENGTH = HEADER_LENGTH * Int32Array.BYTES_PER_ELEMENT;
const DEFAULT_CAPACITY = 64 * 1024;
const MAX_CAPACITY = 0x7fffffff;

const encoder = new TextEncoder();
const decoder = new TextDecoder("utf-8", { fatal: true });

export function createInputBuffer(
  capacity = DEFAULT_CAPACITY,
): SharedArrayBuffer {
  if (
    !Number.isInteger(capacity) ||
    capacity <= 0 ||
    capacity > MAX_CAPACITY
  ) {
    throw new Error(
      "Terminal input buffer capacity must be a positive integer",
    );
  }
  return new SharedArrayBuffer(HEADER_BYTE_LENGTH + capacity);
}

export function resolveInput(
  buffer: SharedArrayBuffer,
  input: string,
): boolean {
  const { header, payload } = getBufferViews(buffer);
  if (
    Atomics.compareExchange(
      header,
      INPUT_STATE_INDEX,
      INPUT_WAITING,
      INPUT_RESOLVING,
    ) !== INPUT_WAITING
  ) {
    return false;
  }

  try {
    const bytes = encoder.encode(input);
    if (bytes.byteLength > payload.byteLength) {
      throw new Error("Input exceeds terminal buffer capacity");
    }

    payload.set(bytes);
    Atomics.store(header, INPUT_LENGTH_INDEX, bytes.byteLength);
    Atomics.store(header, INPUT_STATE_INDEX, INPUT_READY);
    Atomics.notify(header, INPUT_STATE_INDEX);
    return true;
  } catch (error) {
    resetWaiting(header);
    throw error;
  }
}

export function readResolvedInput(buffer: SharedArrayBuffer): string {
  const { header, payload } = getBufferViews(buffer);
  if (Atomics.load(header, INPUT_STATE_INDEX) !== INPUT_READY) {
    throw new Error("Terminal input buffer is not ready");
  }

  try {
    const length = Atomics.load(header, INPUT_LENGTH_INDEX);
    if (length < 0 || length > payload.byteLength) {
      throw new Error("Invalid terminal input length");
    }
    return decoder.decode(payload.subarray(0, length));
  } finally {
    resetIdle(header);
  }
}

export function waitForInput(
  buffer: SharedArrayBuffer,
  requestInput: () => void,
): string {
  const { header } = getBufferViews(buffer);
  Atomics.store(header, INPUT_LENGTH_INDEX, 0);
  Atomics.store(header, INPUT_STATE_INDEX, INPUT_WAITING);
  try {
    requestInput();
  } catch (error) {
    resetIdle(header);
    throw error;
  }

  while (true) {
    const state = Atomics.load(header, INPUT_STATE_INDEX);
    if (state === INPUT_READY) {
      return readResolvedInput(buffer);
    }
    if (state === INPUT_WAITING || state === INPUT_RESOLVING) {
      Atomics.wait(header, INPUT_STATE_INDEX, state);
      continue;
    }
    throw new Error("Terminal input buffer is not ready");
  }
}

function getBufferViews(buffer: SharedArrayBuffer): {
  header: Int32Array;
  payload: Uint8Array;
} {
  if (
    !(buffer instanceof SharedArrayBuffer) ||
    buffer.byteLength <= HEADER_BYTE_LENGTH ||
    buffer.byteLength - HEADER_BYTE_LENGTH > MAX_CAPACITY
  ) {
    throw new Error("Invalid terminal input buffer layout");
  }
  return {
    header: new Int32Array(buffer, 0, HEADER_LENGTH),
    payload: new Uint8Array(buffer, HEADER_BYTE_LENGTH),
  };
}

function resetIdle(header: Int32Array): void {
  Atomics.store(header, INPUT_LENGTH_INDEX, 0);
  Atomics.store(header, INPUT_STATE_INDEX, INPUT_IDLE);
}

function resetWaiting(header: Int32Array): void {
  Atomics.store(header, INPUT_LENGTH_INDEX, 0);
  Atomics.store(header, INPUT_STATE_INDEX, INPUT_WAITING);
}
