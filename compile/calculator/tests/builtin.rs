use calclang::{
    ast::{BuiltIn, Value},
    env::Environment,
};

#[test]
fn captured_callback_is_callable_through_environment() {
    let offset = 2.0;
    let env = Environment::new();

    env.borrow_mut()
        .define_builtin("add_offset", move |values| {
            let Some(Value::Float(value)) = values.first() else {
                panic!("expected a float argument");
            };

            Ok(Value::Float(value + offset))
        });

    let Value::BuiltIn(add_offset) = env
        .borrow()
        .get("add_offset")
        .expect("add_offset should be defined")
    else {
        panic!("add_offset should be a built-in");
    };

    assert_eq!(
        add_offset.call(vec![Value::Float(40.0)]).unwrap(),
        Value::Float(42.0)
    );
}

#[test]
fn cloned_builtin_keeps_callback_identity() {
    let built_in = BuiltIn::new(|_| Ok(Value::None));

    assert_eq!(built_in, built_in.clone());
    assert_ne!(built_in, BuiltIn::new(|_| Ok(Value::None)));
}
