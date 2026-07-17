use calclang::{
    ctx::Context,
    env::Environment,
    parser::{Parser, Token},
};

fn main() -> anyhow::Result<()> {
    let env = Environment::new();

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
                println!("AST Node: {res:#?}");

                match res.eval(env.clone(), Context::default()) {
                    Ok(res) => println!("Result: {res}"),
                    Err(e) => {
                        eprintln!("Error: {e}")
                    }
                }

                let env = env.borrow();
                println!("Environment: {env:#?}");
            }
            Err(e) => {
                eprintln!("Error: {e}");
            }
        }
    }
}
