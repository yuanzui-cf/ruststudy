use crate::parser::Parser;

mod ast;
mod parser;

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
                println!("AST Node: {res:#?}");
                println!("Result: {}", res.eval());
            }
            Err(e) => {
                eprintln!("Error: {e}");
            }
        }
    }
}
