//! Graft CLI: semantic dependency manager.

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use graft_engine::{
    filter_breaking_changes, filter_changes_by_type, get_all_status, get_change_details,
    get_changes_for_dependency, get_dependency_status, parse_graft_yaml, parse_lock_file,
};
use std::path::{Path, PathBuf};

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
        dep_name: Option<String>,

        /// Output format (text or json)
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// List changes for a dependency
    Changes {
        /// Dependency name
        dep_name: String,

        /// Filter by change type (breaking, feature, fix, etc.)
        #[arg(long)]
        r#type: Option<String>,

        /// Show only breaking changes
        #[arg(long)]
        breaking: bool,

        /// Output format (text or json)
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Show details of a specific change
    Show {
        /// Dependency and ref in format "dep-name@ref" (e.g., "meta-kb@v2.0.0")
        dep_ref: String,

        /// Output format (text or json)
        #[arg(long, default_value = "text")]
        format: String,
    },
}

fn main() -> Result<()> {
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
    }

    Ok(())
}

fn status_command(dep_name: Option<&str>, format: &str) -> Result<()> {
    // Validate format
    if format != "text" && format != "json" {
        bail!("Invalid format '{format}'. Must be 'text' or 'json'");
    }

    let lock_path = Path::new("graft.lock");

    // Check if lock file exists
    if !lock_path.exists() {
        if format == "json" {
            println!("{{\"dependencies\":{{}}}}");
        } else {
            eprintln!("No dependencies found in graft.lock");
            eprintln!();
            eprintln!("Run 'graft resolve' to resolve dependencies first.");
        }
        return Ok(());
    }

    // Parse lock file
    let lock_file = parse_lock_file(lock_path).context("Failed to parse graft.lock")?;

    if let Some(name) = dep_name {
        // Show status for single dependency
        let status = get_dependency_status(&lock_file, name);

        if let Some(s) = status {
            if format == "json" {
                let json = serde_json::json!({
                    "name": s.name,
                    "current_ref": s.current_ref,
                    "commit": s.commit.as_str(),
                    "consumed_at": s.consumed_at,
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                println!("{}: {}", s.name, s.current_ref);
                println!("  Commit: {}...", &s.commit.as_str()[..7]);
                println!("  Consumed: {}", s.consumed_at);
            }
        } else {
            if format == "json" {
                let json = serde_json::json!({
                    "error": format!("Dependency '{name}' not found in graft.lock")
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                eprintln!("Error: Dependency '{name}' not found in graft.lock");
            }
            std::process::exit(1);
        }
    } else {
        // Show status for all dependencies
        let statuses = get_all_status(&lock_file);

        if statuses.is_empty() {
            if format == "json" {
                println!("{{\"dependencies\":{{}}}}");
            } else {
                eprintln!("No dependencies found in graft.lock");
                eprintln!();
                eprintln!("Run 'graft resolve' to resolve dependencies first.");
            }
            return Ok(());
        }

        if format == "json" {
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
        } else {
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

    Ok(())
}

#[allow(clippy::too_many_lines)]
fn changes_command(
    dep_name: &str,
    change_type: Option<&str>,
    breaking_only: bool,
    format: &str,
) -> Result<()> {
    // Validate format
    if format != "text" && format != "json" {
        bail!("Invalid format '{format}'. Must be 'text' or 'json'");
    }

    // Validate type and breaking are not both specified
    if breaking_only && change_type.is_some() {
        bail!("Cannot specify both --type and --breaking");
    }

    // Find dependency's graft.yaml
    let dep_path = PathBuf::from(".graft").join(dep_name).join("graft.yaml");

    if !dep_path.exists() {
        if format == "json" {
            let json = serde_json::json!({
                "error": format!("Dependency configuration not found: {}", dep_path.display()),
                "suggestion": format!("Check that {dep_name} is resolved in .graft/")
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        } else {
            eprintln!("Error: Dependency configuration not found");
            eprintln!("  Path: {}", dep_path.display());
            eprintln!("  Suggestion: Check that {dep_name} is resolved in .graft/");
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

        if format == "json" {
            let json = serde_json::json!({
                "dependency": dep_name,
                "changes": [],
                "message": format!("No {filter_desc}changes found")
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        } else {
            println!("No {filter_desc}changes found for {dep_name}");
        }
        return Ok(());
    }

    if format == "json" {
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
    } else {
        // Text output
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

    Ok(())
}

#[allow(clippy::too_many_lines)]
fn show_command(dep_ref: &str, format: &str) -> Result<()> {
    // Validate format
    if format != "text" && format != "json" {
        bail!("Invalid format '{format}'. Must be 'text' or 'json'");
    }

    // Parse dep_name@ref format
    let Some((dep_name, ref_name)) = dep_ref.split_once('@') else {
        bail!("Invalid format. Use 'dep-name@ref' (e.g., 'meta-kb@v2.0.0')");
    };

    // Find dependency's graft.yaml
    let dep_path = PathBuf::from(".graft").join(dep_name).join("graft.yaml");

    if !dep_path.exists() {
        if format == "json" {
            let json = serde_json::json!({
                "error": format!("Dependency configuration not found: {}", dep_path.display()),
                "suggestion": format!("Check that {dep_name} is resolved in .graft/")
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        } else {
            eprintln!("Error: Dependency configuration not found");
            eprintln!("  Path: {}", dep_path.display());
            eprintln!("  Suggestion: Check that {dep_name} is resolved in .graft/");
        }
        std::process::exit(1);
    }

    // Parse dependency's graft.yaml
    let config = parse_graft_yaml(&dep_path)
        .with_context(|| format!("Failed to parse {}", dep_path.display()))?;

    // Get change details
    let Some(details) = get_change_details(&config, ref_name) else {
        if format == "json" {
            let json = serde_json::json!({
                "error": format!("Change {ref_name} not found for {dep_name}"),
                "suggestion": format!("Run 'graft changes {dep_name}' to see available changes")
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        } else {
            eprintln!("Error: Change {ref_name} not found for {dep_name}");
            eprintln!("  Run 'graft changes {dep_name}' to see available changes");
        }
        std::process::exit(1);
    };

    if format == "json" {
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
    } else {
        // Text output
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

    Ok(())
}
