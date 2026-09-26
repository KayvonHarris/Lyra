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
    std::env::temp_dir().join(format!("lyra-native-test-{}-{nonce}", std::process::id()))
}

#[test]
fn run_executes_native_program_and_propagates_exit_status() {
    let clang_available = Command::new("clang")
        .arg("--version")
        .status()
        .is_ok_and(|status| status.success());
    if !clang_available {
        eprintln!("skipping native execution test because clang is unavailable");
        return;
    }

    let temp = unique_temp_dir();
    fs::create_dir_all(&temp).expect("temporary test directory should be created");
    let source_stem = format!(
        "exit_42_{}_{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after the Unix epoch")
            .as_nanos()
    );
    let source = temp.join(format!("{source_stem}.ly"));
    fs::write(&source, "fn main() -> Int { return 42; }\n")
        .expect("Lyra source fixture should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_lyra"))
        .arg("run")
        .arg(&source)
        .output()
        .expect("lyra CLI should execute");

    assert_eq!(output.status.code(), Some(42));
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("Program exited with code 42"),
        "stdout was: {}",
        String::from_utf8_lossy(&output.stdout)
    );

    let leftovers = fs::read_dir(std::env::temp_dir())
        .expect("system temp directory should be readable")
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(&format!("{source_stem}-"))
        })
        .collect::<Vec<_>>();
    assert!(
        leftovers.is_empty(),
        "native run left temporary artifacts: {leftovers:?}"
    );

    fs::remove_dir_all(&temp).expect("temporary test directory should be removed");
}
