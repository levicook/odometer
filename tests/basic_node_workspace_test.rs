mod helpers;
use helpers::*;

#[test]
fn basic_node_workspace_test() {
    run_make(&["fixtures.node.basic-node-workspace"]);

    let fixture_path = build_fixture_path("basic-node-workspace");

    let (success, stdout, stderr) = run_odo(&["show"], &fixture_path);
    assert!(success, "odo show failed:\n{}", stderr.join("\n"));
    assert_eq!(
        stdout,
        vec![
            "basic-node-workspace: 1.0.0",
            "bin1: 1.0.0",
            "bin2: 1.0.0",
            "lib1: 1.0.0",
            "lib2: 1.0.0",
        ],
        "stdout:\n{}",
        stdout.join("\n")
    );

    let (success, stdout, stderr) = run_odo(&["roll", "patch", "--workspace"], &fixture_path);
    assert!(success, "odo roll patch failed:\n{}", stderr.join("\n"));
    assert_eq!(
        stdout,
        vec![
            "basic-node-workspace: 1.0.0 → 1.0.1",
            "bin1: 1.0.0 → 1.0.1",
            "bin2: 1.0.0 → 1.0.1",
            "lib1: 1.0.0 → 1.0.1",
            "lib2: 1.0.0 → 1.0.1",
        ],
        "stdout:\n{}",
        stdout.join("\n")
    );

    let (success, stdout, stderr) = run_odo(&["roll", "minor", "--package", "bin1"], &fixture_path);
    assert!(success, "odo roll patch failed:\n{}", stderr.join("\n"));
    assert_eq!(
        stdout,
        vec![
            "bin1: 1.0.1 → 1.1.0", //
        ],
        "stdout:\n{}",
        stdout.join("\n")
    );

    let (success, stdout, stderr) = run_odo(&["show"], &fixture_path);
    assert!(success, "odo show failed:\n{}", stderr.join("\n"));
    assert_eq!(
        stdout,
        vec![
            "basic-node-workspace: 1.0.1",
            "bin1: 1.1.0", //
            "bin2: 1.0.1",
            "lib1: 1.0.1",
            "lib2: 1.0.1",
        ],
        "stdout:\n{}",
        stdout.join("\n")
    );

    let (success, stdout, stderr) = run_odo(&["set", "1.10.0", "--workspace"], &fixture_path);
    assert!(success, "odo show failed:\n{}", stderr.join("\n"));
    assert_eq!(
        stdout,
        vec![
            "basic-node-workspace: 1.0.1 → 1.10.0",
            "bin1: 1.1.0 → 1.10.0",
            "bin2: 1.0.1 → 1.10.0",
            "lib1: 1.0.1 → 1.10.0",
            "lib2: 1.0.1 → 1.10.0",
        ],
        "stdout:\n{}",
        stdout.join("\n")
    );

    let (success, stdout, stderr) = run_odo(&["roll", "minor", "-8", "--workspace"], &fixture_path);
    assert!(success, "odo show failed:\n{}", stderr.join("\n"));
    assert_eq!(
        stdout,
        vec![
            "basic-node-workspace: 1.10.0 → 1.2.0",
            "bin1: 1.10.0 → 1.2.0", //
            "bin2: 1.10.0 → 1.2.0",
            "lib1: 1.10.0 → 1.2.0",
            "lib2: 1.10.0 → 1.2.0",
        ],
        "stdout:\n{}",
        stdout.join("\n")
    );
}
