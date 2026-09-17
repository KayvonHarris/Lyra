use std::{env, fs, process::ExitCode};

fn main() -> ExitCode {
    let Some(path) = env::args().nth(1) else {
        eprintln!("usage: lyra <file.ly>");
        return ExitCode::from(2);
    };

    let source = match fs::read_to_string(&path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("lyra: could not read {path}: {error}");
            return ExitCode::FAILURE;
        }
    };

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
