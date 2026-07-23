use std::{
    cell::RefCell,
    fmt::{Debug, Display},
    rc::Rc,
};

use crate::{
    ctx::Context,
    env::Environment,
    error::{self, Error, InternalError, Result},
};

#[derive(Debug, Clone)]
pub enum Value {
    Integer(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Fn(Vec<String>, Rc<ASTNode>, Rc<RefCell<Environment>>),
    BuiltIn(BuiltIn),
    None,
}

#[derive(Clone)]
pub struct BuiltIn(Rc<dyn Fn(Vec<Value>) -> Result<Value>>);

impl Debug for BuiltIn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BuiltIn(..)")
    }
}

impl PartialEq for BuiltIn {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl BuiltIn {
    pub fn new<F>(callback: F) -> Self
    where
        F: Fn(Vec<Value>) -> Result<Value> + 'static,
    {
        Self(Rc::new(callback))
    }

    pub fn call(&self, values: Vec<Value>) -> Result<Value> {
        (self.0)(values)
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Integer(a), Value::Integer(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::None, Value::None) => true,
            (Value::Fn(_, block1, env1), Value::Fn(_, block2, env2)) => {
                Rc::ptr_eq(block1, block2) && Rc::ptr_eq(env1, env2)
            }
            _ => false,
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Integer(num) => write!(f, "{num}"),
            Self::Float(num) => write!(f, "{num}"),
            Self::Bool(val) => write!(f, "{val}"),
            Self::String(str) => write!(f, "{str}"),
            Self::Fn(_, _, _) | Self::BuiltIn(_) | Self::None => write!(f, ""),
        }
    }
}

impl Value {
    pub fn type_name(&self) -> String {
        match self {
            Self::Integer(_) => "integer".into(),
            Self::Float(_) => "float".into(),
            Self::Bool(_) => "bool".into(),
            Self::String(_) => "string".into(),
            Self::Fn(l, _, _) => format!("fn({})", l.join(",")),
            Self::BuiltIn(_) => "fn".into(),
            Self::None => "none".into(),
        }
    }

    pub fn apply_binary(self, op: &Op, other: Self) -> Result<Self> {
        match (self, other) {
            (Self::Integer(l), Self::Integer(r)) => match op {
                Op::Add => Ok(Self::Integer(l + r)),
                Op::Sub => Ok(Self::Integer(l - r)),
                Op::Mul => Ok(Self::Integer(l * r)),
                Op::Div => Ok(Self::Integer(l / r)),
                Op::Exp => Ok(Self::Float((l as f64).powf(r as f64))),
                Op::Lt => Ok(Self::Bool(l < r)),
                Op::Le => Ok(Self::Bool(l <= r)),
                Op::Gt => Ok(Self::Bool(l > r)),
                Op::Ge => Ok(Self::Bool(l >= r)),
                Op::Eq => Ok(Self::Bool(l == r)),
                Op::Neq => Ok(Self::Bool(l != r)),
                _ => Err(error::error!(
                    Type,
                    "Cannot apply `{op}` on integer and integer",
                )),
            },
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
                _ => Err(error::error!(
                    Type,
                    "Cannot apply `{op}` on float and float",
                )),
            },
            (Self::String(l), r) => match op {
                Op::Add => {
                    let mut buf = l.clone();
                    buf.push_str(&format!("{r}"));
                    Ok(Self::String(buf))
                }
                Op::Eq | Op::Neq => {
                    if let Self::String(r) = r {
                        Ok(Self::Bool(match op {
                            Op::Eq => l == r,
                            Op::Neq => l != r,
                            _ => unreachable!("op is eq or neq"),
                        }))
                    } else {
                        Err(error::error!(
                            Type,
                            "Cannot apply `{op}` on string and {}",
                            r.type_name()
                        ))
                    }
                }
                _ => Err(error::error!(
                    Type,
                    "Cannot apply `{op}` on string and {}",
                    r.type_name()
                )),
            },
            (l, r) if l.type_name() == r.type_name() => match op {
                Op::Eq => Ok(Self::Bool(l == r)),
                Op::Neq => Ok(Self::Bool(l != r)),
                _ => Err(error::error!(
                    Type,
                    "Cannot apply `{op}` on {} and {}",
                    l.type_name(),
                    r.type_name()
                )),
            },
            (l, r) => Err(error::error!(
                Type,
                "Cannot apply `{op}` between {} and {}",
                l.type_name(),
                r.type_name()
            )),
        }
    }

    pub fn apply_unary(self, op: &Op) -> Result<Self> {
        match self {
            Self::Bool(val) => match op {
                Op::Not => Ok(Self::Bool(!val)),
                _ => Err(error::error!(Type, "Cannot apply unary `{op}` on bool")),
            },
            o => Err(error::error!(
                Type,
                "Cannot apply unary `{op}` to {}",
                o.type_name()
            )),
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

    Not,

    And,
    Or,
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
                Self::Not => "!",
                Self::And => "and",
                Self::Or => "or",
            }
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ASTNode {
    Value(Value),
    Identifier(String),
    And(Box<ASTNode>, Box<ASTNode>),
    Or(Box<ASTNode>, Box<ASTNode>),
    Binary(Box<ASTNode>, Op, Box<ASTNode>),
    Negate(Box<ASTNode>),
    Unary(Op, Box<ASTNode>),
    Define(String, Option<Box<ASTNode>>),
    Assignment(String, Box<ASTNode>),
    Program(Vec<ASTNode>, Option<Box<ASTNode>>),
    Block(Vec<ASTNode>, Option<Box<ASTNode>>),
    Condition(Box<ASTNode>, Box<ASTNode>, Option<Box<ASTNode>>),
    Loop(Box<ASTNode>),
    Break(Option<Box<ASTNode>>),
    Return(Option<Box<ASTNode>>),
    Continue,
    Fn(Option<String>, Vec<String>, Rc<ASTNode>),
    Call(Box<ASTNode>, Vec<ASTNode>),
}

impl ASTNode {
    pub fn is_statement_without_semi(&self) -> bool {
        matches!(
            self,
            Self::Condition(_, _, _) | Self::Loop(_) | Self::Fn(_, _, _)
        )
    }

    pub fn eval(&self, env: Rc<RefCell<Environment>>, ctx: Context) -> Result<Value> {
        match self {
            Self::Value(val) => Ok(val.clone()),
            Self::Identifier(identifier) => {
                let env = env.borrow();
                if let Some(res) = env.get(identifier) {
                    Ok(res.clone())
                } else {
                    Err(error::error!(Name, "name '{identifier}' is not defined"))
                }
            }
            Self::And(left, right) | Self::Or(left, right) => {
                let op = match self {
                    Self::And(_, _) => "and",
                    Self::Or(_, _) => "or",
                    _ => unreachable!(),
                };

                let left = left.eval(env.clone(), ctx.clone())?;
                let Value::Bool(l_val) = left else {
                    return Err(error::error!(
                        Type,
                        "Cannot apply logical operator `{op}` on non-boolean operand '{}'",
                        left.type_name()
                    ));
                };

                if match self {
                    Self::And(_, _) => !l_val,
                    Self::Or(_, _) => l_val,
                    _ => unreachable!("self must be `and` or `or`"),
                } {
                    Ok(Value::Bool(l_val))
                } else {
                    let right = right.eval(env.clone(), ctx)?;
                    let Value::Bool(r_val) = right else {
                        return Err(error::error!(
                            Type,
                            "Cannot apply logical operator `{op}` on non-boolean operand '{}'",
                            right.type_name()
                        ));
                    };

                    Ok(Value::Bool(r_val))
                }
            }
            Self::Binary(node_1, op, node_2) => {
                let left = node_1.eval(env.clone(), ctx.clone())?;
                let right = node_2.eval(env, ctx)?;

                left.apply_binary(op, right)
            }
            Self::Unary(op, node) => {
                let res = node.eval(env, ctx)?;

                res.apply_unary(op)
            }
            Self::Negate(node) => {
                let res = node.eval(env, ctx)?;

                Ok(match res {
                    Value::Float(num) => Value::Float(-num),
                    Value::Integer(num) => Value::Integer(-num),
                    o => {
                        return Err(error::error!(
                            Type,
                            "Cannot negate a non-numeric value. Expected Float, but found: {o}"
                        ));
                    }
                })
            }
            Self::Define(identifier, expr) => {
                let res = match expr {
                    Some(expr) => Some(expr.eval(env.clone(), ctx)?),
                    None => None,
                };

                let mut env = env.borrow_mut();
                env.define(identifier, res)?;

                Ok(Value::None)
            }
            Self::Assignment(identifier, expr) => {
                let res = expr.eval(env.clone(), ctx)?;

                let mut env = env.borrow_mut();
                env.assign(identifier, res)?;

                Ok(Value::None)
            }
            Self::Program(statements, expr) => {
                for s in statements {
                    s.eval(env.clone(), ctx.clone())?;
                }

                if let Some(expr) = expr {
                    Ok(expr.eval(env, ctx)?)
                } else {
                    Ok(Value::None)
                }
            }
            Self::Block(statements, expr) => {
                let block_env = Environment::new_child(env);

                for s in statements {
                    s.eval(block_env.clone(), ctx.clone())?;
                }

                if let Some(expr) = expr {
                    Ok(expr.eval(block_env, ctx)?)
                } else {
                    Ok(Value::None)
                }
            }
            Self::Condition(condition, block, else_expr) => {
                let res = condition.eval(env.clone(), ctx.clone())?;

                if let Value::Bool(val) = res {
                    Ok(if val {
                        block.eval(env, ctx)?
                    } else if let Some(else_expr) = else_expr {
                        else_expr.eval(env, ctx)?
                    } else {
                        Value::None
                    })
                } else {
                    Err(error::error!(
                        Type,
                        "Expect bool, found {}",
                        res.type_name()
                    ))
                }
            }
            Self::Loop(block) => {
                let loop_ctx = Context {
                    is_loop: true,
                    ..ctx
                };

                let res = 'l: loop {
                    let res = block.eval(env.clone(), loop_ctx.clone());

                    match res {
                        Ok(_) => Ok(()),
                        Err(Error::Internal(InternalError::LoopBreak(val))) => {
                            break 'l val.unwrap_or(Value::None);
                        }
                        Err(Error::Internal(InternalError::LoopContinue)) => continue,
                        Err(err) => Err(err),
                    }?;
                };

                Ok(res)
            }
            Self::Break(expr) => {
                if !ctx.is_loop {
                    Err(error::error!(Runtime, "Cannot use break outside a loop"))
                } else {
                    let res = match expr {
                        Some(expr) => Some(expr.eval(env, ctx)?),
                        None => None,
                    };

                    Err(error::error!(Internal, InternalError::LoopBreak(res)))
                }
            }
            Self::Continue => {
                if !ctx.is_loop {
                    Err(error::error!(Runtime, "Cannot use continue outside a loop"))
                } else {
                    Err(error::error!(Internal, InternalError::LoopContinue))
                }
            }
            Self::Fn(identifier, args, block) => {
                let func = Value::Fn(args.clone(), block.clone(), env.clone());

                if let Some(identifier) = identifier {
                    let mut env = env.borrow_mut();
                    env.define_or_assign(identifier, func.clone());
                }

                Ok(func)
            }
            Self::Return(expr) => {
                if ctx.depth == 0 {
                    Err(error::error!(
                        Runtime,
                        "Cannot use return outside a function"
                    ))
                } else {
                    let res = match expr {
                        Some(expr) => Some(expr.eval(env, ctx)?),
                        None => None,
                    };

                    Err(error::error!(Internal, InternalError::FunctionReturn(res)))
                }
            }
            Self::Call(expr, vals) => {
                if ctx.depth > 1000 {
                    return Err(error::error!(Runtime, "maximum recursion depth exceeded"));
                }

                let expr = expr.eval(env.clone(), ctx.clone())?;

                if !matches!(expr, Value::Fn(_, _, _) | Value::BuiltIn(_)) {
                    return Err(error::error!(Type, "Expect a function, found {expr}"));
                }

                let mut evaluated_vals = Vec::with_capacity(vals.len());
                for val in vals {
                    evaluated_vals.push(val.eval(env.clone(), ctx.clone())?);
                }

                let (args, block, f_env) = match expr {
                    Value::Fn(args, block, f_env) => (args, block, f_env),
                    Value::BuiltIn(build_in) => return build_in.call(evaluated_vals),
                    _ => unreachable!("expr is function"),
                };

                let run_env = Environment::new_child(f_env);

                let mut env_borrowed = run_env.borrow_mut();
                for (i, arg) in args.iter().enumerate() {
                    let val = evaluated_vals.get(i).cloned().unwrap_or(Value::None);
                    env_borrowed.define_or_assign(arg, val);
                }
                drop(env_borrowed);

                let res = match block.eval(
                    run_env,
                    Context {
                        is_loop: false,
                        depth: ctx.depth + 1,
                    },
                ) {
                    Ok(res) => Ok(res),
                    Err(Error::Internal(InternalError::FunctionReturn(val))) => {
                        Ok(val.unwrap_or(Value::None))
                    }
                    Err(err) => Err(err),
                }?;

                Ok(res)
            }
        }
    }
}
