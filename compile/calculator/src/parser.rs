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
                        self.consume()?;

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
            } else if res.is_statement_without_semi() && self.peek()? != &end_tok {
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
        self.expr_at(0)
    }

    fn expr_at(&mut self, min_bp: u8) -> Result<ASTNode> {
        let tok = self.consume()?;

        let mut left = match tok {
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

            Token::Op(Op::Sub) => {
                if min_bp > 40 {
                    return Err(error::error!(
                        Syntax,
                        "Unary '-' not allowed here. Use parentheses, e.g., -(-x) or (-1)"
                    ));
                }

                let res = self.expr_at(60)?;

                Ok(ASTNode::Negate(Box::new(res)))
            }
            Token::Op(Op::Not) => {
                let res = self.expr_at(60)?;

                Ok(ASTNode::Unary(Op::Not, Box::new(res)))
            }

            Token::End => Err(error::error!(
                Syntax,
                "Expected a number or an expression, but reached the end of the expression"
            )),
            _ => Err(error::error!(
                Syntax,
                "Expected a number or an expression, but found `{tok}`"
            )),
        }?;

        loop {
            let next = self.peek()?;
            let bp = next.binding_power();

            if let Some((left_bp, right_bp)) = bp {
                if left_bp < min_bp {
                    break;
                }

                let tok = self.consume()?;

                left = match tok {
                    Token::LP => {
                        let mut vals = Vec::new();
                        if !matches!(self.peek()?, Token::RP) {
                            let val = self.expr()?;
                            vals.push(val);

                            loop {
                                let tok = self.peek()?;

                                if matches!(tok, Token::Comma) {
                                    self.consume()?;
                                    vals.push(self.expr()?);
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

                        ASTNode::Call(Box::new(left), vals)
                    }
                    Token::Op(Op::And) => {
                        let right = self.expr_at(right_bp)?;

                        ASTNode::And(Box::new(left), Box::new(right))
                    }
                    Token::Op(Op::Or) => {
                        let right = self.expr_at(right_bp)?;

                        ASTNode::Or(Box::new(left), Box::new(right))
                    }
                    Token::Op(op) => {
                        let right = self.expr_at(right_bp)?;

                        ASTNode::Binary(Box::new(left), op, Box::new(right))
                    }
                    _ => unreachable!(),
                }
            } else {
                break;
            }
        }

        Ok(left)
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
