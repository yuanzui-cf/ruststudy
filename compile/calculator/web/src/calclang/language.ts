import type * as Monaco from "monaco-editor";

import {
  COMPLETIONS,
  FLOAT_LITERAL,
  KEYWORDS,
  WORD_OPERATORS,
  type CompletionCategory,
} from "./language-data";

export function registerCalclang(monaco: typeof Monaco): void {
  monaco.languages.register({ id: "calclang" });

  monaco.languages.setLanguageConfiguration("calclang", {
    comments: { lineComment: "//", blockComment: ["/*", "*/"] },
    brackets: [
      ["{", "}"],
      ["(", ")"],
    ],
    autoClosingPairs: [
      { open: "{", close: "}" },
      { open: "(", close: ")" },
      { open: '"', close: '"', notIn: ["string", "comment"] },
    ],
    surroundingPairs: [
      { open: "{", close: "}" },
      { open: "(", close: ")" },
      { open: '"', close: '"' },
    ],
    indentationRules: {
      increaseIndentPattern: /\{[^}]*$/,
      decreaseIndentPattern: /^\s*\}/,
    },
  });

  monaco.languages.setMonarchTokensProvider("calclang", {
    keywords: [...KEYWORDS],
    wordOperators: [...WORD_OPERATORS],
    tokenizer: {
      root: [
        [
          /[a-zA-Z_][a-zA-Z0-9_]*/,
          {
            cases: {
              "@keywords": "keyword",
              "@wordOperators": "operator",
              "@default": "identifier",
            },
          },
        ],
        [FLOAT_LITERAL, "number.float"],
        [/\/\*/, "comment", "@comment"],
        [/\/\/.*$/, "comment"],
        [/"/, "string", "@string"],
        [/[{}()]/, "@brackets"],
        [/[;,]/, "delimiter"],
        [/[+\-*\/^=!<>]+/, "operator"],
        [/\s+/, "white"],
      ],
      comment: [
        [/[^*]+/, "comment"],
        [/\*\//, "comment", "@pop"],
        [/\*/, "comment"],
      ],
      string: [
        [/[^\\"\n]+/, "string"],
        [/\\[tnr"'\\]/, "string.escape"],
        [/"/, "string", "@pop"],
        [/\n/, "string.invalid", "@pop"],
      ],
    },
  });

  monaco.languages.registerCompletionItemProvider("calclang", {
    provideCompletionItems(model, position) {
      const word = model.getWordUntilPosition(position);
      const range = {
        startLineNumber: position.lineNumber,
        endLineNumber: position.lineNumber,
        startColumn: word.startColumn,
        endColumn: word.endColumn,
      };

      return {
        suggestions: COMPLETIONS.map((item) => ({
          label: item.label,
          detail: item.detail,
          kind: completionKind(monaco, item.category),
          insertText: item.insertText ?? item.label,
          insertTextRules:
            item.category === "snippet"
              ? monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet
              : undefined,
          range,
        })),
      };
    },
  });
}

function completionKind(
  monaco: typeof Monaco,
  category: CompletionCategory,
): Monaco.languages.CompletionItemKind {
  switch (category) {
    case "keyword":
      return monaco.languages.CompletionItemKind.Keyword;
    case "snippet":
      return monaco.languages.CompletionItemKind.Snippet;
    case "constant":
      return monaco.languages.CompletionItemKind.Constant;
    case "function":
      return monaco.languages.CompletionItemKind.Function;
  }
}
