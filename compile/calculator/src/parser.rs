// letter ::= [a-zA-Z_]
// digit ::= [0-9]
// identifier ::= letter ( letter | digit )*
// float ::= digit+ ( "." digit* )? | "." digit+
//
// expr ::= "-"? term ( ( "+" | "-" ) term )*
// term ::= power ( ( "*" | "/" ) power )*
// power ::= factor ( "^" power )?
// factor ::= identifier | float | "(" expr ")"
//
// assignment ::= identifier "=" expr
//
// statement ::= ( assignment | expr )? ";"
//
// program ::= statement* ( assignment | expr )?

use std::{fmt::Display, iter::Peekable, str::Chars, vec::IntoIter};

use crate::ast::{ASTNode, Op};

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Float(f64),
    Identifier(String),
    Add,
    Neg,
    Mul,
    Div,
    Exp,
    LP,
    RP,
    Assign,
    Semi,
    End,
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Token::Float(num) = self {
            write!(f, "{num}")?;
        } else if let Token::Identifier(id) = self {
            write!(f, "{id}")?;
        } else {
            write!(
                f,
                "{}",
                match self {
                    Token::Add => "+",
                    Token::Neg => "-",
                    Token::Mul => "*",
                    Token::Div => "/",
                    Token::Exp => "^",
                    Token::LP => "(",
                    Token::RP => ")",
                    Token::Assign => "=",
                    Token::Semi => ";",
                    Token::Identifier(_) | Token::Float(_) | Token::End => "",
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
                Some('+') => tokens.push(Token::Add),
                Some('-') => tokens.push(Token::Neg),
                Some('*') => tokens.push(Token::Mul),
                Some('/') => tokens.push(Token::Div),
                Some('^') => tokens.push(Token::Exp),
                Some('(') => tokens.push(Token::LP),
                Some(')') => tokens.push(Token::RP),
                Some('.') => {
                    let num = Token::get_float(&mut expr, '.', true)?;
                    tokens.push(Token::Float(num));
                }
                Some(num) if num.is_ascii_digit() => {
                    let num = Token::get_float(&mut expr, num, false)?;
                    tokens.push(Token::Float(num));
                }
                Some('=') => tokens.push(Token::Assign),
                Some(';') => tokens.push(Token::Semi),
                Some(other) if other.is_whitespace() => {
                    continue;
                }
                Some(other) if other.is_alphabetic() || other == '_' => {
                    let identifier = Token::get_identifier(&mut expr, other);
                    tokens.push(Token::Identifier(identifier));
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
        let mut statements: Vec<ASTNode> = Vec::new();
        let mut expr: Option<Box<ASTNode>> = None;

        while self.peek()? != &Token::End {
            if self.peek()? == &Token::Semi {
                self.consume()?;
                continue;
            }

            let res = self.assignment()?;

            if self.peek()? == &Token::Semi {
                self.consume()?;
                statements.push(res);
            } else {
                expr = Some(Box::new(res));
                break;
            }
        }

        Ok(ASTNode::Program(statements, expr))
    }

    fn assignment(&mut self) -> anyhow::Result<ASTNode> {
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

                Ok(ASTNode::Assignment(identifier, Box::new(res)))
            } else {
                self.expr()
            }
        } else {
            anyhow::bail!("Token end unexpectedly")
        }
    }

    fn expr(&mut self) -> anyhow::Result<ASTNode> {
        let mut neg_flag = false;

        if self.peek()? == &Token::Neg {
            self.consume()?;
            neg_flag = true;
        }

        let mut res = self.term()?;

        if neg_flag {
            res = ASTNode::Negate(Box::new(res));
        }

        loop {
            let tok = self.peek()?;

            match tok {
                Token::Add => {
                    self.consume()?;
                    res = ASTNode::Binary(Box::new(res), Op::Add, Box::new(self.term()?));
                }
                Token::Neg => {
                    self.consume()?;
                    res = ASTNode::Binary(Box::new(res), Op::Neg, Box::new(self.term()?));
                }
                _ => break,
            };
        }

        Ok(res)
    }

    fn term(&mut self) -> anyhow::Result<ASTNode> {
        let mut res = self.power()?;

        loop {
            let tok = self.peek()?;

            match tok {
                Token::Mul => {
                    self.consume()?;
                    res = ASTNode::Binary(Box::new(res), Op::Mul, Box::new(self.power()?));
                }
                Token::Div => {
                    self.consume()?;
                    res = ASTNode::Binary(Box::new(res), Op::Div, Box::new(self.power()?));
                }
                _ => break,
            };
        }

        Ok(res)
    }

    fn power(&mut self) -> anyhow::Result<ASTNode> {
        let mut res = self.factor()?;

        let tok = self.peek()?;

        if matches!(tok, Token::Exp) {
            self.consume()?;
            res = ASTNode::Binary(Box::new(res), Op::Exp, Box::new(self.power()?));
        }

        Ok(res)
    }

    fn factor(&mut self) -> anyhow::Result<ASTNode> {
        let tok = self.consume()?;

        match tok {
            Token::Float(num) => Ok(ASTNode::Float(num)),
            Token::Identifier(identifier) => Ok(ASTNode::Identifier(identifier)),
            Token::LP => {
                let res = self.expr()?;

                if self.peek()? != &Token::RP {
                    anyhow::bail!("Expected `)`, but not found");
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
}
