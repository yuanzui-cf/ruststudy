export const KEYWORDS = [
  "let",
  "if",
  "else",
  "loop",
  "break",
  "continue",
  "fn",
  "return",
  "true",
  "false",
  "none",
] as const;

export const WORD_OPERATORS = ["and", "or"] as const;

export const FLOAT_LITERAL =
  /(?:[0-9]+(?:\.[0-9]*)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?/;

export type CompletionCategory =
  | "keyword"
  | "snippet"
  | "function"
  | "constant";

export interface CompletionSpec {
  label: string;
  detail: string;
  category: CompletionCategory;
  insertText?: string;
}

const keywords: readonly CompletionSpec[] = [
  "else",
  "break",
  "continue",
  "true",
  "false",
  "none",
  "and",
  "or",
].map((label) => ({ label, detail: "calclang keyword", category: "keyword" }));

const snippets: readonly CompletionSpec[] = [
  {
    label: "let",
    detail: "variable definition",
    category: "snippet",
    insertText: "let ${1:name} = ${2:value};",
  },
  {
    label: "if",
    detail: "conditional expression",
    category: "snippet",
    insertText: "if ${1:condition} {\n\t${2}\n}",
  },
  {
    label: "if else",
    detail: "conditional with else",
    category: "snippet",
    insertText: "if ${1:condition} {\n\t${2}\n} else {\n\t${3}\n}",
  },
  {
    label: "loop",
    detail: "loop expression",
    category: "snippet",
    insertText: "loop {\n\t${1}\n}",
  },
  {
    label: "fn",
    detail: "named function",
    category: "snippet",
    insertText: "fn ${1:name}(${2:args}) {\n\t${3}\n}",
  },
  {
    label: "anonymous fn",
    detail: "anonymous function",
    category: "snippet",
    insertText: "fn(${1:args}) {\n\t${2}\n}",
  },
  {
    label: "return",
    detail: "return expression",
    category: "snippet",
    insertText: "return ${1:value};",
  },
];

const constants = [
  "PI",
  "E",
  "SQRT2",
  "SQRT1_2",
  "LN2",
  "LN10",
  "LOG2E",
  "LOG10E",
].map<CompletionSpec>((label) => ({
  label,
  detail: "JavaScript Math constant",
  category: "constant",
}));

const hostFunctions = [
  "println",
  "typeof",
  "sin",
  "cos",
  "tan",
  "asin",
  "acos",
  "atan",
  "atan2",
  "sinh",
  "cosh",
  "tanh",
  "abs",
  "sqrt",
  "cbrt",
  "exp",
  "log",
  "log2",
  "log10",
  "pow",
  "floor",
  "ceil",
  "round",
  "trunc",
  "sign",
  "random",
  "min",
  "max",
].map<CompletionSpec>((label) => ({
  label,
  detail: label === "min" || label === "max" ? "calclang prelude function" : "JavaScript host function",
  category: "function",
}));

export const COMPLETIONS: readonly CompletionSpec[] = [
  ...keywords,
  ...snippets,
  ...constants,
  ...hostFunctions,
];

export const COMPLETION_NAMES = COMPLETIONS.map(({ label }) => label);
