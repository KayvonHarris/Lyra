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
        "lyra-native-float-test-{}-{nonce}",
        std::process::id()
    ))
}

fn clang_available() -> bool {
    Command::new("clang")
        .arg("--version")
        .status()
        .is_ok_and(|status| status.success())
}

#[test]
fn native_float_pipeline_executes_mixed_numeric_control_flow() {
    if !clang_available() {
        eprintln!("skipping native Float test because clang is unavailable");
        return;
    }

    let temp = unique_temp_dir();
    fs::create_dir_all(&temp).expect("temporary test directory should be created");
    let source = temp.join("native_float.ly");
    fs::write(
        &source,
        r#"fn adjust(value: Float, delta: Float) -> Float {
    var result = value + delta;
    if result > 42.0 {
        result = result - 1.0;
    } else {
        result = result + 1.0;
    }
    return result;
}

fn main() -> Int {
    let promoted = 40 + 2.5;
    let adjusted = adjust(promoted, 1.5);
    if adjusted == 43.0 {
        return 42;
    } else {
        return 1;
    }
}
"#,
    )
    .expect("Lyra Float fixture should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_lyra"))
        .arg("run")
        .arg(&source)
        .output()
        .expect("lyra CLI should execute");

    assert_eq!(
        output.status.code(),
        Some(42),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("Program exited with code 42"),
        "stdout was: {}",
        String::from_utf8_lossy(&output.stdout)
    );

    fs::remove_dir_all(&temp).expect("temporary test directory should be removed");
}
