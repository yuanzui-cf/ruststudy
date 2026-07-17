use std::{fmt::Display, iter::Peekable, str::Chars, vec::IntoIter};

use crate::{
    ast::{ASTNode, Op, Value},
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

    Let,
    If,
    Else,
    Loop,
    Break,
    Continue,

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
                    Token::Loop => "loop",
                    Token::Break => "break",
                    Token::Continue => "continue",
                    Token::Op(_) | Token::Identifier(_) | Token::Value(_) | Token::End => "",
                },
            )?;
        }

        Ok(())
    }
}

impl Token {
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

    pub fn tokenize(expr: &str) -> Result<Vec<Self>> {
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
                        "and" => tokens.push(Token::Op(Op::And)),
                        "or" => tokens.push(Token::Op(Op::Or)),
                        "loop" => tokens.push(Token::Loop),
                        "break" => tokens.push(Token::Break),
                        "continue" => tokens.push(Token::Continue),
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

    pub fn consume(&mut self) -> Result<Token> {
        let Some(next_tok) = self.tokens.next() else {
            return Err(error::error!(Syntax, "Token end unexpectedly"));
        };

        Ok(next_tok)
    }

    pub fn peek(&mut self) -> Result<&Token> {
        self.tokens
            .peek()
            .ok_or_else(|| error::error!(Syntax, "Token end unexpectedly"))
    }

    pub fn parse(&mut self) -> Result<ASTNode> {
        let res = self.program()?;

        if let next = self.peek()?
            && next != &Token::End
        {
            return Err(error::error!(Syntax, "Unexpected trailing token `{next}`."));
        }

        Ok(res)
    }

    fn program(&mut self) -> Result<ASTNode> {
        let (statements, expr) = self.statement_or_expr(Token::End)?;

        Ok(ASTNode::Program(statements, expr))
    }

    fn statement_or_expr(
        &mut self,
        end_tok: Token,
    ) -> Result<(Vec<ASTNode>, Option<Box<ASTNode>>)> {
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
                                return Err(error::error!(
                                    Syntax,
                                    "Statement must end with a semicolon ';'"
                                ));
                            }

                            Ok(ASTNode::Define(identifier, expr))
                        } else {
                            return Err(error::error!(Syntax, "Syntax Error: Expect identifier."));
                        }
                    }
                    Token::Break => {
                        self.consume()?;

                        if matches!(self.peek()?, Token::Semi) {
                            Ok(ASTNode::Break(None))
                        } else if matches!(self.peek()?, Token::RB | Token::End) {
                            return Err(error::error!(
                                Syntax,
                                "Statement must end with a semicolon ';'"
                            ));
                        } else {
                            let res = self.expr()?;

                            if !matches!(self.peek()?, Token::Semi) {
                                return Err(error::error!(
                                    Syntax,
                                    "Statement must end with a semicolon ';'"
                                ));
                            }

                            Ok(ASTNode::Break(Some(Box::new(res))))
                        }
                    }
                    Token::Continue => {
                        if !matches!(self.peek()?, Token::Semi) {
                            return Err(error::error!(
                                Syntax,
                                "Statement must end with a semicolon ';'"
                            ));
                        }

                        Ok(ASTNode::Continue)
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

    fn assignment_or_expr(&mut self) -> Result<ASTNode> {
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
                    return Err(error::error!(
                        Syntax,
                        "Statement must end with a semicolon ';'"
                    ));
                }

                Ok(ASTNode::Assignment(identifier, Box::new(res)))
            } else {
                self.expr()
            }
        } else {
            Err(error::error!(Syntax, "Token end unexpectedly"))
        }
    }

    fn expr(&mut self) -> Result<ASTNode> {
        self.or_expr()
    }

    fn or_expr(&mut self) -> Result<ASTNode> {
        let mut res = self.and_expr()?;

        loop {
            let tok = self.peek()?;

            if matches!(tok, Token::Op(Op::Or)) {
                self.consume()?;
                res = ASTNode::Or(Box::new(res), Box::new(self.and_expr()?));
            } else {
                break;
            }
        }

        Ok(res)
    }

    fn and_expr(&mut self) -> Result<ASTNode> {
        let mut res = self.rel_expr()?;

        loop {
            let tok = self.peek()?;

            if matches!(tok, Token::Op(Op::And)) {
                self.consume()?;
                res = ASTNode::And(Box::new(res), Box::new(self.rel_expr()?));
            } else {
                break;
            }
        }

        Ok(res)
    }

    fn rel_expr(&mut self) -> Result<ASTNode> {
        let mut res = self.add_expr()?;

        let tok = self.peek()?;

        if matches!(
            tok,
            Token::Op(Op::Lt | Op::Gt | Op::Le | Op::Ge | Op::Eq | Op::Neq)
        ) {
            let Token::Op(op) = self.consume()? else {
                unreachable!("next is op")
            };
            res = ASTNode::Binary(Box::new(res), op, Box::new(self.add_expr()?));
        }

        Ok(res)
    }

    fn add_expr(&mut self) -> Result<ASTNode> {
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

    fn mul_expr(&mut self) -> Result<ASTNode> {
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

    fn exp_expr(&mut self) -> Result<ASTNode> {
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

    fn not_expr(&mut self) -> Result<ASTNode> {
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

    fn primary_expr(&mut self) -> Result<ASTNode> {
        let tok = self.consume()?;

        match tok {
            Token::Value(val) => Ok(ASTNode::Value(val)),
            Token::Identifier(identifier) => Ok(ASTNode::Identifier(identifier)),
            Token::If => self.condition_expr(),
            Token::Loop => self.loop_expr(),
            Token::LB => self.block(),
            Token::LP => {
                let res = self.expr()?;

                if let next = self.peek()?
                    && next != &Token::RP
                {
                    return Err(error::error!(Syntax, "Expected `)`, found {next}"));
                } else {
                    self.consume()?;
                }

                Ok(res)
            }
            Token::End => Err(error::error!(
                Syntax,
                "Expected a number or an expression, but reached the end of the expression"
            )),
            _ => Err(error::error!(
                Syntax,
                "Expected a number or an expression, but found `{tok}`"
            )),
        }
    }

    fn block(&mut self) -> Result<ASTNode> {
        let (statements, expr) = self.statement_or_expr(Token::RB)?;

        if let next = self.peek()?
            && next != &Token::RB
        {
            return Err(error::error!(Syntax, "Expected `}}`, found {next}"));
        } else {
            self.consume()?;
        }

        Ok(ASTNode::Block(statements, expr))
    }

    fn condition_expr(&mut self) -> Result<ASTNode> {
        let condition = self.expr()?;

        if self.peek()? != &Token::LB {
            return Err(error::error!(Syntax, "Expected `{{`, but not found"));
        }

        self.consume()?;
        let block = self.block()?;

        let else_block = if self.peek()? == &Token::Else {
            self.consume()?;

            Some(Box::new(match self.consume()? {
                Token::LB => self.block(),
                Token::If => self.condition_expr(),
                o => {
                    return Err(error::error!(
                        Syntax,
                        "Expected block or condition expr, found {o}"
                    ));
                }
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

    fn loop_expr(&mut self) -> Result<ASTNode> {
        if self.peek()? != &Token::LB {
            return Err(error::error!(Syntax, "Expected `{{`, but not found"));
        }

        self.consume()?;
        let block = self.block()?;

        Ok(ASTNode::Loop(Box::new(block)))
    }
}
