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
    if Command::new("clang").arg("--version").output().is_err() {
        eprintln!("skipping native execution test because clang is unavailable");
        return;
    }

    let temp = unique_temp_dir();
    fs::create_dir_all(&temp).expect("temporary test directory should be created");
    let source = temp.join("exit_42.ly");
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

    let stem = source.file_stem().unwrap().to_string_lossy();
    let leftovers = fs::read_dir(std::env::temp_dir())
        .expect("system temp directory should be readable")
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().starts_with(&format!("{stem}-")))
        .collect::<Vec<_>>();
    assert!(leftovers.is_empty(), "native run left temporary artifacts: {leftovers:?}");

    fs::remove_dir_all(&temp).expect("temporary test directory should be removed");
}
