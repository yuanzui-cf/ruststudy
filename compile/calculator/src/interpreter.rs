use std::{cell::RefCell, rc::Rc};

use crate::{
    ast::Value, ctx::Context, env::Environment, error::Result, lexer::Lexer, parser::Parser,
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
