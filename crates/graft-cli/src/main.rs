//! Graft CLI: semantic dependency manager.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use graft_engine::{get_all_status, get_dependency_status, parse_lock_file};
use std::path::Path;

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
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Status { dep_name, format } => {
            status_command(dep_name.as_deref(), &format)?;
        }
    }

    Ok(())
}

fn status_command(dep_name: Option<&str>, format: &str) -> Result<()> {
    // Validate format
    if format != "text" && format != "json" {
        anyhow::bail!("Invalid format '{}'. Must be 'text' or 'json'", format);
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

        match status {
            Some(s) => {
                if format == "json" {
                    let json = serde_json::json!({
                        "name": s.name,
                        "current_ref": s.current_ref,
                        "commit": s.commit.as_str(),
                        "consumed_at": s.consumed_at.to_rfc3339(),
                    });
                    println!("{}", serde_json::to_string_pretty(&json)?);
                } else {
                    println!("{}: {}", s.name, s.current_ref);
                    println!("  Commit: {}...", &s.commit.as_str()[..7]);
                    println!("  Consumed: {}", s.consumed_at.format("%Y-%m-%d %H:%M:%S"));
                }
            }
            None => {
                if format == "json" {
                    let json = serde_json::json!({
                        "error": format!("Dependency '{}' not found in graft.lock", name)
                    });
                    println!("{}", serde_json::to_string_pretty(&json)?);
                } else {
                    eprintln!("Error: Dependency '{}' not found in graft.lock", name);
                }
                std::process::exit(1);
            }
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
                    "consumed_at": status.consumed_at.to_rfc3339(),
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
                    status.consumed_at.format("%Y-%m-%d %H:%M:%S")
                );
            }
        }
    }

    Ok(())
}
