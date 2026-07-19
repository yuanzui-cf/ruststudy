use std::{iter::Peekable, rc::Rc, vec::IntoIter};

use crate::{
    ast::{ASTNode, Op},
    error::{self, Result},
    lexer::Token,
};

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
                    Token::Return => {
                        self.consume()?;

                        if matches!(self.peek()?, Token::Semi) {
                            Ok(ASTNode::Return(None))
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

                            Ok(ASTNode::Return(Some(Box::new(res))))
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
        let mut res = self.not_expr()?;

        loop {
            let tok = self.peek()?;

            if matches!(tok, Token::Op(Op::Mul | Op::Div)) {
                let Token::Op(op) = self.consume()? else {
                    unreachable!("next is op")
                };
                res = ASTNode::Binary(Box::new(res), op, Box::new(self.not_expr()?));
            } else {
                break;
            }
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

        let res = self.exp_expr()?;

        Ok(if not {
            ASTNode::Unary(Op::Not, Box::new(res))
        } else {
            res
        })
    }

    fn exp_expr(&mut self) -> Result<ASTNode> {
        let mut res = self.call_expr()?;

        let tok = self.peek()?;

        if matches!(tok, Token::Op(Op::Exp)) {
            let Token::Op(op) = self.consume()? else {
                unreachable!("next is op")
            };
            res = ASTNode::Binary(Box::new(res), op, Box::new(self.exp_expr()?));
        }

        Ok(res)
    }

    fn call_expr(&mut self) -> Result<ASTNode> {
        let mut expr = self.base_expr()?;

        loop {
            let tok = self.peek()?;
            let mut vals = Vec::new();

            if matches!(tok, Token::LP) {
                self.consume()?;

                if !matches!(self.peek()?, Token::RP) {
                    let val = self.expr()?;
                    vals.push(val);

                    loop {
                        let tok = self.peek()?;

                        if matches!(tok, Token::Comma) {
                            self.consume()?;

                            let val = self.expr()?;
                            vals.push(val);
                        } else {
                            break;
                        }
                    }
                }

                if let next = self.peek()?
                    && next != &Token::RP
                {
                    return Err(error::error!(Syntax, "Expected `)`, found {next}"));
                } else {
                    self.consume()?;
                }

                expr = ASTNode::Call(Box::new(expr), vals);
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn base_expr(&mut self) -> Result<ASTNode> {
        let tok = self.consume()?;

        match tok {
            Token::Value(val) => Ok(ASTNode::Value(val)),
            Token::Identifier(identifier) => Ok(ASTNode::Identifier(identifier)),
            Token::If => self.condition_expr(),
            Token::Loop => self.loop_expr(),
            Token::Fn => self.fn_expr(),
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

    fn fn_expr(&mut self) -> Result<ASTNode> {
        let identifier = if matches!(self.peek()?, Token::Identifier(_)) {
            let Token::Identifier(identifier) = self.consume()? else {
                unreachable!("next is identifier")
            };
            Some(identifier)
        } else {
            None
        };

        let mut args = Vec::new();
        if matches!(self.peek()?, &Token::LP) {
            self.consume()?;

            if !matches!(self.peek()?, &Token::RP) {
                let arg = match self.peek()? {
                    Token::Identifier(_) => {
                        let Token::Identifier(identifier) = self.consume()? else {
                            unreachable!("next is identifier")
                        };
                        identifier
                    }
                    o => return Err(error::error!(Syntax, "Expected identifier, found {o}")),
                };

                args.push(arg);

                loop {
                    if matches!(self.peek()?, &Token::Comma) {
                        self.consume()?;
                        let arg = match self.peek()? {
                            Token::Identifier(_) => {
                                let Token::Identifier(identifier) = self.consume()? else {
                                    unreachable!("next is identifier")
                                };
                                identifier
                            }
                            o => {
                                return Err(error::error!(
                                    Syntax,
                                    "Expected identifier, found {o}"
                                ));
                            }
                        };

                        args.push(arg);
                    } else {
                        break;
                    }
                }
            }

            if let next = self.peek()?
                && next != &Token::RP
            {
                return Err(error::error!(Syntax, "Expected `)`, found {next}"));
            } else {
                self.consume()?;
            }
        } else {
            return Err(error::error!(Syntax, "Expected `(`, but not found"));
        }

        if self.peek()? != &Token::LB {
            return Err(error::error!(Syntax, "Expected `{{`, but not found"));
        }

        self.consume()?;
        let block = self.block()?;

        Ok(ASTNode::Fn(identifier, args, Rc::new(block)))
    }
}
