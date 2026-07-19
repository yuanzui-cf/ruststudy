export interface CalclangFunctionDescriptor {
  __calclangType: string;
}

export const HOST_CONSTANTS = {
  PI: Math.PI,
  E: Math.E,
  SQRT2: Math.SQRT2,
  SQRT1_2: Math.SQRT1_2,
  LN2: Math.LN2,
  LN10: Math.LN10,
  LOG2E: Math.LOG2E,
  LOG10E: Math.LOG10E,
} as const;

function isDescriptor(value: unknown): value is CalclangFunctionDescriptor {
  return (
    typeof value === "object" &&
    value !== null &&
    "__calclangType" in value &&
    typeof value.__calclangType === "string"
  );
}

function display(value: unknown): string {
  if (value === undefined || value === null || isDescriptor(value)) {
    return "";
  }
  return String(value);
}

function typeName(value: unknown): string {
  if (isDescriptor(value)) {
    return value.__calclangType;
  }
  if (value === undefined || value === null) {
    return "none";
  }
  if (typeof value === "number") {
    return "float";
  }
  if (typeof value === "boolean") {
    return "bool";
  }
  if (typeof value === "string") {
    return "string";
  }
  throw new TypeError("Unsupported calclang host value");
}

export function createHostFunctions(emit: (line: string) => void) {
  return {
    println: (...values: unknown[]) => {
      emit(values.map(display).join(""));
      return undefined;
    },
    typeof: (...values: unknown[]) => {
      if (values.length !== 1) {
        throw new Error(`Expect one arg in typeof, found ${values.length}`);
      }
      return typeName(values[0]);
    },
    sin: Math.sin,
    cos: Math.cos,
    tan: Math.tan,
    asin: Math.asin,
    acos: Math.acos,
    atan: Math.atan,
    atan2: Math.atan2,
    sinh: Math.sinh,
    cosh: Math.cosh,
    tanh: Math.tanh,
    abs: Math.abs,
    sqrt: Math.sqrt,
    cbrt: Math.cbrt,
    exp: Math.exp,
    log: Math.log,
    log2: Math.log2,
    log10: Math.log10,
    pow: Math.pow,
    floor: Math.floor,
    ceil: Math.ceil,
    round: Math.round,
    trunc: Math.trunc,
    sign: Math.sign,
    random: Math.random,
  };
}
