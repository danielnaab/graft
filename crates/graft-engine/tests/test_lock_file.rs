use graft_engine::lock::{parse_lock_file, parse_lock_file_str, write_lock_file};
use std::env;
use std::path::PathBuf;

#[test]
fn test_parse_repo_lock_file() {
    // Find the repo root (where Cargo.toml is)
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let manifest_path = PathBuf::from(manifest_dir);
    let repo_root = manifest_path.parent().unwrap().parent().unwrap();
    let lock_path = repo_root.join("graft.lock");

    // Parse the actual graft.lock file
    let lock = parse_lock_file(&lock_path).expect("Failed to parse repo's graft.lock");

    // Validate structure
    assert_eq!(lock.api_version, "graft/v0");
    assert!(!lock.dependencies.is_empty());

    // Check for expected dependencies (this repo has these)
    let expected_deps = vec![
        "living-specifications",
        "meta-knowledge-base",
        "python-starter",
        "rust-starter",
    ];

    for dep_name in expected_deps {
        let entry = lock
            .get(dep_name)
            .unwrap_or_else(|| panic!("Expected dependency '{}' not found", dep_name));

        // Validate entry fields are non-empty
        assert!(!entry.source.as_str().is_empty());
        assert!(!entry.git_ref.as_str().is_empty());
        assert_eq!(entry.commit.as_str().len(), 40); // SHA-1 hash
        assert!(!entry.consumed_at.is_empty());

        // Validate timestamp has expected structure
        assert!(
            entry.consumed_at.contains('-'),
            "Timestamp should be ISO 8601"
        );
    }
}

#[test]
fn test_round_trip_repo_lock() {
    // Find the repo root
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let manifest_path = PathBuf::from(manifest_dir);
    let repo_root = manifest_path.parent().unwrap().parent().unwrap();
    let lock_path = repo_root.join("graft.lock");

    // Parse original
    let original_lock = parse_lock_file(&lock_path).expect("Failed to parse repo's graft.lock");

    // Write to temp file
    let temp_dir = env::temp_dir();
    let temp_lock = temp_dir.join("graft-test-round-trip.lock");

    write_lock_file(&temp_lock, &original_lock)
        .expect("Failed to write lock file to temp location");

    // Parse the written file
    let parsed_lock = parse_lock_file(&temp_lock).expect("Failed to parse written lock file back");

    // Compare
    assert_eq!(original_lock.api_version, parsed_lock.api_version);
    assert_eq!(
        original_lock.dependencies.len(),
        parsed_lock.dependencies.len()
    );

    for (name, original_entry) in &original_lock.dependencies {
        let parsed_entry = parsed_lock
            .get(name)
            .unwrap_or_else(|| panic!("Dependency '{}' missing after round-trip", name));

        assert_eq!(original_entry.source.as_str(), parsed_entry.source.as_str());
        assert_eq!(
            original_entry.git_ref.as_str(),
            parsed_entry.git_ref.as_str()
        );
        assert_eq!(original_entry.commit.as_str(), parsed_entry.commit.as_str());
        assert_eq!(original_entry.consumed_at, parsed_entry.consumed_at);
    }

    // Clean up
    let _ = std::fs::remove_file(temp_lock);
}

#[test]
fn test_write_alphabetical_ordering() {
    use graft_core::domain::{CommitHash, GitRef, GitUrl, LockEntry, LockFile};

    let mut lock = LockFile::new();

    // Add dependencies in non-alphabetical order
    lock.insert(
        "z-dependency".to_string(),
        LockEntry::new(
            GitUrl::new("https://github.com/org/z.git").unwrap(),
            GitRef::new("v1.0.0").unwrap(),
            CommitHash::new("a".repeat(40)).unwrap(),
            "2026-01-31T10:30:00Z",
        ),
    );

    lock.insert(
        "a-dependency".to_string(),
        LockEntry::new(
            GitUrl::new("https://github.com/org/a.git").unwrap(),
            GitRef::new("v2.0.0").unwrap(),
            CommitHash::new("b".repeat(40)).unwrap(),
            "2026-01-31T10:30:00Z",
        ),
    );

    lock.insert(
        "m-dependency".to_string(),
        LockEntry::new(
            GitUrl::new("https://github.com/org/m.git").unwrap(),
            GitRef::new("v3.0.0").unwrap(),
            CommitHash::new("c".repeat(40)).unwrap(),
            "2026-01-31T10:30:00Z",
        ),
    );

    // Write to temp file
    let temp_dir = env::temp_dir();
    let temp_lock = temp_dir.join("graft-test-alphabetical.lock");

    write_lock_file(&temp_lock, &lock).expect("Failed to write lock file");

    // Read as string and check order
    let contents = std::fs::read_to_string(&temp_lock).expect("Failed to read temp lock file");

    // The YAML should have dependencies in alphabetical order
    // Find positions of each dependency name
    let pos_a = contents.find("a-dependency").unwrap();
    let pos_m = contents.find("m-dependency").unwrap();
    let pos_z = contents.find("z-dependency").unwrap();

    assert!(
        pos_a < pos_m,
        "a-dependency should come before m-dependency"
    );
    assert!(
        pos_m < pos_z,
        "m-dependency should come before z-dependency"
    );

    // Clean up
    let _ = std::fs::remove_file(temp_lock);
}

#[test]
fn test_handles_missing_lock_file() {
    let result = parse_lock_file("/nonexistent/path/to/graft.lock");
    assert!(result.is_err());

    if let Err(graft_core::error::GraftError::LockFileNotFound { path }) = result {
        assert!(path.contains("nonexistent"));
    } else {
        panic!("Expected LockFileNotFound error");
    }
}

#[test]
fn test_validates_lock_file_on_parse() {
    let invalid_yaml = r#"
apiVersion: graft/v0
dependencies:
  bad-dep:
    source: "https://github.com/org/repo.git"
    ref: "v1.0.0"
    commit: "invalid-hash"
    consumed_at: "2026-01-31T10:30:00Z"
"#;

    let result = parse_lock_file_str(invalid_yaml, "test.lock");
    assert!(result.is_err());
}
