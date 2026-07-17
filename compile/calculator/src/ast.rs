use std::{
    cell::RefCell,
    fmt::{Debug, Display},
    rc::Rc,
};

use crate::env::Environment;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Float(f64),
    Bool(bool),
    None,
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Float(num) => write!(f, "{num}"),
            Self::Bool(val) => write!(f, "{val}"),
            Self::None => write!(f, "none"),
        }
    }
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Float(_) => "float",
            Self::Bool(_) => "bool",
            Self::None => "none",
        }
    }

    pub fn apply_binary(self, op: &Op, other: Self) -> anyhow::Result<Self> {
        match (self, other) {
            (Self::Float(l), Self::Float(r)) => match op {
                Op::Add => Ok(Self::Float(l + r)),
                Op::Sub => Ok(Self::Float(l - r)),
                Op::Mul => Ok(Self::Float(l * r)),
                Op::Div => Ok(Self::Float(l / r)),
                Op::Exp => Ok(Self::Float(l.powf(r))),
                Op::Lt => Ok(Self::Bool(l < r)),
                Op::Le => Ok(Self::Bool(l <= r)),
                Op::Gt => Ok(Self::Bool(l > r)),
                Op::Ge => Ok(Self::Bool(l >= r)),
                Op::Eq => Ok(Self::Bool(l == r)),
                Op::Neq => Ok(Self::Bool(l != r)),
            },
            (l, r) if l.type_name() == r.type_name() => match op {
                Op::Eq => Ok(Self::Bool(l == r)),
                Op::Neq => Ok(Self::Bool(l != r)),
                _ => anyhow::bail!(
                    "TypeError: Cannot apply `{op}` on {} and {}",
                    l.type_name(),
                    r.type_name()
                ),
            },
            (l, r) => anyhow::bail!(
                "TypeError: Cannot apply `{op}` between {} and {}",
                l.type_name(),
                r.type_name()
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Exp,

    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Neq,
}

impl Display for Op {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Add => "+",
                Self::Sub => "-",
                Self::Mul => "*",
                Self::Div => "/",
                Self::Exp => "^",
                Self::Lt => "<",
                Self::Le => "<=",
                Self::Gt => ">",
                Self::Ge => ">=",
                Self::Eq => "==",
                Self::Neq => "!=",
            }
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ASTNode {
    Value(Value),
    Identifier(String),
    Negate(Box<ASTNode>),
    Binary(Box<ASTNode>, Op, Box<ASTNode>),
    Define(String, Option<Box<ASTNode>>),
    Assignment(String, Box<ASTNode>),
    Program(Vec<ASTNode>, Option<Box<ASTNode>>),
    Block(Vec<ASTNode>, Option<Box<ASTNode>>),
    Condition(Box<ASTNode>, Box<ASTNode>, Option<Box<ASTNode>>),
}

impl ASTNode {
    pub fn eval(&self, env: Rc<RefCell<Environment>>) -> anyhow::Result<Value> {
        match self {
            Self::Value(val) => Ok(val.clone()),
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
                let left = node_1.eval(env.clone())?;
                let right = node_2.eval(env)?;

                left.apply_binary(op, right)
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
            Self::Block(statements, expr) => {
                let block_env = Environment::new_child(env);

                for s in statements {
                    s.eval(block_env.clone())?;
                }

                if let Some(expr) = expr {
                    Ok(expr.eval(block_env)?)
                } else {
                    Ok(Value::None)
                }
            }
            Self::Condition(condition, block, else_expr) => {
                let res = condition.eval(env.clone())?;

                if let Value::Bool(val) = res {
                    Ok(if val {
                        block.eval(env)?
                    } else if let Some(else_expr) = else_expr {
                        else_expr.eval(env)?
                    } else {
                        Value::None
                    })
                } else {
                    anyhow::bail!("Expect bool, found {}", res.type_name())
                }
            }
        }
    }
}
