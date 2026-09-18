use std::{env, fs, process::ExitCode};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(first) = args.next() else {
        eprintln!("usage: lyra [--emit-llvm] <file.ly>");
        return ExitCode::from(2);
    };

    let (emit_llvm, path) = if first == "--emit-llvm" {
        let Some(path) = args.next() else {
            eprintln!("usage: lyra --emit-llvm <file.ly>");
            return ExitCode::from(2);
        };
        (true, path)
    } else {
        (false, first)
    };

    let source = match fs::read_to_string(&path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("lyra: could not read {path}: {error}");
            return ExitCode::FAILURE;
        }
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
