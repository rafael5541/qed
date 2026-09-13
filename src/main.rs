use std::env;
use std::fs;
use std::io::IsTerminal;

use qed::interp::Interpreter;
use qed::lexer::Lexer;
use qed::parser::Parser;

fn main() {
    let path = match env::args().nth(1) {
        Some(path) => path,
        None => {
            eprintln!("usage: qed <file.formula>");
            std::process::exit(2);
        }
    };
    let src = match fs::read_to_string(&path) {
        Ok(src) => src,
        Err(err) => {
            eprintln!("{path}: {err}");
            std::process::exit(1);
        }
    };
    let tokens = match Lexer::new(&src).lex() {
        Ok(tokens) => tokens,
        Err(err) => {
            eprintln!("{path}: {err}");
            std::process::exit(1);
        }
    };
    let program = match Parser::new(tokens).parse() {
        Ok(program) => program,
        Err(err) => {
            eprintln!("{path}: {err}");
            std::process::exit(1);
        }
    };
    let stdin_text = if std::io::stdin().is_terminal() {
        String::new()
    } else {
        match std::io::read_to_string(std::io::stdin()) {
            Ok(text) => text,
            Err(err) => {
                eprintln!("stdin: {err}");
                std::process::exit(1);
            }
        }
    };
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    if let Err(err) = Interpreter::run_streaming(&program, &stdin_text, &src, &mut handle) {
        eprintln!("{path}: {err}");
        std::process::exit(1);
    }
}
