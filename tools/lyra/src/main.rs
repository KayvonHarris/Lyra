use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(first) = args.next() else {
        print_usage();
        return ExitCode::from(2);
    };

    if first == "run" {
        let Some(path) = args.next() else {
            eprintln!("usage: lyra run <file.ly>");
            return ExitCode::from(2);
        };
        return run_native(&path);
    }

    if first == "build" {
        let Some(path) = args.next() else {
            eprintln!("usage: lyra build <file.ly> [-o output]");
            return ExitCode::from(2);
        };
        let output = parse_output_path(&mut args).unwrap_or_else(|| default_output_path(&path));
        return build_native(&path, &output);
    }

    let (emit_llvm, path) = if first == "--emit-llvm" {
        let Some(path) = args.next() else {
            eprintln!("usage: lyra --emit-llvm <file.ly>");
            return ExitCode::from(2);
        };
        (true, path)
    } else {
        (false, first)
    };

    let source = match read_source(&path) {
        Ok(source) => source,
        Err(code) => return code,
    };

    if emit_llvm {
        match lyra_driver::compile_to_llvm(&source) {
            Ok(llvm) => {
                print!("{llvm}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("lyra: {error:?}");
                ExitCode::FAILURE
            }
        }
    } else {
        let output = lyra_driver::compile(&source);
        if output.diagnostics.is_empty() {
            println!("{:#?}", output.module);
            ExitCode::SUCCESS
        } else {
            for diagnostic in output.diagnostics {
                eprintln!("{diagnostic:?}");
            }
            ExitCode::FAILURE
        }
    }
}

fn build_native(path: &str, output: &Path) -> ExitCode {
    let source = match read_source(path) {
        Ok(source) => source,
        Err(code) => return code,
    };
    let llvm = match lyra_driver::compile_to_llvm(&source) {
        Ok(llvm) => llvm,
        Err(error) => {
            eprintln!("lyra: {error:?}");
            return ExitCode::FAILURE;
        }
    };

    let llvm_path = output.with_extension("ll");
    if let Err(error) = fs::write(&llvm_path, llvm) {
        eprintln!("lyra: could not write {}: {error}", llvm_path.display());
        return ExitCode::FAILURE;
    }

    let status = Command::new("clang")
        .arg(&llvm_path)
        .arg("-o")
        .arg(output)
        .status();

    let cleanup = fs::remove_file(&llvm_path);
    if let Err(error) = cleanup {
        eprintln!(
            "lyra: could not remove temporary LLVM file {}: {error}",
            llvm_path.display()
        );
    }

    match status {
        Ok(status) if status.success() => {
            println!("Built {}", output.display());
            ExitCode::SUCCESS
        }
        Ok(status) => {
            eprintln!("lyra: clang failed with {status}");
            ExitCode::FAILURE
        }
        Err(error) => {
            eprintln!("lyra: could not invoke clang: {error}");
            eprintln!("lyra: install clang/LLVM and ensure clang is on PATH");
            ExitCode::FAILURE
        }
    }
}

fn run_native(path: &str) -> ExitCode {
    let mut output = env::temp_dir();
    let stem = Path::new(path)
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("lyra-program");
    output.push(format!("{stem}-{}", std::process::id()));

    let build_status = build_native(path, &output);
    if build_status != ExitCode::SUCCESS {
        return build_status;
    }

    let status = Command::new(&output).status();
    let _ = fs::remove_file(&output);

    match status {
        Ok(status) => match status.code() {
            Some(code) => {
                println!("Program exited with code {code}");
                if code == 0 {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::from(u8::try_from(code).unwrap_or(1))
                }
            }
            None => {
                eprintln!("lyra: program terminated by signal");
                ExitCode::FAILURE
            }
        },
        Err(error) => {
            eprintln!("lyra: could not run {}: {error}", output.display());
            ExitCode::FAILURE
        }
    }
}

fn read_source(path: &str) -> Result<String, ExitCode> {
    fs::read_to_string(path).map_err(|error| {
        eprintln!("lyra: could not read {path}: {error}");
        ExitCode::FAILURE
    })
}

fn parse_output_path(args: &mut impl Iterator<Item = String>) -> Option<PathBuf> {
    while let Some(arg) = args.next() {
        if arg == "-o" {
            return args.next().map(PathBuf::from);
        }
    }
    None
}

fn default_output_path(source: &str) -> PathBuf {
    Path::new(source)
        .file_stem()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("a.out"))
}

fn print_usage() {
    eprintln!("usage: lyra <file.ly>");
    eprintln!("       lyra --emit-llvm <file.ly>");
    eprintln!("       lyra build <file.ly> [-o output]");
    eprintln!("       lyra run <file.ly>");
}
