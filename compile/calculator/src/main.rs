use std::collections::HashMap;

use crate::{
    ast::Value,
    parser::{Parser, Token},
};

mod ast;
mod parser;

fn main() -> anyhow::Result<()> {
    loop {
        let expr = utils::io::input::input!("Input expr: ", String)?;

        let tokens = match Token::tokenize(&expr) {
            Ok(toks) => toks,
            Err(e) => {
                eprintln!("Error Tokenize: {e}");
                continue;
            }
        };

        println!("Tokens: {tokens:#?}");

        let mut parser = Parser::new(tokens);

        match parser.parse() {
            Ok(res) => {
                let mut env: HashMap<String, Value> = HashMap::new();
                println!("AST Node: {res:#?}");

                match res.eval(&mut env) {
                    Ok(res) => println!("Result: {res}"),
                    Err(e) => {
                        eprintln!("Error: {e}")
                    }
                }
            }
            Err(e) => {
                eprintln!("Error: {e}");
            }
        }
    }
}
