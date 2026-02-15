//! End-to-end integration tests for graft CLI
//!
//! These tests validate the full CLI workflow using real git repositories and
//! graft configurations.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

/// Helper to get the repository root (where Cargo.toml is)
fn repo_root() -> PathBuf {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    PathBuf::from(manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Helper to run graft CLI command
fn run_graft(args: &[&str], cwd: &Path) -> std::process::Output {
    // Get the path to the compiled graft binary
    let graft_bin = env::var("CARGO_BIN_EXE_graft").unwrap_or_else(|_| {
        // Fallback: build it on the fly
        let cargo_exe = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
        Command::new(&cargo_exe)
            .args(["build", "-p", "graft-cli"])
            .current_dir(&repo_root())
            .output()
            .expect("Failed to build graft binary");

        // Path to the built binary
        repo_root()
            .join("target/debug/graft")
            .to_str()
            .unwrap()
            .to_string()
    });

    Command::new(graft_bin)
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("Failed to execute graft command")
}

/// Helper to check if command succeeded
fn assert_success(output: &std::process::Output, context: &str) {
    if !output.status.success() {
        eprintln!("Command failed: {}", context);
        eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        panic!(
            "{} failed with exit code {:?}",
            context,
            output.status.code()
        );
    }
}

/// Helper to initialize a git repository
fn init_git_repo(path: &Path, initial_content: &str) {
    Command::new("git")
        .args(["init"])
        .current_dir(path)
        .output()
        .expect("Failed to git init");

    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(path)
        .output()
        .expect("Failed to configure git user");

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(path)
        .output()
        .expect("Failed to configure git email");

    fs::write(path.join("README.md"), initial_content).expect("Failed to write README");

    Command::new("git")
        .args(["add", "."])
        .current_dir(path)
        .output()
        .expect("Failed to git add");

    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(path)
        .output()
        .expect("Failed to git commit");
}

/// Test 1: Resolve this repository's own graft.yaml dependencies
#[test]
fn test_resolve_repo_dependencies() {
    let root = repo_root();
    let graft_yaml = root.join("graft.yaml");

    // Skip if graft.yaml doesn't exist
    if !graft_yaml.exists() {
        eprintln!("Skipping test: graft.yaml not found");
        return;
    }

    // Run status to see current state (should work even if deps not resolved)
    let _output = run_graft(&["status"], &root);
    // Status may fail if no lock file, but should not crash

    // Check that .graft directory exists (dependencies should already be resolved)
    let graft_dir = root.join(".graft");
    assert!(
        graft_dir.exists(),
        ".graft directory should exist (dependencies resolved)"
    );

    // Verify we can run status successfully now
    let output = run_graft(&["status"], &root);
    assert_success(&output, "graft status on repo dependencies");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should show some dependencies
    assert!(
        stdout.contains("Dependencies")
            || stdout.contains("dependency")
            || stdout.contains("dependencies"),
        "Status output should mention dependencies"
    );
}

/// Test 2: Round-trip status → resolve → status
#[test]
fn test_status_resolve_status_roundtrip() {
    // Create a temporary workspace with a mock dependency
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let workspace = temp_dir.path();

    // Create a mock dependency repository
    let dep_dir = workspace.join("mock-dep");
    fs::create_dir(&dep_dir).expect("Failed to create dep dir");
    init_git_repo(&dep_dir, "Mock dependency content");

    // Create consumer repository
    let consumer_dir = workspace.join("consumer");
    fs::create_dir(&consumer_dir).expect("Failed to create consumer dir");
    init_git_repo(&consumer_dir, "Consumer repository");

    // Create graft.yaml in consumer
    let graft_yaml = format!(
        r#"apiVersion: graft/v0
deps:
  mock-dep: "file://{}#main"
"#,
        dep_dir.display()
    );
    fs::write(consumer_dir.join("graft.yaml"), graft_yaml).expect("Failed to write graft.yaml");

    // Step 1: Run initial status (no dependencies resolved yet)
    let _output1 = run_graft(&["status"], &consumer_dir);
    // May succeed or fail depending on whether lock file exists

    // Step 2: Resolve dependencies
    let output2 = run_graft(&["resolve"], &consumer_dir);
    assert_success(&output2, "graft resolve");

    let stdout2 = String::from_utf8_lossy(&output2.stdout);
    assert!(
        stdout2.contains("mock-dep") || stdout2.contains("Resolved"),
        "Resolve output should show dependency resolution"
    );

    // Verify .graft directory was created
    assert!(
        consumer_dir.join(".graft").exists(),
        ".graft directory should be created"
    );
    assert!(
        consumer_dir.join(".graft/mock-dep").exists(),
        ".graft/mock-dep should exist"
    );

    // Verify lock file was created
    assert!(
        consumer_dir.join("graft.lock").exists(),
        "graft.lock should be created after resolve"
    );

    // Step 3: Run status again (should succeed now)
    let output3 = run_graft(&["status"], &consumer_dir);
    assert_success(&output3, "graft status after resolve");

    let stdout3 = String::from_utf8_lossy(&output3.stdout);
    assert!(
        stdout3.contains("mock-dep"),
        "Status should show resolved dependency"
    );
}

/// Test 3: Upgrade with rollback on failure
#[test]
fn test_upgrade_with_rollback() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let workspace = temp_dir.path();

    // Create a dependency repository with multiple versions
    let dep_dir = workspace.join("dep-with-versions");
    fs::create_dir(&dep_dir).expect("Failed to create dep dir");
    init_git_repo(&dep_dir, "Version 1.0.0");

    // Add a graft.yaml for v1.0.0 that also declares future v1.1.0 change
    // (This is required because the upgrade CLI currently reads graft.yaml before checkout)
    let graft_yaml_v1 = r#"apiVersion: graft/v0
changes:
  v1.0.0:
    type: feature
    description: "Initial version"
  v1.1.0:
    type: breaking
    description: "Breaking change with failing verification"
    verify: "exit-1-cmd"
commands:
  exit-1-cmd:
    run: "exit 1"
"#;
    fs::write(dep_dir.join("graft.yaml"), graft_yaml_v1)
        .expect("Failed to write graft.yaml v1.0.0");
    Command::new("git")
        .args(["add", "."])
        .current_dir(&dep_dir)
        .output()
        .expect("Failed to add graft.yaml");
    Command::new("git")
        .args(["commit", "-m", "Add graft.yaml"])
        .current_dir(&dep_dir)
        .output()
        .expect("Failed to commit graft.yaml");

    // Create v1.0.0 tag
    Command::new("git")
        .args(["tag", "v1.0.0"])
        .current_dir(&dep_dir)
        .output()
        .expect("Failed to create tag");

    // Create v1.1.0 - just add a new file (graft.yaml already has v1.1.0 change declared)
    fs::write(dep_dir.join("file.txt"), "Version 1.1.0").expect("Failed to write file");
    Command::new("git")
        .args(["add", "."])
        .current_dir(&dep_dir)
        .output()
        .expect("Failed to add");
    Command::new("git")
        .args(["commit", "-m", "Version 1.1.0"])
        .current_dir(&dep_dir)
        .output()
        .expect("Failed to commit");

    Command::new("git")
        .args(["tag", "v1.1.0"])
        .current_dir(&dep_dir)
        .output()
        .expect("Failed to create v1.1.0 tag");

    // Create consumer repository
    let consumer_dir = workspace.join("consumer");
    fs::create_dir(&consumer_dir).expect("Failed to create consumer dir");
    init_git_repo(&consumer_dir, "Consumer repository");

    // Create graft.yaml in consumer pointing to v1.0.0
    let graft_yaml = format!(
        r#"apiVersion: graft/v0
deps:
  dep-with-versions: "file://{}#v1.0.0"
"#,
        dep_dir.display()
    );
    fs::write(consumer_dir.join("graft.yaml"), graft_yaml)
        .expect("Failed to write consumer graft.yaml");

    // Resolve to v1.0.0
    let output = run_graft(&["resolve"], &consumer_dir);
    assert_success(&output, "graft resolve to v1.0.0");

    // Attempt upgrade to v1.1.0 (should fail due to verification failure)
    let output = run_graft(
        &["upgrade", "dep-with-versions", "--to", "v1.1.0"],
        &consumer_dir,
    );

    // Upgrade should fail (exit code non-zero)
    assert!(
        !output.status.success(),
        "Upgrade should fail due to verification failure"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{}{}", stdout, stderr);

    // Should mention rollback
    assert!(
        combined.contains("rollback")
            || combined.contains("Rollback")
            || combined.contains("rolled back")
            || combined.contains("restored"),
        "Output should mention rollback: {}",
        combined
    );

    // Verify dependency is still at v1.0.0 (rollback succeeded)
    let lock_content =
        fs::read_to_string(consumer_dir.join("graft.lock")).expect("Lock file should still exist");

    assert!(
        lock_content.contains("v1.0.0"),
        "Lock file should still reference v1.0.0 after rollback"
    );
}

/// Test 4: Validate command catches issues
#[test]
fn test_validate_command() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let workspace = temp_dir.path();

    // Create a repository with valid graft.yaml
    init_git_repo(workspace, "Test content");

    let graft_yaml = r#"apiVersion: graft/v0
deps:
  nonexistent: "https://github.com/example/nonexistent.git#main"
"#;
    fs::write(workspace.join("graft.yaml"), graft_yaml).expect("Failed to write graft.yaml");

    // Run validate (should succeed for config validation)
    let output = run_graft(&["validate", "--config"], workspace);
    assert_success(&output, "graft validate --config");

    // Run full validate (should fail because dependency not resolved)
    let output = run_graft(&["validate"], workspace);
    assert!(
        !output.status.success(),
        "Validate should fail when dependency not resolved"
    );
}

/// Test 5: Changes and show commands
#[test]
fn test_changes_and_show_commands() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let workspace = temp_dir.path();

    // Create a dependency with changes
    let dep_dir = workspace.join("dep");
    fs::create_dir(&dep_dir).expect("Failed to create dep dir");
    init_git_repo(&dep_dir, "Initial version");

    // Add graft.yaml with changes
    let graft_yaml_dep = r#"apiVersion: graft/v0
changes:
  v1.0.0:
    type: feature
    description: "Initial release"
  v1.1.0:
    type: breaking
    description: "Breaking API change"
    migration: "migrate-script"
commands:
  migrate-script:
    run: "echo 'Running migration'"
"#;
    fs::write(dep_dir.join("graft.yaml"), graft_yaml_dep).expect("Failed to write dep graft.yaml");
    Command::new("git")
        .args(["add", "."])
        .current_dir(&dep_dir)
        .output()
        .expect("Failed to add");
    Command::new("git")
        .args(["commit", "-m", "Add graft.yaml"])
        .current_dir(&dep_dir)
        .output()
        .expect("Failed to commit");

    // Create consumer
    let consumer_dir = workspace.join("consumer");
    fs::create_dir(&consumer_dir).expect("Failed to create consumer dir");
    init_git_repo(&consumer_dir, "Consumer");

    let graft_yaml = format!(
        r#"apiVersion: graft/v0
deps:
  dep: "file://{}#main"
"#,
        dep_dir.display()
    );
    fs::write(consumer_dir.join("graft.yaml"), graft_yaml)
        .expect("Failed to write consumer graft.yaml");

    // Resolve dependency
    let output = run_graft(&["resolve"], &consumer_dir);
    assert_success(&output, "graft resolve");

    // Test changes command
    let output = run_graft(&["changes", "dep"], &consumer_dir);
    assert_success(&output, "graft changes dep");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("v1.0.0") || stdout.contains("v1.1.0"),
        "Changes output should show versions"
    );

    // Test show command
    let output = run_graft(&["show", "dep@v1.1.0"], &consumer_dir);
    assert_success(&output, "graft show dep@v1.1.0");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("breaking") || stdout.contains("Breaking"),
        "Show output should indicate breaking change"
    );
    assert!(
        stdout.contains("migrate-script"),
        "Show output should show migration command"
    );
}

/// Test 6: Add and remove commands
#[test]
fn test_add_and_remove_commands() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let workspace = temp_dir.path();

    // Create a dependency repository
    let dep_dir = workspace.join("new-dep");
    fs::create_dir(&dep_dir).expect("Failed to create dep dir");
    init_git_repo(&dep_dir, "New dependency");

    // Create consumer repository
    let consumer_dir = workspace.join("consumer");
    fs::create_dir(&consumer_dir).expect("Failed to create consumer dir");
    init_git_repo(&consumer_dir, "Consumer");

    // Create minimal graft.yaml
    let graft_yaml = r#"apiVersion: graft/v0
deps: {}
"#;
    fs::write(consumer_dir.join("graft.yaml"), graft_yaml).expect("Failed to write graft.yaml");

    // Add dependency
    let dep_source = format!("file://{}#main", dep_dir.display());
    let output = run_graft(&["add", "new-dep", &dep_source], &consumer_dir);
    assert_success(&output, "graft add new-dep");

    // Verify graft.yaml was updated
    let graft_yaml_content =
        fs::read_to_string(consumer_dir.join("graft.yaml")).expect("Failed to read graft.yaml");
    assert!(
        graft_yaml_content.contains("new-dep"),
        "graft.yaml should contain new-dep"
    );

    // Verify dependency was resolved
    assert!(
        consumer_dir.join(".graft/new-dep").exists(),
        "Dependency should be resolved"
    );

    // Remove dependency
    let output = run_graft(&["remove", "new-dep"], &consumer_dir);
    assert_success(&output, "graft remove new-dep");

    // Verify graft.yaml no longer contains dependency
    let graft_yaml_content =
        fs::read_to_string(consumer_dir.join("graft.yaml")).expect("Failed to read graft.yaml");
    assert!(
        !graft_yaml_content.contains("new-dep") || graft_yaml_content.contains("deps: {}"),
        "graft.yaml should not contain new-dep after removal"
    );

    // Verify .graft/new-dep was removed
    assert!(
        !consumer_dir.join(".graft/new-dep").exists(),
        "Dependency directory should be removed"
    );
}

/// Test 7: Fetch and sync commands
#[test]
fn test_fetch_and_sync_commands() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let workspace = temp_dir.path();

    // Create a dependency repository
    let dep_dir = workspace.join("dep");
    fs::create_dir(&dep_dir).expect("Failed to create dep dir");
    init_git_repo(&dep_dir, "Version 1");

    Command::new("git")
        .args(["tag", "v1.0.0"])
        .current_dir(&dep_dir)
        .output()
        .expect("Failed to create tag");

    // Create consumer
    let consumer_dir = workspace.join("consumer");
    fs::create_dir(&consumer_dir).expect("Failed to create consumer dir");
    init_git_repo(&consumer_dir, "Consumer");

    let graft_yaml = format!(
        r#"apiVersion: graft/v0
deps:
  dep: "file://{}#v1.0.0"
"#,
        dep_dir.display()
    );
    fs::write(consumer_dir.join("graft.yaml"), graft_yaml).expect("Failed to write graft.yaml");

    // Resolve
    let output = run_graft(&["resolve"], &consumer_dir);
    assert_success(&output, "graft resolve");

    // Test fetch
    let output = run_graft(&["fetch"], &consumer_dir);
    assert_success(&output, "graft fetch");

    // Test sync
    let output = run_graft(&["sync"], &consumer_dir);
    assert_success(&output, "graft sync");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("up-to-date") || stdout.contains("dep"),
        "Sync should report status"
    );
}
