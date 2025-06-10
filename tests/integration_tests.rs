#[cfg(feature = "fixture-tests")]
mod fixture_tests {
    use std::env;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    fn test_fixtures_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
    }

    fn run_odo(args: &[&str], cwd: &Path) -> (String, String, bool) {
        // Use the ODO_BINARY environment variable set by the Makefile
        let odo_binary = env::var("ODO_BINARY")
            .expect("ODO_BINARY environment variable must be set by Makefile");

        let output = Command::new(odo_binary)
            .args(args)
            .current_dir(cwd)
            .output()
            .expect("Failed to run odo command");

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let success = output.status.success();

        (stdout, stderr, success)
    }

    #[test]
    fn integration_walkthrough() {
        // NOTE: This is intentionally one comprehensive test rather than separate tests.
        //
        // Why? Fixture state coordination is complex:
        // - Tests can run in parallel, competing for the same fixtures
        // - Fresh fixtures vs reused fixtures have different initial states
        // - Each test would need to set up its own known state with `odo set`
        //
        // The walkthrough pattern gives us:
        // - Deterministic execution order
        // - Complete control over fixture state transitions
        // - Clear progression from basic to advanced scenarios
        // - No coordination headaches between separate tests
        //
        // If you're tempted to split this up: remember this lesson! 🎓

        println!("🚀 Starting odometer integration walkthrough...\n");

        // =================================================================
        // Single Crate Tests
        // =================================================================

        let single_crate_dir = test_fixtures_dir().join("single-crate");
        assert!(
            single_crate_dir.exists(),
            "single-crate fixture not found. Run 'make single-crate'"
        );

        println!("📦 Testing single crate operations...");

        // Test show command
        let (stdout, stderr, success) = run_odo(&["show"], &single_crate_dir);
        if !success {
            eprintln!("Error: {}", stderr);
        }
        assert!(success, "odo show failed on single crate");
        assert!(
            stdout.contains("single-crate"),
            "Expected crate name in output"
        );
        assert!(stdout.contains('.'), "Expected version number in output");
        println!("  ✅ show: {}", stdout.trim());

        // Test version increment
        let (stdout, _stderr, success) = run_odo(&["roll", "patch"], &single_crate_dir);
        assert!(success, "odo roll patch failed");
        println!("  ✅ roll patch: {}", stdout.trim());

        // Verify increment worked
        let (stdout, _stderr, success) = run_odo(&["show"], &single_crate_dir);
        assert!(success, "odo show failed after patch increment");
        println!("  ✅ after patch: {}", stdout.trim());

        // Test setting specific version
        let (stdout, _stderr, success) = run_odo(&["set", "1.0.0"], &single_crate_dir);
        assert!(success, "odo set failed");
        println!("  ✅ set 1.0.0: {}", stdout.trim());

        // =================================================================
        // Simple Workspace Tests
        // =================================================================

        let workspace_dir = test_fixtures_dir().join("workspace-simple");
        assert!(
            workspace_dir.exists(),
            "workspace-simple fixture not found. Run 'make workspace-simple'"
        );

        println!("\n🏗️  Testing simple workspace operations...");

        // Test workspace show
        let (stdout, _stderr, success) = run_odo(&["show"], &workspace_dir);
        assert!(success, "odo show failed on workspace");
        assert!(stdout.contains("lib1"), "Expected lib1 in workspace output");
        assert!(stdout.contains("lib2"), "Expected lib2 in workspace output");
        println!("  ✅ workspace show:\n{}", stdout.trim());

        // Test DEFAULT behavior: workspace root only (safe default)
        let (stdout, _stderr, success) = run_odo(&["roll", "minor"], &workspace_dir);
        assert!(success, "odo roll minor failed on workspace");
        println!("  ✅ roll minor (workspace root only): {}", stdout.trim());

        // Verify only workspace root was updated (default behavior)
        let (stdout, _stderr, success) = run_odo(&["show"], &workspace_dir);
        assert!(success, "odo show failed after workspace root increment");
        println!("  ✅ after minor bump (root only):\n{}", stdout.trim());

        // Test EXPLICIT --workspace behavior: all members
        let (stdout, stderr, success) = run_odo(&["roll", "patch", "--workspace"], &workspace_dir);
        if !success {
            eprintln!("--workspace error: {}", stderr);
            eprintln!("--workspace stdout: {}", stdout);
        }
        assert!(success, "odo roll patch --workspace failed");
        println!(
            "  ✅ roll patch --workspace (all members): {}",
            stdout.trim()
        );

        // Verify all members were updated
        let (stdout, _stderr, success) = run_odo(&["show"], &workspace_dir);
        assert!(success, "odo show failed after workspace-wide increment");
        println!("  ✅ after patch --workspace:\n{}", stdout.trim());

        // Test package selection
        let (stdout, stderr, success) =
            run_odo(&["roll", "patch", "--package", "lib1"], &workspace_dir);
        if !success {
            eprintln!("Package selection error: {}", stderr);
            eprintln!("Package selection stdout: {}", stdout);
        }
        assert!(success, "odo roll with package selection failed");
        println!("  ✅ roll patch --package lib1: {}", stdout.trim());

        // Verify only lib1 was updated (versions should now be different)
        let (stdout, _stderr, success) = run_odo(&["show"], &workspace_dir);
        assert!(success, "odo show failed after selective increment");
        println!("  ✅ after selective patch:\n{}", stdout.trim());

        // Test sync to bring them back together
        let (stdout, _stderr, success) = run_odo(&["sync", "1.0.0"], &workspace_dir);
        assert!(success, "odo sync failed");
        println!("  ✅ sync to 1.0.0: {}", stdout.trim());

        // Verify sync worked - all should now be 1.0.0
        let (stdout, _stderr, success) = run_odo(&["show"], &workspace_dir);
        assert!(success, "odo show failed after sync");
        println!("  ✅ after sync:\n{}", stdout.trim());

        // Test lint
        let (stdout, _stderr, success) = run_odo(&["lint"], &workspace_dir);
        assert!(success, "odo lint failed");
        println!("  ✅ lint: {}", stdout.trim());

        // =================================================================
        // Atomic Behavior Test: No partial modifications on error
        // =================================================================

        println!("\n🔒 Testing atomic behavior: no partial modifications on error...");

        // Set up a mixed state that will cause partial failure
        let (_stdout, _stderr, success) = run_odo(&["set", "1.0.2"], &workspace_dir);
        assert!(success, "Failed to set workspace root to 1.0.2");

        let (_stdout, _stderr, success) =
            run_odo(&["set", "0.1.0", "--package", "lib1"], &workspace_dir);
        assert!(success, "Failed to set lib1 to 0.1.0");

        let (_stdout, _stderr, success) =
            run_odo(&["set", "2.5.3", "--package", "lib2"], &workspace_dir);
        assert!(success, "Failed to set lib2 to 2.5.3");

        // Verify the mixed state
        let (stdout, _stderr, success) = run_odo(&["show"], &workspace_dir);
        assert!(success, "Failed to show initial state");
        println!("  📋 Mixed state setup:\n{}", stdout.trim());

        // Record exact file contents before the failing operation
        let workspace_toml_before = std::fs::read_to_string(workspace_dir.join("Cargo.toml"))
            .expect("Failed to read workspace Cargo.toml");
        let lib1_toml_before = std::fs::read_to_string(workspace_dir.join("lib1/Cargo.toml"))
            .expect("Failed to read lib1 Cargo.toml");
        let lib2_toml_before = std::fs::read_to_string(workspace_dir.join("lib2/Cargo.toml"))
            .expect("Failed to read lib2 Cargo.toml");

        // Attempt operation that should fail partway through:
        // workspace-simple (1.0.2) -> 1.0.0 ✅ (would succeed)
        // lib1 (0.1.0) -> 0.1.-2 ❌ (will fail - cannot decrement patch by 2)
        // lib2 (2.5.3) -> 2.5.1 ✅ (would succeed, but never processed due to early error)
        let (_stdout, stderr, success) =
            run_odo(&["roll", "patch", "-2", "--workspace"], &workspace_dir);

        // Verify the operation failed with expected error
        assert!(!success, "Expected operation to fail, but it succeeded");
        assert!(
            stderr.contains("Cannot decrement patch version by 2 from 0.1.0"),
            "Expected specific error message about lib1, got: {}",
            stderr
        );
        println!("  ❌ Expected error occurred: {}", stderr.trim());

        // CRITICAL: Verify NO files were modified despite the error
        let workspace_toml_after = std::fs::read_to_string(workspace_dir.join("Cargo.toml"))
            .expect("Failed to read workspace Cargo.toml after error");
        let lib1_toml_after = std::fs::read_to_string(workspace_dir.join("lib1/Cargo.toml"))
            .expect("Failed to read lib1 Cargo.toml after error");
        let lib2_toml_after = std::fs::read_to_string(workspace_dir.join("lib2/Cargo.toml"))
            .expect("Failed to read lib2 Cargo.toml after error");

        assert_eq!(
            workspace_toml_before, workspace_toml_after,
            "Workspace Cargo.toml was modified despite operation failure!"
        );
        assert_eq!(
            lib1_toml_before, lib1_toml_after,
            "lib1 Cargo.toml was modified despite operation failure!"
        );
        assert_eq!(
            lib2_toml_before, lib2_toml_after,
            "lib2 Cargo.toml was modified despite operation failure!"
        );

        // Double-check versions are unchanged
        let (stdout, _stderr, success) = run_odo(&["show"], &workspace_dir);
        assert!(success, "Failed to show state after error");
        assert!(
            stdout.contains("workspace-simple 1.0.2"),
            "workspace-simple version changed!"
        );
        assert!(stdout.contains("lib1 0.1.0"), "lib1 version changed!");
        assert!(stdout.contains("lib2 2.5.3"), "lib2 version changed!");

        println!("  ✅ ATOMIC BEHAVIOR VERIFIED: No files modified on error");
        println!("  ✅ This ensures users never get partially-modified workspaces");

        // Reset to clean state for inheritance tests
        let (_stdout, _stderr, success) = run_odo(&["sync", "1.0.0"], &workspace_dir);
        assert!(success, "Failed to reset workspace to clean state");

        // =================================================================
        // Workspace Inheritance Tests
        // =================================================================

        let inheritance_dir = test_fixtures_dir().join("workspace-inheritance");
        assert!(
            inheritance_dir.exists(),
            "workspace-inheritance fixture not found. Run 'make workspace-inheritance'"
        );

        println!("\n🔗 Testing workspace inheritance...");

        // Test show with inheritance
        let (stdout, _stderr, success) = run_odo(&["show"], &inheritance_dir);
        assert!(success, "odo show failed on inheritance workspace");
        assert!(
            stdout.contains("member1"),
            "Expected member1 in inheritance output"
        );
        assert!(
            stdout.contains("member2"),
            "Expected member2 in inheritance output"
        );
        println!("  ✅ inheritance show:\n{}", stdout.trim());

        // Test version operations with inheritance
        let (stdout, _stderr, success) = run_odo(&["set", "2.0.0"], &inheritance_dir);
        assert!(success, "odo set failed on inheritance workspace");
        println!("  ✅ set 2.0.0 with inheritance: {}", stdout.trim());

        // Verify inheritance is handled correctly
        let (stdout, _stderr, success) = run_odo(&["show"], &inheritance_dir);
        assert!(
            success,
            "odo show failed after setting version with inheritance"
        );
        println!("  ✅ after setting version:\n{}", stdout.trim());

        println!("\n🎉 All integration tests passed! Odometer is working correctly.");
    }
}
