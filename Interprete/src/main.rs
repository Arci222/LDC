use std::{env, fs, process};

use lalrpop_util::lalrpop_mod;

mod ast;
mod codegen;

lalrpop_mod!(pub parser);

fn main() {
    let mut arguments = env::args().skip(1);
    let first = arguments.next();
    let emit_c = first.as_deref() == Some("--emit-c");
    let path = if emit_c { arguments.next() } else { first }.unwrap_or_else(|| {
        eprintln!("Uso: Interprete [--emit-c] <file.spln>");
        process::exit(2);
    });

    let source = fs::read_to_string(&path).unwrap_or_else(|error| {
        eprintln!("Impossibile leggere '{path}': {error}");
        process::exit(2);
    });

    let program = parser::ProgramParser::new()
        .parse(&source)
        .unwrap_or_else(|error| {
            eprintln!("Errore di sintassi in '{path}': {error}");
            process::exit(1);
        });

    if emit_c {
        print!("{}", codegen::generate(&program));
        return;
    }

    let output = program.run().unwrap_or_else(|error| {
        eprintln!("Errore semantico in '{path}': {error}");
        process::exit(1);
    });

    for line in output {
        println!("{line}");
    }
}
