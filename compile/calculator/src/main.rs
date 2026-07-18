use calclang::{
    ctx::Context,
    env::Environment,
    parser::{Parser, Token},
};
use clap::{Arg, ArgGroup, command};

fn main() -> anyhow::Result<()> {
    let matches = command!()
        .arg(
            Arg::new("expr")
                .value_name("EXPR")
                .help("Input an calc expr"),
        )
        .arg(
            Arg::new("input")
                .long("input")
                .short('i')
                .value_name("FILE")
                .required(false)
                .help("Input an .calc file"),
        )
        .group(
            ArgGroup::new("source")
                .args(["expr", "input"])
                .multiple(false),
        )
        .get_matches();

    let env = Environment::new();

    if let Some(expr) = matches.get_one::<String>("expr") {
        let tokens = Token::tokenize(expr)?;
        let mut parser = Parser::new(tokens);
        let ast = parser.parse()?;
        let res = ast.eval(env, Context::default())?;

        println!("{res}");
    } else if let Some(input) = matches.get_one::<String>("input") {
        let expr = std::fs::read_to_string(input)?;

        let tokens = Token::tokenize(&expr)?;
        let mut parser = Parser::new(tokens);
        let ast = parser.parse()?;
        let res = ast.eval(env, Context::default())?;

        println!("{res}");
    } else {
        loop {
            let mut expr = utils::io::input::input!("calc > ", String)?;

            if expr.trim() == ".exit" {
                break;
            }

            while expr.trim_end().ends_with('\\') {
                let trimmed_len = expr
                    .trim_end_matches(|c: char| c == '\\' || c.is_whitespace())
                    .len();
                expr.truncate(trimmed_len);

                let new = utils::io::input::input!("       ", String)?;
                expr.push_str(&new);
            }

            let tokens = match Token::tokenize(&expr) {
                Ok(toks) => toks,
                Err(e) => {
                    eprintln!("{e}");
                    continue;
                }
            };

            let mut parser = Parser::new(tokens);

            match parser.parse() {
                Ok(res) => match res.eval(env.clone(), Context::default()) {
                    Ok(res) => println!("{res}"),
                    Err(e) => {
                        eprintln!("{e}")
                    }
                },
                Err(e) => {
                    eprintln!("{e}");
                }
            }
        }
    }

    Ok(())
}
