use std::process::exit;

use calclang::{ctx::Context, env::Environment, lexer::Lexer, parser::Parser};
use clap::{Arg, ArgGroup, command};
use rustyline::{
    Completer, Editor, Helper, Highlighter, Hinter,
    error::ReadlineError,
    validate::{ValidationContext, ValidationResult, Validator},
};

#[derive(Helper, Completer, Highlighter, Hinter)]
struct InputHelper;

impl Validator for InputHelper {
    fn validate(&self, ctx: &mut ValidationContext) -> rustyline::Result<ValidationResult> {
        let input = ctx.input();

        if input.trim_end().ends_with('\\')
            || input.chars().filter(|&c| c == '(').count()
                != input.chars().filter(|&c| c == ')').count()
            || input.chars().filter(|&c| c == '{').count()
                != input.chars().filter(|&c| c == '}').count()
        {
            Ok(ValidationResult::Incomplete)
        } else {
            Ok(ValidationResult::Valid(None))
        }
    }
}

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

    let mut env_borrowed = env.borrow_mut();
    env_borrowed.define_builtin("println", |vals| {
        for val in vals {
            print!("{val}");
        }

        println!();

        Ok(calclang::ast::Value::None)
    });
    env_borrowed.define_builtin("exit", |_| {
        exit(0);
    });
    drop(env_borrowed);

    if let Some(expr) = matches.get_one::<String>("expr") {
        let tokens = Lexer::tokenize(expr).map_err(|e| anyhow::anyhow!("{e}"))?;
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().map_err(|e| anyhow::anyhow!("{e}"))?;
        let res = ast
            .eval(env, Context::default())
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        println!("{res}");
    } else if let Some(input) = matches.get_one::<String>("input") {
        let expr = std::fs::read_to_string(input)?;

        let tokens = Lexer::tokenize(&expr).map_err(|e| anyhow::anyhow!("{e}"))?;
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().map_err(|e| anyhow::anyhow!("{e}"))?;
        let res = ast
            .eval(env, Context::default())
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        println!("{res}");
    } else {
        let mut rl = Editor::new()?;
        rl.set_helper(Some(InputHelper));

        let _ = rl.load_history(".calc_history");

        loop {
            let readline = rl.readline("calc>> ");

            match readline {
                Ok(expr) => {
                    rl.add_history_entry(expr.as_str())?;

                    let tokens = match Lexer::tokenize(&expr) {
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
                Err(ReadlineError::Interrupted) => {
                    println!("KeyboardInterrupt");
                    println!("TIPS: If you want to exit, use \"exit()\" or \"Ctrl + D\" instead.");
                }
                Err(ReadlineError::Eof) => {
                    break;
                }
                Err(err) => {
                    eprintln!("REPL Error: {:?}", err);
                }
            }
        }
    }

    Ok(())
}
