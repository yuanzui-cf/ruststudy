# Calclang Web Playground Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (- [ ]) syntax for tracking.

**Goal:** Build a Bun-managed Visual Studio-style browser playground that runs calclang through an interruptible Web Worker, injects JavaScript host functions, and edits source with Monaco language support.

**Architecture:** A callback-capable Rust interpreter is exported with wasm-bindgen. A module Worker owns the WASM instance and JavaScript host environment, while a main-thread controller owns Run/Stop, the ten-minute kill timer, stale-run filtering, and rolling output. Vite builds a framework-free TypeScript interface with native Monaco styling.

**Tech Stack:** Rust 2024, wasm-bindgen, wasm-pack, Bun, TypeScript, Vite, Monaco Editor, Bun test, happy-dom

---

## File Map

Rust files:

- Modify: compile/calculator/Cargo.toml — target-specific WASM dependencies and test dependency.
- Modify: compile/calculator/src/ast.rs — cloneable callback-backed BuiltIn.
- Modify: compile/calculator/src/ctx.rs — derive the existing default context cleanly.
- Modify: compile/calculator/src/env.rs — generic built-in registration.
- Modify: compile/calculator/src/error.rs — stable public error category.
- Modify: compile/calculator/src/lib.rs — export interpreter and WASM modules.
- Create: compile/calculator/src/interpreter.rs — reusable parse/evaluate/environment facade.
- Create: compile/calculator/src/wasm.rs — wasm-bindgen interpreter and JS value bridge.
- Create: compile/calculator/tests/builtin.rs — callback identity and invocation tests.
- Create: compile/calculator/tests/interpreter.rs — fresh/reset environment tests.
- Create: compile/calculator/tests/wasm.rs — browser-targeted host bridge tests.

Frontend files:

- Create: compile/calculator/web/package.json — Bun scripts and dependencies.
- Create: compile/calculator/web/bunfig.toml — Bun test configuration.
- Create: compile/calculator/web/tsconfig.json — strict browser/worker TypeScript settings.
- Create: compile/calculator/web/vite.config.ts — relative-base Vite build.
- Create: compile/calculator/web/index.html — Visual Studio shell.
- Create: compile/calculator/web/src/vite-env.d.ts — Vite raw import and Worker typings.
- Create: compile/calculator/web/src/styles.css — shell and responsive split styling.
- Create: compile/calculator/web/src/main.ts — Monaco and UI composition root.
- Create: compile/calculator/web/src/calclang/host.ts — JavaScript constants and host functions.
- Create: compile/calculator/web/src/calclang/host.test.ts — host semantics tests.
- Create: compile/calculator/web/src/calclang/prelude.calc — min/max definitions.
- Create: compile/calculator/web/src/calclang/language-data.ts — EBNF-derived language metadata.
- Create: compile/calculator/web/src/calclang/language-data.test.ts — tokenizer/completion inventory tests.
- Create: compile/calculator/web/src/calclang/language.ts — Monaco registration.
- Create: compile/calculator/web/src/runtime/protocol.ts — typed main/Worker messages.
- Create: compile/calculator/web/src/runtime/output-batcher.ts — 100-entry Worker batches.
- Create: compile/calculator/web/src/runtime/output-batcher.test.ts — batching tests.
- Create: compile/calculator/web/src/runtime/calclang.worker.ts — WASM loading and evaluation.
- Create: compile/calculator/web/src/runtime/run-controller.ts — Worker lifecycle and timeout.
- Create: compile/calculator/web/src/runtime/run-controller.test.ts — lifecycle tests.
- Create: compile/calculator/web/src/ui/output-window.ts — rolling Output/Problems DOM.
- Create: compile/calculator/web/src/ui/output-window.test.ts — 10,000-entry rolling-window tests.
- Create: compile/calculator/web/src/ui/shell.test.ts — required shell structure test.
- Create: compile/calculator/web/src/sample.calc — initial playground source.
- Modify: README.md — Bun development and production build commands.

Generated but not committed:

- compile/calculator/web/src/generated/calclang/ — wasm-pack output.
- compile/calculator/web/dist/ — Vite production output.

### Task 1: Make Rust built-ins callback-capable

**Files:**

- Modify: compile/calculator/src/ast.rs
- Modify: compile/calculator/src/env.rs
- Create: compile/calculator/tests/builtin.rs

- [ ] **Step 1: Write the failing built-in tests**

Create compile/calculator/tests/builtin.rs:

~~~rust
use calclang::{
    ast::{BuiltIn, Value},
    env::Environment,
};

#[test]
fn captured_callback_is_callable_through_environment() {
    let offset = 2.0;
    let env = Environment::new();
    env.borrow_mut().define_builtin("add_offset", move |values| {
        let Some(Value::Float(value)) = values.first() else {
            panic!("expected float");
        };
        Ok(Value::Float(*value + offset))
    });

    let Value::BuiltIn(callback) = env.borrow().get("add_offset").unwrap() else {
        panic!("expected built-in");
    };

    assert_eq!(
        callback.call(vec![Value::Float(40.0)]).unwrap(),
        Value::Float(42.0)
    );
}

#[test]
fn cloned_builtin_keeps_callback_identity() {
    let callback = BuiltIn::new(|_| Ok(Value::None));
    assert_eq!(callback, callback.clone());
    assert_ne!(callback, BuiltIn::new(|_| Ok(Value::None)));
}
~~~

- [ ] **Step 2: Run the test and verify RED**

Run:

~~~text
cargo test -p calculator --test builtin
~~~

Expected: compilation fails because BuiltIn has no new or call method and define_builtin accepts only a function pointer.

- [ ] **Step 3: Replace the BuiltIn alias with a callback wrapper**

In compile/calculator/src/ast.rs, replace the function-pointer alias with:

~~~rust
#[derive(Clone)]
pub struct BuiltIn {
    callback: Rc<dyn Fn(Vec<Value>) -> Result<Value>>,
}

impl BuiltIn {
    pub fn new<F>(callback: F) -> Self
    where
        F: Fn(Vec<Value>) -> Result<Value> + 'static,
    {
        Self {
            callback: Rc::new(callback),
        }
    }

    pub fn call(&self, values: Vec<Value>) -> Result<Value> {
        (self.callback)(values)
    }
}

impl Debug for BuiltIn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("BuiltIn").finish()
    }
}

impl PartialEq for BuiltIn {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.callback, &other.callback)
    }
}
~~~

Add this Value equality arm:

~~~rust
(Value::BuiltIn(a), Value::BuiltIn(b)) => a == b,
~~~

In ASTNode::Call, replace direct function-pointer invocation with:

~~~rust
Value::BuiltIn(built_in) => return built_in.call(evaluated_vals),
~~~

In compile/calculator/src/env.rs, replace define_builtin with:

~~~rust
pub fn define_builtin<F>(&mut self, name: &str, callback: F)
where
    F: Fn(Vec<Value>) -> Result<Value> + 'static,
{
    self.store.insert(
        name.into(),
        Value::BuiltIn(BuiltIn::new(callback)),
    );
}
~~~

- [ ] **Step 4: Run focused and native regression tests**

Run:

~~~text
cargo test -p calculator --test builtin
cargo test -p calculator
~~~

Expected: both commands pass, including native main.rs built-ins.

- [ ] **Step 5: Commit**

~~~text
git add compile/calculator/src/ast.rs compile/calculator/src/env.rs compile/calculator/tests/builtin.rs
git commit -m "refactor(calculator): Support callback built-ins"
~~~

### Task 2: Add a reusable interpreter facade

**Files:**

- Create: compile/calculator/src/interpreter.rs
- Create: compile/calculator/tests/interpreter.rs
- Modify: compile/calculator/src/ctx.rs
- Modify: compile/calculator/src/error.rs
- Modify: compile/calculator/src/lib.rs

- [ ] **Step 1: Write failing facade tests**

Create compile/calculator/tests/interpreter.rs:

~~~rust
use calclang::{
    ast::Value,
    error::Error,
    interpreter::Interpreter,
};

#[test]
fn evaluations_share_environment_until_reset() {
    let mut interpreter = Interpreter::new();
    interpreter.eval("let answer = 40;").unwrap();

    assert_eq!(
        interpreter.eval("answer + 2").unwrap(),
        Value::Float(42.0)
    );

    interpreter.reset();
    assert!(matches!(
        interpreter.eval("answer"),
        Err(Error::Name(_))
    ));
}

#[test]
fn constants_and_callbacks_can_be_injected() {
    let mut interpreter = Interpreter::new();
    interpreter.define("PI", Value::Float(std::f64::consts::PI)).unwrap();
    interpreter.define_builtin("twice", |values| {
        let Some(Value::Float(value)) = values.first() else {
            return Ok(Value::None);
        };
        Ok(Value::Float(*value * 2.0))
    });

    assert_eq!(
        interpreter.eval("twice(PI)").unwrap(),
        Value::Float(std::f64::consts::PI * 2.0)
    );
}

#[test]
fn errors_expose_stable_categories() {
    assert_eq!(Error::Name("missing".into()).category(), "NameError");
}
~~~

- [ ] **Step 2: Run the test and verify RED**

Run:

~~~text
cargo test -p calculator --test interpreter
~~~

Expected: compilation fails because the interpreter module and Error::category do not exist.

- [ ] **Step 3: Implement the facade**

Create compile/calculator/src/interpreter.rs:

~~~rust
use std::{cell::RefCell, rc::Rc};

use crate::{
    ast::Value,
    ctx::Context,
    env::Environment,
    error::Result,
    lexer::Lexer,
    parser::Parser,
};

#[derive(Debug)]
pub struct Interpreter {
    env: Rc<RefCell<Environment>>,
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

impl Interpreter {
    pub fn new() -> Self {
        Self {
            env: Environment::new(),
        }
    }

    pub fn reset(&mut self) {
        self.env = Environment::new();
    }

    pub fn define(&mut self, name: &str, value: Value) -> Result<()> {
        self.env.borrow_mut().define(name, Some(value))
    }

    pub fn define_builtin<F>(&mut self, name: &str, callback: F)
    where
        F: Fn(Vec<Value>) -> Result<Value> + 'static,
    {
        self.env.borrow_mut().define_builtin(name, callback);
    }

    pub fn eval(&self, source: &str) -> Result<Value> {
        let tokens = Lexer::tokenize(source)?;
        let mut parser = Parser::new(tokens);
        let ast = parser.parse()?;
        ast.eval(self.env.clone(), Context::default())
    }
}
~~~

Add to compile/calculator/src/lib.rs:

~~~rust
pub mod interpreter;
~~~

Add to impl Error in compile/calculator/src/error.rs:

~~~rust
impl Error {
    pub fn category(&self) -> &'static str {
        match self {
            Self::Syntax(_) => "SyntaxError",
            Self::Type(_) => "TypeError",
            Self::Name(_) => "NameError",
            Self::Runtime(_) => "RuntimeError",
            Self::Internal(_) => "InternalError",
        }
    }

    pub fn message(&self) -> String {
        match self {
            Self::Syntax(message)
            | Self::Type(message)
            | Self::Name(message)
            | Self::Runtime(message) => message.clone(),
            Self::Internal(_) => "Internal interpreter control flow escaped".into(),
        }
    }
}
~~~

In compile/calculator/src/ctx.rs, replace the manual Default implementation with:

~~~rust
#[derive(Debug, Clone, Default)]
pub struct Context {
    pub is_loop: bool,
    pub depth: usize,
}
~~~

- [ ] **Step 4: Verify GREEN**

Run:

~~~text
cargo test -p calculator --test interpreter
cargo test -p calculator
~~~

Expected: all tests pass.

- [ ] **Step 5: Commit**

~~~text
git add compile/calculator/src/interpreter.rs compile/calculator/src/ctx.rs compile/calculator/src/error.rs compile/calculator/src/lib.rs compile/calculator/tests/interpreter.rs
git commit -m "feat(calculator): Add interpreter facade"
~~~

### Task 3: Export the interpreter with wasm-bindgen

**Files:**

- Modify: compile/calculator/Cargo.toml
- Modify: compile/calculator/src/lib.rs
- Create: compile/calculator/src/wasm.rs
- Create: compile/calculator/tests/wasm.rs

- [ ] **Step 1: Add target-specific test dependencies and the failing browser test**

Add to compile/calculator/Cargo.toml:

~~~toml
[[bin]]
name = "calculator"
path = "src/main.rs"
required-features = ["native-cli"]

[features]
default = ["native-cli"]
native-cli = []

[target.'cfg(target_arch = "wasm32")'.dependencies]
js-sys = "0.3"
wasm-bindgen = "0.2"

[target.'cfg(target_arch = "wasm32")'.dev-dependencies]
wasm-bindgen-test = "0.3"
~~~

Create compile/calculator/tests/wasm.rs:

~~~rust
#![cfg(target_arch = "wasm32")]

use calclang::wasm::WasmInterpreter;
use js_sys::Function;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn javascript_host_function_is_callable() {
    let mut interpreter = WasmInterpreter::new();
    let double = Function::new_with_args("value", "return value * 2;");
    interpreter.define_host_function("double", double);

    let result = interpreter.evaluate("double(21)").unwrap();
    assert_eq!(result.as_f64(), Some(42.0));
}

#[wasm_bindgen_test]
fn javascript_exception_becomes_runtime_error() {
    let mut interpreter = WasmInterpreter::new();
    let fail = Function::new_no_args("throw new Error('host failed');");
    interpreter.define_host_function("fail", fail);

    let error = interpreter.evaluate("fail()").unwrap_err();
    let category = js_sys::Reflect::get(&error, &"category".into()).unwrap();
    assert_eq!(category.as_string().as_deref(), Some("RuntimeError"));
}

#[wasm_bindgen_test]
fn primitive_constants_cross_the_bridge() {
    let mut interpreter = WasmInterpreter::new();
    interpreter.define_value("PI", std::f64::consts::PI.into()).unwrap();
    assert_eq!(
        interpreter.evaluate("PI").unwrap().as_f64(),
        Some(std::f64::consts::PI)
    );
}

#[wasm_bindgen_test]
fn unsupported_host_return_becomes_type_error() {
    let mut interpreter = WasmInterpreter::new();
    let object = Function::new_no_args("return {};");
    interpreter.define_host_function("object", object);

    let error = interpreter.evaluate("object()").unwrap_err();
    let category = js_sys::Reflect::get(&error, &"category".into()).unwrap();
    assert_eq!(category.as_string().as_deref(), Some("TypeError"));
}
~~~

Add to compile/calculator/src/lib.rs:

~~~rust
#[cfg(target_arch = "wasm32")]
pub mod wasm;
~~~

- [ ] **Step 2: Ensure the Rust WASM build tool is available**

Run:

~~~text
wasm-pack --version
~~~

If it is missing, request approval for the global Cargo installation and run:

~~~text
cargo install wasm-pack --locked
~~~

Expected: wasm-pack reports its installed version. This is a Rust tool and does not install or invoke Node.

- [ ] **Step 3: Run the browser test and verify RED**

Run:

~~~text
wasm-pack test --headless --chrome compile/calculator --no-default-features
~~~

Expected: compilation fails because WasmInterpreter is not implemented.

- [ ] **Step 4: Implement the WASM bridge**

Create compile/calculator/src/wasm.rs with:

~~~rust
use js_sys::{Array, Function, Object, Reflect};
use wasm_bindgen::prelude::*;

use crate::{
    ast::Value,
    error::{self, Error},
    interpreter::Interpreter,
};

#[wasm_bindgen]
#[derive(Default)]
pub struct WasmInterpreter {
    inner: Interpreter,
}

#[wasm_bindgen]
impl WasmInterpreter {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: Interpreter::new(),
        }
    }

    pub fn reset(&mut self) {
        self.inner.reset();
    }

    pub fn define_value(&mut self, name: &str, value: JsValue) -> Result<(), JsValue> {
        let value = js_to_value(value).map_err(error_to_js)?;
        self.inner.define(name, value).map_err(error_to_js)
    }

    pub fn define_host_function(&mut self, name: &str, function: Function) {
        self.inner.define_builtin(name, move |values| {
            let args = Array::new();
            for value in &values {
                args.push(&value_to_js(value));
            }

            let result = function
                .apply(&JsValue::UNDEFINED, &args)
                .map_err(|value| {
                    let message = js_error_message(&value);
                    error::error!(Runtime, "{message}")
                })?;

            js_to_value(result)
        });
    }

    pub fn evaluate(&self, source: &str) -> Result<JsValue, JsValue> {
        self.inner
            .eval(source)
            .map(|value| value_to_js(&value))
            .map_err(error_to_js)
    }
}

fn js_to_value(value: JsValue) -> crate::error::Result<Value> {
    if value.is_null() || value.is_undefined() {
        Ok(Value::None)
    } else if let Some(value) = value.as_f64() {
        Ok(Value::Float(value))
    } else if let Some(value) = value.as_bool() {
        Ok(Value::Bool(value))
    } else if let Some(value) = value.as_string() {
        Ok(Value::String(value))
    } else {
        Err(error::error!(
            Type,
            "Host function returned an unsupported JavaScript value"
        ))
    }
}

fn value_to_js(value: &Value) -> JsValue {
    match value {
        Value::Float(value) => JsValue::from_f64(*value),
        Value::Bool(value) => JsValue::from_bool(*value),
        Value::String(value) => JsValue::from_str(value),
        Value::None => JsValue::UNDEFINED,
        Value::Fn(_, _, _) | Value::BuiltIn(_) => {
            let descriptor = Object::new();
            Reflect::set(
                &descriptor,
                &"__calclangType".into(),
                &value.type_name().into(),
            )
            .expect("plain object property assignment");
            descriptor.into()
        }
    }
}

fn js_error_message(value: &JsValue) -> String {
    Reflect::get(value, &"message".into())
        .ok()
        .and_then(|message| message.as_string())
        .or_else(|| value.as_string())
        .unwrap_or_else(|| "JavaScript host function failed".into())
}

fn error_to_js(error: Error) -> JsValue {
    let object = Object::new();
    Reflect::set(
        &object,
        &"category".into(),
        &error.category().into(),
    )
    .expect("plain object property assignment");
    Reflect::set(
        &object,
        &"message".into(),
        &error.message().into(),
    )
    .expect("plain object property assignment");
    object.into()
}
~~~

- [ ] **Step 5: Verify native and browser builds**

Run:

~~~text
cargo test -p calculator
wasm-pack test --headless --chrome compile/calculator --no-default-features
cargo build -p calculator --lib --target wasm32-unknown-unknown --release
~~~

Expected: all commands pass and the release WASM now contains callable wasm-bindgen exports before post-processing.

- [ ] **Step 6: Commit**

~~~text
git add compile/calculator/Cargo.toml compile/calculator/src/lib.rs compile/calculator/src/wasm.rs compile/calculator/tests/wasm.rs Cargo.lock
git commit -m "feat(calculator): Export WASM interpreter"
~~~

### Task 4: Scaffold the Bun and Vite frontend

**Files:**

- Create: compile/calculator/web/package.json
- Create: compile/calculator/web/bunfig.toml
- Create: compile/calculator/web/tsconfig.json
- Create: compile/calculator/web/vite.config.ts
- Create: compile/calculator/web/index.html
- Create: compile/calculator/web/src/vite-env.d.ts
- Create: compile/calculator/web/src/main.ts
- Create: compile/calculator/web/src/styles.css
- Create: .gitignore

- [ ] **Step 1: Create the package manifest**

Create compile/calculator/web/package.json:

~~~json
{
  "name": "calclang-playground",
  "private": true,
  "type": "module",
  "scripts": {
    "wasm": "wasm-pack build .. --target web --out-dir web/src/generated/calclang --out-name calclang --release --no-default-features",
    "dev": "bun run wasm && vite",
    "test": "bun test",
    "build": "bun run wasm && tsc --noEmit && vite build"
  },
  "dependencies": {
    "monaco-editor": "latest"
  },
  "devDependencies": {
    "@types/bun": "latest",
    "happy-dom": "latest",
    "typescript": "latest",
    "vite": "latest"
  }
}
~~~

Create compile/calculator/web/bunfig.toml:

~~~toml
[test]
root = "src"
~~~

- [ ] **Step 2: Install with Bun and record the lockfile**

Run from compile/calculator/web:

~~~text
bun install
~~~

Expected: Bun creates bun.lock and installs Monaco, Vite, TypeScript, and happy-dom.

- [ ] **Step 3: Add strict TypeScript and Vite configuration**

Create compile/calculator/web/tsconfig.json:

~~~json
{
  "compilerOptions": {
    "target": "ES2022",
    "useDefineForClassFields": true,
    "module": "ESNext",
    "moduleResolution": "Bundler",
    "strict": true,
    "noUncheckedIndexedAccess": true,
    "lib": ["ES2022", "DOM", "WebWorker"],
    "types": ["bun", "vite/client"],
    "skipLibCheck": true,
    "noEmit": true
  },
  "include": ["src", "vite.config.ts"]
}
~~~

Create compile/calculator/web/vite.config.ts:

~~~ts
import { defineConfig } from "vite";

export default defineConfig({
  base: "./",
  worker: {
    format: "es",
  },
});
~~~

Create compile/calculator/web/src/vite-env.d.ts:

~~~ts
/// <reference types="vite/client" />

declare module "*.calc?raw" {
  const source: string;
  export default source;
}
~~~

- [ ] **Step 4: Add the smallest buildable entry point**

Create compile/calculator/web/index.html:

~~~html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Calclang Playground</title>
  </head>
  <body>
    <main id="app">Calclang Playground</main>
    <script type="module" src="/src/main.ts"></script>
  </body>
</html>
~~~

Create compile/calculator/web/src/main.ts:

~~~ts
import "./styles.css";
~~~

Create compile/calculator/web/src/styles.css:

~~~css
html,
body,
#app {
  width: 100%;
  height: 100%;
  margin: 0;
}

body {
  background: #1e1e1e;
  color: #d8d8d8;
  font-family: "Segoe UI", sans-serif;
}
~~~

Create or extend .gitignore:

~~~text
.superpowers/
compile/calculator/web/node_modules/
compile/calculator/web/dist/
compile/calculator/web/src/generated/
~~~

- [ ] **Step 5: Verify the scaffold**

Run from compile/calculator/web:

~~~text
bun test
bun run wasm
bun run build
~~~

Expected: zero Bun test failures, wasm-pack generates bindings, and Vite produces dist without TypeScript errors.

- [ ] **Step 6: Commit**

~~~text
git add .gitignore compile/calculator/web/package.json compile/calculator/web/bun.lock compile/calculator/web/bunfig.toml compile/calculator/web/tsconfig.json compile/calculator/web/vite.config.ts compile/calculator/web/index.html compile/calculator/web/src/vite-env.d.ts compile/calculator/web/src/main.ts compile/calculator/web/src/styles.css
git commit -m "build(calculator): Scaffold Bun playground"
~~~

### Task 5: Define the JavaScript host environment and prelude

**Files:**

- Create: compile/calculator/web/src/calclang/host.ts
- Create: compile/calculator/web/src/calclang/host.test.ts
- Create: compile/calculator/web/src/calclang/prelude.calc

- [ ] **Step 1: Write failing host tests**

Create compile/calculator/web/src/calclang/host.test.ts:

~~~ts
import { describe, expect, test } from "bun:test";
import {
  HOST_CONSTANTS,
  createHostFunctions,
} from "./host";

describe("host environment", () => {
  test("delegates numeric functions to Math", () => {
    const functions = createHostFunctions(() => {});
    expect(functions.sin(Math.PI / 2)).toBe(1);
    expect(functions.sqrt(9)).toBe(3);
  });

  test("println concatenates with calclang display semantics", () => {
    const lines: string[] = [];
    const functions = createHostFunctions((line) => lines.push(line));
    functions.println("answer=", 42, undefined);
    expect(lines).toEqual(["answer=42"]);
  });

  test("typeof understands primitives and function descriptors", () => {
    const functions = createHostFunctions(() => {});
    expect(functions.typeof(1)).toBe("float");
    expect(functions.typeof(undefined)).toBe("none");
    expect(functions.typeof({ __calclangType: "fn(x)" })).toBe("fn(x)");
  });

  test("exports all requested Math constants", () => {
    expect(HOST_CONSTANTS).toEqual({
      PI: Math.PI,
      E: Math.E,
      SQRT2: Math.SQRT2,
      SQRT1_2: Math.SQRT1_2,
      LN2: Math.LN2,
      LN10: Math.LN10,
      LOG2E: Math.LOG2E,
      LOG10E: Math.LOG10E,
    });
  });
});
~~~

- [ ] **Step 2: Run the test and verify RED**

Run from compile/calculator/web:

~~~text
bun test src/calclang/host.test.ts
~~~

Expected: FAIL because host.ts does not exist.

- [ ] **Step 3: Implement host functions**

Create compile/calculator/web/src/calclang/host.ts:

~~~ts
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
  return typeof value === "object"
    && value !== null
    && "__calclangType" in value
    && typeof value.__calclangType === "string";
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

export function createHostFunctions(
  emit: (line: string) => void,
) {
  return {
    println: (...values) => {
      emit(values.map(display).join(""));
      return undefined;
    },
    typeof: (...values) => {
      if (values.length !== 1) {
        throw new Error(
          "Expect one arg in typeof, found " + values.length,
        );
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
~~~

Create compile/calculator/web/src/calclang/prelude.calc:

~~~calc
fn min(a, b) {
    if a < b { a } else { b }
}

fn max(a, b) {
    if a > b { a } else { b }
}
~~~

- [ ] **Step 4: Verify GREEN**

Run:

~~~text
bun test src/calclang/host.test.ts
~~~

Expected: 4 tests pass.

- [ ] **Step 5: Commit**

~~~text
git add compile/calculator/web/src/calclang
git commit -m "feat(calculator): Add JavaScript host API"
~~~

### Task 6: Register EBNF-based Monaco language support

**Files:**

- Create: compile/calculator/web/src/calclang/language-data.ts
- Create: compile/calculator/web/src/calclang/language-data.test.ts
- Create: compile/calculator/web/src/calclang/language.ts

- [ ] **Step 1: Write failing language-data tests**

Create compile/calculator/web/src/calclang/language-data.test.ts:

~~~ts
import { describe, expect, test } from "bun:test";
import {
  COMPLETION_NAMES,
  FLOAT_LITERAL,
  KEYWORDS,
  WORD_OPERATORS,
} from "./language-data";

describe("calclang language data", () => {
  test("matches every EBNF float form", () => {
    for (const value of ["0", "12", "1.", ".5", "1.25", "1e3", ".5E-2"]) {
      expect(new RegExp("^(?:" + FLOAT_LITERAL.source + ")$").test(value)).toBe(true);
    }
  });

  test("rejects incomplete float forms", () => {
    for (const value of [".", "1e", "1e+", "1.2.3"]) {
      expect(new RegExp("^(?:" + FLOAT_LITERAL.source + ")$").test(value)).toBe(false);
    }
  });

  test("contains every grammar keyword and word operator", () => {
    expect(KEYWORDS).toEqual([
      "let", "if", "else", "loop", "break", "continue", "fn", "return",
      "true", "false", "none",
    ]);
    expect(WORD_OPERATORS).toEqual(["and", "or"]);
  });

  test("completes host and prelude names", () => {
    for (const name of ["println", "typeof", "PI", "sin", "random", "min", "max"]) {
      expect(COMPLETION_NAMES).toContain(name);
    }
  });
});
~~~

- [ ] **Step 2: Run the test and verify RED**

Run:

~~~text
bun test src/calclang/language-data.test.ts
~~~

Expected: FAIL because language-data.ts does not exist.

- [ ] **Step 3: Implement plain language metadata**

Create compile/calculator/web/src/calclang/language-data.ts:

~~~ts
export const KEYWORDS = [
  "let", "if", "else", "loop", "break", "continue", "fn", "return",
  "true", "false", "none",
] as const;

export const WORD_OPERATORS = ["and", "or"] as const;

export const FLOAT_LITERAL =
  /(?:[0-9]+(?:\.[0-9]*)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?/;

export interface CompletionSpec {
  label: string;
  detail: string;
  insertText?: string;
  snippet?: boolean;
}

export const COMPLETIONS: readonly CompletionSpec[] = [
  ...KEYWORDS.map((label) => ({ label, detail: "calclang keyword" })),
  { label: "and", detail: "logical and" },
  { label: "or", detail: "logical or" },
  { label: "let", detail: "variable definition", insertText: "let ${1:name} = ${2:value};", snippet: true },
  { label: "if", detail: "conditional expression", insertText: "if ${1:condition} {\n\t${2}\n}", snippet: true },
  { label: "ifelse", detail: "conditional with else", insertText: "if ${1:condition} {\n\t${2}\n} else {\n\t${3}\n}", snippet: true },
  { label: "loop", detail: "loop expression", insertText: "loop {\n\t${1}\n}", snippet: true },
  { label: "fn", detail: "named function", insertText: "fn ${1:name}(${2:args}) {\n\t${3}\n}", snippet: true },
  { label: "anonymous fn", detail: "anonymous function", insertText: "fn(${1:args}) {\n\t${2}\n}", snippet: true },
  { label: "return", detail: "return expression", insertText: "return ${1:value};", snippet: true },
  { label: "println", detail: "println(...values): none" },
  { label: "typeof", detail: "typeof(value): string" },
  ...["PI", "E", "SQRT2", "SQRT1_2", "LN2", "LN10", "LOG2E", "LOG10E"]
    .map((label) => ({ label, detail: "JavaScript Math constant" })),
  ...["sin", "cos", "tan", "asin", "acos", "atan", "atan2", "sinh", "cosh", "tanh",
    "abs", "sqrt", "cbrt", "exp", "log", "log2", "log10", "pow", "floor", "ceil",
    "round", "trunc", "sign", "random"]
    .map((label) => ({ label, detail: "JavaScript Math host function" })),
  { label: "min", detail: "min(a, b): float" },
  { label: "max", detail: "max(a, b): float" },
];

export const COMPLETION_NAMES = COMPLETIONS.map(({ label }) => label);
~~~

- [ ] **Step 4: Register Monaco without changing its theme CSS**

Create compile/calculator/web/src/calclang/language.ts:

~~~ts
import type * as Monaco from "monaco-editor";
import {
  COMPLETIONS,
  FLOAT_LITERAL,
  KEYWORDS,
  WORD_OPERATORS,
} from "./language-data";

export function registerCalclang(monaco: typeof Monaco): void {
  monaco.languages.register({ id: "calclang" });

  monaco.languages.setLanguageConfiguration("calclang", {
    comments: { lineComment: "//", blockComment: ["/*", "*/"] },
    brackets: [["{", "}"], ["(", ")"]],
    autoClosingPairs: [
      { open: "{", close: "}" },
      { open: "(", close: ")" },
      { open: "\"", close: "\"", notIn: ["string", "comment"] },
    ],
    surroundingPairs: [
      { open: "{", close: "}" },
      { open: "(", close: ")" },
      { open: "\"", close: "\"" },
    ],
    indentationRules: {
      increaseIndentPattern: /\{[^}]*$/,
      decreaseIndentPattern: /^\s*\}/,
    },
  });

  monaco.languages.setMonarchTokensProvider("calclang", {
    keywords: [...KEYWORDS],
    wordOperators: [...WORD_OPERATORS],
    operators: ["+", "-", "*", "/", "^", "=", "==", "!=", "<", "<=", ">", ">=", "!"],
    tokenizer: {
      root: [
        [/[a-zA-Z_][a-zA-Z0-9_]*/, {
          cases: {
            "@keywords": "keyword",
            "@wordOperators": "operator",
            "@default": "identifier",
          },
        }],
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
        [/[^\\\"\n]+/, "string"],
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
          kind: item.snippet
            ? monaco.languages.CompletionItemKind.Snippet
            : monaco.languages.CompletionItemKind.Function,
          insertText: item.insertText ?? item.label,
          insertTextRules: item.snippet
            ? monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet
            : undefined,
          range,
        })),
      };
    },
  });
}
~~~

- [ ] **Step 5: Verify GREEN and type checking**

Run:

~~~text
bun test src/calclang/language-data.test.ts
bunx tsc --noEmit
~~~

Expected: 4 tests pass and TypeScript reports no errors.

- [ ] **Step 6: Commit**

~~~text
git add compile/calculator/web/src/calclang/language-data.ts compile/calculator/web/src/calclang/language-data.test.ts compile/calculator/web/src/calclang/language.ts
git commit -m "feat(calculator): Add Monaco language support"
~~~

### Task 7: Implement batched and rolling output

**Files:**

- Create: compile/calculator/web/src/runtime/output-batcher.ts
- Create: compile/calculator/web/src/runtime/output-batcher.test.ts
- Create: compile/calculator/web/src/ui/output-window.ts
- Create: compile/calculator/web/src/ui/output-window.test.ts

- [ ] **Step 1: Write failing batching tests**

Create compile/calculator/web/src/runtime/output-batcher.test.ts:

~~~ts
import { expect, test } from "bun:test";
import { OutputBatcher } from "./output-batcher";

test("posts every one hundred entries in order", () => {
  const batches: string[][] = [];
  const batcher = new OutputBatcher((entries) => batches.push(entries));

  for (let index = 0; index < 205; index += 1) {
    batcher.push(String(index));
  }
  batcher.flush();

  expect(batches.map((batch) => batch.length)).toEqual([100, 100, 5]);
  expect(batches.flat()).toEqual(
    Array.from({ length: 205 }, (_, index) => String(index)),
  );
});
~~~

Create compile/calculator/web/src/ui/output-window.test.ts:

~~~ts
import { expect, test } from "bun:test";
import { Window } from "happy-dom";
import { OutputWindow } from "./output-window";

test("keeps the newest ten thousand DOM entries", () => {
  const window = new Window();
  const container = window.document.createElement("div");
  const output = new OutputWindow(container as unknown as HTMLElement);

  output.append(
    Array.from({ length: 10_005 }, (_, index) => "line-" + index),
  );

  expect(container.childElementCount).toBe(10_000);
  expect(container.firstElementChild?.textContent).toBe("line-5");
  expect(container.lastElementChild?.textContent).toBe("line-10004");
});
~~~

- [ ] **Step 2: Run the tests and verify RED**

Run:

~~~text
bun test src/runtime/output-batcher.test.ts src/ui/output-window.test.ts
~~~

Expected: FAIL because both modules are missing.

- [ ] **Step 3: Implement batching**

Create compile/calculator/web/src/runtime/output-batcher.ts:

~~~ts
export class OutputBatcher {
  readonly #entries: string[] = [];

  constructor(
    private readonly emit: (entries: string[]) => void,
    private readonly batchSize = 100,
  ) {}

  push(entry: string): void {
    this.#entries.push(entry);
    if (this.#entries.length >= this.batchSize) {
      this.flush();
    }
  }

  flush(): void {
    if (this.#entries.length === 0) {
      return;
    }
    this.emit(this.#entries.splice(0));
  }
}
~~~

- [ ] **Step 4: Implement rolling DOM output**

Create compile/calculator/web/src/ui/output-window.ts:

~~~ts
export class OutputWindow {
  constructor(
    private readonly container: HTMLElement,
    private readonly limit = 10_000,
  ) {}

  clear(): void {
    this.container.replaceChildren();
  }

  append(entries: readonly string[], className = "output-entry"): void {
    const fragment = document.createDocumentFragment();
    for (const entry of entries) {
      const row = document.createElement("div");
      row.className = className;
      row.textContent = entry;
      fragment.append(row);
    }
    this.container.append(fragment);

    while (this.container.childElementCount > this.limit) {
      this.container.firstElementChild?.remove();
    }
    this.container.scrollTop = this.container.scrollHeight;
  }
}
~~~

In tests, temporarily assign happy-dom's document around append:

~~~ts
Object.defineProperty(globalThis, "document", {
  configurable: true,
  value: window.document,
});
~~~

- [ ] **Step 5: Verify GREEN**

Run:

~~~text
bun test src/runtime/output-batcher.test.ts src/ui/output-window.test.ts
~~~

Expected: 2 tests pass.

- [ ] **Step 6: Commit**

~~~text
git add compile/calculator/web/src/runtime/output-batcher.ts compile/calculator/web/src/runtime/output-batcher.test.ts compile/calculator/web/src/ui/output-window.ts compile/calculator/web/src/ui/output-window.test.ts
git commit -m "feat(calculator): Add rolling output pipeline"
~~~

### Task 8: Implement the Worker protocol and WASM runtime

**Files:**

- Create: compile/calculator/web/src/runtime/protocol.ts
- Create: compile/calculator/web/src/runtime/calclang.worker.ts

- [ ] **Step 1: Define the message contract**

Create compile/calculator/web/src/runtime/protocol.ts:

~~~ts
export interface RunRequest {
  type: "run";
  runId: number;
  source: string;
  preserveEnvironment: boolean;
}

export type WorkerRequest = RunRequest;

export type WorkerResponse =
  | { type: "output"; runId: number; entries: string[] }
  | { type: "complete"; runId: number; result?: string }
  | { type: "problem"; runId: number; category: string; message: string };

export function isWorkerResponse(value: unknown): value is WorkerResponse {
  if (typeof value !== "object" || value === null || !("type" in value)) {
    return false;
  }
  const type = (value as { type: unknown }).type;
  return type === "output" || type === "complete" || type === "problem";
}
~~~

- [ ] **Step 2: Implement the Worker**

Create compile/calculator/web/src/runtime/calclang.worker.ts:

~~~ts
/// <reference lib="webworker" />

import init, {
  WasmInterpreter,
} from "../generated/calclang/calclang";
import prelude from "../calclang/prelude.calc?raw";
import {
  HOST_CONSTANTS,
  createHostFunctions,
} from "../calclang/host";
import { OutputBatcher } from "./output-batcher";
import type {
  RunRequest,
  WorkerResponse,
} from "./protocol";

const scope = self as DedicatedWorkerGlobalScope;
const wasmReady = init();
let interpreter: WasmInterpreter | undefined;

function post(message: WorkerResponse): void {
  scope.postMessage(message);
}

function createInterpreter(batcher: OutputBatcher): WasmInterpreter {
  const runtime = new WasmInterpreter();
  for (const [name, value] of Object.entries(HOST_CONSTANTS)) {
    runtime.define_value(name, value);
  }
  for (const [name, callback] of Object.entries(
    createHostFunctions((line) => batcher.push(line)),
  )) {
    runtime.define_host_function(name, callback);
  }
  runtime.evaluate(prelude);
  return runtime;
}

scope.onmessage = async (event: MessageEvent<RunRequest>) => {
  const request = event.data;
  if (request.type !== "run") {
    return;
  }

  const batcher = new OutputBatcher((entries) => {
    post({ type: "output", runId: request.runId, entries });
  });

  try {
    await wasmReady;
    if (!request.preserveEnvironment || interpreter === undefined) {
      interpreter = createInterpreter(batcher);
    } else {
      for (const [name, callback] of Object.entries(
        createHostFunctions((line) => batcher.push(line)),
      )) {
        interpreter.define_host_function(name, callback);
      }
    }

    const result = interpreter.evaluate(request.source);
    batcher.flush();
    const resultText =
      result === undefined || typeof result === "object"
        ? undefined
        : String(result);
    post({
      type: "complete",
      runId: request.runId,
      result: resultText,
    });
  } catch (value) {
    batcher.flush();
    const error = value as { category?: unknown; message?: unknown };
    post({
      type: "problem",
      runId: request.runId,
      category: typeof error.category === "string"
        ? error.category
        : "RuntimeError",
      message: typeof error.message === "string"
        ? error.message
        : "Unknown calclang runtime failure",
    });
  }
};
~~~

- [ ] **Step 3: Type-check against generated WASM bindings**

Run:

~~~text
bun run wasm
bunx tsc --noEmit
~~~

Expected: generated methods match Worker usage and TypeScript reports no errors.

- [ ] **Step 4: Commit**

~~~text
git add compile/calculator/web/src/runtime/protocol.ts compile/calculator/web/src/runtime/calclang.worker.ts
git commit -m "feat(calculator): Run WASM in a Web Worker"
~~~

### Task 9: Add main-thread run lifecycle control

**Files:**

- Create: compile/calculator/web/src/runtime/run-controller.ts
- Create: compile/calculator/web/src/runtime/run-controller.test.ts

- [ ] **Step 1: Write failing lifecycle tests**

Create compile/calculator/web/src/runtime/run-controller.test.ts with a FakeWorker implementing postMessage, terminate, and emit. Cover:

~~~ts
import { describe, expect, test } from "bun:test";
import { RunController, type WorkerLike } from "./run-controller";
import type { WorkerResponse } from "./protocol";

class FakeWorker implements WorkerLike {
  onmessage: ((event: MessageEvent<WorkerResponse>) => void) | null = null;
  readonly messages: unknown[] = [];
  terminated = false;

  postMessage(message: unknown): void {
    this.messages.push(message);
  }

  terminate(): void {
    this.terminated = true;
  }

  emit(message: WorkerResponse): void {
    this.onmessage?.({ data: message } as MessageEvent<WorkerResponse>);
  }
}

function setup(timeoutMs = 600_000) {
  const workers: FakeWorker[] = [];
  const output: string[] = [];
  const problems: string[] = [];
  const controller = new RunController({
    createWorker: () => {
      const worker = new FakeWorker();
      workers.push(worker);
      return worker;
    },
    timeoutMs,
    onReset: () => {
      output.length = 0;
      problems.length = 0;
    },
    onOutput: (entries) => output.push(...entries),
    onProblem: (category, message) => problems.push(category + ": " + message),
    onRunningChange: () => {},
  });
  return { controller, workers, output, problems };
}

describe("RunController", () => {
  test("a second run replaces an active worker", () => {
    const state = setup();
    state.controller.run("loop {}", false);
    state.controller.run("1 + 1", false);
    expect(state.workers[0]?.terminated).toBe(true);
    expect(state.workers).toHaveLength(2);
    state.controller.dispose();
  });

  test("manual stop terminates and reports to Output", () => {
    const state = setup();
    state.controller.run("loop {}", false);
    state.controller.stop();
    expect(state.workers[0]?.terminated).toBe(true);
    expect(state.output).toEqual(["Execution stopped by user."]);
  });

  test("timeout terminates and reports the required RuntimeError", async () => {
    const state = setup(5);
    state.controller.run("loop {}", false);
    await Bun.sleep(15);
    expect(state.workers[0]?.terminated).toBe(true);
    expect(state.output).toEqual([
      "RuntimeError: Loop execution exceeded the 10-minute time limit.",
    ]);
  });

  test("ignores stale run messages", () => {
    const state = setup();
    state.controller.run("1", false);
    const first = state.workers[0]!;
    state.controller.run("2", false);
    first.emit({ type: "output", runId: 1, entries: ["stale"] });
    expect(state.output).toEqual([]);
    state.controller.dispose();
  });
});
~~~

- [ ] **Step 2: Run and verify RED**

Run:

~~~text
bun test src/runtime/run-controller.test.ts
~~~

Expected: FAIL because run-controller.ts does not exist.

- [ ] **Step 3: Implement RunController**

Create compile/calculator/web/src/runtime/run-controller.ts implementing:

~~~ts
import {
  isWorkerResponse,
  type RunRequest,
  type WorkerResponse,
} from "./protocol";

export interface WorkerLike {
  onmessage: ((event: MessageEvent<WorkerResponse>) => void) | null;
  postMessage(message: unknown): void;
  terminate(): void;
}

export interface RunControllerOptions {
  createWorker: () => WorkerLike;
  timeoutMs?: number;
  onReset: () => void;
  onOutput: (entries: string[]) => void;
  onProblem: (category: string, message: string) => void;
  onRunningChange: (running: boolean) => void;
}

const TIMEOUT_MESSAGE =
  "RuntimeError: Loop execution exceeded the 10-minute time limit.";

export class RunController {
  readonly #options: Required<RunControllerOptions>;
  #worker: WorkerLike | undefined;
  #activeRunId: number | undefined;
  #nextRunId = 1;
  #timer: ReturnType<typeof setTimeout> | undefined;

  constructor(options: RunControllerOptions) {
    this.#options = {
      ...options,
      timeoutMs: options.timeoutMs ?? 600_000,
    };
  }

  run(source: string, preserveEnvironment = false): void {
    if (this.#activeRunId !== undefined) {
      this.#discardWorker();
    }
    this.#options.onReset();

    const worker = this.#worker ?? this.#createWorker();
    const runId = this.#nextRunId++;
    this.#activeRunId = runId;
    this.#options.onRunningChange(true);
    this.#timer = setTimeout(() => {
      if (this.#activeRunId !== runId) {
        return;
      }
      this.#discardWorker();
      this.#options.onOutput([TIMEOUT_MESSAGE]);
      this.#options.onRunningChange(false);
    }, this.#options.timeoutMs);

    const request: RunRequest = {
      type: "run",
      runId,
      source,
      preserveEnvironment,
    };
    worker.postMessage(request);
  }

  stop(): void {
    if (this.#activeRunId === undefined) {
      return;
    }
    this.#discardWorker();
    this.#options.onOutput(["Execution stopped by user."]);
    this.#options.onRunningChange(false);
  }

  dispose(): void {
    this.#discardWorker();
  }

  #createWorker(): WorkerLike {
    const worker = this.#options.createWorker();
    worker.onmessage = (event) => this.#handleMessage(event.data);
    this.#worker = worker;
    return worker;
  }

  #handleMessage(value: unknown): void {
    if (!isWorkerResponse(value) || value.runId !== this.#activeRunId) {
      return;
    }
    if (value.type === "output") {
      this.#options.onOutput(value.entries);
      return;
    }

    this.#clearActiveRun();
    this.#options.onRunningChange(false);
    if (value.type === "problem") {
      this.#options.onProblem(value.category, value.message);
    } else if (value.result !== undefined && value.result !== "") {
      this.#options.onOutput([value.result]);
    }
  }

  #clearActiveRun(): void {
    if (this.#timer !== undefined) {
      clearTimeout(this.#timer);
      this.#timer = undefined;
    }
    this.#activeRunId = undefined;
  }

  #discardWorker(): void {
    this.#clearActiveRun();
    this.#worker?.terminate();
    this.#worker = undefined;
  }
}
~~~

- [ ] **Step 4: Verify GREEN**

Run:

~~~text
bun test src/runtime/run-controller.test.ts
~~~

Expected: 4 lifecycle tests pass with no pending timers.

- [ ] **Step 5: Commit**

~~~text
git add compile/calculator/web/src/runtime/run-controller.ts compile/calculator/web/src/runtime/run-controller.test.ts
git commit -m "feat(calculator): Control playground runs"
~~~

### Task 10: Build the Visual Studio shell and compose Monaco

**Files:**

- Modify: compile/calculator/web/index.html
- Modify: compile/calculator/web/src/styles.css
- Modify: compile/calculator/web/src/main.ts
- Create: compile/calculator/web/src/ui/shell.test.ts
- Create: compile/calculator/web/src/sample.calc

- [ ] **Step 1: Write the failing shell contract test**

Create compile/calculator/web/src/ui/shell.test.ts:

~~~ts
import { expect, test } from "bun:test";

test("shell contains the approved controls and omits runtime internals", async () => {
  const html = await Bun.file(new URL("../../index.html", import.meta.url)).text();
  for (const id of [
    "editor", "run-button", "stop-button", "output",
    "problems", "output-tab", "problems-tab", "cursor-position",
  ]) {
    expect(html).toContain('id="' + id + '"');
  }
  expect(html).not.toContain("Process running in isolated Web Worker");
  expect(html).not.toContain("10:00");
});
~~~

- [ ] **Step 2: Run and verify RED**

Run:

~~~text
bun test src/ui/shell.test.ts
~~~

Expected: FAIL because the scaffold has none of the required shell elements.

- [ ] **Step 3: Replace index.html with the approved shell**

Use semantic buttons and tab panels with this hierarchy:

~~~html
<div id="app" class="vs-shell">
  <header class="title-bar">Calclang Playground — Microsoft Visual Studio</header>
  <nav class="menu-bar" aria-label="Application menu">
    <span>File</span><span>Edit</span><span>View</span><span>Run</span><span>Help</span>
  </nav>
  <div class="command-bar">
    <button id="run-button" type="button">▶ Run</button>
    <button id="stop-button" type="button" disabled>■ Stop</button>
    <span class="target">calclang.wasm</span>
  </div>
  <main class="workspace">
    <section class="editor-pane">
      <div class="document-tabs"><span id="dirty-indicator">●</span> main.calc</div>
      <div id="editor" aria-label="Calclang editor"></div>
    </section>
    <section class="tool-pane">
      <div class="tool-tabs" role="tablist">
        <button id="output-tab" role="tab" aria-selected="true">Output</button>
        <button id="problems-tab" role="tab" aria-selected="false">Problems <span id="problem-count">0</span></button>
      </div>
      <div id="output" class="tool-content" role="tabpanel"></div>
      <div id="problems" class="tool-content" role="tabpanel" hidden></div>
    </section>
  </main>
  <footer class="status-bar">
    <span>Fresh environment</span>
    <span id="wasm-status">WASM ready</span>
    <span id="cursor-position">Ln 1, Col 1</span>
    <span>Spaces: 4</span><span>UTF-8</span>
  </footer>
</div>
~~~

Keep the standard head, viewport meta, title, and module script from the scaffold.

- [ ] **Step 4: Implement the Visual Studio layout CSS**

In styles.css, implement:

~~~css
:root {
  color-scheme: dark;
  font-family: "Segoe UI", sans-serif;
  background: #1e1e1e;
}

* { box-sizing: border-box; }
html, body, #app { width: 100%; height: 100%; margin: 0; overflow: hidden; }
button { font: inherit; color: inherit; }

.vs-shell {
  height: 100%;
  display: grid;
  grid-template-rows: 31px 29px 38px minmax(0, 1fr) 23px;
  color: #d8d8d8;
  background: #1e1e1e;
}

.title-bar {
  display: grid;
  place-items: center;
  background: #181818;
  border-bottom: 1px solid #333;
  color: #c9c9c9;
  font-size: 12px;
}

.menu-bar, .command-bar {
  display: flex;
  align-items: center;
  gap: 20px;
  padding: 0 14px;
  border-bottom: 1px solid #111;
  background: #2d2d30;
  font-size: 12px;
}

.command-bar { gap: 8px; background: #333337; }
.command-bar button { height: 26px; padding: 0 14px; border: 1px solid #555; background: #2b2b2f; }
#run-button { border-color: #498b52; background: #2f5d34; }
#stop-button:not(:disabled) { border-color: #6c4649; background: #493032; }
.target { margin-left: 6px; padding: 5px 12px; border: 1px solid #555; background: #27272a; }

.workspace {
  min-height: 0;
  display: grid;
  grid-template-columns: minmax(420px, 1.45fr) minmax(310px, .9fr);
}

.editor-pane, .tool-pane { min-width: 0; min-height: 0; display: flex; flex-direction: column; }
.editor-pane { border-right: 4px solid #333337; }
.document-tabs, .tool-tabs { height: 34px; flex: 0 0 34px; background: #252526; border-bottom: 1px solid #111; }
.document-tabs { width: 155px; padding: 9px 12px; border-top: 1px solid #68217a; background: #1e1e1e; font-size: 12px; }
#dirty-indicator { color: transparent; }
#dirty-indicator.visible { color: #e3c66b; }
#editor { min-height: 0; flex: 1; }

.tool-tabs { display: flex; align-items: end; }
.tool-tabs button { height: 32px; padding: 0 14px; border: 0; background: #252526; color: #aaa; }
.tool-tabs button[aria-selected="true"] { border-top: 1px solid #68217a; background: #1e1e1e; color: #eee; }
.tool-content { min-height: 0; flex: 1; overflow: auto; padding: 14px 16px; font: 13px/1.65 Consolas, monospace; }
.output-entry { white-space: pre-wrap; overflow-wrap: anywhere; }
.problem-entry { color: #f48771; white-space: pre-wrap; }

.status-bar {
  display: flex;
  align-items: center;
  gap: 18px;
  padding: 0 9px;
  background: #68217a;
  color: white;
  font-size: 11px;
}
.status-bar #wasm-status { margin-left: auto; }

@media (max-width: 850px) {
  .workspace {
    grid-template-columns: 1fr;
    grid-template-rows: minmax(300px, 1.25fr) minmax(180px, .75fr);
  }
  .editor-pane { border-right: 0; border-bottom: 4px solid #333337; }
}
~~~

- [ ] **Step 5: Add an initial calclang sample**

Create compile/calculator/web/src/sample.calc:

~~~calc
// JavaScript Math host functions and calclang prelude
let angle = PI / 3;
println("sin(PI / 3) = " + sin(angle));
println("min(8, 3) = " + min(8, 3));

sqrt(2)
~~~

- [ ] **Step 6: Compose Monaco and runtime in main.ts**

Set MonacoEnvironment to use monaco-editor/esm/vs/editor/editor.worker?worker, register calclang, create the editor with language calclang and theme vs-dark, then:

~~~ts
const output = new OutputWindow(required("output"));
const problems = new OutputWindow(required("problems"));

const controller = new RunController({
  createWorker: () => new Worker(
    new URL("./runtime/calclang.worker.ts", import.meta.url),
    { type: "module" },
  ),
  onReset: () => {
    output.clear();
    problems.clear();
    setProblemCount(0);
    selectTool("output");
  },
  onOutput: (entries) => output.append(entries),
  onProblem: (category, message) => {
    problems.append([category + ": " + message], "problem-entry");
    setProblemCount(1);
    selectTool("problems");
  },
  onRunningChange: (running) => {
    stopButton.disabled = !running;
  },
});

runButton.addEventListener("click", () => {
  controller.run(editor.getValue(), false);
});
stopButton.addEventListener("click", () => controller.stop());
editor.onDidChangeModelContent(() => dirtyIndicator.classList.add("visible"));
editor.onDidChangeCursorPosition(({ position }) => {
  cursorPosition.textContent =
    "Ln " + position.lineNumber + ", Col " + position.column;
});
window.addEventListener("beforeunload", () => controller.dispose());
~~~

Implement required, selectTool, and setProblemCount as small local functions that query the exact IDs from index.html, throw on missing required elements, update aria-selected, and toggle hidden on Output/Problems. Do not add an editor-change callback that stops or restarts the controller.

- [ ] **Step 7: Verify shell, tests, and production build**

Run:

~~~text
bun test
bun run build
~~~

Expected: shell contract passes, all prior Bun tests pass, TypeScript passes, and Vite builds Monaco plus the module Worker.

- [ ] **Step 8: Commit**

~~~text
git add compile/calculator/web/index.html compile/calculator/web/src/styles.css compile/calculator/web/src/main.ts compile/calculator/web/src/ui/shell.test.ts compile/calculator/web/src/sample.calc
git commit -m "feat(calculator): Build playground interface"
~~~

### Task 11: Document and verify the complete playground

**Files:**

- Modify: README.md

- [ ] **Step 1: Add focused usage documentation**

Document:

~~~text
cd compile/calculator/web
bun install
bun run dev
~~~

Explain that Bun runs Vite, wasm-pack must be installed, Run uses a fresh environment, editing does not stop active code, Stop kills it, and production assets are created by bun run build.

- [ ] **Step 2: Run Rust formatting and static checks**

Run:

~~~text
cargo fmt --all -- --check
cargo clippy -p calculator --all-targets -- -D warnings
cargo test -p calculator
~~~

Expected: formatting is clean, Clippy reports zero warnings, and every native Rust test passes.

- [ ] **Step 3: Run browser-targeted WASM verification**

Run:

~~~text
wasm-pack test --headless --chrome compile/calculator --no-default-features
~~~

Expected: all host bridge browser tests pass.

- [ ] **Step 4: Run the Bun and release build gate**

Run from compile/calculator/web:

~~~text
bun test
bun run wasm
bun run build
~~~

Expected: all Bun tests pass, release bindings regenerate, and Vite emits dist without errors or warnings.

- [ ] **Step 5: Perform a browser smoke test**

Run:

~~~text
bun run dev
~~~

Verify in the browser:

1. The initial sample prints sin and min results and returns sqrt(2).
2. Host completion includes println, typeof, PI, sin, and random.
3. EBNF syntax colors numbers, strings, comments, operators, and keywords.
4. Editing during loop execution does not stop that execution.
5. Stop ends the loop and writes Execution stopped by user.
6. Starting a new run replaces an active run.
7. Output beyond 10,000 entries removes the oldest DOM rows while new rows continue.
8. The page shows no elapsed timer and no Worker implementation label.
9. Narrow viewport stacks Output below Monaco.

- [ ] **Step 6: Commit documentation**

~~~text
git add README.md
git commit -m "docs(calculator): Document web playground"
~~~

- [ ] **Step 7: Inspect final scope**

Run:

~~~text
git status --short
git log --oneline 468c056..HEAD
~~~

Expected: the worktree is clean and the branch contains only the focused implementation commits listed in this plan.
