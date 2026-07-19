export const INPUT_STATE_INDEX = 0;
export const INPUT_LENGTH_INDEX = 1;

export const INPUT_IDLE = 0;
export const INPUT_WAITING = 1;
export const INPUT_READY = 2;

const HEADER_LENGTH = 2;
const HEADER_BYTE_LENGTH = HEADER_LENGTH * Int32Array.BYTES_PER_ELEMENT;
const DEFAULT_CAPACITY = 64 * 1024;

const encoder = new TextEncoder();
const decoder = new TextDecoder();

export function createInputBuffer(
  capacity = DEFAULT_CAPACITY,
): SharedArrayBuffer {
  if (!Number.isInteger(capacity) || capacity <= 0) {
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
  if (Atomics.load(header, INPUT_STATE_INDEX) !== INPUT_WAITING) {
    return false;
  }

  const bytes = encoder.encode(input);
  if (bytes.byteLength > payload.byteLength) {
    throw new Error("Input exceeds terminal buffer capacity");
  }

  payload.set(bytes);
  Atomics.store(header, INPUT_LENGTH_INDEX, bytes.byteLength);
  Atomics.store(header, INPUT_STATE_INDEX, INPUT_READY);
  Atomics.notify(header, INPUT_STATE_INDEX);
  return true;
}

export function readResolvedInput(buffer: SharedArrayBuffer): string {
  const { header, payload } = getBufferViews(buffer);
  if (Atomics.load(header, INPUT_STATE_INDEX) !== INPUT_READY) {
    throw new Error("Terminal input buffer is not ready");
  }

  const length = Atomics.load(header, INPUT_LENGTH_INDEX);
  if (length < 0 || length > payload.byteLength) {
    throw new Error("Invalid terminal input length");
  }
  const input = decoder.decode(payload.subarray(0, length));
  Atomics.store(header, INPUT_LENGTH_INDEX, 0);
  Atomics.store(header, INPUT_STATE_INDEX, INPUT_IDLE);
  return input;
}

export function waitForInput(
  buffer: SharedArrayBuffer,
  requestInput: () => void,
): string {
  const { header } = getBufferViews(buffer);
  Atomics.store(header, INPUT_STATE_INDEX, INPUT_WAITING);
  requestInput();

  while (Atomics.load(header, INPUT_STATE_INDEX) === INPUT_WAITING) {
    Atomics.wait(header, INPUT_STATE_INDEX, INPUT_WAITING);
  }

  return readResolvedInput(buffer);
}

function getBufferViews(buffer: SharedArrayBuffer): {
  header: Int32Array;
  payload: Uint8Array;
} {
  if (buffer.byteLength <= HEADER_BYTE_LENGTH) {
    throw new Error("Invalid terminal input buffer layout");
  }
  return {
    header: new Int32Array(buffer, 0, HEADER_LENGTH),
    payload: new Uint8Array(buffer, HEADER_BYTE_LENGTH),
  };
}
