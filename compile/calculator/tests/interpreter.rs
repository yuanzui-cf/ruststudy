use calclang::{ast::Value, error::Error, interpreter::Interpreter};

#[test]
fn evaluations_share_environment_until_reset() {
    let mut interpreter = Interpreter::new();
    interpreter.eval("let answer = 40;").unwrap();

    assert_eq!(interpreter.eval("answer + 2").unwrap(), Value::Float(42.0));

    interpreter.reset();
    assert!(matches!(interpreter.eval("answer"), Err(Error::Name(_))));
}

#[test]
fn constants_and_callbacks_can_be_injected() {
    let mut interpreter = Interpreter::new();
    interpreter
        .define("PI", Value::Float(std::f64::consts::PI))
        .unwrap();
    interpreter.define_builtin("twice", |values| {
        let Some(Value::Float(value)) = values.first() else {
            return Ok(Value::None);
        };
        Ok(Value::Float(value * 2.0))
    });

    assert_eq!(
        interpreter.eval("twice(PI)").unwrap(),
        Value::Float(std::f64::consts::PI * 2.0)
    );
}

#[test]
fn errors_expose_stable_categories_and_messages() {
    let error = Error::Name("missing".into());
    assert_eq!(error.category(), "NameError");
    assert_eq!(error.message(), "missing");
}
