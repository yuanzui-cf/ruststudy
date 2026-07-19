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
    Reflect::set(&object, &"category".into(), &error.category().into())
        .expect("plain object property assignment");
    Reflect::set(&object, &"message".into(), &error.message().into())
        .expect("plain object property assignment");
    object.into()
}
