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
    interpreter
        .define_value("PI", std::f64::consts::PI.into())
        .unwrap();
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
