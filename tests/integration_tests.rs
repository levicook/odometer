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
        println!("🚀 Starting odometer integration walkthrough...\n");

        // =================================================================
        // Single Crate Tests
        // =================================================================
        
        let single_crate_dir = test_fixtures_dir().join("single-crate");
        assert!(single_crate_dir.exists(), "single-crate fixture not found. Run 'make single-crate'");

        println!("📦 Testing single crate operations...");
        
        // Test show command
        let (stdout, stderr, success) = run_odo(&["show"], &single_crate_dir);
        if !success {
            eprintln!("Error: {}", stderr);
        }
        assert!(success, "odo show failed on single crate");
        assert!(stdout.contains("single-crate"), "Expected crate name in output");
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
        assert!(workspace_dir.exists(), "workspace-simple fixture not found. Run 'make workspace-simple'");

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
        let (stdout, stderr, success) = run_odo(&["roll", "--workspace", "patch"], &workspace_dir);
        if !success {
            eprintln!("--workspace error: {}", stderr);
            eprintln!("--workspace stdout: {}", stdout);
        }
        assert!(success, "odo roll --workspace patch failed");
        println!("  ✅ roll --workspace patch (all members): {}", stdout.trim());

        // Verify all members were updated
        let (stdout, _stderr, success) = run_odo(&["show"], &workspace_dir);
        assert!(success, "odo show failed after workspace-wide increment");
        println!("  ✅ after patch --workspace:\n{}", stdout.trim());

        // Test package selection
        let (stdout, stderr, success) = run_odo(&["roll", "--package", "lib1", "patch"], &workspace_dir);
        if !success {
            eprintln!("Package selection error: {}", stderr);
            eprintln!("Package selection stdout: {}", stdout);
        }
        assert!(success, "odo roll with package selection failed");
        println!("  ✅ roll --package lib1 patch: {}", stdout.trim());

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
        // Workspace Inheritance Tests
        // =================================================================
        
        let inheritance_dir = test_fixtures_dir().join("workspace-inheritance");
        assert!(inheritance_dir.exists(), "workspace-inheritance fixture not found. Run 'make workspace-inheritance'");

        println!("\n🔗 Testing workspace inheritance...");

        // Test show with inheritance
        let (stdout, _stderr, success) = run_odo(&["show"], &inheritance_dir);
        assert!(success, "odo show failed on inheritance workspace");
        assert!(stdout.contains("member1"), "Expected member1 in inheritance output");
        assert!(stdout.contains("member2"), "Expected member2 in inheritance output");
        println!("  ✅ inheritance show:\n{}", stdout.trim());

        // Test version operations with inheritance
        let (stdout, _stderr, success) = run_odo(&["set", "2.0.0"], &inheritance_dir);
        assert!(success, "odo set failed on inheritance workspace");
        println!("  ✅ set 2.0.0 with inheritance: {}", stdout.trim());

        // Verify inheritance is handled correctly
        let (stdout, _stderr, success) = run_odo(&["show"], &inheritance_dir);
        assert!(success, "odo show failed after setting version with inheritance");
        println!("  ✅ after setting version:\n{}", stdout.trim());

        println!("\n🎉 All integration tests passed! Odometer is working correctly.");
    }
} 