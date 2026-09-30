use std::process::Command;

#[test]
fn v01_factorial_example_compiles_and_runs_natively() {
    let clang_available = Command::new("clang")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success());
    if !clang_available {
        eprintln!("skipping native example test because clang is unavailable");
        return;
    }

    let source = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/v0.1/factorial.ly"
    );
    let output = Command::new(env!("CARGO_BIN_EXE_lyra"))
        .arg("run")
        .arg(source)
        .output()
        .expect("Lyra CLI should execute the factorial example");

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
}
