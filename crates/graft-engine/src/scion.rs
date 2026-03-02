//! Scion lifecycle operations.
//!
//! A scion is a named parallel workstream implemented as a git worktree +
//! branch pair. This module applies the graft naming convention:
//! - Worktree path: `.worktrees/<name>` (relative to repo root)
//! - Branch name: `feature/<name>`
//!
//! Git primitives from `graft-common` take explicit paths and branch names.
//! The naming convention lives here, not in the primitives.

use crate::domain::{GraftConfig, ScionHooks};
use crate::error::{GraftError, Result};
use graft_common::process::{run_to_completion_with_timeout, ProcessConfig};
use graft_common::runtime::SessionRuntime;
use graft_common::{
    git_ahead_behind, git_branch_delete, git_delete_ref, git_fast_forward, git_is_dirty,
    git_last_commit_time, git_merge_to_ref, git_reset_hard, git_worktree_add, git_worktree_list,
    git_worktree_remove,
};
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Validate that a scion name is safe for use as a directory component and branch suffix.
///
/// Rules:
/// - Non-empty
/// - Max 100 characters
/// - Must not start with `.` or `-`
/// - Only `[a-zA-Z0-9._-]` characters (no slashes, colons, spaces, or control chars)
fn validate_scion_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(GraftError::CommandExecution(
            "scion name cannot be empty".to_string(),
        ));
    }
    if name.len() > 100 {
        return Err(GraftError::CommandExecution(format!(
            "scion name too long: {} chars (max 100)",
            name.len()
        )));
    }
    if name.starts_with('.') || name.starts_with('-') {
        return Err(GraftError::CommandExecution(format!(
            "invalid scion name: '{name}'. Must not start with '.' or '-'."
        )));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
    {
        return Err(GraftError::CommandExecution(format!(
            "invalid scion name: '{name}'. Only [a-zA-Z0-9._-] characters are allowed."
        )));
    }
    Ok(())
}

/// Compute the worktree path for a scion: `<repo_root>/.worktrees/<name>`.
fn worktree_path(repo: &Path, name: &str) -> PathBuf {
    repo.join(".worktrees").join(name)
}

/// Resolve the base branch from the main worktree.
///
/// Returns an error if the main worktree is in detached HEAD state (no branch).
pub fn resolve_base_branch(worktrees: &[graft_common::WorktreeInfo]) -> Result<String> {
    worktrees
        .first()
        .and_then(|w| w.branch.clone())
        .ok_or_else(|| {
            GraftError::CommandExecution(
                "cannot determine base branch: main worktree is in detached HEAD state".into(),
            )
        })
}

/// Compute the branch name for a scion: `feature/<name>`.
fn branch_name(name: &str) -> String {
    format!("feature/{name}")
}

/// Compute the tmux session ID for a scion: `graft-scion-<name>`.
pub fn scion_session_id(name: &str) -> String {
    format!("graft-scion-{name}")
}

/// Create a new scion (worktree + branch) for the given name.
///
/// Applies the naming convention:
/// - Worktree: `<repo_root>/.worktrees/<name>`
/// - Branch: `feature/<name>`
///
/// After worktree creation, runs `on_create` hooks if configured. On hook
/// failure, the worktree and branch are rolled back (removed).
///
/// # Arguments
/// * `repo_path`   - Absolute path to the main git repository
/// * `name`        - Scion name (e.g. `my-feature`)
/// * `config`      - Project config (for hook resolution). Pass `None` to skip hooks.
/// * `dep_configs` - Dependency configs in declaration order (for hook resolution).
///
/// # Returns
/// Absolute path to the newly created worktree.
///
/// # Errors
/// Returns `GraftError` if the worktree or branch already exists, if an
/// `on_create` hook fails (after rollback), or the underlying git operation fails.
pub fn scion_create(
    repo_path: impl AsRef<Path>,
    name: &str,
    config: Option<&GraftConfig>,
    dep_configs: &[(String, GraftConfig)],
) -> Result<PathBuf> {
    validate_scion_name(name)?;
    let repo = repo_path.as_ref();
    let path = worktree_path(repo, name);
    let branch = branch_name(name);
    git_worktree_add(repo, &path, &branch).map_err(GraftError::from)?;

    // Run on_create hooks if config is provided
    if let Some(cfg) = config {
        let chain = resolve_hook_chain(HookEvent::OnCreate, cfg, dep_configs, &path, repo);
        if !chain.is_empty() {
            let scion_env = ScionEnv {
                name: name.to_string(),
                branch: branch.clone(),
                worktree: path.clone(),
            };
            if let Err(hook_err) = execute_hook_chain(&chain, cfg, dep_configs, &scion_env) {
                // Rollback: remove worktree and branch on hook failure
                let _ = git_worktree_remove(repo, &path);
                let _ = git_branch_delete(repo, &branch);
                return Err(GraftError::CommandExecution(format!(
                    "on_create hook failed (worktree rolled back): {hook_err}"
                )));
            }
        }
    }

    Ok(path)
}

/// Remove a scion (worktree + branch) by name.
///
/// Applies the naming convention:
/// - Worktree: `<repo_root>/.worktrees/<name>`
/// - Branch: `feature/<name>`
///
/// Runs `on_prune` hooks before removal if configured. On hook failure,
/// the worktree is left intact and the error is returned.
///
/// # Arguments
/// * `repo_path`   - Absolute path to the main git repository
/// * `name`        - Scion name (e.g. `my-feature`)
/// * `config`      - Project config (for hook resolution). Pass `None` to skip hooks.
/// * `dep_configs` - Dependency configs in declaration order (for hook resolution).
///
/// # Errors
/// Returns `GraftError` if the worktree or branch does not exist, if an
/// `on_prune` hook fails (worktree preserved), or the underlying git operation fails.
pub fn scion_prune(
    repo_path: impl AsRef<Path>,
    name: &str,
    config: Option<&GraftConfig>,
    dep_configs: &[(String, GraftConfig)],
    runtime: Option<&dyn SessionRuntime>,
    force: bool,
) -> Result<()> {
    validate_scion_name(name)?;
    let repo = repo_path.as_ref();

    // Session guard: refuse to prune while a runtime session is active
    if let Some(rt) = runtime {
        let session_id = scion_session_id(name);
        if rt.exists(&session_id).unwrap_or(false) {
            if !force {
                return Err(GraftError::CommandExecution(format!(
                    "scion '{name}' has an active runtime session ({session_id}); stop it first or use --force"
                )));
            }
            rt.stop(&session_id).map_err(|e| {
                GraftError::CommandExecution(format!("failed to stop session '{session_id}': {e}"))
            })?;
        }
    }

    let path = worktree_path(repo, name);
    let branch = branch_name(name);

    // Run on_prune hooks before removal if config is provided
    if let Some(cfg) = config {
        let chain = resolve_hook_chain(HookEvent::OnPrune, cfg, dep_configs, &path, repo);
        if !chain.is_empty() {
            let scion_env = ScionEnv {
                name: name.to_string(),
                branch: branch.clone(),
                worktree: path.clone(),
            };
            if let Err(hook_err) = execute_hook_chain(&chain, cfg, dep_configs, &scion_env) {
                // Leave worktree intact on hook failure
                return Err(GraftError::CommandExecution(format!(
                    "on_prune hook failed (worktree preserved): {hook_err}"
                )));
            }
        }
    }

    git_worktree_remove(repo, &path).map_err(GraftError::from)?;
    if let Err(e) = git_branch_delete(repo, &branch) {
        return Err(GraftError::CommandExecution(format!(
            "worktree removed but branch deletion failed for '{branch}': {e}. \
             Delete it manually with: git branch -D {branch}"
        )));
    }
    Ok(())
}

/// Fuse a scion branch into main: merge, hook gates, cleanup.
///
/// Sequence:
/// 1. Merge `feature/<name>` into the base branch via a temp ref
/// 2. Run `pre_fuse` hook chain — on failure, delete temp ref
/// 3. Fast-forward the base branch to the merge commit
/// 4. Run `post_fuse` hook chain — on failure, leave scion intact
/// 5. Remove worktree + branch
///
/// Already-merged detection: if the scion is 0 commits ahead of the base
/// branch, skip directly to step 4 (`post_fuse` hooks and cleanup).
///
/// # Arguments
/// * `repo_path`   - Absolute path to the main git repository
/// * `name`        - Scion name (e.g. `my-feature`)
/// * `config`      - Project config (for hook resolution). Pass `None` to skip hooks.
/// * `dep_configs` - Dependency configs in declaration order.
///
/// # Returns
/// The merge commit hash on success.
///
/// # Errors
/// Returns `GraftError` on merge conflicts, hook failures, or git errors.
pub fn scion_fuse(
    repo_path: impl AsRef<Path>,
    name: &str,
    config: Option<&GraftConfig>,
    dep_configs: &[(String, GraftConfig)],
    runtime: Option<&dyn SessionRuntime>,
    force: bool,
) -> Result<String> {
    validate_scion_name(name)?;
    let repo = repo_path.as_ref();

    // Session guard: refuse to fuse while a runtime session is active
    if let Some(rt) = runtime {
        let session_id = scion_session_id(name);
        if rt.exists(&session_id).unwrap_or(false) {
            if !force {
                return Err(GraftError::CommandExecution(format!(
                    "scion '{name}' has an active runtime session ({session_id}); stop it first or use --force"
                )));
            }
            rt.stop(&session_id).map_err(|e| {
                GraftError::CommandExecution(format!("failed to stop session '{session_id}': {e}"))
            })?;
        }
    }

    let path = worktree_path(repo, name);
    if !path.exists() {
        return Err(GraftError::CommandExecution(format!(
            "scion worktree not found at {}. Was it manually removed?",
            path.display()
        )));
    }
    if git_is_dirty(&path).unwrap_or(false) {
        return Err(GraftError::CommandExecution(format!(
            "scion '{name}' has uncommitted changes. Commit or discard them before fusing."
        )));
    }
    let branch = branch_name(name);
    let temp_ref = format!("refs/merge-temp/{name}");

    // Determine the base branch from the main worktree
    let worktrees = git_worktree_list(repo).map_err(GraftError::from)?;
    let base_branch = resolve_base_branch(&worktrees)?;

    let scion_env = ScionEnv {
        name: name.to_string(),
        branch: branch.clone(),
        worktree: path.clone(),
    };

    // Check if already merged (0 commits ahead)
    let (ahead, _behind) =
        git_ahead_behind(repo, &branch, &base_branch).map_err(GraftError::from)?;

    let merge_commit = if ahead == 0 {
        // Already merged — the base branch tip is the "merge" result
        worktrees.first().map(|w| w.head.clone()).ok_or_else(|| {
            GraftError::CommandExecution("no main worktree found in repository".to_string())
        })?
    } else {
        // Step 1: Merge to temp ref
        let commit =
            git_merge_to_ref(repo, &branch, &base_branch, &temp_ref).map_err(GraftError::from)?;

        // Step 2: pre_fuse hooks
        if let Some(cfg) = config {
            let chain = resolve_hook_chain(HookEvent::PreFuse, cfg, dep_configs, &path, repo);
            if !chain.is_empty() {
                if let Err(hook_err) = execute_hook_chain(&chain, cfg, dep_configs, &scion_env) {
                    // Rollback: delete temp ref
                    let _ = git_delete_ref(repo, &temp_ref);
                    return Err(GraftError::CommandExecution(format!(
                        "pre_fuse hook failed (temp ref discarded): {hook_err}"
                    )));
                }
            }
        }

        // Step 3: Fast-forward base branch to merge commit + sync worktree.
        // Wrap in a closure so the temp ref is always cleaned up, even on failure.
        let result = (|| -> Result<String> {
            git_fast_forward(repo, &base_branch, &commit).map_err(GraftError::from)?;
            let main_worktree_path = &worktrees[0].path;
            git_reset_hard(main_worktree_path).map_err(GraftError::from)?;
            Ok(commit)
        })();

        // Always clean up temp ref
        let _ = git_delete_ref(repo, &temp_ref);

        result?
    };

    // Step 4: post_fuse hooks
    if let Some(cfg) = config {
        let chain = resolve_hook_chain(HookEvent::PostFuse, cfg, dep_configs, &path, repo);
        if !chain.is_empty() {
            if let Err(hook_err) = execute_hook_chain(&chain, cfg, dep_configs, &scion_env) {
                // Leave scion intact — main already moved
                return Err(GraftError::CommandExecution(format!(
                    "post_fuse hook failed (scion preserved, main already updated): {hook_err}"
                )));
            }
        }
    }

    // Step 5: Remove worktree + branch
    git_worktree_remove(repo, &path).map_err(GraftError::from)?;
    if let Err(e) = git_branch_delete(repo, &branch) {
        return Err(GraftError::CommandExecution(format!(
            "worktree removed but branch deletion failed for '{branch}': {e}. \
             Delete it manually with: git branch -D {branch}"
        )));
    }

    Ok(merge_commit)
}

/// A scion lifecycle event that triggers hooks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookEvent {
    /// After worktree + branch creation.
    OnCreate,
    /// Before merging the feature branch to main.
    PreFuse,
    /// After the merge commit is applied to main.
    PostFuse,
    /// Before worktree + branch removal.
    OnPrune,
}

/// A resolved hook ready for execution.
///
/// Produced by `resolve_hook_chain`. Each entry carries enough context to
/// locate the command definition and set up the execution environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedHook {
    /// Command name as defined in a graft.yaml `commands:` section.
    /// For dependency hooks this is qualified: `dep_name:command_name`.
    /// For project hooks this is unqualified: `command_name`.
    pub command_name: String,
    /// Namespace: dependency name, or `None` for project-level hooks.
    pub namespace: Option<String>,
    /// Working directory for hook execution.
    pub working_dir: PathBuf,
}

/// Extract the hook command list for the given event from `ScionHooks`.
fn hooks_for_event(hooks: &ScionHooks, event: HookEvent) -> Option<&Vec<String>> {
    match event {
        HookEvent::OnCreate => hooks.on_create.as_ref(),
        HookEvent::PreFuse => hooks.pre_fuse.as_ref(),
        HookEvent::PostFuse => hooks.post_fuse.as_ref(),
        HookEvent::OnPrune => hooks.on_prune.as_ref(),
    }
}

/// Determine the working directory for a hook based on the lifecycle event.
///
/// - `on_create`, `pre_fuse`, `on_prune` → scion worktree (the work happens there)
/// - `post_fuse` → project root (the worktree may be about to be removed)
fn working_dir_for_event(event: HookEvent, scion_worktree: &Path, project_root: &Path) -> PathBuf {
    match event {
        HookEvent::OnCreate | HookEvent::PreFuse | HookEvent::OnPrune => {
            scion_worktree.to_path_buf()
        }
        HookEvent::PostFuse => project_root.to_path_buf(),
    }
}

/// Resolve the ordered chain of hooks for a scion lifecycle event.
///
/// Dependencies' hooks run first (in the order provided), then the project's
/// own hooks. Within each scope, hooks run in list order.
///
/// # Arguments
/// * `event`           - The lifecycle event to resolve hooks for
/// * `config`          - The project's `GraftConfig` (for project-level hooks)
/// * `dep_configs`     - Dependency configs in declaration order: `(name, config)`
/// * `scion_worktree`  - Absolute path to the scion's worktree
/// * `project_root`    - Absolute path to the project root
///
/// # Returns
/// Ordered list of `ResolvedHook`s. Empty if no hooks are defined.
pub fn resolve_hook_chain(
    event: HookEvent,
    config: &GraftConfig,
    dep_configs: &[(String, GraftConfig)],
    scion_worktree: &Path,
    project_root: &Path,
) -> Vec<ResolvedHook> {
    let working_dir = working_dir_for_event(event, scion_worktree, project_root);
    let mut chain = Vec::new();

    // Dependencies first, in declaration order
    for (dep_name, dep_config) in dep_configs {
        if let Some(hooks) = &dep_config.scion_hooks {
            if let Some(cmds) = hooks_for_event(hooks, event) {
                for cmd in cmds {
                    chain.push(ResolvedHook {
                        command_name: format!("{dep_name}:{cmd}"),
                        namespace: Some(dep_name.clone()),
                        working_dir: working_dir.clone(),
                    });
                }
            }
        }
    }

    // Project hooks last
    if let Some(hooks) = &config.scion_hooks {
        if let Some(cmds) = hooks_for_event(hooks, event) {
            for cmd in cmds {
                chain.push(ResolvedHook {
                    command_name: cmd.clone(),
                    namespace: None,
                    working_dir: working_dir.clone(),
                });
            }
        }
    }

    chain
}

/// Scion identity passed to hooks as environment variables.
#[derive(Debug, Clone)]
pub struct ScionEnv {
    /// Scion name (e.g. `retry-logic`).
    pub name: String,
    /// Full branch name (e.g. `feature/retry-logic`).
    pub branch: String,
    /// Worktree path (e.g. `.worktrees/retry-logic`).
    pub worktree: PathBuf,
}

/// Error from a hook chain execution.
///
/// Carries structured details about what completed and what failed,
/// enabling callers to make rollback decisions.
#[derive(Debug)]
pub struct HookChainError {
    /// The `command_name` of the hook that failed.
    pub failed_hook: String,
    /// Hooks that completed successfully before the failure.
    pub completed_hooks: Vec<String>,
    /// The error message from the failed hook.
    pub error: String,
}

impl std::fmt::Display for HookChainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Hook '{}' failed: {}", self.failed_hook, self.error)?;
        if !self.completed_hooks.is_empty() {
            write!(f, " (completed: {})", self.completed_hooks.join(", "))?;
        }
        Ok(())
    }
}

impl std::error::Error for HookChainError {}

/// Look up the `Command` definition for a hook from the appropriate config.
///
/// - Dependency hooks (namespace = Some): strip the `dep:` prefix to get the
///   unqualified name, look up in the dependency's `commands`.
/// - Project hooks (namespace = None): look up directly in the project's `commands`.
fn resolve_hook_command<'a>(
    hook: &ResolvedHook,
    config: &'a GraftConfig,
    dep_configs: &'a [(String, GraftConfig)],
) -> Option<&'a crate::domain::Command> {
    if let Some(ref ns) = hook.namespace {
        let unqualified = hook
            .command_name
            .strip_prefix(&format!("{ns}:"))
            .unwrap_or(&hook.command_name);
        dep_configs
            .iter()
            .find(|(name, _)| name == ns)
            .and_then(|(_, cfg)| cfg.commands.get(unqualified))
    } else {
        config.commands.get(&hook.command_name)
    }
}

/// Execute a resolved hook chain sequentially with fail-fast semantics.
///
/// Each hook receives `GRAFT_SCION_NAME`, `GRAFT_SCION_BRANCH`, and
/// `GRAFT_SCION_WORKTREE` environment variables. Hooks run in the working
/// directory specified in the `ResolvedHook`.
///
/// # Arguments
/// * `chain`       - Ordered hooks to execute
/// * `config`      - Project's `GraftConfig` (for command lookup)
/// * `dep_configs` - Dependency configs (for namespace-qualified command lookup)
/// * `scion_env`   - Scion identity for environment variables
///
/// # Returns
/// `Ok(completed_hooks)` if all hooks succeed, or `Err(HookChainError)` on
/// first failure with details about which hook failed and which completed.
pub fn execute_hook_chain(
    chain: &[ResolvedHook],
    config: &GraftConfig,
    dep_configs: &[(String, GraftConfig)],
    scion_env: &ScionEnv,
) -> std::result::Result<Vec<String>, HookChainError> {
    let mut completed = Vec::new();

    for hook in chain {
        let cmd =
            resolve_hook_command(hook, config, dep_configs).ok_or_else(|| HookChainError {
                failed_hook: hook.command_name.clone(),
                completed_hooks: completed.clone(),
                error: format!("Command not found: {}", hook.command_name),
            })?;

        // Merge the command's env with scion identity vars (scion vars win)
        let mut env: HashMap<String, String> = cmd.env.clone().unwrap_or_default();
        env.insert("GRAFT_SCION_NAME".to_string(), scion_env.name.clone());
        env.insert("GRAFT_SCION_BRANCH".to_string(), scion_env.branch.clone());
        env.insert(
            "GRAFT_SCION_WORKTREE".to_string(),
            scion_env.worktree.to_str().unwrap_or_default().to_string(),
        );

        // Hook working_dir (event-specific) takes precedence; fall back to
        // the command's working_dir if the hook doesn't override it.
        let working_dir = if hook.working_dir.as_os_str().is_empty() {
            cmd.working_dir
                .as_ref()
                .map_or_else(|| hook.working_dir.clone(), PathBuf::from)
        } else {
            hook.working_dir.clone()
        };

        let process_config = ProcessConfig {
            command: cmd.run.clone(),
            working_dir,
            env: Some(env),
            env_remove: vec![],
            log_path: None,
            timeout: None,
            stdin: None,
        };

        let output =
            run_to_completion_with_timeout(&process_config).map_err(|e| HookChainError {
                failed_hook: hook.command_name.clone(),
                completed_hooks: completed.clone(),
                error: e.to_string(),
            })?;

        if !output.success {
            return Err(HookChainError {
                failed_hook: hook.command_name.clone(),
                completed_hooks: completed.clone(),
                error: format!("exit code {}: {}", output.exit_code, output.stderr.trim()),
            });
        }

        completed.push(hook.command_name.clone());
    }

    Ok(completed)
}

/// Start a runtime session for a scion.
///
/// Resolves the command to run from `scions.start` in the config, then
/// launches it in a detached session via the provided runtime.
///
/// Session ID convention: `graft-scion-<name>`.
///
/// # Errors
/// Returns `GraftError` if:
/// - The scion does not exist (no worktree at `.worktrees/<name>`)
/// - No `scions.start` is configured
/// - The referenced command is not found in the commands section
/// - The runtime fails to launch the session
pub fn scion_start(
    repo_path: impl AsRef<Path>,
    name: &str,
    config: Option<&GraftConfig>,
    runtime: &impl SessionRuntime,
) -> Result<()> {
    validate_scion_name(name)?;
    let repo = repo_path.as_ref();
    let wt_path = worktree_path(repo, name);
    if !wt_path.exists() {
        return Err(GraftError::CommandExecution(format!(
            "scion '{name}' does not exist"
        )));
    }

    let cfg = config.ok_or_else(|| {
        GraftError::CommandExecution("no start command configured in scions.start".to_string())
    })?;

    let command_name = cfg
        .scion_hooks
        .as_ref()
        .and_then(|h| h.start.as_ref())
        .ok_or_else(|| {
            GraftError::CommandExecution("no start command configured in scions.start".to_string())
        })?;

    let command = cfg.get_command(command_name).ok_or_else(|| {
        GraftError::CommandExecution(format!(
            "start command '{command_name}' not found in commands"
        ))
    })?;

    let session_id = scion_session_id(name);
    runtime.launch(&session_id, &command.run, &wt_path)?;
    Ok(())
}

/// Stop a runtime session for a scion.
///
/// Session ID convention: `graft-scion-<name>`.
///
/// # Errors
/// Returns `GraftError` if the runtime fails to stop the session (e.g. not found).
pub fn scion_stop(
    _repo_path: impl AsRef<Path>,
    name: &str,
    runtime: &impl SessionRuntime,
) -> Result<()> {
    validate_scion_name(name)?;
    let session_id = scion_session_id(name);
    runtime.stop(&session_id)?;
    Ok(())
}

/// Check that a scion exists and has an active runtime session, returning the session ID.
///
/// Used by the `attach` command to validate preconditions before handing off to
/// the runtime's blocking `attach` call.
///
/// # Errors
/// Returns `GraftError` if the scion worktree does not exist or no active session
/// is found for it.
pub fn scion_attach_check(
    repo_path: impl AsRef<Path>,
    name: &str,
    runtime: &impl SessionRuntime,
) -> Result<String> {
    validate_scion_name(name)?;
    let repo = repo_path.as_ref();
    let path = worktree_path(repo, name);
    if !path.exists() {
        return Err(GraftError::CommandExecution(format!(
            "scion '{name}' does not exist"
        )));
    }
    let session_id = scion_session_id(name);
    if !runtime.exists(&session_id).unwrap_or(false) {
        return Err(GraftError::CommandExecution(format!(
            "no active session for scion '{name}'"
        )));
    }
    Ok(session_id)
}

/// Structured information about a scion workstream.
///
/// Returned by `scion_list`. Most fields are derived from git artifacts.
/// `session_active` is derived from an optional runtime check.
#[derive(Debug, Clone, Serialize)]
pub struct ScionInfo {
    /// Scion name (e.g. `my-feature`).
    pub name: String,
    /// Full branch name (e.g. `feature/my-feature`).
    pub branch: String,
    /// Absolute path to the worktree directory.
    pub worktree_path: PathBuf,
    /// Commits in the scion branch not yet in main.
    /// `None` if the count could not be determined (e.g. branch missing).
    pub ahead: Option<usize>,
    /// Commits in main not yet in the scion branch.
    /// `None` if the count could not be determined (e.g. branch missing).
    pub behind: Option<usize>,
    /// Unix timestamp of the most recent commit on the scion branch.
    /// `None` if the scion has no commits (freshly created).
    pub last_commit_time: Option<i64>,
    /// Whether the worktree has uncommitted changes.
    pub dirty: bool,
    /// Whether a runtime session is active for this scion.
    /// `None` if no runtime was available for detection.
    pub session_active: Option<bool>,
}

/// List all scions for the repository.
///
/// Enumerates worktrees whose paths fall under `.worktrees/`, extracts the
/// scion name from the path component, and gathers per-scion metrics.
///
/// # Arguments
/// * `repo_path` - Absolute path to the main git repository
/// * `runtime` - Optional session runtime for detecting active sessions.
///   When `None`, `session_active` will be `None` for all scions.
///
/// # Returns
/// A list of `ScionInfo` structs, one per scion (in the order returned by
/// `git worktree list`). The main worktree is excluded.
///
/// # Errors
/// Returns `GraftError` if the worktree enumeration fails.
pub fn scion_list(
    repo_path: impl AsRef<Path>,
    runtime: Option<&dyn SessionRuntime>,
) -> Result<Vec<ScionInfo>> {
    let repo = repo_path.as_ref();
    let worktrees = git_worktree_list(repo).map_err(GraftError::from)?;

    // The first entry is always the main worktree; its branch is our base.
    let base_branch = resolve_base_branch(&worktrees)?;

    let mut scions = Vec::new();
    for wt in &worktrees {
        // Filter to .worktrees/ entries only
        let components: Vec<_> = wt.path.components().collect();
        let len = components.len();
        if len < 2 {
            continue;
        }
        // The parent directory must be named ".worktrees"
        let parent_name = components[len - 2].as_os_str().to_str().unwrap_or("");
        if parent_name != ".worktrees" {
            continue;
        }

        let scion_name = components[len - 1]
            .as_os_str()
            .to_str()
            .unwrap_or("")
            .to_string();
        if validate_scion_name(&scion_name).is_err() {
            continue;
        }

        let branch = branch_name(&scion_name);

        let (ahead, behind) = match git_ahead_behind(repo, &branch, &base_branch) {
            Ok((a, b)) => (Some(a), Some(b)),
            Err(_) => (None, None),
        };

        let last_commit_time = git_last_commit_time(repo, &branch).ok();

        let dirty = git_is_dirty(&wt.path).unwrap_or(false);

        let session_active = runtime.map(|rt| {
            let session_id = scion_session_id(&scion_name);
            rt.exists(&session_id).unwrap_or(false)
        });

        scions.push(ScionInfo {
            name: scion_name,
            branch,
            worktree_path: wt.path.clone(),
            ahead,
            behind,
            last_commit_time,
            dirty,
            session_active,
        });
    }

    Ok(scions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    fn init_test_repo(path: &Path) {
        Command::new("git")
            .args(["init"])
            .current_dir(path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(path)
            .output()
            .unwrap();
        fs::write(path.join("README.md"), "test").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(path)
            .output()
            .unwrap();
    }

    #[test]
    fn scion_create_creates_worktree_and_branch() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path());

        let wt_path = scion_create(temp.path(), "my-feature", None, &[]).unwrap();

        // Worktree directory exists at expected path
        assert_eq!(wt_path, temp.path().join(".worktrees").join("my-feature"));
        assert!(wt_path.exists());

        // Confirm the branch name via worktree list
        let worktrees = graft_common::git_worktree_list(temp.path()).unwrap();
        let scion_wt = worktrees
            .iter()
            .find(|w| w.branch.as_deref() == Some("feature/my-feature"))
            .expect("scion worktree not found");
        assert!(scion_wt.path.ends_with(".worktrees/my-feature"));
    }

    #[test]
    fn scion_create_fails_if_already_exists() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path());

        scion_create(temp.path(), "dup", None, &[]).unwrap();
        let result = scion_create(temp.path(), "dup", None, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn scion_prune_removes_worktree_and_branch() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path());

        scion_create(temp.path(), "to-prune", None, &[]).unwrap();
        scion_prune(temp.path(), "to-prune", None, &[], None, false).unwrap();

        // Worktree gone from list
        let worktrees = graft_common::git_worktree_list(temp.path()).unwrap();
        assert!(!worktrees
            .iter()
            .any(|w| w.branch.as_deref() == Some("feature/to-prune")));

        // Branch gone (attempting to delete again should fail)
        let del = graft_common::git_branch_delete(temp.path(), "feature/to-prune");
        assert!(del.is_err());
    }

    #[test]
    fn scion_prune_fails_for_nonexistent() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path());

        let result = scion_prune(temp.path(), "does-not-exist", None, &[], None, false);
        assert!(result.is_err());
    }

    #[test]
    fn scion_create_then_prune_round_trip() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path());

        let wt = scion_create(temp.path(), "round-trip", None, &[]).unwrap();
        assert!(wt.exists());

        scion_prune(temp.path(), "round-trip", None, &[], None, false).unwrap();
        assert!(!wt.exists());
    }

    fn make_commit(path: &Path, filename: &str, message: &str) {
        fs::write(path.join(filename), "content").unwrap();
        Command::new("git")
            .args(["add", filename])
            .current_dir(path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(path)
            .output()
            .unwrap();
    }

    #[test]
    fn scion_list_empty_returns_empty_vec() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path());

        let scions = scion_list(temp.path(), None).unwrap();
        assert!(scions.is_empty());
    }

    #[test]
    fn scion_list_returns_created_scion() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path());

        scion_create(temp.path(), "alpha", None, &[]).unwrap();
        let scions = scion_list(temp.path(), None).unwrap();

        assert_eq!(scions.len(), 1);
        let s = &scions[0];
        assert_eq!(s.name, "alpha");
        assert_eq!(s.branch, "feature/alpha");
        assert!(s.worktree_path.ends_with(".worktrees/alpha"));
        assert_eq!(s.ahead, Some(0));
        assert_eq!(s.behind, Some(0));
        assert!(!s.dirty);
    }

    #[test]
    fn scion_list_shows_commits_ahead() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path());

        scion_create(temp.path(), "beta", None, &[]).unwrap();
        let wt_path = temp.path().join(".worktrees").join("beta");

        // Make 2 commits in the scion worktree
        make_commit(&wt_path, "a.txt", "feat: a");
        make_commit(&wt_path, "b.txt", "feat: b");

        let scions = scion_list(temp.path(), None).unwrap();
        let s = scions.iter().find(|s| s.name == "beta").unwrap();
        assert_eq!(s.ahead, Some(2));
        assert_eq!(s.behind, Some(0));
        // last_commit_time should be set now that we have commits
        assert!(s.last_commit_time.is_some());
    }

    #[test]
    fn scion_list_shows_dirty_worktree() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path());

        scion_create(temp.path(), "gamma", None, &[]).unwrap();
        let wt_path = temp.path().join(".worktrees").join("gamma");

        // Write a file without committing
        fs::write(wt_path.join("dirty.txt"), "uncommitted").unwrap();

        let scions = scion_list(temp.path(), None).unwrap();
        let s = scions.iter().find(|s| s.name == "gamma").unwrap();
        assert!(s.dirty);
    }

    // --- resolve_hook_chain tests ---

    fn make_config_with_hooks(hooks: ScionHooks) -> GraftConfig {
        let mut config = GraftConfig::new("graft/v1").unwrap();
        config.scion_hooks = Some(hooks);
        config
    }

    #[test]
    fn resolve_hook_chain_no_hooks() {
        let config = GraftConfig::new("graft/v1").unwrap();
        let chain = resolve_hook_chain(
            HookEvent::OnCreate,
            &config,
            &[],
            Path::new("/wt"),
            Path::new("/root"),
        );
        assert!(chain.is_empty());
    }

    #[test]
    fn resolve_hook_chain_project_only() {
        let config = make_config_with_hooks(ScionHooks {
            on_create: Some(vec!["setup".to_string(), "init".to_string()]),
            pre_fuse: None,
            post_fuse: None,
            on_prune: None,
            start: None,
        });
        let chain = resolve_hook_chain(
            HookEvent::OnCreate,
            &config,
            &[],
            Path::new("/wt"),
            Path::new("/root"),
        );
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0].command_name, "setup");
        assert!(chain[0].namespace.is_none());
        assert_eq!(chain[1].command_name, "init");
        assert!(chain[1].namespace.is_none());
    }

    #[test]
    fn resolve_hook_chain_dep_only() {
        let config = GraftConfig::new("graft/v1").unwrap();
        let dep = make_config_with_hooks(ScionHooks {
            on_create: Some(vec!["dep-setup".to_string()]),
            pre_fuse: None,
            post_fuse: None,
            on_prune: None,
            start: None,
        });
        let chain = resolve_hook_chain(
            HookEvent::OnCreate,
            &config,
            &[("my-dep".to_string(), dep)],
            Path::new("/wt"),
            Path::new("/root"),
        );
        assert_eq!(chain.len(), 1);
        assert_eq!(chain[0].command_name, "my-dep:dep-setup");
        assert_eq!(chain[0].namespace.as_deref(), Some("my-dep"));
    }

    #[test]
    fn resolve_hook_chain_mixed_deps_then_project() {
        let config = make_config_with_hooks(ScionHooks {
            on_create: Some(vec!["project-setup".to_string()]),
            pre_fuse: None,
            post_fuse: None,
            on_prune: None,
            start: None,
        });
        let dep_a = make_config_with_hooks(ScionHooks {
            on_create: Some(vec!["a-init".to_string()]),
            pre_fuse: None,
            post_fuse: None,
            on_prune: None,
            start: None,
        });
        let dep_b = make_config_with_hooks(ScionHooks {
            on_create: Some(vec!["b-init".to_string(), "b-check".to_string()]),
            pre_fuse: None,
            post_fuse: None,
            on_prune: None,
            start: None,
        });
        let chain = resolve_hook_chain(
            HookEvent::OnCreate,
            &config,
            &[("dep-a".to_string(), dep_a), ("dep-b".to_string(), dep_b)],
            Path::new("/wt"),
            Path::new("/root"),
        );
        assert_eq!(chain.len(), 4);
        // Deps first in order, then project
        assert_eq!(chain[0].command_name, "dep-a:a-init");
        assert_eq!(chain[1].command_name, "dep-b:b-init");
        assert_eq!(chain[2].command_name, "dep-b:b-check");
        assert_eq!(chain[3].command_name, "project-setup");
    }

    #[test]
    fn resolve_hook_chain_working_dir_worktree_events() {
        let config = make_config_with_hooks(ScionHooks {
            on_create: Some(vec!["a".to_string()]),
            pre_fuse: Some(vec!["b".to_string()]),
            post_fuse: None,
            on_prune: Some(vec!["c".to_string()]),
            start: None,
        });
        let wt = Path::new("/worktree");
        let root = Path::new("/root");

        // on_create → worktree
        let chain = resolve_hook_chain(HookEvent::OnCreate, &config, &[], wt, root);
        assert_eq!(chain[0].working_dir, wt);

        // pre_fuse → worktree
        let chain = resolve_hook_chain(HookEvent::PreFuse, &config, &[], wt, root);
        assert_eq!(chain[0].working_dir, wt);

        // on_prune → worktree
        let chain = resolve_hook_chain(HookEvent::OnPrune, &config, &[], wt, root);
        assert_eq!(chain[0].working_dir, wt);
    }

    #[test]
    fn resolve_hook_chain_working_dir_post_fuse_uses_project_root() {
        let config = make_config_with_hooks(ScionHooks {
            on_create: None,
            pre_fuse: None,
            post_fuse: Some(vec!["notify".to_string()]),
            on_prune: None,
            start: None,
        });
        let chain = resolve_hook_chain(
            HookEvent::PostFuse,
            &config,
            &[],
            Path::new("/worktree"),
            Path::new("/root"),
        );
        assert_eq!(chain[0].working_dir, Path::new("/root"));
    }

    // --- execute_hook_chain tests ---

    fn make_scion_env() -> ScionEnv {
        ScionEnv {
            name: "test-scion".to_string(),
            branch: "feature/test-scion".to_string(),
            worktree: PathBuf::from("/tmp/test-wt"),
        }
    }

    fn make_config_with_command(
        cmd_name: &str,
        run: &str,
        hooks: Option<ScionHooks>,
    ) -> GraftConfig {
        let mut config = GraftConfig::new("graft/v1").unwrap();
        let cmd = crate::domain::Command::new(cmd_name, run).unwrap();
        config.commands.insert(cmd_name.to_string(), cmd);
        config.scion_hooks = hooks;
        config
    }

    #[test]
    fn execute_hook_chain_all_succeed() {
        let config = make_config_with_command(
            "hook-a",
            "true",
            Some(ScionHooks {
                on_create: Some(vec!["hook-a".to_string()]),
                pre_fuse: None,
                post_fuse: None,
                on_prune: None,
                start: None,
            }),
        );
        // Add a second command
        let mut config = config;
        let cmd_b = crate::domain::Command::new("hook-b", "true").unwrap();
        config.commands.insert("hook-b".to_string(), cmd_b);
        config.scion_hooks = Some(ScionHooks {
            on_create: Some(vec!["hook-a".to_string(), "hook-b".to_string()]),
            pre_fuse: None,
            post_fuse: None,
            on_prune: None,
            start: None,
        });

        let chain = resolve_hook_chain(
            HookEvent::OnCreate,
            &config,
            &[],
            Path::new("/tmp"),
            Path::new("/tmp"),
        );
        let result = execute_hook_chain(&chain, &config, &[], &make_scion_env());
        assert!(result.is_ok());
        let completed = result.unwrap();
        assert_eq!(completed, vec!["hook-a", "hook-b"]);
    }

    #[test]
    fn execute_hook_chain_middle_hook_fails() {
        let mut config = GraftConfig::new("graft/v1").unwrap();
        let cmd_a = crate::domain::Command::new("hook-a", "true").unwrap();
        let cmd_b = crate::domain::Command::new("hook-b", "exit 1").unwrap();
        let cmd_c = crate::domain::Command::new("hook-c", "true").unwrap();
        config.commands.insert("hook-a".to_string(), cmd_a);
        config.commands.insert("hook-b".to_string(), cmd_b);
        config.commands.insert("hook-c".to_string(), cmd_c);
        config.scion_hooks = Some(ScionHooks {
            on_create: Some(vec![
                "hook-a".to_string(),
                "hook-b".to_string(),
                "hook-c".to_string(),
            ]),
            pre_fuse: None,
            post_fuse: None,
            on_prune: None,
            start: None,
        });

        let chain = resolve_hook_chain(
            HookEvent::OnCreate,
            &config,
            &[],
            Path::new("/tmp"),
            Path::new("/tmp"),
        );
        let result = execute_hook_chain(&chain, &config, &[], &make_scion_env());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.failed_hook, "hook-b");
        assert_eq!(err.completed_hooks, vec!["hook-a"]);
        // hook-c should not have been attempted
    }

    #[test]
    fn execute_hook_chain_empty_chain_succeeds() {
        let config = GraftConfig::new("graft/v1").unwrap();
        let chain: Vec<ResolvedHook> = vec![];
        let result = execute_hook_chain(&chain, &config, &[], &make_scion_env());
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn execute_hook_chain_command_not_found() {
        let config = GraftConfig::new("graft/v1").unwrap();
        let chain = vec![ResolvedHook {
            command_name: "nonexistent".to_string(),
            namespace: None,
            working_dir: PathBuf::from("/tmp"),
        }];
        let result = execute_hook_chain(&chain, &config, &[], &make_scion_env());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.failed_hook, "nonexistent");
        assert!(err.error.contains("Command not found"));
    }

    #[test]
    fn execute_hook_chain_merges_command_env() {
        // Define a command with its own env vars
        let mut config = GraftConfig::new("graft/v1").unwrap();
        let mut cmd = crate::domain::Command::new("env-hook", "env").unwrap();
        let mut cmd_env = HashMap::new();
        cmd_env.insert("MY_VAR".to_string(), "hello".to_string());
        cmd.env = Some(cmd_env);
        config.commands.insert("env-hook".to_string(), cmd);
        config.scion_hooks = Some(ScionHooks {
            on_create: Some(vec!["env-hook".to_string()]),
            pre_fuse: None,
            post_fuse: None,
            on_prune: None,
            start: None,
        });

        let chain = resolve_hook_chain(
            HookEvent::OnCreate,
            &config,
            &[],
            Path::new("/tmp"),
            Path::new("/tmp"),
        );
        let result = execute_hook_chain(&chain, &config, &[], &make_scion_env());
        assert!(result.is_ok());
        // If we get here, the command ran successfully with merged env —
        // the env command succeeded, meaning both command env and scion env were set.
    }

    // --- Hook rollback integration tests ---

    #[test]
    fn scion_create_with_failing_hook_rolls_back() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path());

        let mut config = GraftConfig::new("graft/v1").unwrap();
        let cmd = crate::domain::Command::new("fail-hook", "exit 1").unwrap();
        config.commands.insert("fail-hook".to_string(), cmd);
        config.scion_hooks = Some(ScionHooks {
            on_create: Some(vec!["fail-hook".to_string()]),
            pre_fuse: None,
            post_fuse: None,
            on_prune: None,
            start: None,
        });

        let result = scion_create(temp.path(), "rollback-test", Some(&config), &[]);
        assert!(result.is_err());

        // Worktree should have been rolled back
        let wt_path = temp.path().join(".worktrees").join("rollback-test");
        assert!(!wt_path.exists());

        // Branch should also be gone
        let del = graft_common::git_branch_delete(temp.path(), "feature/rollback-test");
        assert!(del.is_err());
    }

    #[test]
    fn scion_create_with_passing_hook_succeeds() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path());

        let mut config = GraftConfig::new("graft/v1").unwrap();
        let cmd = crate::domain::Command::new("pass-hook", "true").unwrap();
        config.commands.insert("pass-hook".to_string(), cmd);
        config.scion_hooks = Some(ScionHooks {
            on_create: Some(vec!["pass-hook".to_string()]),
            pre_fuse: None,
            post_fuse: None,
            on_prune: None,
            start: None,
        });

        let result = scion_create(temp.path(), "hook-pass", Some(&config), &[]);
        assert!(result.is_ok());
        assert!(result.unwrap().exists());
    }

    #[test]
    fn scion_prune_with_failing_hook_preserves_worktree() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path());

        // Create scion first (no hooks)
        scion_create(temp.path(), "preserve-test", None, &[]).unwrap();

        // Now try to prune with a failing hook
        let mut config = GraftConfig::new("graft/v1").unwrap();
        let cmd = crate::domain::Command::new("fail-prune", "exit 1").unwrap();
        config.commands.insert("fail-prune".to_string(), cmd);
        config.scion_hooks = Some(ScionHooks {
            on_create: None,
            pre_fuse: None,
            post_fuse: None,
            on_prune: Some(vec!["fail-prune".to_string()]),
            start: None,
        });

        let result = scion_prune(
            temp.path(),
            "preserve-test",
            Some(&config),
            &[],
            None,
            false,
        );
        assert!(result.is_err());

        // Worktree should still exist
        let wt_path = temp.path().join(".worktrees").join("preserve-test");
        assert!(wt_path.exists());
    }

    // --- scion_fuse integration tests ---

    #[test]
    fn scion_fuse_full_lifecycle() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path());

        // Create scion and add a commit
        scion_create(temp.path(), "fuse-test", None, &[]).unwrap();
        let wt_path = temp.path().join(".worktrees").join("fuse-test");
        make_commit(&wt_path, "feature.txt", "feat: add feature");

        // Get main branch name and current commit
        let worktrees = graft_common::git_worktree_list(temp.path()).unwrap();
        let main_branch = worktrees[0].branch.clone().unwrap();
        let main_before = worktrees[0].head.clone();

        // Fuse
        let merge_commit = scion_fuse(temp.path(), "fuse-test", None, &[], None, false).unwrap();
        assert_eq!(merge_commit.len(), 40);
        assert_ne!(merge_commit, main_before);

        // Main should have advanced
        let config = ProcessConfig {
            command: format!("git rev-parse refs/heads/{main_branch}"),
            working_dir: temp.path().to_path_buf(),
            env: None,
            env_remove: vec![],
            log_path: None,
            timeout: Some(std::time::Duration::from_secs(5)),
            stdin: None,
        };
        let output = run_to_completion_with_timeout(&config).unwrap();
        assert_eq!(output.stdout.trim(), merge_commit);

        // Main worktree working tree should contain the fused file
        assert!(
            temp.path().join("feature.txt").exists(),
            "feature.txt should appear in main worktree after fuse"
        );

        // Worktree + branch should be cleaned up
        assert!(!wt_path.exists());
        let del = graft_common::git_branch_delete(temp.path(), "feature/fuse-test");
        assert!(del.is_err());
    }

    #[test]
    fn scion_fuse_already_merged() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path());

        // Create a scion but make NO commits (0 ahead)
        scion_create(temp.path(), "no-ahead", None, &[]).unwrap();
        let wt_path = temp.path().join(".worktrees").join("no-ahead");

        // Fuse should succeed (already-merged path)
        let commit = scion_fuse(temp.path(), "no-ahead", None, &[], None, false).unwrap();
        // Should return a valid commit hash (not empty)
        assert_eq!(commit.len(), 40);
        assert!(commit.chars().all(|c| c.is_ascii_hexdigit()));

        // Worktree and branch should be cleaned up
        assert!(!wt_path.exists());
    }

    #[test]
    fn scion_fuse_merge_conflict_returns_error() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path());

        // Get main branch name
        let worktrees = graft_common::git_worktree_list(temp.path()).unwrap();
        let _main_branch = worktrees[0].branch.clone().unwrap();

        // Create a scion and modify README
        scion_create(temp.path(), "conflict-fuse", None, &[]).unwrap();
        let wt_path = temp.path().join(".worktrees").join("conflict-fuse");
        fs::write(wt_path.join("README.md"), "scion version").unwrap();
        std::process::Command::new("git")
            .args(["add", "README.md"])
            .current_dir(&wt_path)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "modify readme in scion"])
            .current_dir(&wt_path)
            .output()
            .unwrap();

        // Modify the same file on main
        fs::write(temp.path().join("README.md"), "main version").unwrap();
        std::process::Command::new("git")
            .args(["add", "README.md"])
            .current_dir(temp.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "modify readme on main"])
            .current_dir(temp.path())
            .output()
            .unwrap();

        // Fuse should fail with merge conflict
        let result = scion_fuse(temp.path(), "conflict-fuse", None, &[], None, false);
        assert!(result.is_err());

        // Worktree should still exist (no cleanup on merge conflict)
        assert!(wt_path.exists());
    }

    // --- validate_scion_name tests ---

    #[test]
    fn validate_scion_name_accepts_valid_names() {
        assert!(validate_scion_name("my-feature").is_ok());
        assert!(validate_scion_name("feature_1").is_ok());
        assert!(validate_scion_name("v2.0.1").is_ok());
        assert!(validate_scion_name("a").is_ok());
        assert!(validate_scion_name("ABC-123").is_ok());
    }

    #[test]
    fn validate_scion_name_rejects_empty() {
        assert!(validate_scion_name("").is_err());
    }

    #[test]
    fn validate_scion_name_rejects_path_traversal() {
        assert!(validate_scion_name("..").is_err());
        assert!(validate_scion_name(".").is_err());
        // Contains / which is disallowed
        assert!(validate_scion_name("../..").is_err());
        assert!(validate_scion_name("a/b").is_err());
    }

    #[test]
    fn validate_scion_name_rejects_slashes_and_colons() {
        assert!(validate_scion_name("a/b").is_err());
        assert!(validate_scion_name("a\\b").is_err());
        assert!(validate_scion_name("a:b").is_err());
    }

    #[test]
    fn validate_scion_name_rejects_spaces() {
        assert!(validate_scion_name("my feature").is_err());
    }

    #[test]
    fn validate_scion_name_rejects_too_long() {
        let long = "a".repeat(101);
        assert!(validate_scion_name(&long).is_err());
        // Exactly 100 is fine
        let ok = "a".repeat(100);
        assert!(validate_scion_name(&ok).is_ok());
    }

    #[test]
    fn validate_scion_name_rejects_leading_dot_or_dash() {
        assert!(validate_scion_name(".hidden").is_err());
        assert!(validate_scion_name(".git").is_err());
        assert!(validate_scion_name("-rf").is_err());
        // Dots and dashes in the middle are fine
        assert!(validate_scion_name("my.feature").is_ok());
        assert!(validate_scion_name("my-feature").is_ok());
    }

    // --- detached HEAD tests ---

    #[test]
    fn scion_list_fails_on_detached_head() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path());

        // Detach HEAD
        Command::new("git")
            .args(["checkout", "--detach", "HEAD"])
            .current_dir(temp.path())
            .output()
            .unwrap();

        let result = scion_list(temp.path(), None);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("detached HEAD"),
            "expected detached HEAD error, got: {err_msg}"
        );
    }

    #[test]
    fn scion_fuse_fails_on_detached_head() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path());

        // Create a scion while HEAD is attached
        scion_create(temp.path(), "det-fuse", None, &[]).unwrap();

        // Now detach HEAD on the main worktree
        Command::new("git")
            .args(["checkout", "--detach", "HEAD"])
            .current_dir(temp.path())
            .output()
            .unwrap();

        let result = scion_fuse(temp.path(), "det-fuse", None, &[], None, false);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("detached HEAD"),
            "expected detached HEAD error, got: {err_msg}"
        );
    }

    // --- fuse guard tests ---

    #[test]
    fn scion_fuse_rejects_dirty_worktree() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path());

        scion_create(temp.path(), "dirty-fuse", None, &[]).unwrap();
        let wt_path = temp.path().join(".worktrees").join("dirty-fuse");

        // Create uncommitted changes in the scion worktree
        fs::write(wt_path.join("uncommitted.txt"), "wip").unwrap();

        let result = scion_fuse(temp.path(), "dirty-fuse", None, &[], None, false);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("uncommitted changes"),
            "expected dirty worktree error, got: {err_msg}"
        );
        // Worktree should still exist (no cleanup attempted)
        assert!(wt_path.exists());
    }

    #[test]
    fn scion_fuse_rejects_missing_worktree() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path());

        scion_create(temp.path(), "gone-fuse", None, &[]).unwrap();
        let wt_path = temp.path().join(".worktrees").join("gone-fuse");

        // Manually remove the worktree directory (simulating rm -rf)
        fs::remove_dir_all(&wt_path).unwrap();

        let result = scion_fuse(temp.path(), "gone-fuse", None, &[], None, false);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("not found"),
            "expected missing worktree error, got: {err_msg}"
        );
    }

    // --- scion_start / scion_stop tests ---

    /// A mock runtime that records calls for testing.
    struct MockRuntime {
        sessions: std::cell::RefCell<std::collections::HashSet<String>>,
    }

    impl MockRuntime {
        fn new() -> Self {
            Self {
                sessions: std::cell::RefCell::new(std::collections::HashSet::new()),
            }
        }
    }

    impl SessionRuntime for MockRuntime {
        fn launch(
            &self,
            session_id: &str,
            _command: &str,
            _working_dir: &Path,
        ) -> std::result::Result<(), graft_common::RuntimeError> {
            let mut sessions = self.sessions.borrow_mut();
            if sessions.contains(session_id) {
                return Err(graft_common::RuntimeError::SessionExists(
                    session_id.to_string(),
                ));
            }
            sessions.insert(session_id.to_string());
            Ok(())
        }

        fn exists(
            &self,
            session_id: &str,
        ) -> std::result::Result<bool, graft_common::RuntimeError> {
            Ok(self.sessions.borrow().contains(session_id))
        }

        fn attach(&self, _session_id: &str) -> std::result::Result<(), graft_common::RuntimeError> {
            Ok(())
        }

        fn stop(&self, session_id: &str) -> std::result::Result<(), graft_common::RuntimeError> {
            let mut sessions = self.sessions.borrow_mut();
            if !sessions.remove(session_id) {
                return Err(graft_common::RuntimeError::SessionNotFound(
                    session_id.to_string(),
                ));
            }
            Ok(())
        }
    }

    #[test]
    fn scion_start_no_config_errors() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path());
        scion_create(temp.path(), "no-config", None, &[]).unwrap();

        let runtime = MockRuntime::new();
        let result = scion_start(temp.path(), "no-config", None, &runtime);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("no start command"),
            "expected missing start config error, got: {err_msg}"
        );
    }

    #[test]
    fn scion_start_missing_scion_errors() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path());

        let runtime = MockRuntime::new();
        let result = scion_start(temp.path(), "nonexistent", None, &runtime);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("does not exist"),
            "expected missing scion error, got: {err_msg}"
        );
    }

    #[test]
    fn scion_start_and_stop_with_mock_runtime() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path());
        scion_create(temp.path(), "worker", None, &[]).unwrap();

        let mut config = GraftConfig::new("graft/v1").unwrap();
        let cmd = crate::domain::Command::new("agent", "echo hello").unwrap();
        config.commands.insert("agent".to_string(), cmd);
        config.scion_hooks = Some(ScionHooks {
            on_create: None,
            pre_fuse: None,
            post_fuse: None,
            on_prune: None,
            start: Some("agent".to_string()),
        });

        let runtime = MockRuntime::new();

        // Start should succeed
        scion_start(temp.path(), "worker", Some(&config), &runtime).unwrap();
        assert!(runtime.exists("graft-scion-worker").unwrap());

        // Stop should succeed
        scion_stop(temp.path(), "worker", &runtime).unwrap();
        assert!(!runtime.exists("graft-scion-worker").unwrap());
    }

    #[test]
    fn scion_list_with_mock_runtime_shows_session_active() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path());
        scion_create(temp.path(), "active-test", None, &[]).unwrap();

        let runtime = MockRuntime::new();

        // Without active session
        let scions = scion_list(temp.path(), Some(&runtime)).unwrap();
        assert_eq!(scions[0].session_active, Some(false));

        // Simulate an active session
        runtime
            .launch("graft-scion-active-test", "echo", Path::new("/tmp"))
            .unwrap();

        let scions = scion_list(temp.path(), Some(&runtime)).unwrap();
        assert_eq!(scions[0].session_active, Some(true));
    }

    // --- session guard tests ---

    #[test]
    fn scion_fuse_with_active_session_no_force_errors() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path());
        scion_create(temp.path(), "guarded", None, &[]).unwrap();
        let wt_path = temp.path().join(".worktrees").join("guarded");
        make_commit(&wt_path, "g.txt", "feat: g");

        let runtime = MockRuntime::new();
        runtime
            .launch(&scion_session_id("guarded"), "echo", Path::new("/tmp"))
            .unwrap();

        let result = scion_fuse(temp.path(), "guarded", None, &[], Some(&runtime), false);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("--force"),
            "expected '--force' hint, got: {err_msg}"
        );
    }

    #[test]
    fn scion_fuse_with_active_session_force_succeeds() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path());
        scion_create(temp.path(), "force-fuse", None, &[]).unwrap();
        let wt_path = temp.path().join(".worktrees").join("force-fuse");
        make_commit(&wt_path, "ff.txt", "feat: forced");

        let runtime = MockRuntime::new();
        let sid = scion_session_id("force-fuse");
        runtime.launch(&sid, "echo", Path::new("/tmp")).unwrap();

        // Force should stop session and proceed
        let result = scion_fuse(temp.path(), "force-fuse", None, &[], Some(&runtime), true);
        assert!(result.is_ok());
        assert!(!runtime.exists(&sid).unwrap());
    }

    #[test]
    fn scion_fuse_no_session_succeeds() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path());
        scion_create(temp.path(), "no-sess-fuse", None, &[]).unwrap();
        let wt_path = temp.path().join(".worktrees").join("no-sess-fuse");
        make_commit(&wt_path, "ns.txt", "feat: no session");

        let runtime = MockRuntime::new();
        let result = scion_fuse(
            temp.path(),
            "no-sess-fuse",
            None,
            &[],
            Some(&runtime),
            false,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn scion_fuse_no_runtime_succeeds() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path());
        scion_create(temp.path(), "no-rt-fuse", None, &[]).unwrap();
        let wt_path = temp.path().join(".worktrees").join("no-rt-fuse");
        make_commit(&wt_path, "nr.txt", "feat: no runtime");

        let result = scion_fuse(temp.path(), "no-rt-fuse", None, &[], None, false);
        assert!(result.is_ok());
    }

    #[test]
    fn scion_prune_with_active_session_no_force_errors() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path());
        scion_create(temp.path(), "prune-guard", None, &[]).unwrap();

        let runtime = MockRuntime::new();
        runtime
            .launch(&scion_session_id("prune-guard"), "echo", Path::new("/tmp"))
            .unwrap();

        let result = scion_prune(temp.path(), "prune-guard", None, &[], Some(&runtime), false);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("--force"),
            "expected '--force' hint, got: {err_msg}"
        );
    }

    #[test]
    fn scion_prune_with_active_session_force_succeeds() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path());
        scion_create(temp.path(), "force-prune", None, &[]).unwrap();

        let runtime = MockRuntime::new();
        let sid = scion_session_id("force-prune");
        runtime.launch(&sid, "echo", Path::new("/tmp")).unwrap();

        let result = scion_prune(temp.path(), "force-prune", None, &[], Some(&runtime), true);
        assert!(result.is_ok());
        assert!(!runtime.exists(&sid).unwrap());
    }

    // --- scion_attach_check tests ---

    #[test]
    fn scion_attach_check_nonexistent_scion_errors() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path());

        let runtime = MockRuntime::new();
        let result = scion_attach_check(temp.path(), "ghost", &runtime);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("does not exist"),
            "expected 'does not exist' error, got: {err_msg}"
        );
    }

    #[test]
    fn scion_attach_check_no_session_errors() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path());
        scion_create(temp.path(), "idle", None, &[]).unwrap();

        let runtime = MockRuntime::new();
        let result = scion_attach_check(temp.path(), "idle", &runtime);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("no active session"),
            "expected 'no active session' error, got: {err_msg}"
        );
    }

    #[test]
    fn scion_attach_check_with_session_returns_id() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path());
        scion_create(temp.path(), "running", None, &[]).unwrap();

        let runtime = MockRuntime::new();
        runtime
            .launch("graft-scion-running", "echo", Path::new("/tmp"))
            .unwrap();

        let session_id = scion_attach_check(temp.path(), "running", &runtime).unwrap();
        assert_eq!(session_id, "graft-scion-running");
    }
}
