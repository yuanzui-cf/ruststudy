//! expr = (Token::Neg)? term ((Token::Add | Token::Neg) term)*
//! term = power ((Token::Mul | Token::Div) power)*
//! power = factor (Token::Exp power)?
//! factor = Token::Number | Token::LP expr Token::RP

use std::{fmt::Display, iter::Peekable, str::Chars, vec::IntoIter};

#[derive(Debug, Clone, Copy, PartialEq)]
enum Token {
    Number(f64),
    Add,
    Neg,
    Mul,
    Div,
    Exp,
    LP,
    RP,
    End,
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Token::Number(num) = self {
            write!(f, "{num}")?;
        } else {
            write!(
                f,
                "{}",
                match *self {
                    Token::Add => "+",
                    Token::Neg => "-",
                    Token::Mul => "*",
                    Token::Div => "/",
                    Token::Exp => "^",
                    Token::LP => "(",
                    Token::RP => ")",
                    Token::Number(_) | Token::End => "",
                },
            )?;
        }

        Ok(())
    }
}

struct Parser {
    tokens: Peekable<IntoIter<Token>>,
}

impl Parser {
    pub fn tokenize(expr: &str) -> anyhow::Result<Self> {
        let mut tokens = vec![];
        let mut expr = expr.chars().peekable();

        fn get_number(
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

        loop {
            let chr = expr.next();

            match chr {
                Some('+') => tokens.push(Token::Add),
                Some('-') => tokens.push(Token::Neg),
                Some('*' | '×') => tokens.push(Token::Mul),
                Some('/' | '÷') => tokens.push(Token::Div),
                Some('^') => tokens.push(Token::Exp),
                Some('(') => tokens.push(Token::LP),
                Some(')') => tokens.push(Token::RP),
                Some('.') => {
                    let num = get_number(&mut expr, '.', true)?;
                    tokens.push(Token::Number(num));
                }
                Some(num) if num.is_ascii_digit() => {
                    let num = get_number(&mut expr, num, false)?;
                    tokens.push(Token::Number(num));
                }
                Some(' ') => {
                    continue;
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

        Ok(Self {
            tokens: tokens.into_iter().peekable(),
        })
    }

    pub fn cusume(&mut self) -> anyhow::Result<Token> {
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

    pub fn parse(&mut self) -> anyhow::Result<f64> {
        let res = self.expr()?;

        if let next = self.peek()?
            && next != &Token::End
        {
            anyhow::bail!("Unexpected trailing token `{next}`.");
        }

        Ok(res)
    }

    pub fn expr(&mut self) -> anyhow::Result<f64> {
        let mut neg_flag = false;

        if self.peek()? == &Token::Neg {
            self.cusume()?;
            neg_flag = true;
        }

        let mut res = self.term()?;

        if neg_flag {
            res = -res;
        }

        loop {
            let tok = self.peek()?;

            match tok {
                Token::Add => {
                    self.cusume()?;
                    res += self.term()?
                }
                Token::Neg => {
                    self.cusume()?;
                    res -= self.term()?
                }
                _ => break,
            };
        }

        Ok(res)
    }

    pub fn term(&mut self) -> anyhow::Result<f64> {
        let mut res = self.power()?;

        loop {
            let tok = self.peek()?;

            match tok {
                Token::Mul => {
                    self.cusume()?;
                    res *= self.power()?
                }
                Token::Div => {
                    self.cusume()?;
                    res /= self.power()?
                }
                _ => break,
            };
        }

        Ok(res)
    }

    pub fn power(&mut self) -> anyhow::Result<f64> {
        let mut res = self.factor()?;

        let tok = self.peek()?;

        if matches!(tok, Token::Exp) {
            self.cusume()?;
            res = res.powf(self.power()?)
        }

        Ok(res)
    }

    pub fn factor(&mut self) -> anyhow::Result<f64> {
        let tok = self.cusume()?;

        match tok {
            Token::Number(num) => Ok(num),
            Token::LP => {
                let res = self.expr()?;

                if self.peek()? != &Token::RP {
                    anyhow::bail!("Expected `)`, but not found");
                } else {
                    self.cusume()?;
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

fn main() -> anyhow::Result<()> {
    loop {
        let expr = utils::io::input::input!("Input expr: ", String)?;

        let mut parser = match Parser::tokenize(&expr) {
            Ok(parser) => parser,
            Err(e) => {
                eprintln!("Error: {e}");
                continue;
            }
        };

        match parser.parse() {
            Ok(res) => {
                println!("Result: {res}");
            }
            Err(e) => {
                eprintln!("Error: {e}");
            }
        }
    }
}
