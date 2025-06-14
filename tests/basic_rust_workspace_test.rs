mod helpers;
use helpers::*;

#[test]
fn basic_rust_workspace_test() {
    run_make(&["fixtures.rust.basic-rust-workspace"]);

    let fixture_path = build_fixture_path("basic-rust-workspace");

    let (success, stdout, stderr) = run_odo(&["show"], &fixture_path);
    assert!(success, "odo show failed:\n{}", stderr.join("\n"));
    assert_eq!(
        stdout,
        vec![
            "bin1: 0.1.0", //
            "bin2: 0.1.0",
            "lib1: 0.1.0",
            "lib2: 0.1.0",
        ],
        "stdout:\n{}",
        stdout.join("\n")
    );

    let (success, stdout, stderr) = run_odo(&["roll", "patch", "--workspace"], &fixture_path);
    assert!(success, "odo roll patch failed:\n{}", stderr.join("\n"));
    assert_eq!(
        stdout,
        vec![
            "bin1: 0.1.0 → 0.1.1", //
            "bin2: 0.1.0 → 0.1.1",
            "lib1: 0.1.0 → 0.1.1",
            "lib2: 0.1.0 → 0.1.1",
        ],
        "stdout:\n{}",
        stdout.join("\n")
    );

    let (success, stdout, stderr) = run_odo(&["roll", "minor", "--package", "bin1"], &fixture_path);
    assert!(success, "odo roll patch failed:\n{}", stderr.join("\n"));
    assert_eq!(
        stdout,
        vec![
            "bin1: 0.1.1 → 0.2.0", //
        ],
        "stdout:\n{}",
        stdout.join("\n")
    );

    let (success, stdout, stderr) = run_odo(&["show"], &fixture_path);
    assert!(success, "odo show failed:\n{}", stderr.join("\n"));
    assert_eq!(
        stdout,
        vec![
            "bin1: 0.2.0", //
            "bin2: 0.1.1",
            "lib1: 0.1.1",
            "lib2: 0.1.1",
        ],
        "stdout:\n{}",
        stdout.join("\n")
    );

    let (success, stdout, stderr) = run_odo(&["set", "0.10.0", "--workspace"], &fixture_path);
    assert!(success, "odo show failed:\n{}", stderr.join("\n"));
    assert_eq!(
        stdout,
        vec![
            "bin1: 0.2.0 → 0.10.0", //
            "bin2: 0.1.1 → 0.10.0",
            "lib1: 0.1.1 → 0.10.0",
            "lib2: 0.1.1 → 0.10.0",
        ],
        "stdout:\n{}",
        stdout.join("\n")
    );

    let (success, stdout, stderr) = run_odo(&["roll", "minor", "-2", "--workspace"], &fixture_path);
    assert!(success, "odo show failed:\n{}", stderr.join("\n"));
    assert_eq!(
        stdout,
        vec![
            "bin1: 0.10.0 → 0.8.0", //
            "bin2: 0.10.0 → 0.8.0",
            "lib1: 0.10.0 → 0.8.0",
            "lib2: 0.10.0 → 0.8.0",
        ],
        "stdout:\n{}",
        stdout.join("\n")
    );
}
