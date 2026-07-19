# Calclang Completion And Branding Fix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restore Monaco's visible completion and snippet UI while removing third-party product branding from the Calclang playground.

**Architecture:** Keep the existing Calclang Monarch tokenizer and completion provider unchanged, but load Monaco through its complete public package entry so editor contributions such as suggest and snippets are registered. Add source contracts for the entry point and title text, then verify the actual completion widget in headless Chrome.

**Tech Stack:** Bun, TypeScript, Vite, Monaco Editor 0.55, happy-dom, ChromeDriver

---

## File Map

- Modify: `compile/calculator/web/src/ui/shell.test.ts` - guard the unbranded title and full Monaco entry point.
- Modify: `compile/calculator/web/index.html` - use only the Calclang product name.
- Modify: `compile/calculator/web/src/main.ts` - restore Monaco's complete public entry point.
- Modify: `compile/calculator/web/src/calclang/language.ts` - align the type-only Monaco import with the runtime entry point.
- Modify: `compile/calculator/web/vite.config.ts` - document the expected complete Monaco bundle size.

### Task 1: Add Regression Contracts

**Files:**

- Modify: `compile/calculator/web/src/ui/shell.test.ts`

- [ ] **Step 1: Add failing branding and Monaco-entry tests**

Append these tests:

```ts
test("shell uses only the Calclang product name", async () => {
  const html = await Bun.file(
    new URL("../../index.html", import.meta.url),
  ).text();
  expect(html).toContain("Calclang Playground");
  expect(html).not.toContain("Microsoft");
  expect(html).not.toContain("Visual Studio");
});

test("main loads Monaco's complete editor contributions", async () => {
  const main = await Bun.file(new URL("../main.ts", import.meta.url)).text();
  expect(main).toContain('from "monaco-editor";');
  expect(main).not.toContain("editor.api.js");
});
```

- [ ] **Step 2: Run the focused test and verify RED**

Run:

```sh
cd compile/calculator/web
bun test src/ui/shell.test.ts
```

Expected: two new tests fail because the title contains `Microsoft Visual Studio` and `main.ts` imports `editor.api.js`.

- [ ] **Step 3: Commit the failing regression contracts together with the fix in Task 2**

Do not commit a deliberately broken tree. Continue directly to Task 2.

### Task 2: Restore Completion And Remove Branding

**Files:**

- Modify: `compile/calculator/web/index.html`
- Modify: `compile/calculator/web/src/main.ts`
- Modify: `compile/calculator/web/src/calclang/language.ts`
- Modify: `compile/calculator/web/vite.config.ts`

- [ ] **Step 1: Replace the title-bar text**

Use:

```html
<header class="title-bar">Calclang Playground</header>
```

- [ ] **Step 2: Restore Monaco's public package entry**

Use the same import for runtime and types:

```ts
// src/main.ts
import * as monaco from "monaco-editor";

// src/calclang/language.ts
import type * as Monaco from "monaco-editor";
```

- [ ] **Step 3: Raise the known bundle threshold for the complete editor**

Update the existing Vite build setting:

```ts
build: {
  // The complete Monaco editor includes suggest and snippet contributions.
  chunkSizeWarningLimit: 4_000,
},
```

- [ ] **Step 4: Run focused and full frontend checks**

Run:

```sh
cd compile/calculator/web
bun test src/ui/shell.test.ts
bun test
bunx tsc --noEmit
bun run build
```

Expected: 24 Bun tests pass, TypeScript exits zero, and the production build exits zero without Vite chunk warnings.

- [ ] **Step 5: Verify the visible completion widget in Chrome**

Serve `dist`, open the page through matching ChromeDriver, focus `.view-line`, send `Control+Space`, and inspect `.suggest-widget`.

Expected: the widget becomes visible and contains at least one of `println`, `typeof`, `PI`, `sin`, `random`, `min`, or `max`. Also verify `document.body.innerText` does not contain `Microsoft` or `Visual Studio`.

- [ ] **Step 6: Commit**

```sh
git add compile/calculator/web/index.html \
  compile/calculator/web/src/main.ts \
  compile/calculator/web/src/calclang/language.ts \
  compile/calculator/web/src/ui/shell.test.ts \
  compile/calculator/web/vite.config.ts
git commit -m "fix(calculator): Restore editor completion"
```

### Task 3: Publish The Correction

**Files:** None.

- [ ] **Step 1: Verify repository state**

Run:

```sh
git diff --check
git status --short
git log -3 --oneline
```

Expected: the worktree is clean and the completion fix follows the design-constraint commit.

- [ ] **Step 2: Push the tracked feature branch**

Run:

```sh
git push
```

Expected: `origin/feat/calclang-web-playground` advances to the completion-fix commit without creating a pull request.
