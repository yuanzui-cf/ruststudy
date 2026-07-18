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
    Float(f64),
    Bool(bool),
    Fn(Vec<String>, Rc<ASTNode>, Rc<RefCell<Environment>>),
    None,
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
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
            Self::Float(num) => write!(f, "{num}"),
            Self::Bool(val) => write!(f, "{val}"),
            Self::Fn(_, _, _) => write!(f, "fn"),
            Self::None => write!(f, "none"),
        }
    }
}

impl Value {
    pub fn type_name(&self) -> String {
        match self {
            Self::Float(_) => "float".into(),
            Self::Bool(_) => "bool".into(),
            Self::Fn(l, _, _) => format!("fn({})", l.join(",")),
            Self::None => "none".into(),
        }
    }

    pub fn apply_binary(self, op: &Op, other: Self) -> Result<Self> {
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
                _ => Err(error::error!(
                    Type,
                    "Cannot apply `{op}` on float and float",
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

                let (args, block, f_env) = match expr.eval(env.clone(), ctx.clone())? {
                    Value::Fn(args, block, f_env) => (args, block, f_env),
                    o => return Err(error::error!(Type, "Expect a function, found {o}")),
                };

                let mut evaluated_vals = Vec::with_capacity(vals.len());
                for val in vals {
                    evaluated_vals.push(val.eval(env.clone(), ctx.clone())?);
                }

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
