//! Integration tests for graft-engine

use graft_engine::parse_graft_yaml;

#[test]
fn parses_repository_graft_yaml() {
    // This test parses the actual graft.yaml from the repository root
    let repo_root = std::env::var("CARGO_MANIFEST_DIR")
        .map(|p| {
            std::path::PathBuf::from(p)
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .to_path_buf()
        })
        .unwrap();

    let graft_yaml_path = repo_root.join("graft.yaml");

    // Skip test if file doesn't exist (e.g., in minimal test environment)
    if !graft_yaml_path.exists() {
        eprintln!(
            "Skipping test: graft.yaml not found at {:?}",
            graft_yaml_path
        );
        return;
    }

    let config =
        parse_graft_yaml(&graft_yaml_path).expect("Failed to parse repository's graft.yaml");

    // Verify basic structure
    assert_eq!(config.api_version, "graft/v0");

    // Verify we have the expected dependencies
    assert!(config.has_dependency("python-starter"));
    assert!(config.has_dependency("meta-knowledge-base"));
    assert!(config.has_dependency("rust-starter"));
    assert!(config.has_dependency("living-specifications"));

    // Verify dependency details
    let meta_kb = config.get_dependency("meta-knowledge-base").unwrap();
    assert_eq!(
        meta_kb.git_url.as_str(),
        "https://github.com/danielnaab/meta-knowledge-base.git"
    );
    assert_eq!(meta_kb.git_ref.as_str(), "main");
}
