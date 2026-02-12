//! Grove - Multi-repo workspace manager with graft awareness.

mod tui;

use anyhow::{Context, Result};
use clap::Parser;
use grove_core::{ConfigLoader, RepoRegistry};
use grove_engine::{GitoxideStatus, WorkspaceRegistry, YamlConfigLoader};

#[derive(Parser, Debug)]
#[command(name = "grove")]
#[command(about = "Multi-repo workspace manager with graft awareness")]
#[command(version)]
struct Cli {
    /// Path to workspace configuration file
    #[arg(
        short,
        long,
        env = "GROVE_WORKSPACE",
        default_value = "~/.config/grove/workspace.yaml"
    )]
    workspace: String,
}

fn main() -> Result<()> {
    // Initialize logger (RUST_LOG env var controls verbosity)
    env_logger::init();

    log::info!("Grove {} starting", env!("CARGO_PKG_VERSION"));
    log::debug!("Platform: {}", std::env::consts::OS);

    let cli = Cli::parse();

    // Expand tilde in config path
    let config_path = shellexpand::full(&cli.workspace)
        .context("Failed to expand workspace path")?
        .to_string();

    log::debug!("Loading workspace config from: {config_path}");

    // Load workspace configuration
    let loader = YamlConfigLoader::new();
    let config = loader
        .load_workspace(&config_path)
        .inspect_err(|e| {
            // Provide helpful error message for missing config file
            if e.to_string().contains("No such file") || e.to_string().contains("not found") {
                eprintln!("Error: Workspace config not found: {config_path}\n");
                eprintln!("Suggestions:");
                eprintln!("  • Create the config directory:");
                eprintln!("      mkdir -p ~/.config/grove");
                eprintln!();
                eprintln!("  • Create a workspace config file:");
                eprintln!("      cat > ~/.config/grove/workspace.yaml <<'EOF'");
                eprintln!("name: my-workspace");
                eprintln!("repositories:");
                eprintln!("  - path: ~/src/project1");
                eprintln!("    tags: [rust]");
                eprintln!("EOF");
                eprintln!();
                eprintln!("  • Or specify a different config:");
                eprintln!("      grove --workspace /path/to/config.yaml");
                eprintln!();
                eprintln!("  • Or use environment variable:");
                eprintln!("      export GROVE_WORKSPACE=/path/to/config.yaml");
                eprintln!("      grove");
                eprintln!();
                eprintln!("Documentation: https://github.com/.../docs/user-guide.md");
            }
        })
        .with_context(|| format!("Failed to load workspace from '{config_path}'"))?;

    log::info!("Loaded workspace: {}", config.name);
    log::debug!("Repositories: {}", config.repositories.len());

    // Extract workspace name before moving config
    let workspace_name = config.name.to_string();

    // Create registry with git status adapter
    let git_status = GitoxideStatus::new();
    let mut registry = WorkspaceRegistry::new(config, git_status);

    // Refresh status for all repositories
    log::debug!("Refreshing repository status...");
    let stats = registry
        .refresh_all()
        .context("Failed to refresh repository status")?;

    // Log refresh statistics
    if stats.all_successful() {
        log::info!("Successfully refreshed {} repositories", stats.successful);
    } else {
        log::warn!(
            "Refreshed {}/{} repositories ({} errors)",
            stats.successful,
            stats.total(),
            stats.failed
        );
    }

    // Create detail provider (stateless, separate instance for single-responsibility)
    let detail_provider = GitoxideStatus::new();

    // Launch TUI
    log::debug!("Launching TUI...");
    tui::run(registry, detail_provider, workspace_name).context("TUI error")?;

    log::info!("Grove exiting normally");
    Ok(())
}
