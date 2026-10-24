use std::{
    env,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process,
};

use lalrpop_util::lalrpop_mod;

mod ast;
mod codegen;

lalrpop_mod!(pub parser);

fn main() {
    let mut arguments = env::args_os().skip(1);
    let first = arguments.next().unwrap_or_else(|| {
        eprintln!("Uso: ldc <file.spln> | ldc --run <file.spln> | ldc --emit-c <file.spln>");
        process::exit(2);
    });
    let (mode, path) = if first == OsStr::new("--emit-c") {
        (Mode::EmitStdout, arguments.next())
    } else if first == OsStr::new("--run") {
        (Mode::Run, arguments.next())
    } else if first == OsStr::new("--help") || first == OsStr::new("-h") {
        println!("Uso: ldc <file.spln>              genera C in <cartella sorgente>/ldc-output/");
        println!("     ldc --run <file.spln>       interpreta il sorgente");
        println!("     ldc --emit-c <file.spln>   scrive il C su stdout");
        return;
    } else {
        (Mode::GenerateFile, Some(first))
    };
    let path = path.unwrap_or_else(|| {
        eprintln!("Manca il percorso del file sorgente.");
        process::exit(2);
    });
    if arguments.next().is_some() {
        eprintln!("Passa un solo file per comando; ripeti ldc per gli altri file.");
        process::exit(2);
    }
    let path = PathBuf::from(path);
    let absolute_path = fs::canonicalize(&path).unwrap_or_else(|error| {
        eprintln!("Impossibile risolvere '{}': {error}", path.display());
        process::exit(2);
    });

    let source = fs::read_to_string(&absolute_path).unwrap_or_else(|error| {
        eprintln!("Impossibile leggere '{}': {error}", absolute_path.display());
        process::exit(2);
    });

    let program = parser::ProgramParser::new()
        .parse(&source)
        .unwrap_or_else(|error| {
            eprintln!("Errore di sintassi in '{}': {error}", absolute_path.display());
            process::exit(1);
        });

    match mode {
        Mode::EmitStdout => print!("{}", codegen::generate(&program)),
        Mode::GenerateFile => match write_generated_c(&absolute_path, &program) {
            Ok(output_path) => println!("C generato in: {}", output_path.display()),
            Err(error) => {
                eprintln!("Errore durante la generazione C: {error}");
                process::exit(1);
            }
        },
        Mode::Run => run_program(program, &absolute_path),
    }
}

enum Mode {
    GenerateFile,
    EmitStdout,
    Run,
}

fn run_program(program: ast::Program, path: &Path) {
    let output = program.run().unwrap_or_else(|error| {
        eprintln!("Errore semantico in '{}': {error}", path.display());
        process::exit(1);
    });

    for line in output {
        println!("{line}");
    }
}

fn write_generated_c(source_path: &Path, program: &ast::Program) -> Result<PathBuf, String> {
    const GENERATED_MARKER: &str = "/* Generato da ldc: puoi modificare questo file. */";

    let parent = source_path
        .parent()
        .ok_or_else(|| "il file sorgente non ha una cartella genitore".to_string())?;
    let output_directory = parent.join("ldc-output");
    fs::create_dir_all(&output_directory).map_err(|error| {
        format!("non posso creare '{}': {error}", output_directory.display())
    })?;

    let stem = source_path
        .file_stem()
        .unwrap_or_else(|| OsStr::new("output"))
        .to_string_lossy();
    let output_path = output_directory.join(format!("{stem}.c"));
    if output_path.exists() {
        let existing = fs::read_to_string(&output_path).map_err(|error| {
            format!("non posso leggere '{}': {error}", output_path.display())
        })?;
        if !existing.lines().next().is_some_and(|line| line == GENERATED_MARKER) {
            return Err(format!(
                "'{}' esiste gia' e non sembra generato da ldc; non l'ho sovrascritto",
                output_path.display()
            ));
        }
    }

    let generated_c = format!("{GENERATED_MARKER}\n{}", codegen::generate(program));
    fs::write(&output_path, generated_c)
        .map_err(|error| format!("non posso scrivere '{}': {error}", output_path.display()))?;
    Ok(output_path)
}
