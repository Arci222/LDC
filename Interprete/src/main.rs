use std::{
    env,
    ffi::{OsStr, OsString},
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

use lalrpop_util::lalrpop_mod;

mod ast;
mod codegen;

lalrpop_mod!(pub parser);

const GENERATED_MARKER: &str = "/* Generato da ldc: puoi modificare questo file. */";

fn main() -> ExitCode {
    match run_cli(env::args_os().skip(1)) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("Errore: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run_cli(arguments: impl Iterator<Item = OsString>) -> Result<ExitCode, String> {
    let command = parse_command(arguments)?;
    match command {
        CliCommand::Help => {
            print_help();
            Ok(ExitCode::SUCCESS)
        }
        CliCommand::EmitC(path) => {
            let program = load_program(&path)?;
            emit_c(&program)?;
            Ok(ExitCode::SUCCESS)
        }
        CliCommand::Run(path) => {
            let program = load_program(&path)?;
            run_interpreter(program, &path)
        }
        CliCommand::Generate(path) => {
            let program = load_program(&path)?;
            let output_path = write_generated_c(&path, &program)?;
            println!("Generated:\n{}", output_path.display());
            Ok(ExitCode::SUCCESS)
        }
        CliCommand::Build(path) => {
            let program = load_program(&path)?;
            build_executable(&path, &program)?;
            Ok(ExitCode::SUCCESS)
        }
        CliCommand::Execute(path) => {
            let program = load_program(&path)?;
            let executable_path = build_executable(&path, &program)?;
            execute_binary(&executable_path)
        }
    }
}

enum CliCommand {
    Help,
    Generate(PathBuf),
    EmitC(PathBuf),
    Run(PathBuf),
    Build(PathBuf),
    Execute(PathBuf),
}

fn parse_command(mut arguments: impl Iterator<Item = OsString>) -> Result<CliCommand, String> {
    let first = arguments.next().ok_or_else(usage)?;
    if first == OsStr::new("--help") || first == OsStr::new("-h") {
        return Ok(CliCommand::Help);
    }

    let (mode, path) = if first == OsStr::new("--emit-c") {
        (CliMode::EmitC, arguments.next().map(PathBuf::from))
    } else if first == OsStr::new("--run") {
        (CliMode::Run, arguments.next().map(PathBuf::from))
    } else if first == OsStr::new("--build") {
        (CliMode::Build, arguments.next().map(PathBuf::from))
    } else if first == OsStr::new("--exec") {
        (CliMode::Execute, arguments.next().map(PathBuf::from))
    } else if first.to_string_lossy().starts_with('-') {
        return Err(format!(
            "opzione sconosciuta '{}'. Usa --help per vedere i comandi.",
            first.to_string_lossy()
        ));
    } else {
        (CliMode::Generate, Some(PathBuf::from(first)))
    };

    let path = path.ok_or_else(|| format!("manca il file sorgente.\n{}", usage()))?;
    if arguments.next().is_some() {
        return Err(format!("passa un solo file per comando.\n{}", usage()));
    }

    Ok(match mode {
        CliMode::Generate => CliCommand::Generate(path),
        CliMode::EmitC => CliCommand::EmitC(path),
        CliMode::Run => CliCommand::Run(path),
        CliMode::Build => CliCommand::Build(path),
        CliMode::Execute => CliCommand::Execute(path),
    })
}

enum CliMode {
    Generate,
    EmitC,
    Run,
    Build,
    Execute,
}

fn usage() -> String {
    "Uso: ldc <file.spln> | ldc --emit-c <file.spln> | ldc --run <file.spln> | ldc --build <file.spln> | ldc --exec <file.spln>".to_string()
}

fn print_help() {
    println!("{}", usage());
    println!("  <file.spln>         genera C in ldc-output accanto al sorgente");
    println!("  --emit-c <file>      scrive il C generato su stdout");
    println!("  --run <file>         interpreta il sorgente");
    println!("  --build <file>       genera il C e compila ldc-output/main");
    println!("  --exec <file>        compila e avvia ldc-output/main");
}

fn load_program(source_path: &Path) -> Result<ast::Program, String> {
    let absolute_path = fs::canonicalize(source_path).map_err(|error| {
        format!(
            "non riesco a risolvere '{}': {error}",
            source_path.display()
        )
    })?;
    let source = fs::read_to_string(&absolute_path).map_err(|error| {
        format!(
            "non riesco a leggere '{}': {error}",
            absolute_path.display()
        )
    })?;
    parser::ProgramParser::new()
        .parse(&source)
        .map_err(|error| {
            format!(
                "errore di sintassi in '{}': {error}",
                absolute_path.display()
            )
        })
}

fn run_interpreter(program: ast::Program, source_path: &Path) -> Result<ExitCode, String> {
    let output = program
        .run()
        .map_err(|error| format!("errore semantico in '{}': {error}", source_path.display()))?;
    for line in output {
        println!("{line}");
    }
    Ok(ExitCode::SUCCESS)
}

fn emit_c(program: &ast::Program) -> Result<(), String> {
    io::stdout()
        .write_all(codegen::generate(program).as_bytes())
        .map_err(|error| format!("non riesco a scrivere il C su stdout: {error}"))
}

fn write_generated_c(source_path: &Path, program: &ast::Program) -> Result<PathBuf, String> {
    let stem = source_path
        .file_stem()
        .unwrap_or_else(|| OsStr::new("output"));
    let mut file_name = stem.to_os_string();
    file_name.push(".c");
    write_generated_c_named(source_path, program, &file_name)
}

fn write_generated_c_named(
    source_path: &Path,
    program: &ast::Program,
    file_name: &OsStr,
) -> Result<PathBuf, String> {
    let output_directory = output_directory(source_path)?;
    fs::create_dir_all(&output_directory).map_err(|error| {
        format!(
            "non riesco a creare '{}': {error}",
            output_directory.display()
        )
    })?;
    let output_path = output_directory.join(file_name);

    if output_path.exists() {
        let existing = fs::read_to_string(&output_path).map_err(|error| {
            format!("non riesco a leggere '{}': {error}", output_path.display())
        })?;
        if !existing
            .lines()
            .next()
            .is_some_and(|line| line == GENERATED_MARKER)
        {
            return Err(format!(
                "'{}' esiste gia' e non sembra generato da ldc; non l'ho sovrascritto",
                output_path.display()
            ));
        }
    }

    let generated_c = format!("{GENERATED_MARKER}\n{}", codegen::generate(program));
    fs::write(&output_path, generated_c)
        .map_err(|error| format!("non riesco a scrivere '{}': {error}", output_path.display()))?;
    Ok(output_path)
}

fn output_directory(source_path: &Path) -> Result<PathBuf, String> {
    let absolute_path = fs::canonicalize(source_path).map_err(|error| {
        format!(
            "non riesco a risolvere '{}': {error}",
            source_path.display()
        )
    })?;
    let parent = absolute_path.parent().ok_or_else(|| {
        format!(
            "non riesco a determinare la cartella di '{}'",
            absolute_path.display()
        )
    })?;
    Ok(parent.join("ldc-output"))
}

fn build_executable(source_path: &Path, program: &ast::Program) -> Result<PathBuf, String> {
    let c_path = write_generated_c_named(source_path, program, OsStr::new("main.c"))?;
    let output_directory = c_path
        .parent()
        .ok_or_else(|| "non riesco a trovare la cartella del C generato".to_string())?;
    let compiler = find_c_compiler()?;
    let executable_name = if cfg!(windows) { "main.exe" } else { "main" };
    let executable_path = output_directory.join(executable_name);

    println!("Generated C: {}", c_path.display());
    println!(
        "Using C compiler: {}",
        compiler.executable.to_string_lossy()
    );

    let status = compile_c(&compiler, output_directory, executable_name)?;
    if !status.success() {
        return Err(format!(
            "la compilazione con '{}' e' fallita ({status})",
            compiler.executable.to_string_lossy()
        ));
    }

    println!("Build completed: {}", executable_path.display());
    Ok(executable_path)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CompilerKind {
    Clang,
    Gcc,
    Msvc,
}

struct CCompiler {
    executable: OsString,
    kind: CompilerKind,
}

fn find_c_compiler() -> Result<CCompiler, String> {
    #[cfg(windows)]
    let candidates = [
        ("clang", CompilerKind::Clang),
        ("gcc", CompilerKind::Gcc),
        ("cl.exe", CompilerKind::Msvc),
    ];
    #[cfg(not(windows))]
    let candidates = [
        ("clang", CompilerKind::Clang),
        ("gcc", CompilerKind::Gcc),
        ("cc", CompilerKind::Gcc),
    ];

    for (name, kind) in candidates {
        let probe_argument = if kind == CompilerKind::Msvc {
            "/?"
        } else {
            "--version"
        };
        if Command::new(name).arg(probe_argument).output().is_ok() {
            return Ok(CCompiler {
                executable: OsString::from(name),
                kind,
            });
        }
    }

    let searched = if cfg!(windows) {
        "clang, gcc e cl.exe"
    } else {
        "clang, gcc e cc"
    };
    Err(format!(
        "nessun compilatore C trovato nel PATH. Ho cercato {searched}; installane uno e riapri il terminale."
    ))
}

fn compile_c(
    compiler: &CCompiler,
    output_directory: &Path,
    executable_name: &str,
) -> Result<std::process::ExitStatus, String> {
    let mut command = Command::new(&compiler.executable);
    command.current_dir(output_directory);

    if compiler.kind == CompilerKind::Msvc {
        command.args(["/nologo", "/std:c11", "main.c"]);
        command.arg(format!("/Fe{executable_name}"));
    } else {
        command.args(["-std=c11", "main.c", "-o", executable_name]);
        if compiler.kind == CompilerKind::Gcc || !cfg!(windows) {
            command.arg("-lm");
        }
    }

    command
        .status()
        .map_err(|error| format!("non riesco ad avviare il compilatore C: {error}"))
}

fn execute_binary(executable_path: &Path) -> Result<ExitCode, String> {
    let status = Command::new(executable_path).status().map_err(|error| {
        format!(
            "non riesco ad avviare '{}': {error}",
            executable_path.display()
        )
    })?;
    Ok(ExitCode::from(status.code().unwrap_or(1) as u8))
}
