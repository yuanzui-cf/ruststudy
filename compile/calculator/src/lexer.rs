use std::{fmt::Display, iter::Peekable, str::Chars};

use crate::{
    ast::{Op, Value},
    error::{self, Result},
};

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Identifier(String),

    // Types
    Value(Value),

    // Symbols
    Op(Op),
    /// {
    LB,
    /// }
    RB,
    /// (
    LP,
    /// )
    RP,
    /// =
    Assign,
    /// ;
    Semi,
    /// ,
    Comma,

    Let,
    If,
    Else,
    Loop,
    Break,
    Continue,
    Fn,
    Return,

    End,
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Token::Value(val) = self {
            write!(f, "{val}")?;
        } else if let Token::Identifier(id) = self {
            write!(f, "{id}")?;
        } else if let Token::Op(op) = self {
            write!(f, "{op}")?;
        } else {
            write!(
                f,
                "{}",
                match self {
                    Token::LB => "{",
                    Token::RB => "}",
                    Token::LP => "(",
                    Token::RP => ")",
                    Token::Assign => "=",
                    Token::Semi => ";",
                    Token::Comma => ",",
                    Token::Let => "let",
                    Token::If => "if",
                    Token::Else => "else",
                    Token::Loop => "loop",
                    Token::Break => "break",
                    Token::Continue => "continue",
                    Token::Fn => "fn",
                    Token::Return => "return",
                    Token::Op(_) | Token::Identifier(_) | Token::Value(_) | Token::End => "",
                },
            )?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Lexer;

impl Lexer {
    fn get_float(expr: &mut Peekable<Chars<'_>>, first_chr: char, is_decimal: bool) -> Result<f64> {
        let mut is_decimal = is_decimal;
        let mut res = String::new();
        res.push(first_chr);

        while let Some(next) = expr.peek()
            && (next.is_ascii_digit() || next == &'.')
        {
            let Some(chr) = expr.next() else {
                unreachable!("next is ascii digit");
            };

            if chr == '.' {
                if is_decimal {
                    return Err(error::error!(Syntax, "Invalid syntax `.` found"));
                } else {
                    is_decimal = true;
                }
            }

            res.push(chr);
        }

        let num = res
            .parse::<f64>()
            .map_err(|e| error::error!(Syntax, "Failed to parse {res} as 64-bit number: {e}"))?;

        Ok(num)
    }

    fn get_identifier(expr: &mut Peekable<Chars<'_>>, first_chr: char) -> String {
        let mut res = String::from(first_chr);

        while let Some(next) = expr.peek()
            && (next.is_alphabetic() || next.is_ascii_digit() || next == &'_')
        {
            let Some(chr) = expr.next() else {
                unreachable!("next is match identifier format");
            };

            res.push(chr);
        }

        res
    }

    pub fn tokenize(expr: &str) -> Result<Vec<Token>> {
        let mut tokens = vec![];
        let mut expr = expr.chars().peekable();

        loop {
            let chr = expr.next();

            match chr {
                Some('+') => tokens.push(Token::Op(Op::Add)),
                Some('-') => tokens.push(Token::Op(Op::Sub)),
                Some('*') => tokens.push(Token::Op(Op::Mul)),
                Some('/') => {
                    if let Some(next) = expr.peek() {
                        if next == &'/' {
                            expr.next();
                            while let Some(next) = expr.next()
                                && next != '\n'
                            {}
                            continue;
                        } else if next == &'*' {
                            let mut is_closed = false;

                            while let Some(c) = expr.next() {
                                if c == '*' && expr.peek() == Some(&'/') {
                                    expr.next();
                                    is_closed = true;
                                    break;
                                }
                            }

                            if !is_closed {
                                return Err(error::error!(
                                    Syntax,
                                    "Unterminated block comment found"
                                ));
                            }

                            continue;
                        }
                    }

                    tokens.push(Token::Op(Op::Div))
                }
                Some('^') => tokens.push(Token::Op(Op::Exp)),
                Some('{') => tokens.push(Token::LB),
                Some('}') => tokens.push(Token::RB),
                Some('(') => tokens.push(Token::LP),
                Some(')') => tokens.push(Token::RP),
                Some('.') => {
                    let num = Lexer::get_float(&mut expr, '.', true)?;
                    tokens.push(Token::Value(Value::Float(num)));
                }
                Some(num) if num.is_ascii_digit() => {
                    let num = Lexer::get_float(&mut expr, num, false)?;
                    tokens.push(Token::Value(Value::Float(num)));
                }
                Some('=') => {
                    if let Some(next) = expr.peek()
                        && next == &'='
                    {
                        expr.next();
                        tokens.push(Token::Op(Op::Eq))
                    } else {
                        tokens.push(Token::Assign)
                    }
                }
                Some('<') => {
                    if let Some(next) = expr.peek()
                        && next == &'='
                    {
                        expr.next();
                        tokens.push(Token::Op(Op::Le))
                    } else {
                        tokens.push(Token::Op(Op::Lt))
                    }
                }
                Some('>') => {
                    if let Some(next) = expr.peek()
                        && next == &'='
                    {
                        expr.next();
                        tokens.push(Token::Op(Op::Ge))
                    } else {
                        tokens.push(Token::Op(Op::Gt))
                    }
                }
                Some('!') => {
                    if let Some(next) = expr.peek()
                        && next == &'='
                    {
                        expr.next();
                        tokens.push(Token::Op(Op::Neq))
                    } else {
                        tokens.push(Token::Op(Op::Not))
                    }
                }
                Some(';') => tokens.push(Token::Semi),
                Some(',') => tokens.push(Token::Comma),
                // Some('/') => {
                // }
                Some(other) if other.is_whitespace() => {
                    continue;
                }
                Some(other) if other.is_alphabetic() || other == '_' => {
                    let identifier = Lexer::get_identifier(&mut expr, other);
                    match identifier.as_str() {
                        "let" => tokens.push(Token::Let),
                        "if" => tokens.push(Token::If),
                        "else" => tokens.push(Token::Else),
                        "true" => tokens.push(Token::Value(Value::Bool(true))),
                        "false" => tokens.push(Token::Value(Value::Bool(false))),
                        "none" => tokens.push(Token::Value(Value::None)),
                        "and" => tokens.push(Token::Op(Op::And)),
                        "or" => tokens.push(Token::Op(Op::Or)),
                        "loop" => tokens.push(Token::Loop),
                        "break" => tokens.push(Token::Break),
                        "continue" => tokens.push(Token::Continue),
                        "fn" => tokens.push(Token::Fn),
                        "return" => tokens.push(Token::Return),
                        _ => tokens.push(Token::Identifier(identifier)),
                    }
                }
                Some(other) => {
                    return Err(error::error!(Syntax, "Invalid syntax `{other}` found"));
                }
                None => {
                    tokens.push(Token::End);
                    break;
                }
            };
        }

        Ok(tokens)
    }
}
