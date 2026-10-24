use std::{
    collections::HashMap,
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
    let project_directory = absolute_path.parent().ok_or_else(|| {
        format!(
            "non trovo la cartella del progetto per '{}': impossibile risolvere include",
            absolute_path.display()
        )
    })?;
    let mut hoisted_functions = Vec::new();
    let mut modules = HashMap::new();
    let source = expand_includes(
        &absolute_path,
        project_directory,
        &source,
        &mut vec![absolute_path.clone()],
        &mut hoisted_functions,
        &mut modules,
    )?;
    let source = rewrite_identifiers(&source, &HashMap::new(), &modules);
    let hoisted_functions = hoisted_functions
        .iter()
        .map(|function| rewrite_identifiers(function, &HashMap::new(), &modules))
        .collect::<Vec<_>>();
    let source = insert_functions_before_main(source, &hoisted_functions)?;
    parser::ProgramParser::new()
        .parse(&source)
        .map_err(|error| {
            format!(
                "errore di sintassi in '{}': {error}",
                absolute_path.display()
            )
        })
}

fn expand_includes(
    source_path: &Path,
    project_directory: &Path,
    source: &str,
    stack: &mut Vec<PathBuf>,
    hoisted_functions: &mut Vec<String>,
    all_modules: &mut HashMap<String, Vec<String>>,
) -> Result<String, String> {
    let mut expanded = String::with_capacity(source.len());
    let mut modules: HashMap<String, Vec<String>> = HashMap::new();
    let mut cursor = 0;
    while let Some((start, end, include_path)) = find_include(source, cursor) {
        expanded.push_str(&source[cursor..start]);
        let included_path =
            fs::canonicalize(project_directory.join(&include_path)).map_err(|error| {
                format!(
                    "include '{}' da '{}': {error}",
                    include_path,
                    source_path.display()
                )
            })?;
        if stack.contains(&included_path) {
            let chain = stack
                .iter()
                .chain(std::iter::once(&included_path))
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(" -> ");
            return Err(format!("ciclo negli include: {chain}"));
        }
        let included_source = fs::read_to_string(&included_path).map_err(|error| {
            format!(
                "non riesco a leggere l'include '{}': {error}",
                included_path.display()
            )
        })?;
        stack.push(included_path.clone());
        let recursively_expanded = expand_includes(
            &included_path,
            project_directory,
            &included_source,
            stack,
            hoisted_functions,
            all_modules,
        )?;
        stack.pop();
        let module_name = included_path
            .file_stem()
            .and_then(OsStr::to_str)
            .unwrap_or("module")
            .to_string();
        let (module_body, functions) = extract_module_functions(&recursively_expanded);
        let mut names = declared_names(&module_body);
        names.extend(functions.iter().map(|(name, _)| name.clone()));
        names.sort();
        names.dedup();
        let renamed = names
            .iter()
            .map(|name| (name.clone(), format!("{module_name}__{name}")))
            .collect::<HashMap<_, _>>();
        modules.insert(module_name.clone(), names.clone());
        all_modules.insert(module_name, names);
        for (_, function_source) in functions {
            hoisted_functions.push(rewrite_identifiers(
                &function_source,
                &renamed,
                &HashMap::new(),
            ));
        }
        expanded.push_str(&rewrite_identifiers(
            &module_body,
            &renamed,
            &HashMap::new(),
        ));
        cursor = end;
    }
    expanded.push_str(&source[cursor..]);
    Ok(rewrite_identifiers(&expanded, &HashMap::new(), &modules))
}

fn insert_functions_before_main(
    mut source: String,
    functions: &[String],
) -> Result<String, String> {
    if functions.is_empty() {
        return Ok(source);
    }
    let main_start = source.find("fn main").ok_or_else(|| {
        "fn main non trovato durante l'inserimento delle funzioni incluse".to_string()
    })?;
    let prelude = format!("{}\n", functions.join("\n"));
    source.insert_str(main_start, &prelude);
    Ok(source)
}

fn extract_module_functions(source: &str) -> (String, Vec<(String, String)>) {
    let bytes = source.as_bytes();
    let mut body = String::with_capacity(source.len());
    let mut functions = Vec::new();
    let mut cursor = 0;
    let mut index = 0;
    let mut brace_depth = 0usize;
    while index < bytes.len() {
        if bytes[index..].starts_with(b"//") {
            index = source[index..]
                .find('\n')
                .map_or(bytes.len(), |n| index + n + 1);
            continue;
        }
        if bytes[index..].starts_with(b"/*") {
            index = source[index + 2..]
                .find("*/")
                .map_or(bytes.len(), |n| index + n + 4);
            continue;
        }
        if bytes[index] == b'"' {
            index = skip_string(bytes, index);
            continue;
        }
        if brace_depth == 0 && bytes[index..].starts_with(b"fn") {
            let prev_is_id =
                index > 0 && (bytes[index - 1].is_ascii_alphanumeric() || bytes[index - 1] == b'_');
            let after_fn = index + 2;
            let next_is_id = after_fn < bytes.len()
                && (bytes[after_fn].is_ascii_alphanumeric() || bytes[after_fn] == b'_');
            if !prev_is_id && !next_is_id {
                let mut name_start = after_fn;
                while name_start < bytes.len() && bytes[name_start].is_ascii_whitespace() {
                    name_start += 1;
                }
                let mut name_end = name_start;
                while name_end < bytes.len()
                    && (bytes[name_end].is_ascii_alphanumeric() || bytes[name_end] == b'_')
                {
                    name_end += 1;
                }
                let name = source[name_start..name_end].to_string();
                let mut open = name_end;
                while open < bytes.len() && bytes[open] != b'{' {
                    if bytes[open] == b'"' {
                        open = skip_string(bytes, open);
                    } else {
                        open += 1;
                    }
                }
                if open < bytes.len() {
                    if let Some(close) = matching_brace(bytes, open) {
                        body.push_str(&source[cursor..index]);
                        functions.push((name, source[index..=close].to_string()));
                        cursor = close + 1;
                        index = close + 1;
                        continue;
                    }
                }
            }
        }
        match bytes[index] {
            b'{' => brace_depth += 1,
            b'}' => brace_depth = brace_depth.saturating_sub(1),
            _ => {}
        }
        index += 1;
    }
    body.push_str(&source[cursor..]);
    (body, functions)
}

fn skip_string(bytes: &[u8], start: usize) -> usize {
    let mut index = start + 1;
    while index < bytes.len() {
        if bytes[index] == b'"' && bytes[index - 1] != b'\\' {
            return index + 1;
        }
        index += 1;
    }
    bytes.len()
}

fn matching_brace(bytes: &[u8], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut index = open;
    while index < bytes.len() {
        if bytes[index..].starts_with(b"//") {
            index = bytes[index..]
                .iter()
                .position(|byte| *byte == b'\n')
                .map_or(bytes.len(), |n| index + n + 1);
            continue;
        }
        if bytes[index..].starts_with(b"/*") {
            index = bytes[index + 2..]
                .windows(2)
                .position(|pair| pair == b"*/")
                .map_or(bytes.len(), |n| index + n + 4);
            continue;
        }
        if bytes[index] == b'"' {
            index = skip_string(bytes, index);
            continue;
        }
        match bytes[index] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
        index += 1;
    }
    None
}

fn declared_names(source: &str) -> Vec<String> {
    let tokens = code_tokens(source);
    let mut names = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if token != "let" {
            continue;
        }
        let mut last_identifier = None;
        for candidate in tokens.iter().skip(index + 1) {
            if candidate == ";" {
                break;
            }
            if candidate == "=" {
                if let Some(name) = last_identifier {
                    names.push(name);
                }
                break;
            }
            if is_identifier(candidate) {
                last_identifier = Some(candidate.clone());
            }
        }
    }
    names.sort();
    names.dedup();
    names
}

fn code_tokens(source: &str) -> Vec<String> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index..].starts_with(b"//") {
            index = source[index..]
                .find('\n')
                .map_or(bytes.len(), |n| index + n + 1);
            continue;
        }
        if bytes[index..].starts_with(b"/*") {
            index = source[index + 2..]
                .find("*/")
                .map_or(bytes.len(), |n| index + n + 4);
            continue;
        }
        if bytes[index] == b'"' {
            index += 1;
            while index < bytes.len() {
                if bytes[index] == b'"' && bytes[index - 1] != b'\\' {
                    index += 1;
                    break;
                }
                index += 1;
            }
            continue;
        }
        if bytes[index].is_ascii_alphabetic() || bytes[index] == b'_' {
            let start = index;
            index += 1;
            while index < bytes.len()
                && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_')
            {
                index += 1;
            }
            tokens.push(source[start..index].to_string());
            continue;
        }
        if bytes[index] == b'=' || bytes[index] == b';' {
            tokens.push((bytes[index] as char).to_string());
        }
        index += 1;
    }
    tokens
}

fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn rewrite_identifiers(
    source: &str,
    names: &HashMap<String, String>,
    modules: &HashMap<String, Vec<String>>,
) -> String {
    let bytes = source.as_bytes();
    let mut result = String::with_capacity(source.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index..].starts_with(b"//") {
            let end = source[index..]
                .find('\n')
                .map_or(bytes.len(), |n| index + n + 1);
            result.push_str(&source[index..end]);
            index = end;
            continue;
        }
        if bytes[index..].starts_with(b"/*") {
            let end = source[index + 2..]
                .find("*/")
                .map_or(bytes.len(), |n| index + n + 4);
            result.push_str(&source[index..end]);
            index = end;
            continue;
        }
        if bytes[index] == b'"' {
            let start = index;
            index += 1;
            while index < bytes.len() {
                if bytes[index] == b'"' && bytes[index - 1] != b'\\' {
                    index += 1;
                    break;
                }
                index += 1;
            }
            result.push_str(&source[start..index]);
            continue;
        }
        if bytes[index].is_ascii_alphabetic() || bytes[index] == b'_' {
            let start = index;
            index += 1;
            while index < bytes.len()
                && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_')
            {
                index += 1;
            }
            let identifier = &source[start..index];
            let mut dot = index;
            while dot < bytes.len() && bytes[dot].is_ascii_whitespace() {
                dot += 1;
            }
            if dot < bytes.len() && bytes[dot] == b'.' {
                let next_start = dot + 1;
                let mut next_end = next_start;
                while next_end < bytes.len() && bytes[next_end].is_ascii_whitespace() {
                    next_end += 1;
                }
                let name_start = next_end;
                if name_start < bytes.len()
                    && (bytes[name_start].is_ascii_alphabetic() || bytes[name_start] == b'_')
                {
                    let mut name_end = name_start + 1;
                    while name_end < bytes.len()
                        && (bytes[name_end].is_ascii_alphanumeric() || bytes[name_end] == b'_')
                    {
                        name_end += 1;
                    }
                    if modules.get(identifier).is_some_and(|declared| {
                        declared
                            .iter()
                            .any(|name| name == &source[name_start..name_end])
                    }) {
                        result.push_str(identifier);
                        result.push_str("__");
                        result.push_str(&source[name_start..name_end]);
                        index = name_end;
                        continue;
                    }
                }
            }
            result.push_str(
                names
                    .get(identifier)
                    .map(String::as_str)
                    .unwrap_or(identifier),
            );
            continue;
        }
        let char_len = source[index..]
            .chars()
            .next()
            .map(char::len_utf8)
            .unwrap_or(1);
        result.push_str(&source[index..index + char_len]);
        index += char_len;
    }
    result
}

fn find_include(source: &str, from: usize) -> Option<(usize, usize, String)> {
    let bytes = source.as_bytes();
    let mut index = from;
    let mut in_string = false;
    while index < bytes.len() {
        if !in_string && bytes[index..].starts_with(b"//") {
            index = source[index..]
                .find('\n')
                .map_or(bytes.len(), |offset| index + offset + 1);
            continue;
        }
        if !in_string && bytes[index..].starts_with(b"/*") {
            index = source[index + 2..]
                .find("*/")
                .map_or(bytes.len(), |offset| index + 2 + offset + 2);
            continue;
        }
        if bytes[index] == b'"' && (index == 0 || bytes[index - 1] != b'\\') {
            in_string = !in_string;
            index += 1;
            continue;
        }
        if !in_string && bytes[index..].starts_with(b"include") {
            let previous_is_id =
                index > 0 && (bytes[index - 1].is_ascii_alphanumeric() || bytes[index - 1] == b'_');
            let mut end = index + "include".len();
            if previous_is_id {
                index += 1;
                continue;
            }
            while end < bytes.len() && bytes[end].is_ascii_whitespace() {
                end += 1;
            }
            if end == bytes.len() || bytes[end] != b'(' {
                index += 1;
                continue;
            }
            end += 1;
            while end < bytes.len() && bytes[end].is_ascii_whitespace() {
                end += 1;
            }
            if end == bytes.len() || bytes[end] != b'"' {
                index += 1;
                continue;
            }
            let path_start = end + 1;
            end = path_start;
            while end < bytes.len() && bytes[end] != b'"' {
                end += 1;
            }
            if end == bytes.len() {
                return None;
            }
            let include_path = source[path_start..end].to_string();
            end += 1;
            while end < bytes.len() && bytes[end].is_ascii_whitespace() {
                end += 1;
            }
            if end == bytes.len() || bytes[end] != b')' {
                index += 1;
                continue;
            }
            end += 1;
            while end < bytes.len() && bytes[end].is_ascii_whitespace() {
                end += 1;
            }
            if end == bytes.len() || bytes[end] != b';' {
                index += 1;
                continue;
            }
            return Some((index, end + 1, include_path));
        }
        index += 1;
    }
    None
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
