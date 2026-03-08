//! Graft CLI: semantic dependency manager.

mod completers;

use anyhow::{bail, Context, Result};
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::engine::ArgValueCompleter;
use graft_common::{
    FsProcessRegistry, ProcessEntry, ProcessRegistry, ProcessStatus, SessionRuntime, TmuxRuntime,
};
use graft_engine::{
    add_dependency_to_config, apply_lock, fetch_all_dependencies, fetch_dependency,
    filter_breaking_changes, filter_changes_by_type, get_all_status, get_change_details,
    get_changes_for_dependency, get_dependency_status, get_state, invalidate_cached_state,
    is_submodule, list_state_queries, load_dep_configs, parse_graft_yaml, parse_lock_file,
    remove_dependency_from_config, remove_dependency_from_lock, remove_submodule,
    resolve_all_dependencies, resolve_and_create_lock, resolve_dependency, scion_attach_check,
    scion_create, scion_fuse, scion_list, scion_prune, scion_run, scion_start, scion_stop,
    sync_all_dependencies, validate_config_schema, validate_integrity, write_lock_file,
    VerifyLevel,
};
use std::path::{Path, PathBuf};

/// Output format for commands that support structured output.
#[derive(Clone, Debug, clap::ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(Parser)]
#[command(name = "graft")]
#[command(about = "Semantic dependency manager for knowledge bases")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show status of dependencies
    Status {
        /// Optional dependency name to show status for
        #[arg(add = ArgValueCompleter::new(completers::complete_dep_names))]
        dep_name: Option<String>,

        /// Output format
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    /// List changes for a dependency
    Changes {
        /// Dependency name
        #[arg(add = ArgValueCompleter::new(completers::complete_dep_names))]
        dep_name: String,

        /// Filter by change type (breaking, feature, fix, etc.)
        #[arg(long)]
        r#type: Option<String>,

        /// Show only breaking changes
        #[arg(long)]
        breaking: bool,

        /// Output format
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    /// Show details of a specific change
    Show {
        /// Dependency and ref in format "dep-name@ref" (e.g., "meta-kb@v2.0.0")
        dep_ref: String,

        /// Output format
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    /// Validate graft configuration and integrity
    Validate {
        /// Validate only graft.yaml config
        #[arg(long)]
        config: bool,

        /// Validate only graft.lock schema
        #[arg(long)]
        lock: bool,

        /// Validate only .graft/ integrity
        #[arg(long)]
        integrity: bool,

        /// Output format
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    /// Resolve dependencies specified in graft.yaml
    Resolve,
    /// Fetch updates from remote repositories
    Fetch {
        /// Optional dependency name to fetch (fetches all if not specified)
        #[arg(add = ArgValueCompleter::new(completers::complete_dep_names))]
        dep_name: Option<String>,
    },
    /// Sync dependencies to match lock file state
    Sync {
        /// Optional dependency name to sync (syncs all if not specified)
        #[arg(add = ArgValueCompleter::new(completers::complete_dep_names))]
        dep_name: Option<String>,
    },
    /// Apply dependency version to lock file without migrations
    Apply {
        /// Dependency name
        #[arg(add = ArgValueCompleter::new(completers::complete_dep_names))]
        dep_name: String,

        /// Target ref to apply (e.g., "main", "v1.0.0")
        #[arg(long)]
        to: String,
    },
    /// Upgrade dependency to new version with migrations
    Upgrade {
        /// Dependency name
        #[arg(add = ArgValueCompleter::new(completers::complete_dep_names))]
        dep_name: String,

        /// Target ref to upgrade to (e.g., "v2.0.0")
        #[arg(long)]
        to: String,

        /// Skip migration command (not recommended)
        #[arg(long)]
        skip_migration: bool,

        /// Skip verification command (not recommended)
        #[arg(long)]
        skip_verify: bool,

        /// Show what would be done without making changes
        #[arg(long)]
        dry_run: bool,
    },
    /// Add a dependency to graft.yaml
    Add {
        /// Dependency name (used in .graft/<name>/)
        name: String,

        /// Source URL and ref in format "url#ref" (e.g., "<https://github.com/org/repo.git#main>")
        source_ref: String,

        /// Add to config only, don't resolve (clone)
        #[arg(long)]
        no_resolve: bool,
    },
    /// Remove a dependency from graft.yaml
    Remove {
        /// Dependency name to remove
        #[arg(add = ArgValueCompleter::new(completers::complete_dep_names))]
        name: String,

        /// Keep files in .graft/<name>/ instead of deleting
        #[arg(long)]
        keep_files: bool,
    },
    /// Execute a command from graft.yaml
    Run {
        /// Command name or dep:command (e.g., "test" or "meta-kb:migrate")
        #[arg(add = ArgValueCompleter::new(completers::complete_run_commands))]
        command: Option<String>,

        /// Show rendered stdin without executing the command
        #[arg(long)]
        dry_run: bool,

        /// Arguments to pass to the command
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// State query operations
    State {
        #[command(subcommand)]
        subcommand: StateCommands,
    },
    /// List active processes managed by graft
    Ps {
        /// Filter by repository path (shows only processes for this repo)
        #[arg(long, value_hint = clap::ValueHint::DirPath)]
        repo: Option<String>,
    },
    /// Show command and sequence metadata for a dependency
    Catalog {
        /// Dependency or dep:command (e.g., "software-factory" or "software-factory:implement")
        dep_spec: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Manage scion workstreams (parallel git worktrees)
    Scion {
        #[command(subcommand)]
        subcommand: ScionCommands,
    },
}

#[derive(Subcommand)]
enum StateCommands {
    /// List all state queries
    List,
    /// Execute a state query
    Query {
        /// Query name to execute
        #[arg(add = ArgValueCompleter::new(completers::complete_state_names))]
        name: String,

        /// Force refresh (ignore cache)
        #[arg(short, long)]
        refresh: bool,

        /// Output only the data (no metadata)
        #[arg(long)]
        raw: bool,

        /// Pretty-print JSON output
        #[arg(short, long, default_value = "true")]
        pretty: bool,
    },
    /// Invalidate cached state
    Invalidate {
        /// Query name to invalidate (omit for all)
        #[arg(add = ArgValueCompleter::new(completers::complete_state_names))]
        name: Option<String>,

        /// Invalidate all queries
        #[arg(short, long)]
        all: bool,
    },
}

#[derive(Subcommand)]
enum ScionCommands {
    /// List all scion workstreams with their status
    List {
        /// Output format
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    /// Create a new scion workstream (worktree + branch)
    Create {
        /// Scion name (creates .worktrees/<name> on branch feature/<name>)
        name: String,
    },
    /// Remove a scion workstream (worktree + branch)
    Prune {
        /// Scion name to remove
        name: String,
        /// Force prune even if a runtime session is active
        #[arg(long)]
        force: bool,
    },
    /// Fuse a scion into the main branch (merge + cleanup)
    Fuse {
        /// Scion name to fuse
        name: String,
        /// Force fuse even if a runtime session is active
        #[arg(long)]
        force: bool,
    },
    /// Start a worker process in a scion's runtime session
    Start {
        /// Scion name
        name: String,
    },
    /// Stop a scion's runtime session
    Stop {
        /// Scion name
        name: String,
    },
    /// Attach to a scion's runtime session
    Attach {
        /// Scion name
        name: String,
    },
    /// Create a scion if needed and start it (combined workflow)
    Run {
        /// Scion name
        name: String,
    },
}

#[allow(clippy::too_many_lines)]
fn main() -> Result<()> {
    clap_complete::CompleteEnv::with_factory(Cli::command).complete();

    let cli = Cli::parse();

    match cli.command {
        Commands::Status { dep_name, format } => {
            status_command(dep_name.as_deref(), &format)?;
        }
        Commands::Changes {
            dep_name,
            r#type,
            breaking,
            format,
        } => {
            changes_command(&dep_name, r#type.as_deref(), breaking, &format)?;
        }
        Commands::Show { dep_ref, format } => {
            show_command(&dep_ref, &format)?;
        }
        Commands::Validate {
            config,
            lock,
            integrity,
            format,
        } => {
            validate_command(config, lock, integrity, &format)?;
        }
        Commands::Resolve => {
            resolve_command()?;
        }
        Commands::Fetch { dep_name } => {
            fetch_command(dep_name.as_deref())?;
        }
        Commands::Sync { dep_name } => {
            sync_command(dep_name.as_deref())?;
        }
        Commands::Apply { dep_name, to } => {
            apply_command(&dep_name, &to)?;
        }
        Commands::Upgrade {
            dep_name,
            to,
            skip_migration,
            skip_verify,
            dry_run,
        } => {
            upgrade_command(&dep_name, &to, skip_migration, skip_verify, dry_run)?;
        }
        Commands::Add {
            name,
            source_ref,
            no_resolve,
        } => {
            add_command(&name, &source_ref, no_resolve)?;
        }
        Commands::Remove { name, keep_files } => {
            remove_command(&name, keep_files)?;
        }
        Commands::Run {
            command,
            dry_run,
            args,
        } => {
            run_command(command.as_deref(), dry_run, &args)?;
        }
        Commands::State { subcommand } => match subcommand {
            StateCommands::List => {
                state_list_command()?;
            }
            StateCommands::Query {
                name,
                refresh,
                raw,
                pretty,
            } => {
                state_query_command(&name, refresh, raw, pretty)?;
            }
            StateCommands::Invalidate { name, all } => {
                state_invalidate_command(name.as_deref(), all)?;
            }
        },
        Commands::Ps { repo } => {
            ps_command(repo.as_deref())?;
        }
        Commands::Catalog { dep_spec, json } => {
            help_command(&dep_spec, json)?;
        }
        Commands::Scion { subcommand } => match subcommand {
            ScionCommands::List { format } => {
                scion_list_command(&format)?;
            }
            ScionCommands::Create { name } => {
                scion_create_command(&name)?;
            }
            ScionCommands::Prune { name, force } => {
                scion_prune_command(&name, force)?;
            }
            ScionCommands::Fuse { name, force } => {
                scion_fuse_command(&name, force)?;
            }
            ScionCommands::Start { name } => {
                scion_start_command(&name)?;
            }
            ScionCommands::Stop { name } => {
                scion_stop_command(&name)?;
            }
            ScionCommands::Attach { name } => {
                scion_attach_command(&name)?;
            }
            ScionCommands::Run { name } => {
                scion_run_command(&name)?;
            }
        },
    }

    Ok(())
}

fn status_command(dep_name: Option<&str>, format: &OutputFormat) -> Result<()> {
    let lock_path = Path::new("graft.lock");

    // Check if lock file exists
    if !lock_path.exists() {
        match format {
            OutputFormat::Json => println!("{{\"dependencies\":{{}}}}"),
            OutputFormat::Text => {
                eprintln!("No dependencies found in graft.lock");
                eprintln!();
                eprintln!("Run 'graft resolve' to resolve dependencies first.");
            }
        }
        return Ok(());
    }

    // Parse lock file
    let lock_file = parse_lock_file(lock_path).context("Failed to parse graft.lock")?;

    if let Some(name) = dep_name {
        // Show status for single dependency
        let status = get_dependency_status(&lock_file, name);

        if let Some(s) = status {
            match format {
                OutputFormat::Json => {
                    let json = serde_json::json!({
                        "name": s.name,
                        "current_ref": s.current_ref,
                        "commit": s.commit.as_str(),
                        "consumed_at": s.consumed_at,
                    });
                    println!("{}", serde_json::to_string_pretty(&json)?);
                }
                OutputFormat::Text => {
                    println!("{}: {}", s.name, s.current_ref);
                    println!("  Commit: {}...", &s.commit.as_str()[..7]);
                    println!("  Consumed: {}", s.consumed_at);
                }
            }
        } else {
            match format {
                OutputFormat::Json => {
                    let json = serde_json::json!({
                        "error": format!("Dependency '{name}' not found in graft.lock")
                    });
                    println!("{}", serde_json::to_string_pretty(&json)?);
                }
                OutputFormat::Text => {
                    eprintln!("Error: Dependency '{name}' not found in graft.lock");
                }
            }
            std::process::exit(1);
        }
    } else {
        // Show status for all dependencies
        let statuses = get_all_status(&lock_file);

        if statuses.is_empty() {
            match format {
                OutputFormat::Json => println!("{{\"dependencies\":{{}}}}"),
                OutputFormat::Text => {
                    eprintln!("No dependencies found in graft.lock");
                    eprintln!();
                    eprintln!("Run 'graft resolve' to resolve dependencies first.");
                }
            }
            return Ok(());
        }

        match format {
            OutputFormat::Json => {
                let mut deps_map = serde_json::Map::new();
                for (name, status) in &statuses {
                    let status_obj = serde_json::json!({
                        "current_ref": status.current_ref,
                        "commit": status.commit.as_str(),
                        "consumed_at": status.consumed_at,
                    });
                    deps_map.insert(name.clone(), status_obj);
                }
                let json = serde_json::json!({
                    "dependencies": deps_map
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            }
            OutputFormat::Text => {
                println!("Dependencies:");
                for status in statuses.values() {
                    println!(
                        "  {}: {} (commit: {}..., consumed: {})",
                        status.name,
                        status.current_ref,
                        &status.commit.as_str()[..7],
                        status.consumed_at
                    );
                }
            }
        }
    }

    Ok(())
}

#[allow(clippy::too_many_lines)]
fn changes_command(
    dep_name: &str,
    change_type: Option<&str>,
    breaking_only: bool,
    format: &OutputFormat,
) -> Result<()> {
    // Validate type and breaking are not both specified
    if breaking_only && change_type.is_some() {
        bail!("Cannot specify both --type and --breaking");
    }

    // Find dependency's graft.yaml
    let dep_path = PathBuf::from(".graft").join(dep_name).join("graft.yaml");

    if !dep_path.exists() {
        match format {
            OutputFormat::Json => {
                let json = serde_json::json!({
                    "error": format!("Dependency configuration not found: {}", dep_path.display()),
                    "suggestion": format!("Check that {dep_name} is resolved in .graft/")
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            }
            OutputFormat::Text => {
                eprintln!("Error: Dependency configuration not found");
                eprintln!("  Path: {}", dep_path.display());
                eprintln!("  Suggestion: Check that {dep_name} is resolved in .graft/");
            }
        }
        std::process::exit(1);
    }

    // Parse dependency's graft.yaml
    let config = parse_graft_yaml(&dep_path)
        .with_context(|| format!("Failed to parse {}", dep_path.display()))?;

    // Get all changes
    let mut changes = get_changes_for_dependency(&config);

    // Apply filters
    if breaking_only {
        changes = filter_breaking_changes(&changes);
    } else if let Some(t) = change_type {
        changes = filter_changes_by_type(&changes, t);
    }

    // Display results
    if changes.is_empty() {
        let filter_desc = if breaking_only {
            "breaking "
        } else {
            change_type.unwrap_or_default()
        };

        match format {
            OutputFormat::Json => {
                let json = serde_json::json!({
                    "dependency": dep_name,
                    "changes": [],
                    "message": format!("No {filter_desc}changes found")
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            }
            OutputFormat::Text => {
                println!("No {filter_desc}changes found for {dep_name}");
            }
        }
        return Ok(());
    }

    match format {
        OutputFormat::Json => {
            let changes_list: Vec<_> = changes
                .iter()
                .map(|c| {
                    serde_json::json!({
                        "ref": c.ref_name,
                        "type": c.change_type,
                        "description": c.description,
                        "migration": c.migration,
                        "verify": c.verify,
                    })
                })
                .collect();

            let json = serde_json::json!({
                "dependency": dep_name,
                "changes": changes_list
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        OutputFormat::Text => {
            let header = if breaking_only {
                format!("Breaking changes for {dep_name}:")
            } else if let Some(t) = change_type {
                format!(
                    "{} changes for {dep_name}:",
                    t.chars().next().unwrap().to_uppercase().collect::<String>() + &t[1..]
                )
            } else {
                format!("Changes for {dep_name}:")
            };

            println!("{header}");
            println!();

            for change in &changes {
                // Ref and type
                let type_str = change
                    .change_type
                    .as_ref()
                    .map(|t| format!("({t})"))
                    .unwrap_or_default();
                println!("{} {type_str}", change.ref_name);

                // Description
                if let Some(desc) = &change.description {
                    println!("  {desc}");
                }

                // Migration/verification info
                if change.migration.is_some() || change.verify.is_some() {
                    if let Some(mig) = &change.migration {
                        println!("  Migration: {mig}");
                    }
                    if let Some(ver) = &change.verify {
                        println!("  Verify: {ver}");
                    }
                } else {
                    println!("  No migration required");
                }

                println!();
            }
        }
    }

    Ok(())
}

#[allow(clippy::too_many_lines)]
fn show_command(dep_ref: &str, format: &OutputFormat) -> Result<()> {
    // Parse dep_name@ref format
    let Some((dep_name, ref_name)) = dep_ref.split_once('@') else {
        bail!("Invalid format. Use 'dep-name@ref' (e.g., 'meta-kb@v2.0.0')");
    };

    // Find dependency's graft.yaml
    let dep_path = PathBuf::from(".graft").join(dep_name).join("graft.yaml");

    if !dep_path.exists() {
        match format {
            OutputFormat::Json => {
                let json = serde_json::json!({
                    "error": format!("Dependency configuration not found: {}", dep_path.display()),
                    "suggestion": format!("Check that {dep_name} is resolved in .graft/")
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            }
            OutputFormat::Text => {
                eprintln!("Error: Dependency configuration not found");
                eprintln!("  Path: {}", dep_path.display());
                eprintln!("  Suggestion: Check that {dep_name} is resolved in .graft/");
            }
        }
        std::process::exit(1);
    }

    // Parse dependency's graft.yaml
    let config = parse_graft_yaml(&dep_path)
        .with_context(|| format!("Failed to parse {}", dep_path.display()))?;

    // Get change details
    let Some(details) = get_change_details(&config, ref_name) else {
        match format {
            OutputFormat::Json => {
                let json = serde_json::json!({
                    "error": format!("Change {ref_name} not found for {dep_name}"),
                    "suggestion": format!("Run 'graft changes {dep_name}' to see available changes")
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            }
            OutputFormat::Text => {
                eprintln!("Error: Change {ref_name} not found for {dep_name}");
                eprintln!("  Run 'graft changes {dep_name}' to see available changes");
            }
        }
        std::process::exit(1);
    };

    match format {
        OutputFormat::Json => {
            let mut output = serde_json::json!({
                "dependency": dep_name,
                "ref": ref_name,
                "type": details.change.change_type,
                "description": details.change.description,
            });

            // Add migration details if present
            if let Some(cmd) = &details.migration_command {
                output["migration"] = serde_json::json!({
                    "name": cmd.name,
                    "command": cmd.run,
                    "description": cmd.description,
                    "working_dir": cmd.working_dir,
                });
            } else {
                output["migration"] = serde_json::Value::Null;
            }

            // Add verification details if present
            if let Some(cmd) = &details.verify_command {
                output["verify"] = serde_json::json!({
                    "name": cmd.name,
                    "command": cmd.run,
                    "description": cmd.description,
                    "working_dir": cmd.working_dir,
                });
            } else {
                output["verify"] = serde_json::Value::Null;
            }

            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        OutputFormat::Text => {
            println!("Change: {dep_name}@{ref_name}");
            println!();

            // Display type
            if let Some(t) = &details.change.change_type {
                println!("Type: {t}");
            }

            // Display description
            if let Some(desc) = &details.change.description {
                println!("Description: {desc}");
                println!();
            }

            // Display migration details
            if let Some(cmd) = &details.migration_command {
                println!("Migration: {}", cmd.name);
                println!("  Command: {}", cmd.run);
                if let Some(desc) = &cmd.description {
                    println!("  Description: {desc}");
                }
                if let Some(wd) = &cmd.working_dir {
                    println!("  Working directory: {wd}");
                }
                println!();
            }

            // Display verification details
            if let Some(cmd) = &details.verify_command {
                println!("Verification: {}", cmd.name);
                println!("  Command: {}", cmd.run);
                if let Some(desc) = &cmd.description {
                    println!("  Description: {desc}");
                }
                if let Some(wd) = &cmd.working_dir {
                    println!("  Working directory: {wd}");
                }
                println!();
            }

            // Show if no migration/verification required
            if details.migration_command.is_none() && details.verify_command.is_none() {
                println!("No migration or verification required");
                println!();
            }
        }
    }

    Ok(())
}

#[allow(clippy::too_many_lines, clippy::if_not_else)]
fn validate_command(
    config_only: bool,
    lock_only: bool,
    integrity_only: bool,
    format: &OutputFormat,
) -> Result<()> {
    // Validate flag combinations
    let flags_set = [config_only, lock_only, integrity_only]
        .iter()
        .filter(|&&x| x)
        .count();
    if flags_set > 1 {
        bail!("--config, --lock, and --integrity are mutually exclusive");
    }

    let is_text = matches!(format, OutputFormat::Text);

    // Determine what to validate based on flags
    let validate_config = config_only || flags_set == 0;
    let validate_lock = lock_only || flags_set == 0;
    let validate_integrity_mode = integrity_only || flags_set == 0;

    let mut all_errors = Vec::new();
    let mut all_warnings = Vec::new();
    let mut integrity_failed = false;

    // For JSON output, collect results
    let mut json_config = serde_json::json!({ "valid": true, "errors": [] });
    let mut json_lock = serde_json::json!({ "valid": true, "errors": [] });
    let mut json_integrity = serde_json::json!({ "valid": true, "results": [] });

    // Validate graft.yaml
    if validate_config {
        if is_text {
            println!("Validating graft.yaml...");
        }

        let config_path = Path::new("graft.yaml");
        if !config_path.exists() {
            let error_msg = "graft.yaml not found";
            all_errors.push(error_msg.to_string());
            json_config["valid"] = serde_json::Value::Bool(false);
            json_config["errors"]
                .as_array_mut()
                .unwrap()
                .push(serde_json::Value::String(error_msg.to_string()));

            if is_text {
                eprintln!("  ✗ {error_msg}");
                println!();
            }
        } else {
            match parse_graft_yaml(config_path) {
                Ok(config) => {
                    let errors = validate_config_schema(&config);

                    if errors.is_empty() {
                        if is_text {
                            println!("  ✓ Schema is valid");
                            println!();
                        }
                    } else {
                        json_config["valid"] = serde_json::Value::Bool(false);
                        for error in &errors {
                            all_errors.push(error.message.clone());
                            json_config["errors"]
                                .as_array_mut()
                                .unwrap()
                                .push(serde_json::Value::String(error.message.clone()));

                            if is_text {
                                eprintln!("  ✗ {}", error.message);
                            }
                        }
                        if is_text {
                            println!();
                        }
                    }
                }
                Err(e) => {
                    let error_msg = format!("Failed to parse graft.yaml: {e}");
                    all_errors.push(error_msg.clone());
                    json_config["valid"] = serde_json::Value::Bool(false);
                    json_config["errors"]
                        .as_array_mut()
                        .unwrap()
                        .push(serde_json::Value::String(error_msg.clone()));

                    if is_text {
                        eprintln!("  ✗ {error_msg}");
                        println!();
                    }
                }
            }
        }
    }

    // Validate graft.lock
    if validate_lock {
        if is_text {
            println!("Validating graft.lock...");
        }

        let lock_path = Path::new("graft.lock");
        if !lock_path.exists() {
            let warning_msg = "graft.lock not found (run 'graft resolve' to create)";
            all_warnings.push(warning_msg.to_string());

            if is_text {
                println!("  ⚠ {warning_msg}");
                println!();
            }
        } else {
            match parse_lock_file(lock_path) {
                Ok(lock_file) => {
                    if lock_file.dependencies.is_empty() {
                        let warning_msg = "graft.lock is empty";
                        all_warnings.push(warning_msg.to_string());

                        if is_text {
                            println!("  ⚠ {warning_msg}");
                            println!();
                        }
                    } else if is_text {
                        println!("  ✓ Schema is valid");
                        println!();
                    }
                }
                Err(e) => {
                    let error_msg = format!("Failed to read graft.lock: {e}");
                    all_errors.push(error_msg.clone());
                    json_lock["valid"] = serde_json::Value::Bool(false);
                    json_lock["errors"]
                        .as_array_mut()
                        .unwrap()
                        .push(serde_json::Value::String(error_msg.clone()));

                    if is_text {
                        eprintln!("  ✗ {error_msg}");
                        println!();
                    }
                }
            }
        }
    }

    // Validate integrity (.graft/ matches lock file)
    if validate_integrity_mode {
        if is_text {
            println!("Validating integrity...");
        }

        let lock_path = Path::new("graft.lock");
        if !lock_path.exists() {
            let error_msg = "graft.lock not found (cannot validate integrity)";
            all_errors.push(error_msg.to_string());
            json_integrity["valid"] = serde_json::Value::Bool(false);

            if is_text {
                eprintln!("  ✗ graft.lock not found");
                println!();
            }
        } else {
            match parse_lock_file(lock_path) {
                Ok(lock_file) => {
                    if lock_file.dependencies.is_empty() {
                        let warning_msg = "graft.lock is empty";
                        all_warnings.push(warning_msg.to_string());

                        if is_text {
                            println!("  ⚠ {warning_msg}");
                            println!();
                        }
                    } else {
                        // Run integrity validation
                        let results = validate_integrity(".graft", &lock_file);

                        // Process results
                        for result in &results {
                            if result.valid {
                                if is_text {
                                    println!("  ✓ {}: {}", result.name, result.message);
                                }
                            } else {
                                integrity_failed = true;
                                json_integrity["valid"] = serde_json::Value::Bool(false);

                                if is_text {
                                    eprintln!("  ✗ {}: {}", result.name, result.message);
                                }
                            }

                            // Add to JSON results
                            json_integrity["results"]
                                .as_array_mut()
                                .unwrap()
                                .push(serde_json::json!({
                                    "name": result.name,
                                    "valid": result.valid,
                                    "expected_commit": result.expected_commit.as_str(),
                                    "actual_commit": result.actual_commit.as_ref().map(graft_engine::domain::CommitHash::as_str),
                                    "message": result.message,
                                }));
                        }

                        if integrity_failed {
                            all_errors.push("Integrity check failed".to_string());
                        }

                        if is_text {
                            println!();
                        }
                    }
                }
                Err(e) => {
                    let error_msg = format!("Failed to validate integrity: {e}");
                    all_errors.push(error_msg.clone());
                    json_integrity["valid"] = serde_json::Value::Bool(false);

                    if is_text {
                        eprintln!("  ✗ {error_msg}");
                        println!();
                    }
                }
            }
        }
    }

    // Output summary
    match format {
        OutputFormat::Json => {
            let mut output = serde_json::json!({});

            if validate_config {
                output["config"] = json_config;
            }
            if validate_lock {
                output["lock"] = json_lock;
            }
            if validate_integrity_mode {
                output["integrity"] = json_integrity;
            }

            // Overall status
            let overall = if all_errors.is_empty() {
                "passed"
            } else {
                "failed"
            };
            output["overall"] = serde_json::Value::String(overall.to_string());

            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        OutputFormat::Text => {
            if !all_errors.is_empty() {
                let error_count = all_errors.len();
                let warning_count = all_warnings.len();

                if warning_count > 0 {
                    eprintln!(
                        "Validation failed with {error_count} error(s) and {warning_count} warning(s)"
                    );
                } else {
                    eprintln!("Validation failed with {error_count} error(s)");
                }
            } else if !all_warnings.is_empty() {
                let warning_count = all_warnings.len();
                println!("Validation passed with {warning_count} warning(s)");
            } else {
                println!("Validation successful");
            }
        }
    }

    // Exit with appropriate code
    if !all_errors.is_empty() {
        if integrity_failed {
            std::process::exit(2);
        } else {
            std::process::exit(1);
        }
    }

    Ok(())
}

#[allow(clippy::too_many_lines)]
fn resolve_command() -> Result<()> {
    let config_path = Path::new("graft.yaml");

    // Check if graft.yaml exists
    if !config_path.exists() {
        eprintln!("Error: graft.yaml not found in current directory");
        std::process::exit(1);
    }

    // Parse graft.yaml
    let config = parse_graft_yaml(config_path).context("Failed to parse graft.yaml")?;

    // Display header
    println!(
        "Found configuration: {}",
        config_path.canonicalize()?.display()
    );
    println!("API Version: {}", config.api_version);
    println!("Dependencies: {}", config.dependencies.len());
    println!();

    // Check if there are dependencies to resolve
    if config.dependencies.is_empty() {
        println!("No dependencies to resolve.");
        return Ok(());
    }

    println!("Resolving dependencies...");
    println!();

    // Resolve all dependencies and get their status for display
    let deps_directory = ".graft";
    let results = resolve_all_dependencies(&config, deps_directory);

    // Display resolution results
    let mut succeeded = 0;
    let mut failed = 0;

    for result in &results {
        if result.success {
            succeeded += 1;
            if let Some(path) = &result.local_path {
                let absolute_path = path.canonicalize().unwrap_or_else(|_| path.clone());
                if result.newly_cloned {
                    println!("✓ {}: cloned to {}", result.name, absolute_path.display());
                } else {
                    println!("✓ {}: resolved to {}", result.name, absolute_path.display());
                }
            }
        } else {
            failed += 1;
            if let Some(error) = &result.error {
                eprintln!("✗ {}: {}", result.name, error);

                // Provide helpful suggestions
                if error.contains("Legacy clone detected") {
                    eprintln!("  Suggestion: Delete the directory and re-run resolve");
                } else if error.contains("Authentication failed")
                    || error.contains("Could not resolve host")
                {
                    eprintln!("  Suggestion: Check network connectivity and SSH key configuration");
                }
            }
        }
    }

    println!();
    println!("Resolved: {succeeded}/{}", results.len());

    // Exit early if any dependencies failed
    if failed > 0 {
        eprintln!("\nSome dependencies failed to resolve.");
        std::process::exit(1);
    }

    // Create/update lock file
    println!();
    println!("Updating lock file...");

    match resolve_and_create_lock(&config, deps_directory) {
        Ok(lock_file) => {
            let lock_path = Path::new("graft.lock");
            write_lock_file(lock_path, &lock_file).context("Failed to write graft.lock")?;
            println!("✓ graft.lock updated");
        }
        Err(e) => {
            eprintln!("✗ Failed to create lock file: {e}");
            std::process::exit(1);
        }
    }

    println!();
    println!("All dependencies resolved successfully!");

    Ok(())
}

fn fetch_command(dep_name: Option<&str>) -> Result<()> {
    let config_path = Path::new("graft.yaml");

    // Check if graft.yaml exists
    if !config_path.exists() {
        eprintln!("Error: graft.yaml not found in current directory");
        std::process::exit(1);
    }

    // Parse graft.yaml
    let config = parse_graft_yaml(config_path).context("Failed to parse graft.yaml")?;

    // Determine which dependencies to fetch
    if let Some(name) = dep_name {
        // Fetch specific dependency
        if !config.dependencies.contains_key(name) {
            eprintln!("Error: Dependency '{name}' not found in graft.yaml");
            std::process::exit(1);
        }

        println!("Fetching {name}...");

        let deps_directory = ".graft";
        let result = fetch_dependency(name, deps_directory)?;

        if result.success {
            println!("  ✓ Fetched successfully");
        } else if let Some(error) = result.error {
            eprintln!("  ✗ {error}");
            std::process::exit(1);
        }
    } else {
        // Fetch all dependencies
        if config.dependencies.is_empty() {
            println!("No dependencies to fetch.");
            return Ok(());
        }

        println!("Fetching all dependencies...");
        println!();

        let deps_directory = ".graft";
        let results = fetch_all_dependencies(&config, deps_directory);

        let mut success_count = 0;
        let mut error_count = 0;

        for result in &results {
            if result.success {
                success_count += 1;
                println!("  ✓ {}: fetched successfully", result.name);
            } else if let Some(error) = &result.error {
                error_count += 1;
                eprintln!("  ✗ {}: {error}", result.name);
            }
        }

        println!();

        if error_count == 0 {
            let dep_word = if success_count == 1 {
                "dependency"
            } else {
                "dependencies"
            };
            println!("✓ Successfully fetched {success_count} {dep_word}");
        } else {
            let error_word = if error_count == 1 { "error" } else { "errors" };
            println!("Fetched {success_count}, {error_count} {error_word}");
            if error_count > 0 && success_count == 0 {
                std::process::exit(1);
            }
        }
    }

    Ok(())
}

#[allow(clippy::too_many_lines)]
fn sync_command(dep_name: Option<&str>) -> Result<()> {
    let lock_path = Path::new("graft.lock");

    // Check if lock file exists
    if !lock_path.exists() {
        eprintln!("Error: graft.lock not found");
        eprintln!("Run 'graft resolve' to create the lock file.");
        std::process::exit(1);
    }

    // Parse lock file
    let lock_file = parse_lock_file(lock_path).context("Failed to read graft.lock")?;

    if lock_file.dependencies.is_empty() {
        println!("No dependencies in lock file.");
        return Ok(());
    }

    let deps_directory = ".graft";

    if let Some(name) = dep_name {
        // Sync specific dependency
        if let Some(entry) = lock_file.dependencies.get(name) {
            println!("Syncing {name}...");

            let result = graft_engine::sync_dependency(name, entry, deps_directory)?;

            if result.success {
                println!("  ✓ {}", result.message);
            } else {
                eprintln!("  ✗ {}", result.message);
                std::process::exit(1);
            }
        } else {
            eprintln!("Error: Dependency '{name}' not found in graft.lock");
            std::process::exit(1);
        }
    } else {
        // Sync all dependencies
        println!("Syncing dependencies to lock file...");
        println!();

        let results = sync_all_dependencies(&lock_file, deps_directory);

        let mut success_count = 0;
        let total = results.len();

        for result in &results {
            if result.success {
                success_count += 1;
                println!("  ✓ {}: {}", result.name, result.message);
            } else {
                eprintln!("  ✗ {}: {}", result.name, result.message);
            }
        }

        println!();

        if success_count == total {
            println!("Synced: {success_count}/{total} dependencies");
        } else {
            let failed = total - success_count;
            println!("Synced: {success_count}/{total} dependencies ({failed} failed)");
            std::process::exit(1);
        }
    }

    Ok(())
}

#[allow(clippy::too_many_lines)]
fn apply_command(dep_name: &str, to: &str) -> Result<()> {
    let config_path = Path::new("graft.yaml");
    let lock_path = Path::new("graft.lock");
    let deps_directory = ".graft";

    // Parse graft.yaml
    let config = parse_graft_yaml(config_path).context("Failed to parse graft.yaml")?;

    // Apply the dependency lock
    let result = apply_lock(&config, lock_path, dep_name, to, deps_directory)
        .context("Failed to apply dependency version")?;

    // Display success
    println!();
    println!("Applied {}@{}", result.name, result.git_ref);
    println!("  Source: {}", result.source);
    println!("  Commit: {}...", &result.commit.as_str()[..7]);
    println!("Updated graft.lock");
    println!();
    println!("Note: No migrations were run.");

    Ok(())
}

#[allow(clippy::too_many_lines)]
fn upgrade_command(
    dep_name: &str,
    to: &str,
    skip_migration: bool,
    skip_verify: bool,
    dry_run: bool,
) -> Result<()> {
    let config_path = Path::new("graft.yaml");
    let lock_path = Path::new("graft.lock");
    let deps_directory = ".graft";

    // Parse consumer's graft.yaml
    let consumer_config =
        parse_graft_yaml(config_path).context("Failed to parse consumer graft.yaml")?;

    // Check dependency exists
    if !consumer_config.dependencies.contains_key(dep_name) {
        bail!(
            "Dependency '{}' not found in graft.yaml\nAvailable dependencies: {}",
            dep_name,
            consumer_config
                .dependencies
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    let dep_spec = &consumer_config.dependencies[dep_name];
    let source = dep_spec.git_url.as_str();

    // Parse dependency's graft.yaml
    let dep_config_path = PathBuf::from(deps_directory)
        .join(dep_name)
        .join("graft.yaml");
    let dep_config =
        parse_graft_yaml(&dep_config_path).context("Failed to parse dependency graft.yaml")?;

    // Resolve ref to commit hash
    let dep_repo_path = PathBuf::from(deps_directory).join(dep_name);

    // Try to fetch the ref (best effort)
    let _ = std::process::Command::new("git")
        .args(["-C", dep_repo_path.to_str().unwrap(), "fetch", "origin", to])
        .output();

    // Resolve ref to commit
    let output = std::process::Command::new("git")
        .args(["-C", dep_repo_path.to_str().unwrap(), "rev-parse", to])
        .output()
        .context("Failed to resolve git ref")?;

    if !output.status.success() {
        bail!(
            "Failed to resolve ref '{}': {}",
            to,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let commit = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Display upgrade info
    println!();
    println!("Upgrading {dep_name} → {to}");
    println!("  Source: {source}");
    println!("  Commit: {}...", &commit[..7]);
    println!();

    // Show warnings if skipping steps
    if skip_migration {
        println!("  Warning: Skipping migration command");
    }
    if skip_verify {
        println!("  Warning: Skipping verification command");
    }

    // Handle dry-run mode
    if dry_run {
        println!("DRY RUN MODE - No changes will be made");
        println!();

        // Get change details to show what would happen
        let change = dep_config.changes.get(to).ok_or_else(|| {
            anyhow::anyhow!("Change '{to}' not found in dependency configuration")
        })?;

        println!("Planned operations:");
        println!();

        // Step 1: Snapshot
        println!("1. Create snapshot for rollback");
        println!("   Snapshot: graft.lock");
        println!();

        // Step 2: Migration
        if let Some(ref migration_cmd) = change.migration {
            if skip_migration {
                println!("2. Migration command (SKIPPED)");
                println!("   Name: {migration_cmd}");
                println!();
            } else {
                println!("2. Run migration command");
                if let Some(cmd) = dep_config.commands.get(migration_cmd) {
                    println!("   Name: {migration_cmd}");
                    println!("   Command: {}", cmd.run);
                    if let Some(ref desc) = cmd.description {
                        println!("   Description: {desc}");
                    }
                    if let Some(ref wd) = cmd.working_dir {
                        println!("   Working directory: {wd}");
                    }
                } else {
                    println!("   Warning: Migration command '{migration_cmd}' not found in config");
                }
                println!();
            }
        } else {
            println!("2. No migration required");
            println!();
        }

        // Step 3: Verification
        if let Some(ref verify_cmd) = change.verify {
            if skip_verify {
                println!("3. Verification command (SKIPPED)");
                println!("   Name: {verify_cmd}");
                println!();
            } else {
                println!("3. Run verification command");
                if let Some(cmd) = dep_config.commands.get(verify_cmd) {
                    println!("   Name: {verify_cmd}");
                    println!("   Command: {}", cmd.run);
                    if let Some(ref desc) = cmd.description {
                        println!("   Description: {desc}");
                    }
                    if let Some(ref wd) = cmd.working_dir {
                        println!("   Working directory: {wd}");
                    }
                } else {
                    println!("   Warning: Verification command '{verify_cmd}' not found in config");
                }
                println!();
            }
        } else {
            println!("3. No verification required");
            println!();
        }

        // Step 4: Lock file update
        println!("4. Update graft.lock");
        println!("   Dependency: {dep_name}");
        println!("   New ref: {to}");
        println!("   New commit: {}...", &commit[..7]);
        println!();

        println!("✓ Dry run complete - no changes made");
        println!();
        println!("To perform the upgrade, run without --dry-run:");
        println!("  graft upgrade {dep_name} --to {to}");
        return Ok(());
    }

    // Perform the upgrade
    let result = graft_engine::upgrade_dependency(
        &dep_config,
        &consumer_config,
        lock_path,
        dep_name,
        to,
        &commit,
        ".",
        deps_directory,
        skip_migration,
        skip_verify,
    )
    .context("Failed to upgrade dependency")?;

    // Display results
    println!();

    if result.success {
        // Show migration result
        if let Some(ref migration) = result.migration_result {
            println!("Migration completed:");
            if !migration.stdout.is_empty() {
                println!("  {}", migration.stdout.trim());
            }
        }

        // Show verification result
        if let Some(ref verify) = result.verify_result {
            println!("Verification passed:");
            if !verify.stdout.is_empty() {
                println!("  {}", verify.stdout.trim());
            }
        }

        println!();
        println!("✓ Upgrade complete");
        println!("Updated graft.lock: {dep_name}@{to}");
    } else {
        eprintln!("✗ Upgrade failed");
        if let Some(ref error) = result.error {
            eprintln!("  Error: {error}");
        }
        eprintln!();
        eprintln!("All changes have been rolled back");
        eprintln!("Lock file remains unchanged");

        // Show command output if available
        if let Some(ref migration) = result.migration_result {
            if !migration.stderr.is_empty() {
                eprintln!();
                eprintln!("Migration output:");
                eprintln!("  {}", migration.stderr.trim());
            }
        }

        if let Some(ref verify) = result.verify_result {
            if !verify.stderr.is_empty() {
                eprintln!();
                eprintln!("Verification output:");
                eprintln!("  {}", verify.stderr.trim());
            }
        }

        std::process::exit(1);
    }

    Ok(())
}

fn add_command(name: &str, source_ref: &str, no_resolve: bool) -> Result<()> {
    let config_path = Path::new("graft.yaml");

    // Parse source#ref format
    let Some((source, git_ref)) = source_ref.rsplit_once('#') else {
        bail!("Error: Source must include ref in format 'url#ref' (e.g., 'https://github.com/org/repo.git#main')");
    };

    if source.is_empty() {
        bail!("Error: URL cannot be empty");
    }

    if git_ref.is_empty() {
        bail!("Error: Ref cannot be empty");
    }

    // Check config file exists
    if !config_path.exists() {
        bail!("Error: graft.yaml not found. Create it first or run 'graft init'.");
    }

    println!("Adding dependency: {name}");
    println!();
    println!("Source: {source}");
    println!("Ref: {git_ref}");
    println!();

    // Add to config
    add_dependency_to_config(config_path, name, source, git_ref)
        .context("Failed to add dependency to graft.yaml")?;

    println!("✓ Added to graft.yaml");

    // Optionally resolve (clone)
    if no_resolve {
        println!();
        println!("Run 'graft resolve' to clone the dependency.");
    } else {
        // Parse config to get the dependency spec
        let config = parse_graft_yaml(config_path).context("Failed to parse graft.yaml")?;

        // Find the dependency
        let Some(dep) = config.dependencies.get(name) else {
            bail!("Internal error: dependency not found after adding");
        };

        // Resolve it
        let result = resolve_dependency(dep, ".graft").context("Failed to resolve dependency")?;

        if result.success {
            let action = if result.newly_cloned {
                "Cloned"
            } else {
                "Resolved"
            };
            println!("✓ {action} to {}", result.local_path.unwrap().display());

            // Update lock file
            let lock_path = Path::new("graft.lock");
            let lock_file = resolve_and_create_lock(&config, ".graft")
                .context("Failed to update graft.lock")?;
            write_lock_file(lock_path, &lock_file).context("Failed to write graft.lock")?;
            println!("✓ Updated graft.lock");
        } else {
            eprintln!(
                "✗ {}",
                result.error.unwrap_or_else(|| "Unknown error".to_string())
            );
            eprintln!();
            eprintln!("Dependency added to graft.yaml but not resolved.");
            eprintln!("Run 'graft resolve' to retry.");
            std::process::exit(1);
        }
    }

    println!();
    println!("Dependency added successfully!");

    Ok(())
}

fn remove_command(name: &str, keep_files: bool) -> Result<()> {
    let config_path = Path::new("graft.yaml");
    let lock_path = Path::new("graft.lock");
    let deps_path = PathBuf::from(".graft").join(name);

    // Check config file exists
    if !config_path.exists() {
        bail!("Error: graft.yaml not found");
    }

    println!("Removing dependency: {name}");
    println!();

    // Remove from config
    remove_dependency_from_config(config_path, name)
        .context("Failed to remove dependency from graft.yaml")?;
    println!("✓ Removed from graft.yaml");

    // Remove from lock file (if exists)
    if lock_path.exists() {
        remove_dependency_from_lock(lock_path, name)
            .context("Failed to remove dependency from graft.lock")?;
        println!("✓ Removed from graft.lock");
    }

    // Handle submodule/files removal
    if deps_path.exists() {
        if is_submodule(&deps_path) {
            if keep_files {
                println!("⚠ Kept submodule files in {}", deps_path.display());
                println!("  (submodule entry still removed from .gitmodules)");
            } else {
                remove_submodule(&deps_path).context("Failed to remove submodule")?;
                println!("✓ Removed submodule {}", deps_path.display());
            }
        } else if keep_files {
            println!("⚠ Kept files in {} (legacy clone)", deps_path.display());
        } else {
            // Legacy clone - use fs::remove_dir_all
            std::fs::remove_dir_all(&deps_path)
                .with_context(|| format!("Failed to delete {}", deps_path.display()))?;
            println!("✓ Deleted {}", deps_path.display());
        }
    } else if !keep_files {
        println!("  (no files found at {})", deps_path.display());
    }

    println!();
    println!("Dependency removed successfully!");

    Ok(())
}

/// Find graft.yaml by searching current directory and parent directories.
fn find_graft_yaml() -> Option<PathBuf> {
    let mut current = std::env::current_dir().ok()?;

    loop {
        let candidate = current.join("graft.yaml");
        if candidate.is_file() {
            return Some(candidate);
        }

        // Move to parent directory
        if !current.pop() {
            break;
        }
    }

    None
}

#[allow(clippy::too_many_lines)]
fn run_command(command_name: Option<&str>, dry_run: bool, args: &[String]) -> Result<()> {
    // No command specified - list available commands
    let Some(command_name) = command_name else {
        let Some(config_path) = find_graft_yaml() else {
            eprintln!("Error: No graft.yaml found in current directory or parent directories");
            std::process::exit(1);
        };

        let config = parse_graft_yaml(&config_path)
            .with_context(|| format!("Failed to parse {}", config_path.display()))?;

        if config.commands.is_empty() {
            println!("No commands defined in {}", config_path.display());
            return Ok(());
        }

        println!("\nAvailable commands in {}:\n", config_path.display());

        // Find longest command name for alignment
        let max_name_len = config.commands.keys().map(String::len).max().unwrap_or(0);

        for (name, command) in &config.commands {
            let description = command.description.as_deref().unwrap_or("");
            println!("  {name:<max_name_len$}  {description}");
        }

        println!("\nUse: graft run <command-name>");
        return Ok(());
    };

    // Check if command contains ':' (dependency command)
    if let Some((dep_name, cmd_name)) = command_name.split_once(':') {
        if dep_name.is_empty() || cmd_name.is_empty() {
            bail!("Error: Invalid command format: '{command_name}'\n  Expected format: <dependency>:<command>");
        }

        run_dependency_command(dep_name, cmd_name, dry_run, args)?;
    } else {
        // Execute from current repo
        run_current_repo_command(command_name, dry_run, args)?;
    }

    Ok(())
}

/// Build a shell command string by substituting `{name}` placeholders or appending args.
///
/// `run` should already have script paths resolved (via `resolve_script_in_command`)
/// if needed. If placeholders are found, args fill them in order with shell escaping.
/// Otherwise, args are appended space-separated.
fn build_command_with_args(run: &str, args: &[String]) -> String {
    let (substituted, had_placeholders) = graft_engine::substitute_placeholders(run, args);
    if had_placeholders {
        substituted
    } else if args.is_empty() {
        run.to_string()
    } else {
        format!("{run} {}", args.join(" "))
    }
}

#[allow(clippy::too_many_lines)]
fn run_current_repo_command(command_name: &str, dry_run: bool, args: &[String]) -> Result<()> {
    // Find graft.yaml
    let Some(graft_yaml_path) = find_graft_yaml() else {
        eprintln!("Error: No graft.yaml found in current directory or parent directories");
        std::process::exit(1);
    };

    // Parse graft.yaml
    let config =
        parse_graft_yaml(&graft_yaml_path).context("Failed to parse current repo graft.yaml")?;

    // Determine base directory relative to graft.yaml's location
    let base_dir = graft_yaml_path.parent().unwrap_or(Path::new("."));

    // Determine repo name for state caching
    let repo_name = base_dir
        .canonicalize()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "unknown".to_string());

    // Check if it's a sequence (checked before command lookup)
    if config.sequences.contains_key(command_name) {
        let _ps_guard = ProcessRegistrationGuard::register(
            &format!("graft run {command_name}"),
            Some(base_dir.to_path_buf()),
        );
        let cmd_ctx = graft_engine::CommandContext::local(base_dir, &repo_name, &repo_name, false);
        let exit_code = graft_engine::execute_sequence(&config, command_name, &cmd_ctx, args)?;
        if exit_code != 0 {
            std::process::exit(exit_code);
        }
        return Ok(());
    }

    // Check if command exists
    let Some(cmd) = config.commands.get(command_name) else {
        eprintln!(
            "Error: Command '{command_name}' not found in {}",
            graft_yaml_path.display()
        );

        if config.commands.is_empty() && config.sequences.is_empty() {
            eprintln!("  No commands or sequences defined in graft.yaml");
        } else {
            if !config.commands.is_empty() {
                eprintln!("\nAvailable commands:");
                for (name, command) in &config.commands {
                    let desc = command.description.as_deref().unwrap_or("");
                    eprintln!("  {name}  {desc}");
                }
            }
            if !config.sequences.is_empty() {
                eprintln!("\nAvailable sequences:");
                for (name, seq) in &config.sequences {
                    let desc = seq.description.as_deref().unwrap_or("");
                    eprintln!("  {name}  {desc}");
                }
            }
        }

        std::process::exit(1);
    };

    let cmd_ctx = graft_engine::CommandContext::local(base_dir, &repo_name, &repo_name, false);

    // Handle dry-run mode
    if dry_run {
        if cmd.stdin.is_some() {
            let rendered = graft_engine::resolve_command_stdin(cmd, &config, &cmd_ctx, args)?;
            if let Some(text) = rendered {
                print!("{text}");
            }
        } else {
            // No stdin: print the command that would execute
            println!("{}", build_command_with_args(&cmd.run, args));
        }
        return Ok(());
    }

    // Register this process so `graft ps` can see it from another shell.
    // The guard deregisters automatically when it goes out of scope.
    let _ps_guard = ProcessRegistrationGuard::register(
        &format!("graft run {command_name}"),
        Some(base_dir.to_path_buf()),
    );

    // Use context-aware execution if command has stdin or context
    if cmd.needs_context() {
        // Display what we're running (stderr so stdout is clean for piping)
        eprintln!("\nExecuting: {command_name}");
        if let Some(desc) = &cmd.description {
            eprintln!("  {desc}");
        }
        eprintln!("  Command: {}", cmd.run);
        if !cmd.reads.is_empty() {
            eprintln!("  Reads:   {}", cmd.reads.join(", "));
        }
        if !cmd.writes.is_empty() {
            eprintln!("  Writes:  {}", cmd.writes.join(", "));
        }
        if !cmd.context.is_empty() {
            eprintln!("  Context: {}", cmd.context.join(", "));
        }
        if cmd.stdin.is_some() {
            eprintln!("  Stdin: (template/literal)");
        }
        if !args.is_empty() {
            eprintln!("  Arguments: {}", args.join(" "));
        }
        eprintln!();

        let result = graft_engine::execute_command_with_context(cmd, &config, &cmd_ctx, args)?;

        // Print stdout/stderr
        if !result.stdout.is_empty() {
            println!("{}", result.stdout);
        }
        if !result.stderr.is_empty() {
            eprintln!("{}", result.stderr);
        }

        if !result.success {
            eprintln!("\n✗ Command failed with exit code {}", result.exit_code);
            std::process::exit(result.exit_code);
        }

        eprintln!("\n✓ Command completed successfully");
        return Ok(());
    }

    // Display what we're running (stderr so stdout is clean for piping)
    eprintln!("\nExecuting: {command_name}");
    if let Some(desc) = &cmd.description {
        eprintln!("  {desc}");
    }
    eprintln!("  Command: {}", cmd.run);
    if !cmd.reads.is_empty() {
        eprintln!("  Reads:   {}", cmd.reads.join(", "));
    }
    if !cmd.writes.is_empty() {
        eprintln!("  Writes:  {}", cmd.writes.join(", "));
    }
    if !args.is_empty() {
        eprintln!("  Arguments: {}", args.join(" "));
    }
    if let Some(ref wd) = cmd.working_dir {
        eprintln!("  Working directory: {wd}");
    }
    eprintln!();

    // Determine working directory
    let working_dir = if let Some(ref cmd_dir) = cmd.working_dir {
        base_dir.join(cmd_dir)
    } else {
        base_dir.to_path_buf()
    };

    // Validate working directory exists
    if !working_dir.exists() {
        eprintln!(
            "Error: Working directory does not exist: {}",
            working_dir.display()
        );
        std::process::exit(1);
    }

    // Build full command with args (substitute placeholders or append)
    let full_command = build_command_with_args(&cmd.run, args);

    // Set up environment variables
    let mut process_cmd = std::process::Command::new("sh");
    process_cmd
        .arg("-c")
        .arg(&full_command)
        .current_dir(&working_dir);

    // Add environment variables if specified
    if let Some(env_vars) = &cmd.env {
        for (key, value) in env_vars {
            process_cmd.env(key, value);
        }
    }

    // Set up run-state: create dir, enforce reads:, inject GRAFT_STATE_DIR.
    // Done for all streaming commands so scripts can always rely on GRAFT_STATE_DIR.
    let run_state_dir = graft_engine::setup_run_state(cmd, &config, base_dir)?;
    process_cmd.env("GRAFT_STATE_DIR", run_state_dir.to_string_lossy().as_ref());

    // Execute command with output streaming
    let status = process_cmd.status().with_context(|| {
        format!(
            "Failed to execute command '{}' in directory '{}'",
            cmd.run,
            working_dir.display()
        )
    })?;

    // Check exit code
    if !status.success() {
        let exit_code = status.code().unwrap_or(1);
        eprintln!();
        eprintln!("✗ Command failed with exit code {exit_code}");
        std::process::exit(exit_code);
    }

    eprintln!();
    eprintln!("✓ Command completed successfully");

    Ok(())
}

#[allow(clippy::too_many_lines)]
fn run_dependency_command(
    dep_name: &str,
    command_name: &str,
    dry_run: bool,
    args: &[String],
) -> Result<()> {
    // Find consumer's graft.yaml (for finding the consumer repo root)
    let Some(consumer_graft_path) = find_graft_yaml() else {
        eprintln!("Error: No graft.yaml found in current directory or parent directories");
        std::process::exit(1);
    };
    let consumer_dir = consumer_graft_path
        .parent()
        .unwrap_or(Path::new("."))
        .to_path_buf();

    // Find dependency's graft.yaml
    let dep_dir = consumer_dir.join(".graft").join(dep_name);
    let dep_yaml_path = dep_dir.join("graft.yaml");

    if !dep_yaml_path.exists() {
        eprintln!(
            "Error: Dependency configuration not found: {}",
            dep_yaml_path.display()
        );
        eprintln!("  Suggestion: Check that {dep_name} is resolved in .graft/");
        std::process::exit(1);
    }

    // Parse dependency's graft.yaml
    let config = parse_graft_yaml(&dep_yaml_path)
        .with_context(|| format!("Failed to parse {}", dep_yaml_path.display()))?;

    // Determine repo name for state caching
    let repo_name = consumer_dir
        .canonicalize()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "unknown".to_string());

    // Check if it's a sequence
    if config.sequences.contains_key(command_name) {
        let _ps_guard = ProcessRegistrationGuard::register(
            &format!("graft run {dep_name}:{command_name}"),
            Some(consumer_dir.clone()),
        );
        let cmd_ctx = graft_engine::CommandContext::dependency(
            &dep_dir,
            &consumer_dir,
            &repo_name,
            &repo_name,
            false,
        );
        let exit_code = graft_engine::execute_sequence(&config, command_name, &cmd_ctx, args)?;
        if exit_code != 0 {
            std::process::exit(exit_code);
        }
        return Ok(());
    }

    // Check if command exists
    let Some(cmd) = config.commands.get(command_name) else {
        eprintln!("Error: Command '{command_name}' not found in {dep_name}/graft.yaml");

        if config.commands.is_empty() && config.sequences.is_empty() {
            eprintln!("  No commands or sequences defined in {dep_name}/graft.yaml");
        } else {
            let available: Vec<&str> = config.commands.keys().map(String::as_str).collect();
            if !available.is_empty() {
                eprintln!("  Available commands: {}", available.join(", "));
            }
            let seq_names: Vec<&str> = config.sequences.keys().map(String::as_str).collect();
            if !seq_names.is_empty() {
                eprintln!("  Available sequences: {}", seq_names.join(", "));
            }
        }

        std::process::exit(1);
    };

    // Build dependency command context: source_dir = dep dir, consumer_dir = repo root
    let cmd_ctx = graft_engine::CommandContext::dependency(
        &dep_dir,
        &consumer_dir,
        &repo_name,
        &repo_name,
        false,
    );

    // Handle dry-run mode
    if dry_run {
        if cmd.stdin.is_some() {
            let rendered = graft_engine::resolve_command_stdin(cmd, &config, &cmd_ctx, args)?;
            if let Some(text) = rendered {
                print!("{text}");
            }
        } else {
            // No stdin: print the command that would execute (with script resolution)
            let resolved_run =
                graft_engine::resolve_script_in_command(&cmd.run, &cmd_ctx.source_dir);
            println!("{}", build_command_with_args(&resolved_run, args));
        }
        return Ok(());
    }

    // Register this process so `graft ps` can see it from another shell.
    let _ps_guard = ProcessRegistrationGuard::register(
        &format!("graft run {dep_name}:{command_name}"),
        Some(consumer_dir.clone()),
    );

    // Display what we're running (stderr so stdout is clean for piping)
    eprintln!("\nExecuting: {dep_name}:{command_name}");
    if let Some(desc) = &cmd.description {
        eprintln!("  {desc}");
    }
    eprintln!("  Command: {}", cmd.run);
    if !cmd.reads.is_empty() {
        eprintln!("  Reads:   {}", cmd.reads.join(", "));
    }
    if !cmd.writes.is_empty() {
        eprintln!("  Writes:  {}", cmd.writes.join(", "));
    }
    if !cmd.context.is_empty() {
        eprintln!("  Context: {}", cmd.context.join(", "));
    }
    if cmd.stdin.is_some() {
        eprintln!("  Stdin: (template/literal)");
    }
    if !args.is_empty() {
        eprintln!("  Arguments: {}", args.join(" "));
    }
    if let Some(ref wd) = cmd.working_dir {
        eprintln!("  Working directory: {wd}");
    }
    eprintln!("  Source: .graft/{dep_name}/");
    eprintln!();

    // Commands with stdin/context need captured execution (for template rendering)
    if cmd.needs_context() {
        let result = graft_engine::execute_command_with_context(cmd, &config, &cmd_ctx, args)?;

        if !result.stdout.is_empty() {
            println!("{}", result.stdout);
        }
        if !result.stderr.is_empty() {
            eprintln!("{}", result.stderr);
        }

        if !result.success {
            eprintln!("\n✗ Command failed with exit code {}", result.exit_code);
            std::process::exit(result.exit_code);
        }

        eprintln!("\n✓ Command completed successfully");
        return Ok(());
    }

    // Simple commands: stream output directly for real-time feedback
    let resolved_run = graft_engine::resolve_script_in_command(&cmd.run, &cmd_ctx.source_dir);
    let full_command = build_command_with_args(&resolved_run, args);

    let working_dir = if let Some(ref cmd_dir) = cmd.working_dir {
        cmd_ctx.consumer_dir.join(cmd_dir)
    } else {
        cmd_ctx.consumer_dir.clone()
    };

    let mut process_cmd = std::process::Command::new("sh");
    process_cmd
        .arg("-c")
        .arg(&full_command)
        .current_dir(&working_dir);

    // Add environment variables
    if let Some(env_vars) = &cmd.env {
        for (key, value) in env_vars {
            process_cmd.env(key, value);
        }
    }
    // Inject GRAFT_DEP_DIR for dependency commands; unset it for local commands
    // so they don't inherit it from a parent process environment.
    if cmd_ctx.is_dependency() {
        process_cmd.env(
            "GRAFT_DEP_DIR",
            cmd_ctx.source_dir.to_string_lossy().as_ref(),
        );
    } else {
        process_cmd.env_remove("GRAFT_DEP_DIR");
    }

    // Set up run-state: create dir, enforce reads:, inject GRAFT_STATE_DIR.
    // Done for all streaming commands so scripts can always rely on GRAFT_STATE_DIR.
    let run_state_dir = graft_engine::setup_run_state(cmd, &config, &cmd_ctx.consumer_dir)?;
    process_cmd.env("GRAFT_STATE_DIR", run_state_dir.to_string_lossy().as_ref());

    let status = process_cmd.status().with_context(|| {
        format!(
            "Failed to execute command '{}' in directory '{}'",
            cmd.run,
            working_dir.display()
        )
    })?;

    if !status.success() {
        let exit_code = status.code().unwrap_or(1);
        eprintln!("\n✗ Command failed with exit code {exit_code}");
        std::process::exit(exit_code);
    }

    eprintln!("\n✓ Command completed successfully");

    Ok(())
}

/// Print command metadata for a single command or full catalog for a dependency.
///
/// `dep_spec` is either `"dep-name"` (catalog mode) or `"dep-name:command-name"` (detail mode).
/// When `json` is true, output is machine-readable JSON.
#[allow(clippy::too_many_lines)]
fn help_command(dep_spec: &str, json: bool) -> Result<()> {
    // Parse dep_spec into dep name and optional command name
    let (dep_name, command_name) = if let Some(colon) = dep_spec.find(':') {
        let dep = &dep_spec[..colon];
        let cmd = &dep_spec[colon + 1..];
        (dep, Some(cmd))
    } else {
        (dep_spec, None)
    };

    // Find consumer's graft.yaml to locate .graft/ directory
    let Some(consumer_graft_path) = find_graft_yaml() else {
        eprintln!("Error: No graft.yaml found in current directory or parent directories");
        std::process::exit(1);
    };
    let consumer_dir = consumer_graft_path
        .parent()
        .unwrap_or(Path::new("."))
        .to_path_buf();

    let dep_dir = consumer_dir.join(".graft").join(dep_name);
    let dep_yaml_path = dep_dir.join("graft.yaml");

    if !dep_yaml_path.exists() {
        eprintln!("Error: Dependency not found: {}", dep_yaml_path.display());
        eprintln!("  Is {dep_name} resolved in .graft/?");
        std::process::exit(1);
    }

    let config = parse_graft_yaml(&dep_yaml_path)
        .with_context(|| format!("Failed to parse {}", dep_yaml_path.display()))?;

    match command_name {
        None => {
            // Catalog mode: list all commands and sequences
            if json {
                let commands_json: serde_json::Map<String, serde_json::Value> = config
                    .commands
                    .iter()
                    .map(|(name, cmd)| {
                        let args: Vec<serde_json::Value> = graft_common::parse_commands_from_str(
                            &std::fs::read_to_string(&dep_yaml_path).unwrap_or_default(),
                        )
                        .ok()
                        .and_then(|m| m.get(name).and_then(|d| d.args.clone()))
                        .unwrap_or_default()
                        .into_iter()
                        .map(|a| {
                            serde_json::json!({
                                "name": a.name,
                                "type": format!("{:?}", a.arg_type).to_lowercase(),
                                "required": a.required,
                                "positional": a.positional,
                                "description": a.description,
                            })
                        })
                        .collect();
                        (
                            name.clone(),
                            serde_json::json!({
                                "description": cmd.description,
                                "category": cmd.category,
                                "example": cmd.example,
                                "reads": cmd.reads,
                                "writes": cmd.writes,
                                "args": args,
                            }),
                        )
                    })
                    .collect();

                let sequences_json: serde_json::Map<String, serde_json::Value> = config
                    .sequences
                    .iter()
                    .map(|(name, seq)| {
                        let step_names: Vec<&str> =
                            seq.steps.iter().map(|s| s.name.as_str()).collect();
                        (
                            name.clone(),
                            serde_json::json!({
                                "description": seq.description,
                                "category": seq.category,
                                "example": seq.example,
                                "steps": step_names,
                                "on_step_fail": seq.on_step_fail.as_ref().map(|osf| serde_json::json!({
                                    "step": osf.step,
                                    "recovery": osf.recovery,
                                    "max": osf.max,
                                })),
                                "args": seq.args.iter().map(|a| serde_json::json!({
                                    "name": a.name,
                                    "type": format!("{:?}", a.arg_type).to_lowercase(),
                                    "required": a.required,
                                    "positional": a.positional,
                                    "description": a.description,
                                })).collect::<Vec<_>>(),
                            }),
                        )
                    })
                    .collect();

                let out = serde_json::json!({
                    "commands": commands_json,
                    "sequences": sequences_json,
                });
                println!("{}", serde_json::to_string_pretty(&out)?);
            } else {
                // Text catalog mode
                if !config.commands.is_empty() {
                    println!("Commands:");
                    let mut names: Vec<&String> = config.commands.keys().collect();
                    names.sort();
                    for name in names {
                        let cmd = &config.commands[name];
                        let cat = cmd.category.as_deref().unwrap_or("");
                        let desc = cmd.description.as_deref().unwrap_or("");
                        let cat_tag = if cat.is_empty() {
                            String::new()
                        } else {
                            format!(" [{cat}]")
                        };
                        println!("  {name:<20}{cat_tag:<14} {desc}");
                        if !cmd.reads.is_empty() {
                            println!("    Reads:  {}", cmd.reads.join(", "));
                        }
                        if !cmd.writes.is_empty() {
                            println!("    Writes: {}", cmd.writes.join(", "));
                        }
                    }
                    println!();
                }
                if !config.sequences.is_empty() {
                    println!("Sequences:");
                    let mut names: Vec<&String> = config.sequences.keys().collect();
                    names.sort();
                    for name in names {
                        let seq = &config.sequences[name];
                        let cat = seq.category.as_deref().unwrap_or("");
                        let desc = seq.description.as_deref().unwrap_or("");
                        let cat_tag = if cat.is_empty() {
                            String::new()
                        } else {
                            format!(" [{cat}]")
                        };
                        let step_names: Vec<&str> =
                            seq.steps.iter().map(|s| s.name.as_str()).collect();
                        println!("  {name:<20}{cat_tag:<14} {desc}");
                        println!("    Steps: {}", step_names.join(" → "));
                    }
                }
            }
        }
        Some(name) => {
            // Detail mode: single command or sequence
            if let Some(cmd) = config.commands.get(name) {
                if json {
                    // Load args from graft-common for richer type info
                    let raw_args = graft_common::parse_commands_from_str(
                        &std::fs::read_to_string(&dep_yaml_path).unwrap_or_default(),
                    )
                    .ok()
                    .and_then(|m| m.get(name).and_then(|d| d.args.clone()))
                    .unwrap_or_default();
                    let args_json: Vec<serde_json::Value> = raw_args
                        .iter()
                        .map(|a| {
                            serde_json::json!({
                                "name": a.name,
                                "type": format!("{:?}", a.arg_type).to_lowercase(),
                                "required": a.required,
                                "positional": a.positional,
                                "description": a.description,
                            })
                        })
                        .collect();
                    let out = serde_json::json!({
                        "name": name,
                        "description": cmd.description,
                        "category": cmd.category,
                        "example": cmd.example,
                        "reads": cmd.reads,
                        "writes": cmd.writes,
                        "args": args_json,
                    });
                    println!("{}", serde_json::to_string_pretty(&out)?);
                } else {
                    println!();
                    println!("  {name} — {}", cmd.description.as_deref().unwrap_or(""));
                    if let Some(cat) = &cmd.category {
                        println!("  Category: {cat}");
                    }
                    if let Some(ex) = &cmd.example {
                        println!("  Example:  {ex}");
                    }
                    if !cmd.reads.is_empty() {
                        println!("  Reads:    {}", cmd.reads.join(", "));
                    }
                    if !cmd.writes.is_empty() {
                        println!("  Writes:   {}", cmd.writes.join(", "));
                    }
                    // Print args from graft-common (has positional/required info)
                    let raw_args = graft_common::parse_commands_from_str(
                        &std::fs::read_to_string(&dep_yaml_path).unwrap_or_default(),
                    )
                    .ok()
                    .and_then(|m| m.get(name).and_then(|d| d.args.clone()))
                    .unwrap_or_default();
                    if !raw_args.is_empty() {
                        println!();
                        println!("  Arguments:");
                        for a in &raw_args {
                            let type_str = format!("{:?}", a.arg_type).to_lowercase();
                            let req = if a.required { "required" } else { "optional" };
                            let pos = if a.positional { ", positional" } else { "" };
                            let desc = a.description.as_deref().unwrap_or("");
                            println!("    {}  ({type_str}, {req}{pos})  {desc}", a.name);
                        }
                    }
                }
            } else if let Some(seq) = config.sequences.get(name) {
                if json {
                    let step_names: Vec<&str> = seq.steps.iter().map(|s| s.name.as_str()).collect();
                    let out = serde_json::json!({
                        "name": name,
                        "description": seq.description,
                        "category": seq.category,
                        "example": seq.example,
                        "steps": step_names,
                        "on_step_fail": seq.on_step_fail.as_ref().map(|osf| serde_json::json!({
                            "step": osf.step,
                            "recovery": osf.recovery,
                            "max": osf.max,
                        })),
                        "args": seq.args.iter().map(|a| serde_json::json!({
                            "name": a.name,
                            "type": format!("{:?}", a.arg_type).to_lowercase(),
                            "required": a.required,
                            "positional": a.positional,
                            "description": a.description,
                        })).collect::<Vec<_>>(),
                    });
                    println!("{}", serde_json::to_string_pretty(&out)?);
                } else {
                    let step_names: Vec<&str> = seq.steps.iter().map(|s| s.name.as_str()).collect();
                    println!();
                    println!("  {name} — {}", seq.description.as_deref().unwrap_or(""));
                    if let Some(cat) = &seq.category {
                        println!("  Category: {cat}");
                    }
                    if let Some(ex) = &seq.example {
                        println!("  Example:  {ex}");
                    }
                    println!("  Steps:    {}", step_names.join(" → "));
                    if let Some(osf) = &seq.on_step_fail {
                        println!(
                            "  Retry:    {} fails → {} (max {})",
                            osf.step, osf.recovery, osf.max
                        );
                    }
                    if !seq.args.is_empty() {
                        println!();
                        println!("  Arguments:");
                        for a in &seq.args {
                            let type_str = format!("{:?}", a.arg_type).to_lowercase();
                            let req = if a.required { "required" } else { "optional" };
                            let pos = if a.positional { ", positional" } else { "" };
                            let desc = a.description.as_deref().unwrap_or("");
                            println!("    {}  ({type_str}, {req}{pos})  {desc}", a.name);
                        }
                    }
                }
            } else {
                eprintln!("Error: unknown command: {name}");
                std::process::exit(1);
            }
        }
    }

    Ok(())
}

fn state_list_command() -> Result<()> {
    // Find graft.yaml
    let graft_path =
        find_graft_yaml().context("No graft.yaml found in current directory or parents")?;

    // Parse config
    let config = parse_graft_yaml(&graft_path)?;

    if config.state.is_empty() {
        println!("No state queries defined in graft.yaml");
        return Ok(());
    }

    // Get current commit hash
    let repo_path = graft_path
        .parent()
        .context("Failed to get repository path")?;
    let commit_hash = get_current_commit(repo_path)?;

    // Use repository name as both workspace and repo name (simplified for Stage 1)
    let repo_name = repo_path
        .file_name()
        .and_then(|n| n.to_str())
        .context("Failed to get repository name")?;

    // List queries with cache status
    let statuses = list_state_queries(&config.state, repo_name, repo_name, &commit_hash);

    println!("State queries defined in graft.yaml:\n");

    for status in statuses {
        println!("{}", status.name);
        println!("  Command: {}", status.command);
        if status.cached {
            if let Some(timestamp) = status.cache_timestamp {
                println!("  Cached:  Yes ({timestamp})");
            } else {
                println!("  Cached:  Yes");
            }
        } else {
            println!("  Cached:  No");
        }
        println!();
    }

    Ok(())
}

#[allow(clippy::too_many_lines)]
fn state_query_command(name: &str, refresh: bool, raw: bool, pretty: bool) -> Result<()> {
    // Find graft.yaml
    let graft_path =
        find_graft_yaml().context("No graft.yaml found in current directory or parents")?;

    // Parse config
    let config = parse_graft_yaml(&graft_path)?;

    // Get repository path
    let repo_path = graft_path
        .parent()
        .context("Failed to get repository path")?;

    // Get current commit hash
    let commit_hash = get_current_commit(repo_path)?;

    // Use repository name as both workspace and repo name (simplified for Stage 1)
    let repo_name = repo_path
        .file_name()
        .and_then(|n| n.to_str())
        .context("Failed to get repository name")?;

    // Resolve: configured state query first, then run-state store
    let data = if let Some(query) = config.state.get(name) {
        let result = get_state(
            query,
            repo_name,
            repo_name,
            repo_path,
            &commit_hash,
            refresh,
        )?;
        if raw {
            if pretty {
                println!("{}", serde_json::to_string_pretty(&result.data)?);
            } else {
                println!("{}", serde_json::to_string(&result.data)?);
            }
            return Ok(());
        }
        if pretty {
            println!("{}", serde_json::to_string_pretty(&result)?);
        } else {
            println!("{}", serde_json::to_string(&result)?);
        }
        return Ok(());
    } else if let Some(value) = graft_engine::get_run_state_entry(name, repo_path) {
        value
    } else {
        bail!("State '{name}' not found in graft.yaml state section or run-state store");
    };

    // Output run-state data (no metadata wrapper for run-state entries)
    if pretty {
        println!("{}", serde_json::to_string_pretty(&data)?);
    } else {
        println!("{}", serde_json::to_string(&data)?);
    }

    Ok(())
}

fn state_invalidate_command(name: Option<&str>, all: bool) -> Result<()> {
    // Find graft.yaml
    let graft_path =
        find_graft_yaml().context("No graft.yaml found in current directory or parents")?;

    // Get repository path
    let repo_path = graft_path
        .parent()
        .context("Failed to get repository path")?;

    // Use repository name as both workspace and repo name (simplified for Stage 1)
    let repo_name = repo_path
        .file_name()
        .and_then(|n| n.to_str())
        .context("Failed to get repository name")?;

    let count = if all || name.is_none() {
        // Invalidate all
        invalidate_cached_state(repo_name, repo_name, None)?
    } else {
        // Invalidate specific query
        invalidate_cached_state(repo_name, repo_name, name)?
    };

    if all || name.is_none() {
        println!("✓ Invalidated all state caches ({count} file(s) deleted)");
    } else if let Some(query_name) = name {
        println!("✓ Invalidated cache for '{query_name}' ({count} file(s) deleted)");
    }

    Ok(())
}

/// RAII guard that registers a process entry on creation and deregisters on drop.
///
/// Uses the current process's PID so `graft ps` from another shell can see
/// that a `graft run` command is active.
struct ProcessRegistrationGuard {
    registry: FsProcessRegistry,
    pid: u32,
}

impl ProcessRegistrationGuard {
    /// Register the current process in the default process registry.
    ///
    /// Returns `None` if the registry cannot be opened (non-fatal).
    fn register(command: &str, repo_path: Option<PathBuf>) -> Option<Self> {
        let registry = FsProcessRegistry::new(FsProcessRegistry::default_path()).ok()?;
        let pid = std::process::id();
        let entry = ProcessEntry::new_running(pid, command, repo_path, None);
        registry.register(entry).ok()?;
        Some(Self { registry, pid })
    }
}

impl Drop for ProcessRegistrationGuard {
    fn drop(&mut self) {
        let _ = self.registry.deregister(self.pid);
    }
}

/// Format a Unix timestamp as a relative "time ago" string.
fn format_unix_time_ago(ts: i64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
        .unwrap_or(0);
    let secs = now.saturating_sub(ts);
    if secs < 60 {
        "just now".to_string()
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else if secs < 86400 {
        format!("{}h ago", secs / 3600)
    } else {
        format!("{}d ago", secs / 86400)
    }
}

fn scion_list_command(format: &OutputFormat) -> Result<()> {
    let repo_path = std::env::current_dir().context("Failed to determine current directory")?;

    // Try to create runtime for session detection
    let runtime = TmuxRuntime::new().ok();
    let runtime_ref = runtime
        .as_ref()
        .map(|r| r as &dyn graft_common::SessionRuntime);

    let scions = scion_list(&repo_path, runtime_ref).context("Failed to list scions")?;

    match format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&scions)
                .context("Failed to serialize scion list to JSON")?;
            println!("{json}");
        }
        OutputFormat::Text => {
            if scions.is_empty() {
                println!("No scions");
                return Ok(());
            }
            for s in &scions {
                let ahead_behind = match (s.ahead, s.behind) {
                    (Some(a), Some(b)) => format!("{a} ahead, {b} behind"),
                    _ => "? ahead, ? behind".to_string(),
                };
                let time_str = s.last_commit_time.map_or_else(
                    || "no commits".to_string(),
                    |t| format!("last: {}", format_unix_time_ago(t)),
                );
                let dirty_str = if s.dirty { "  dirty" } else { "" };
                let session_str = match s.session_active {
                    Some(true) => "  ● active",
                    _ => "",
                };
                let verify_str = match s.verify_status {
                    Some(VerifyLevel::Ok) => "  ✓ pass",
                    Some(VerifyLevel::Fail) => "  ✗ fail",
                    Some(VerifyLevel::Unknown) => "  ~ partial",
                    None => "",
                };
                println!(
                    "{:<25} {:<22} {:<20}{}{}{}",
                    s.name, ahead_behind, time_str, dirty_str, session_str, verify_str
                );
            }
        }
    }
    Ok(())
}

/// Try to load the project's graft.yaml config. Returns None if not found.
fn try_load_graft_config(repo_path: &Path) -> Option<graft_engine::GraftConfig> {
    let config_path = repo_path.join("graft.yaml");
    if config_path.exists() {
        parse_graft_yaml(&config_path).ok()
    } else {
        None
    }
}

fn scion_create_command(name: &str) -> Result<()> {
    let repo_path = std::env::current_dir().context("Failed to determine current directory")?;
    let config = try_load_graft_config(&repo_path);
    let (dep_configs, dep_warnings) = config
        .as_ref()
        .map(|c| load_dep_configs(&repo_path, c))
        .unwrap_or_default();
    for w in &dep_warnings {
        eprintln!("warning: {w}");
    }
    let wt_path = scion_create(&repo_path, name, config.as_ref(), &dep_configs)
        .with_context(|| format!("Failed to create scion '{name}'"))?;
    println!("Created scion '{name}' at {}", wt_path.display());
    println!("  worktree: {}", wt_path.display());
    println!("  branch:   feature/{name}");
    Ok(())
}

fn scion_prune_command(name: &str, force: bool) -> Result<()> {
    let repo_path = std::env::current_dir().context("Failed to determine current directory")?;
    let config = try_load_graft_config(&repo_path);
    let (dep_configs, dep_warnings) = config
        .as_ref()
        .map(|c| load_dep_configs(&repo_path, c))
        .unwrap_or_default();
    for w in &dep_warnings {
        eprintln!("warning: {w}");
    }
    let runtime = TmuxRuntime::new().ok();
    let runtime_ref = runtime.as_ref().map(|r| r as &dyn SessionRuntime);
    scion_prune(
        &repo_path,
        name,
        config.as_ref(),
        &dep_configs,
        runtime_ref,
        force,
    )
    .with_context(|| format!("Failed to prune scion '{name}'"))?;
    println!("Pruned scion '{name}'");
    Ok(())
}

fn scion_start_command(name: &str) -> Result<()> {
    let repo_path = std::env::current_dir().context("Failed to determine current directory")?;
    let config = try_load_graft_config(&repo_path);
    let (dep_configs, dep_warnings) = config
        .as_ref()
        .map(|c| load_dep_configs(&repo_path, c))
        .unwrap_or_default();
    for w in &dep_warnings {
        eprintln!("warning: {w}");
    }
    let runtime = TmuxRuntime::new().context("tmux is required for scion sessions")?;
    scion_start(&repo_path, name, config.as_ref(), &dep_configs, &runtime)
        .with_context(|| format!("Failed to start scion '{name}'"))?;
    println!("Started scion '{name}' (session: graft-scion-{name})");
    Ok(())
}

fn scion_stop_command(name: &str) -> Result<()> {
    let repo_path = std::env::current_dir().context("Failed to determine current directory")?;
    let runtime = TmuxRuntime::new().context("tmux is required for scion sessions")?;
    scion_stop(&repo_path, name, &runtime)
        .with_context(|| format!("Failed to stop scion '{name}'"))?;
    println!("Stopped scion '{name}'");
    Ok(())
}

fn scion_attach_command(name: &str) -> Result<()> {
    let repo_path = std::env::current_dir().context("Failed to determine current directory")?;
    let runtime = TmuxRuntime::new().context("tmux is required for scion sessions")?;
    let session_id = scion_attach_check(&repo_path, name, &runtime)
        .with_context(|| format!("Failed to attach to scion '{name}'"))?;
    runtime
        .attach(&session_id)
        .with_context(|| format!("Failed to attach to session '{session_id}'"))?;
    Ok(())
}

fn scion_run_command(name: &str) -> Result<()> {
    let repo_path = std::env::current_dir().context("Failed to determine current directory")?;
    let config = try_load_graft_config(&repo_path);
    let (dep_configs, dep_warnings) = config
        .as_ref()
        .map(|c| load_dep_configs(&repo_path, c))
        .unwrap_or_default();
    for w in &dep_warnings {
        eprintln!("warning: {w}");
    }
    let runtime = TmuxRuntime::new().context("tmux is required for scion sessions")?;
    match scion_run(&repo_path, name, config.as_ref(), &dep_configs, &runtime) {
        Ok(wt_path) => {
            println!("Scion '{name}' is running (session: graft-scion-{name})");
            println!("  worktree: {}", wt_path.display());
            println!("  attach:   graft scion attach {name}");
            Ok(())
        }
        Err(graft_engine::GraftError::SessionAlreadyActive { .. }) => {
            println!("Scion '{name}' already has an active session.");
            println!("  attach: graft scion attach {name}");
            Ok(())
        }
        Err(e) => Err(e).with_context(|| format!("Failed to run scion '{name}'")),
    }
}

fn scion_fuse_command(name: &str, force: bool) -> Result<()> {
    let repo_path = std::env::current_dir().context("Failed to determine current directory")?;
    let config = try_load_graft_config(&repo_path);
    let (dep_configs, dep_warnings) = config
        .as_ref()
        .map(|c| load_dep_configs(&repo_path, c))
        .unwrap_or_default();
    for w in &dep_warnings {
        eprintln!("warning: {w}");
    }
    let runtime = TmuxRuntime::new().ok();
    let runtime_ref = runtime.as_ref().map(|r| r as &dyn SessionRuntime);
    let merge_commit = scion_fuse(
        &repo_path,
        name,
        config.as_ref(),
        &dep_configs,
        runtime_ref,
        force,
    )
    .with_context(|| format!("Failed to fuse scion '{name}'"))?;
    println!("Fused scion '{name}' into main");
    println!("  merge commit: {merge_commit}");
    println!("  worktree and branch cleaned up");
    Ok(())
}

fn ps_command(repo_filter: Option<&str>) -> Result<()> {
    ps_command_impl(FsProcessRegistry::default_path(), repo_filter)
}

fn ps_command_impl(registry_path: std::path::PathBuf, repo_filter: Option<&str>) -> Result<()> {
    let registry =
        FsProcessRegistry::new(registry_path).context("Failed to open process registry")?;
    let mut entries = registry
        .list_active()
        .context("Failed to list active processes")?;

    if let Some(filter) = repo_filter {
        let filter_canonical =
            std::fs::canonicalize(filter).unwrap_or_else(|_| PathBuf::from(filter));
        entries.retain(|e| {
            e.repo_path
                .as_ref()
                .map(|p| std::fs::canonicalize(p).unwrap_or_else(|_| p.clone()))
                .as_deref()
                == Some(filter_canonical.as_path())
        });
    }

    if entries.is_empty() {
        println!("No active processes.");
        return Ok(());
    }

    println!("Active processes ({}):", entries.len());
    for entry in &entries {
        let status_str = format_process_status(&entry.status);
        let repo_str = entry
            .repo_path
            .as_deref()
            .and_then(|p| p.to_str())
            .unwrap_or("(none)");
        println!("  PID {}  {}", entry.pid, entry.command);
        println!("    Repository: {repo_str}");
        println!("    Started:    {}", entry.start_time);
        println!("    Status:     {status_str}");
    }

    Ok(())
}

fn format_process_status(status: &ProcessStatus) -> String {
    match status {
        ProcessStatus::Running => "Running".to_string(),
        ProcessStatus::Completed { exit_code } => format!("Completed (exit {exit_code})"),
        ProcessStatus::Failed { error } => format!("Failed: {error}"),
    }
}

/// Get current commit hash for a repository.
fn get_current_commit(repo_path: &Path) -> Result<String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_path)
        .output()
        .context("Failed to run git rev-parse")?;

    if !output.status.success() {
        bail!("Failed to get current commit hash");
    }

    let commit = String::from_utf8_lossy(&output.stdout).trim().to_string();

    Ok(commit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use graft_common::ProcessEntry;
    use tempfile::tempdir;

    #[test]
    fn test_ps_command_empty_registry() {
        let dir = tempdir().unwrap();
        let result = ps_command_impl(dir.path().to_path_buf(), None);
        assert!(
            result.is_ok(),
            "ps_command_impl should succeed with empty registry: {result:?}"
        );
    }

    #[test]
    fn test_ps_command_with_active_entry() {
        let dir = tempdir().unwrap();
        let pid = std::process::id(); // Current process is always alive.
        let entry = ProcessEntry {
            pid,
            command: "echo test".to_string(),
            repo_path: Some(PathBuf::from("/tmp/test-repo")),
            start_time: "2026-02-19T10:00:00Z".to_string(),
            log_path: None,
            status: ProcessStatus::Running,
        };
        let content = serde_json::to_string(&entry).unwrap();
        std::fs::write(dir.path().join(format!("{pid}.json")), content).unwrap();

        let result = ps_command_impl(dir.path().to_path_buf(), None);
        assert!(
            result.is_ok(),
            "ps_command_impl should succeed with active entry: {result:?}"
        );
    }

    #[test]
    fn test_ps_command_repo_filter_matches() {
        let dir = tempdir().unwrap();
        let pid = std::process::id();
        let repo_path = "/tmp/specific-repo";
        let entry = ProcessEntry {
            pid,
            command: "make test".to_string(),
            repo_path: Some(PathBuf::from(repo_path)),
            start_time: "2026-02-19T10:00:00Z".to_string(),
            log_path: None,
            status: ProcessStatus::Running,
        };
        let content = serde_json::to_string(&entry).unwrap();
        std::fs::write(dir.path().join(format!("{pid}.json")), content).unwrap();

        let result = ps_command_impl(dir.path().to_path_buf(), Some(repo_path));
        assert!(
            result.is_ok(),
            "ps_command_impl with matching filter should succeed: {result:?}"
        );
    }

    #[test]
    fn test_ps_command_repo_filter_no_match() {
        let dir = tempdir().unwrap();
        let pid = std::process::id();
        let entry = ProcessEntry {
            pid,
            command: "make test".to_string(),
            repo_path: Some(PathBuf::from("/tmp/repo-a")),
            start_time: "2026-02-19T10:00:00Z".to_string(),
            log_path: None,
            status: ProcessStatus::Running,
        };
        let content = serde_json::to_string(&entry).unwrap();
        std::fs::write(dir.path().join(format!("{pid}.json")), content).unwrap();

        // Filter does not match; should print "No active processes."
        let result = ps_command_impl(dir.path().to_path_buf(), Some("/tmp/repo-b"));
        assert!(
            result.is_ok(),
            "ps_command_impl with non-matching filter should succeed: {result:?}"
        );
    }

    #[test]
    fn test_cli_debug_assert() {
        // Verify the augmented Command (with completers) is valid
        Cli::command().debug_assert();
    }

    #[test]
    fn test_ps_command_repo_filter_trailing_slash() {
        let dir = tempdir().unwrap();
        let pid = std::process::id();
        let repo_path = dir.path().join("my-repo");
        std::fs::create_dir_all(&repo_path).unwrap();
        let entry = ProcessEntry {
            pid,
            command: "make test".to_string(),
            repo_path: Some(repo_path.clone()),
            start_time: "2026-02-20T10:00:00Z".to_string(),
            log_path: None,
            status: ProcessStatus::Running,
        };
        let content = serde_json::to_string(&entry).unwrap();
        std::fs::write(dir.path().join(format!("{pid}.json")), content).unwrap();

        // Filter with trailing slash should still match
        let filter = format!("{}/", repo_path.display());
        let result = ps_command_impl(dir.path().to_path_buf(), Some(&filter));
        assert!(
            result.is_ok(),
            "ps_command_impl with trailing-slash filter: {result:?}"
        );
    }
}
