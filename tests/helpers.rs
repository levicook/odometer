use std::{
    path::{Path, PathBuf},
    process::Command,
};

pub fn build_fixture_path(fixture_name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(fixture_name)
}

pub fn run_make(args: &[&str]) {
    let cwd = Path::new(env!("CARGO_MANIFEST_DIR"));

    Command::new("make")
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("Failed to run make command");
}

pub fn run_odo(args: &[&str], cwd: &Path) -> (bool, Vec<String>, Vec<String>) {
    let default_odo_binary = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/debug/odo");

    let odo_binary = std::env::var("ODO_BINARY").unwrap_or_else(|_| {
        run_make(&["build"]);
        default_odo_binary.to_string_lossy().to_string()
    });

    let output = Command::new(odo_binary)
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("Failed to run odo command");

    let success = output.status.success();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    (
        success,
        stdout.split("\n").map(String::from).collect(),
        stderr.split("\n").map(String::from).collect(),
    )
}
