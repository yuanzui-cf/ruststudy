use std::{cell::RefCell, fmt::Display, rc::Rc};

use crate::env::Environment;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Float(f64),
    None,
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Float(num) => write!(f, "{num}"),
            Self::None => write!(f, "none"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Op {
    Add,
    Neg,
    Mul,
    Div,
    Exp,
}

impl Display for Op {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Add => "+",
                Self::Neg => "-",
                Self::Mul => "*",
                Self::Div => "/",
                Self::Exp => "^",
            }
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ASTNode {
    Float(f64),
    Identifier(String),
    Negate(Box<ASTNode>),
    Binary(Box<ASTNode>, Op, Box<ASTNode>),
    Define(String, Option<Box<ASTNode>>),
    Assignment(String, Box<ASTNode>),
    Program(Vec<ASTNode>, Option<Box<ASTNode>>),
}

impl ASTNode {
    pub fn eval(&self, env: Rc<RefCell<Environment>>) -> anyhow::Result<Value> {
        match self {
            Self::Float(num) => Ok(Value::Float(*num)),
            Self::Identifier(identifier) => {
                let env = env.borrow();
                if let Some(res) = env.get(identifier) {
                    Ok(res.clone())
                } else {
                    anyhow::bail!("NameError: name '{identifier}' is not defined")
                }
            }
            Self::Negate(node) => {
                let res = node.eval(env)?;

                Ok(match res {
                    Value::Float(num) => Value::Float(-num),
                    o => anyhow::bail!(
                        "TypeError: Cannot apply unary negation '-' to a non-numeric value. Expected Float, but found: {o}"
                    ),
                })
            }
            Self::Binary(node_1, op, node_2) => {
                let node_1 = node_1.eval(env.clone())?;
                let node_2 = node_2.eval(env.clone())?;

                let left = match node_1 {
                    Value::Float(num) => num,
                    o => anyhow::bail!("TypeError: Cannot apply `{op}` between {o} and {node_2}"),
                };

                let right = match node_2 {
                    Value::Float(num) => num,
                    o => anyhow::bail!("TypeError: Cannot apply `{op}` between {node_1} and {o}"),
                };

                Ok(Value::Float(match op {
                    Op::Add => left + right,
                    Op::Neg => left - right,
                    Op::Mul => left * right,
                    Op::Div => left / right,
                    Op::Exp => left.powf(right),
                }))
            }
            Self::Define(identifier, expr) => {
                let res = match expr {
                    Some(expr) => Some(expr.eval(env.clone())?),
                    None => None,
                };

                let mut env = env.borrow_mut();
                env.define(identifier, res)?;

                Ok(Value::None)
            }
            Self::Assignment(identifier, expr) => {
                let res = expr.eval(env.clone())?;

                let mut env = env.borrow_mut();
                env.assign(identifier, res)?;

                Ok(Value::None)
            }
            Self::Program(statements, expr) => {
                for s in statements {
                    s.eval(env.clone())?;
                }

                if let Some(expr) = expr {
                    Ok(expr.eval(env)?)
                } else {
                    Ok(Value::None)
                }
            }
        }
    }
}
