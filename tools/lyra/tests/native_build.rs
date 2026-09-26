use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

fn unique_temp_dir() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after the Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "lyra-native-build-test-{}-{nonce}",
        std::process::id()
    ))
}

#[test]
fn build_creates_runnable_native_executable_without_llvm_sidecar() {
    if Command::new("clang").arg("--version").output().is_err() {
        eprintln!("skipping native build test because clang is unavailable");
        return;
    }

    let temp = unique_temp_dir();
    fs::create_dir_all(&temp).expect("temporary test directory should be created");
    let source = temp.join("answer.ly");
    let executable = temp.join("answer");
    fs::write(&source, "fn main() -> Int { return 42; }\n")
        .expect("Lyra source fixture should be written");

    let build = Command::new(env!("CARGO_BIN_EXE_lyra"))
        .arg("build")
        .arg(&source)
        .arg("-o")
        .arg(&executable)
        .output()
        .expect("lyra CLI should execute");

    assert!(
        build.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    assert!(executable.is_file(), "native executable was not created");
    assert!(
        !executable.with_extension("ll").exists(),
        "successful build left an LLVM sidecar"
    );

    let run = Command::new(&executable)
        .status()
        .expect("built Lyra executable should run");
    assert_eq!(run.code(), Some(42));

    fs::remove_dir_all(&temp).expect("temporary test directory should be removed");
}
