// letter ::= [a-zA-Z_]
// digit ::= [0-9]
// identifier ::= letter ( letter | digit )*
//
// types ::= float | bool | none
//
// float ::= digit+ ( "." digit* )? | "." digit+
// bool ::= "true" | "false"
// none ::= "none"
//
// expr ::= rel_expr
// rel_expr ::= add_expr ( ("<" | "<=" | ">" | ">=" | "==" | "!=") add_expr )?
// add_expr ::= "-"? mul_expr ( ( "+" | "-" ) mul_expr )*
// mul_expr ::= exp_expr ( ( "*" | "/" ) exp_expr )*
// exp_expr ::= not_expr ( "^" exp_expr )?
// not_expr ::= "!"* primary_expr
// primary_expr ::= identifier | block | condition_expr | types | "(" add_expr ")"
//
// define ::= "let" identifier ( "=" expr )?
// assignment ::= identifier "=" expr
//
// block ::= "{" statement* expr? "}"
//
// condition_expr ::= "if" expr block ( "else" ( condition_expr | block ) )?
//
// statement ::= ( assignment | define | expr )? ";"
//
// program ::= statement* expr?

use std::{fmt::Display, iter::Peekable, str::Chars, vec::IntoIter};

use crate::ast::{ASTNode, Op, Value};

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

    Let,
    If,
    Else,
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
                    Token::Let => "let",
                    Token::If => "if",
                    Token::Else => "else",
                    Token::Op(_) | Token::Identifier(_) | Token::Value(_) | Token::End => "",
                },
            )?;
        }

        Ok(())
    }
}

impl Token {
    fn get_float(
        expr: &mut Peekable<Chars<'_>>,
        first_chr: char,
        is_decimal: bool,
    ) -> anyhow::Result<f64> {
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
                    anyhow::bail!("Invalid syntax `.` found");
                } else {
                    is_decimal = true;
                }
            }

            res.push(chr);
        }

        let num = res
            .parse::<f64>()
            .map_err(|e| anyhow::anyhow!("Failed to parse {res} as 64-bit number: {e}"))?;

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

    pub fn tokenize(expr: &str) -> anyhow::Result<Vec<Self>> {
        let mut tokens = vec![];
        let mut expr = expr.chars().peekable();

        loop {
            let chr = expr.next();

            match chr {
                Some('+') => tokens.push(Token::Op(Op::Add)),
                Some('-') => tokens.push(Token::Op(Op::Sub)),
                Some('*') => tokens.push(Token::Op(Op::Mul)),
                Some('/') => tokens.push(Token::Op(Op::Div)),
                Some('^') => tokens.push(Token::Op(Op::Exp)),
                Some('{') => tokens.push(Token::LB),
                Some('}') => tokens.push(Token::RB),
                Some('(') => tokens.push(Token::LP),
                Some(')') => tokens.push(Token::RP),
                Some('.') => {
                    let num = Token::get_float(&mut expr, '.', true)?;
                    tokens.push(Token::Value(Value::Float(num)));
                }
                Some(num) if num.is_ascii_digit() => {
                    let num = Token::get_float(&mut expr, num, false)?;
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
                Some(other) if other.is_whitespace() => {
                    continue;
                }
                Some(other) if other.is_alphabetic() || other == '_' => {
                    let identifier = Token::get_identifier(&mut expr, other);
                    match identifier.as_str() {
                        "let" => tokens.push(Token::Let),
                        "if" => tokens.push(Token::If),
                        "else" => tokens.push(Token::Else),
                        "true" => tokens.push(Token::Value(Value::Bool(true))),
                        "false" => tokens.push(Token::Value(Value::Bool(false))),
                        "none" => tokens.push(Token::Value(Value::None)),
                        _ => tokens.push(Token::Identifier(identifier)),
                    }
                }
                Some(other) => {
                    anyhow::bail!("Invalid syntax `{other}` found");
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

#[derive(Debug, Clone)]
pub struct Parser {
    tokens: Peekable<IntoIter<Token>>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens: tokens.into_iter().peekable(),
        }
    }

    pub fn consume(&mut self) -> anyhow::Result<Token> {
        let Some(next_tok) = self.tokens.next() else {
            anyhow::bail!("Token end unexpectedly");
        };

        Ok(next_tok)
    }

    pub fn peek(&mut self) -> anyhow::Result<&Token> {
        self.tokens
            .peek()
            .ok_or_else(|| anyhow::anyhow!("Token end unexpectedly"))
    }

    pub fn parse(&mut self) -> anyhow::Result<ASTNode> {
        let res = self.program()?;

        if let next = self.peek()?
            && next != &Token::End
        {
            anyhow::bail!("Unexpected trailing token `{next}`.");
        }

        Ok(res)
    }

    fn program(&mut self) -> anyhow::Result<ASTNode> {
        let (statements, expr) = self.statement_or_expr(Token::End)?;

        Ok(ASTNode::Program(statements, expr))
    }

    fn statement_or_expr(
        &mut self,
        end_tok: Token,
    ) -> anyhow::Result<(Vec<ASTNode>, Option<Box<ASTNode>>)> {
        let mut statements: Vec<ASTNode> = Vec::new();
        let mut expr: Option<Box<ASTNode>> = None;

        while self.peek()? != &end_tok {
            if self.peek()? == &Token::Semi {
                self.consume()?;
                continue;
            }

            let res = {
                let tok = self.peek()?;
                match tok {
                    Token::Let => {
                        self.consume()?;
                        if let Token::Identifier(identifier) = self.consume()? {
                            let mut expr: Option<Box<ASTNode>> = None;

                            if matches!(self.peek()?, Token::Assign) {
                                self.consume()?;
                                let res = self.expr()?;
                                expr = Some(Box::new(res));
                            }

                            if !matches!(self.peek()?, Token::Semi) {
                                anyhow::bail!(
                                    "Syntax Error: Statement must end with a semicolon ';'"
                                );
                            }

                            Ok(ASTNode::Define(identifier, expr))
                        } else {
                            anyhow::bail!("Syntax Error: Expect identifier.");
                        }
                    }
                    _ => self.assignment_or_expr(),
                }
            }?;

            if self.peek()? == &Token::Semi {
                self.consume()?;
                statements.push(res);
            } else {
                expr = Some(Box::new(res));
                break;
            }
        }

        Ok((statements, expr))
    }

    fn assignment_or_expr(&mut self) -> anyhow::Result<ASTNode> {
        let mut tmp_toks = self.tokens.clone();

        if tmp_toks.peek().is_some() {
            if let Some(tok) = tmp_toks.next()
                && let Token::Identifier(identifier) = tok
                && let Some(next) = tmp_toks.peek()
                && matches!(next, Token::Assign)
            {
                self.consume()?;
                self.consume()?;

                let res = self.expr()?;

                if !matches!(self.peek()?, Token::Semi) {
                    anyhow::bail!("Syntax Error: Statement must end with a semicolon ';'");
                }

                Ok(ASTNode::Assignment(identifier, Box::new(res)))
            } else {
                self.expr()
            }
        } else {
            anyhow::bail!("Token end unexpectedly")
        }
    }

    fn expr(&mut self) -> anyhow::Result<ASTNode> {
        self.rel_expr()
    }

    fn rel_expr(&mut self) -> anyhow::Result<ASTNode> {
        let mut res = self.add_expr()?;

        loop {
            let tok = self.peek()?;

            if matches!(
                tok,
                Token::Op(Op::Lt | Op::Gt | Op::Le | Op::Ge | Op::Eq | Op::Neq)
            ) {
                let Token::Op(op) = self.consume()? else {
                    unreachable!("next is op")
                };
                res = ASTNode::Binary(Box::new(res), op, Box::new(self.add_expr()?));
            } else {
                break;
            }
        }

        Ok(res)
    }

    fn add_expr(&mut self) -> anyhow::Result<ASTNode> {
        let mut neg_flag = false;

        if let Token::Op(op) = self.peek()?
            && matches!(op, Op::Sub)
        {
            self.consume()?;
            neg_flag = true;
        }

        let mut res = self.mul_expr()?;

        if neg_flag {
            res = ASTNode::Negate(Box::new(res));
        }

        loop {
            let tok = self.peek()?;

            if matches!(tok, Token::Op(Op::Add | Op::Sub)) {
                let Token::Op(op) = self.consume()? else {
                    unreachable!("next is op")
                };
                res = ASTNode::Binary(Box::new(res), op, Box::new(self.mul_expr()?));
            } else {
                break;
            }
        }

        Ok(res)
    }

    fn mul_expr(&mut self) -> anyhow::Result<ASTNode> {
        let mut res = self.exp_expr()?;

        loop {
            let tok = self.peek()?;

            if matches!(tok, Token::Op(Op::Mul | Op::Div)) {
                let Token::Op(op) = self.consume()? else {
                    unreachable!("next is op")
                };
                res = ASTNode::Binary(Box::new(res), op, Box::new(self.exp_expr()?));
            } else {
                break;
            }
        }

        Ok(res)
    }

    fn exp_expr(&mut self) -> anyhow::Result<ASTNode> {
        let mut res = self.not_expr()?;

        let tok = self.peek()?;

        if matches!(tok, Token::Op(Op::Exp)) {
            let Token::Op(op) = self.consume()? else {
                unreachable!("next is op")
            };
            res = ASTNode::Binary(Box::new(res), op, Box::new(self.exp_expr()?));
        }

        Ok(res)
    }

    fn not_expr(&mut self) -> anyhow::Result<ASTNode> {
        let mut not = false;

        while let tok = self.peek()?
            && matches!(tok, Token::Op(Op::Not))
        {
            self.consume()?;
            not = !not;
        }

        let res = self.primary_expr()?;

        Ok(if not {
            ASTNode::Unary(Op::Not, Box::new(res))
        } else {
            res
        })
    }

    fn primary_expr(&mut self) -> anyhow::Result<ASTNode> {
        let tok = self.consume()?;

        match tok {
            Token::Value(val) => Ok(ASTNode::Value(val)),
            Token::Identifier(identifier) => Ok(ASTNode::Identifier(identifier)),
            Token::If => self.condition_expr(),
            Token::LB => self.block(),
            Token::LP => {
                let res = self.expr()?;

                if let next = self.peek()?
                    && next != &Token::RP
                {
                    anyhow::bail!("Expected `)`, found {next}");
                } else {
                    self.consume()?;
                }

                Ok(res)
            }
            Token::End => {
                anyhow::bail!(
                    "Expected a number or an expression, but reached the end of the expression"
                );
            }
            _ => anyhow::bail!("Expected a number or an expression, but found `{tok}`"),
        }
    }

    fn block(&mut self) -> anyhow::Result<ASTNode> {
        let (statements, expr) = self.statement_or_expr(Token::RB)?;

        if let next = self.peek()?
            && next != &Token::RB
        {
            anyhow::bail!("Expected `}}`, found {next}");
        } else {
            self.consume()?;
        }

        Ok(ASTNode::Block(statements, expr))
    }

    fn condition_expr(&mut self) -> anyhow::Result<ASTNode> {
        let condition = self.expr()?;

        if self.peek()? != &Token::LB {
            anyhow::bail!("Expected `{{`, but not found");
        }

        self.consume()?;
        let block = self.block()?;

        let else_block = if self.peek()? == &Token::Else {
            self.consume()?;

            Some(Box::new(match self.consume()? {
                Token::LB => self.block(),
                Token::If => self.condition_expr(),
                o => anyhow::bail!("Expected block or condition expr, found {o}"),
            }?))
        } else {
            None
        };

        Ok(ASTNode::Condition(
            Box::new(condition),
            Box::new(block),
            else_block,
        ))
    }
}
