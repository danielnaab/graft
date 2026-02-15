//! Test that graft command discovery works in different scenarios.

#[test]
fn test_graft_command_available() {
    // This test verifies that SOME form of graft is available
    // Either via uv or in PATH

    // Try uv-managed (use --help since graft doesn't have --version)
    let uv_result = std::process::Command::new("uv")
        .args(&["run", "--quiet", "python", "-m", "graft", "--help"])
        .output();

    // Try system graft
    let system_result = std::process::Command::new("graft").arg("--help").output();

    let has_uv_graft = uv_result.map(|o| o.status.success()).unwrap_or(false);
    let has_system_graft = system_result.map(|o| o.status.success()).unwrap_or(false);

    assert!(
        has_uv_graft || has_system_graft,
        "Neither uv-managed nor system graft found. \
         At least one is required for tests to run."
    );

    if has_uv_graft {
        println!("Found uv-managed graft");
    }
    if has_system_graft {
        println!("Found system graft in PATH");
    }
}

#[test]
fn test_graft_run_command_works() {
    use std::fs;
    use tempfile::tempdir;

    // Create test repo with graft.yaml
    let temp_dir = tempdir().unwrap();
    let graft_yaml = temp_dir.path().join("graft.yaml");
    fs::write(
        &graft_yaml,
        r#"apiVersion: graft/v1beta1
name: test-repo
description: Test repository

commands:
  test-echo:
    run: echo "Discovery test passed"
    description: Test command
"#,
    )
    .unwrap();

    // Try executing via uv first (development mode)
    let uv_result = std::process::Command::new("uv")
        .args(&["run", "python", "-m", "graft", "run", "test-echo"])
        .current_dir(temp_dir.path())
        .output();

    if let Ok(output) = uv_result {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert!(
                stdout.contains("Discovery test passed"),
                "Expected output to contain 'Discovery test passed', got: {}",
                stdout
            );
            println!("✓ uv-managed graft run works");
            return; // Success!
        }
    }

    // Fall back to system graft
    let system_result = std::process::Command::new("graft")
        .args(&["run", "test-echo"])
        .current_dir(temp_dir.path())
        .output();

    if let Ok(output) = system_result {
        assert!(
            output.status.success(),
            "graft run failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("Discovery test passed"),
            "Expected output to contain 'Discovery test passed', got: {}",
            stdout
        );
        println!("✓ System graft run works");
        return; // Success!
    }

    panic!("Neither uv-managed nor system graft run worked");
}
