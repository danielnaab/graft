//! Sequence execution support.
//!
//! Executes named multi-step command sequences defined in graft.yaml.

use crate::command::{execute_command_with_context, CommandContext};
use crate::domain::GraftConfig;
use crate::error::{GraftError, Result};
use std::io::Write;

/// Execute a named sequence from a graft.yaml config.
///
/// Iterates through the sequence's steps in order, executing each as a command
/// via `execute_command_with_context`. Writes `sequence-state.json` to the
/// run-state directory before each step and updates it on completion or failure.
///
/// The sequence executor uses "pass-all" arg semantics: all args are passed to
/// every step. Steps whose `run:` template does not include a `{name}` placeholder
/// simply receive the args positionally (harmlessly for steps that ignore them).
///
/// When `on_step_fail` is configured for a step that fails, the executor runs
/// the recovery command and retries the failed step up to `max` times. If the
/// recovery command itself exits non-zero, the sequence aborts immediately.
///
/// Returns the exit code of the failed step (or 0 on success).
pub fn execute_sequence(
    config: &GraftConfig,
    sequence_name: &str,
    ctx: &CommandContext,
    args: &[String],
) -> Result<i32> {
    let seq_def = config.sequences.get(sequence_name).ok_or_else(|| {
        GraftError::CommandExecution(format!("Sequence not found: {sequence_name}"))
    })?;
    // Clone to avoid borrow issues during execution
    let seq_def = seq_def.clone();

    let run_state_dir = ctx.consumer_dir.join(".graft").join("run-state");
    std::fs::create_dir_all(&run_state_dir).map_err(|e| {
        GraftError::CommandExecution(format!(
            "Failed to create run-state directory '{}': {e}",
            run_state_dir.display()
        ))
    })?;

    let step_count = seq_def.steps.len();

    // Check for an interrupted prior run and compute the step to resume from.
    let resume_from = read_resume_index(&run_state_dir, sequence_name).unwrap_or(0);

    for (step_index, step_name) in seq_def.steps.iter().enumerate() {
        // Skip steps that were completed before an interruption.
        if step_index < resume_from {
            eprintln!("↷ Skipping {step_name} (already completed)");
            continue;
        }

        let result = execute_step_with_retry(
            config,
            sequence_name,
            step_name,
            step_index,
            step_count,
            &run_state_dir,
            ctx,
            args,
            &seq_def,
        )?;

        if result != 0 {
            return Ok(result);
        }
    }

    // All steps succeeded
    write_sequence_state(
        &run_state_dir,
        sequence_name,
        "",
        step_count.saturating_sub(1),
        step_count,
        "complete",
        None,
    )?;

    // Write checkpoint.json if checkpoint: true is set on this sequence
    if seq_def.checkpoint == Some(true) {
        write_checkpoint_json(&run_state_dir, sequence_name, args)?;
    }

    eprintln!("\n✓ Sequence '{sequence_name}' completed successfully");
    Ok(0)
}

/// Write checkpoint.json to the run-state directory for sequences with `checkpoint: true`.
///
/// The checkpoint file signals that the sequence is awaiting review before proceeding.
/// Format: `{"phase": "awaiting-review", "sequence": "...", "args": {...}, "message": "...", "created_at": "..."}`
pub fn write_checkpoint_json(
    run_state_dir: &std::path::Path,
    sequence: &str,
    args: &[String],
) -> Result<()> {
    let checkpoint_file = run_state_dir.join("checkpoint.json");
    let tmp_file = run_state_dir.join("checkpoint.json.tmp");

    let created_at = chrono::Utc::now().to_rfc3339();

    // Build args object from positional args (key=value pairs or positional indices)
    let mut args_map = serde_json::Map::new();
    for (i, arg) in args.iter().enumerate() {
        if let Some((k, v)) = arg.split_once('=') {
            args_map.insert(k.to_string(), serde_json::json!(v));
        } else {
            args_map.insert(i.to_string(), serde_json::json!(arg));
        }
    }

    let obj = serde_json::json!({
        "phase": "awaiting-review",
        "sequence": sequence,
        "args": args_map,
        "message": "Sequence complete. Review and approve or reject to continue.",
        "created_at": created_at,
    });

    // Atomic write: write to .tmp then rename
    {
        let mut file = std::fs::File::create(&tmp_file).map_err(|e| {
            GraftError::CommandExecution(format!(
                "Failed to write checkpoint.json.tmp '{}': {e}",
                tmp_file.display()
            ))
        })?;
        serde_json::to_writer_pretty(&mut file, &obj).map_err(|e| {
            GraftError::CommandExecution(format!("Failed to serialize checkpoint: {e}"))
        })?;
        writeln!(file).map_err(|e| {
            GraftError::CommandExecution(format!("Failed to write checkpoint.json.tmp: {e}"))
        })?;
    }

    std::fs::rename(&tmp_file, &checkpoint_file).map_err(|e| {
        GraftError::CommandExecution(format!(
            "Failed to rename checkpoint.json.tmp to checkpoint.json: {e}"
        ))
    })?;

    eprintln!("\n⏸  Checkpoint written. Review and approve/reject to continue.");
    Ok(())
}

/// Execute a single step, with retry logic if `on_step_fail` is configured for this step.
///
/// Returns 0 on success, non-zero exit code on final failure.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn execute_step_with_retry(
    config: &GraftConfig,
    sequence_name: &str,
    step_name: &str,
    step_index: usize,
    step_count: usize,
    run_state_dir: &std::path::Path,
    ctx: &CommandContext,
    args: &[String],
    seq_def: &graft_common::SequenceDef,
) -> Result<i32> {
    let command = config.commands.get(step_name).ok_or_else(|| {
        GraftError::CommandExecution(format!(
            "Sequence '{sequence_name}' step '{step_name}' not found in commands"
        ))
    })?;

    // Write sequence-state.json before executing the step
    write_sequence_state(
        run_state_dir,
        sequence_name,
        step_name,
        step_index,
        step_count,
        "running",
        None,
    )?;

    eprintln!(
        "\n[{}/{step_count}] {sequence_name}: {step_name}",
        step_index + 1
    );

    let result = execute_command_with_context(command, config, ctx, args)?;

    if result.success {
        return Ok(0);
    }

    // Step failed — check if retry is configured for this step
    let Some(osf) = &seq_def.on_step_fail else {
        // No retry config — fail immediately
        write_sequence_state(
            run_state_dir,
            sequence_name,
            step_name,
            step_index,
            step_count,
            "failed",
            None,
        )?;
        eprintln!(
            "\n✗ Sequence '{sequence_name}' failed at step '{step_name}' (exit {})",
            result.exit_code
        );
        return Ok(result.exit_code);
    };

    if osf.step != step_name {
        // Retry configured for a different step — fail immediately
        write_sequence_state(
            run_state_dir,
            sequence_name,
            step_name,
            step_index,
            step_count,
            "failed",
            None,
        )?;
        eprintln!(
            "\n✗ Sequence '{sequence_name}' failed at step '{step_name}' (exit {})",
            result.exit_code
        );
        return Ok(result.exit_code);
    }

    // Retry loop
    let recovery_name = osf.recovery.clone();
    let max = osf.max;

    // max: 0 means no retries — fall through to immediate failure.
    if max == 0 {
        write_sequence_state(
            run_state_dir,
            sequence_name,
            step_name,
            step_index,
            step_count,
            "failed",
            None,
        )?;
        eprintln!(
            "\n✗ Sequence '{sequence_name}' failed at step '{step_name}' (exit {})",
            result.exit_code
        );
        return Ok(result.exit_code);
    }

    for iteration in 1..=max {
        eprintln!(
            "\n[retry {iteration}/{max}] {sequence_name}: running recovery '{recovery_name}'"
        );

        // Run recovery command
        let recovery_cmd = config.commands.get(recovery_name.as_str()).ok_or_else(|| {
            GraftError::CommandExecution(format!(
                "Sequence '{sequence_name}' recovery command '{recovery_name}' not found"
            ))
        })?;

        let recovery_result = execute_command_with_context(recovery_cmd, config, ctx, args)?;

        if !recovery_result.success {
            // Recovery failed — abort immediately, no further retries
            write_sequence_state(
                run_state_dir,
                sequence_name,
                step_name,
                step_index,
                step_count,
                "failed",
                Some(iteration),
            )?;
            eprintln!(
                "\n✗ Recovery command '{}' failed (exit {}); aborting",
                recovery_name, recovery_result.exit_code
            );
            return Ok(recovery_result.exit_code);
        }

        // Write retrying state
        write_sequence_state(
            run_state_dir,
            sequence_name,
            step_name,
            step_index,
            step_count,
            "retrying",
            Some(iteration),
        )?;

        eprintln!("\n[retry {iteration}/{max}] {sequence_name}: retrying '{step_name}'");

        // Retry the failed step
        let retry_result = execute_command_with_context(command, config, ctx, args)?;

        if retry_result.success {
            return Ok(0);
        }

        // Still failing — loop to next iteration (or exit after max)
        if iteration == max {
            let iterations_attempted = max + 1;
            write_sequence_state(
                run_state_dir,
                sequence_name,
                step_name,
                step_index,
                step_count,
                "failed",
                Some(iteration),
            )?;
            eprintln!(
                "\n✗ Step '{step_name}' failed after {max} retry attempts ({iterations_attempted} total runs)"
            );
            return Ok(retry_result.exit_code);
        }
    }

    // max >= 1 is guaranteed by the guard above; the loop always returns from
    // inside its body on the final iteration, so this is unreachable.
    unreachable!("retry loop must return on the final iteration")
}

/// Read the resume index from an existing sequence-state.json.
///
/// Returns `Some(step_index)` when the file exists, belongs to `sequence_name`,
/// and has `phase: "running"` or `phase: "retrying"` — meaning the sequence
/// was interrupted mid-run and the step at that index should be the restart point.
///
/// Returns `None` (fresh run) when:
/// - The file is absent or unreadable.
/// - The recorded sequence name does not match.
/// - The phase is `"complete"` or `"failed"` (sequence already terminated cleanly).
fn read_resume_index(run_state_dir: &std::path::Path, sequence_name: &str) -> Option<usize> {
    let state_file = run_state_dir.join("sequence-state.json");
    let content = std::fs::read_to_string(&state_file).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&content).ok()?;

    let recorded_sequence = parsed.get("sequence")?.as_str()?;
    if recorded_sequence != sequence_name {
        return None;
    }

    let phase = parsed.get("phase")?.as_str()?;
    if phase != "running" && phase != "retrying" {
        return None;
    }

    let step_index = usize::try_from(parsed.get("step_index")?.as_u64()?).ok()?;
    Some(step_index)
}

/// Write sequence-state.json to the run-state directory atomically.
///
/// Uses a `.tmp` + rename pattern so grove never observes a partial write,
/// consistent with how `write_checkpoint_json` writes checkpoint.json.
pub fn write_sequence_state(
    run_state_dir: &std::path::Path,
    sequence: &str,
    step: &str,
    step_index: usize,
    step_count: usize,
    phase: &str,
    iteration: Option<u32>,
) -> Result<()> {
    let state_file = run_state_dir.join("sequence-state.json");
    let tmp_file = run_state_dir.join("sequence-state.json.tmp");

    let mut obj = serde_json::json!({
        "sequence": sequence,
        "step": step,
        "step_index": step_index,
        "step_count": step_count,
        "phase": phase,
    });
    if let Some(iter) = iteration {
        obj["iteration"] = serde_json::json!(iter);
    }

    {
        let mut file = std::fs::File::create(&tmp_file).map_err(|e| {
            GraftError::CommandExecution(format!(
                "Failed to write sequence-state.json.tmp '{}': {e}",
                tmp_file.display()
            ))
        })?;

        serde_json::to_writer_pretty(&mut file, &obj).map_err(|e| {
            GraftError::CommandExecution(format!("Failed to serialize sequence state: {e}"))
        })?;

        writeln!(file).map_err(|e| {
            GraftError::CommandExecution(format!("Failed to write sequence-state.json.tmp: {e}"))
        })?;
    }

    std::fs::rename(&tmp_file, &state_file).map_err(|e| {
        GraftError::CommandExecution(format!(
            "Failed to rename sequence-state.json.tmp to sequence-state.json: {e}"
        ))
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Command;
    use tempfile::TempDir;

    fn make_echo_config(commands: &[(&str, &str)]) -> GraftConfig {
        let mut config = GraftConfig::new("graft/v0").unwrap();
        for (name, run) in commands {
            let cmd = Command::new(*name, *run).unwrap();
            config.commands.insert(name.to_string(), cmd);
        }
        config
    }

    #[test]
    fn execute_sequence_runs_all_steps_in_order() {
        let tmp = TempDir::new().unwrap();
        let out_file = tmp.path().join("order.txt");

        let cmd1_run = format!("echo step1 >> {}", out_file.to_string_lossy());
        let cmd2_run = format!("echo step2 >> {}", out_file.to_string_lossy());

        let mut config = make_echo_config(&[("step1", &cmd1_run), ("step2", &cmd2_run)]);

        let seq = graft_common::SequenceDef {
            steps: vec!["step1".to_string(), "step2".to_string()],
            description: None,
            args: vec![],
            on_step_fail: None,
            checkpoint: None,
        };
        config.sequences.insert("test-seq".to_string(), seq);

        let ctx = CommandContext::local(tmp.path(), "test", "test", false);
        let exit_code = execute_sequence(&config, "test-seq", &ctx, &[]).unwrap();

        assert_eq!(exit_code, 0);

        let content = std::fs::read_to_string(&out_file).unwrap();
        assert!(content.contains("step1"), "step1 should have run");
        assert!(content.contains("step2"), "step2 should have run");
        // Verify order
        let step1_pos = content.find("step1").unwrap();
        let step2_pos = content.find("step2").unwrap();
        assert!(step1_pos < step2_pos, "step1 should run before step2");
    }

    #[test]
    fn execute_sequence_stops_on_failure() {
        let tmp = TempDir::new().unwrap();
        let out_file = tmp.path().join("order.txt");

        let cmd1_run = "exit 1".to_string();
        let cmd2_run = format!("echo should_not_run >> {}", out_file.to_string_lossy());

        let mut config = make_echo_config(&[("fail-step", &cmd1_run), ("next-step", &cmd2_run)]);

        let seq = graft_common::SequenceDef {
            steps: vec!["fail-step".to_string(), "next-step".to_string()],
            description: None,
            args: vec![],
            on_step_fail: None,
            checkpoint: None,
        };
        config.sequences.insert("test-seq".to_string(), seq);

        let ctx = CommandContext::local(tmp.path(), "test", "test", false);
        let exit_code = execute_sequence(&config, "test-seq", &ctx, &[]).unwrap();

        assert_ne!(exit_code, 0, "should return non-zero when a step fails");

        // step2 should NOT have run
        assert!(
            !out_file.exists()
                || !std::fs::read_to_string(&out_file)
                    .unwrap()
                    .contains("should_not_run"),
            "step2 should not have run after step1 failure"
        );
    }

    #[test]
    fn execute_sequence_writes_sequence_state_json() {
        let tmp = TempDir::new().unwrap();

        let mut config = make_echo_config(&[("echo-step", "echo hello")]);

        let seq = graft_common::SequenceDef {
            steps: vec!["echo-step".to_string()],
            description: None,
            args: vec![],
            on_step_fail: None,
            checkpoint: None,
        };
        config.sequences.insert("my-seq".to_string(), seq);

        let ctx = CommandContext::local(tmp.path(), "test", "test", false);
        let exit_code = execute_sequence(&config, "my-seq", &ctx, &[]).unwrap();
        assert_eq!(exit_code, 0);

        let state_file = tmp
            .path()
            .join(".graft")
            .join("run-state")
            .join("sequence-state.json");
        assert!(state_file.exists(), "sequence-state.json should be written");

        let content = std::fs::read_to_string(&state_file).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed["sequence"], "my-seq");
        assert_eq!(parsed["phase"], "complete");
    }

    #[test]
    fn execute_sequence_not_found_returns_error() {
        let tmp = TempDir::new().unwrap();
        let config = GraftConfig::new("graft/v0").unwrap();
        let ctx = CommandContext::local(tmp.path(), "test", "test", false);
        let result = execute_sequence(&config, "nonexistent", &ctx, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn retry_succeeds_after_two_failures() {
        let tmp = TempDir::new().unwrap();
        // Counter file tracks how many times the check step has been called
        let counter_file = tmp.path().join("counter.txt");
        std::fs::write(&counter_file, "0").unwrap();

        // check-step: reads counter, increments it, exits 1 if counter <= 2, else exits 0
        let check_run = format!(
            r"c=$(cat {0}); c=$((c+1)); echo $c > {0}; [ $c -gt 2 ]",
            counter_file.to_string_lossy()
        );
        // recovery-step: just succeeds
        let recovery_run = "echo recovering".to_string();

        let mut config =
            make_echo_config(&[("check-step", &check_run), ("recovery-step", &recovery_run)]);

        let seq = graft_common::SequenceDef {
            steps: vec!["check-step".to_string()],
            description: None,
            args: vec![],
            on_step_fail: Some(graft_common::OnStepFail {
                step: "check-step".to_string(),
                recovery: "recovery-step".to_string(),
                max: 3,
            }),
            checkpoint: None,
        };
        config.sequences.insert("retry-seq".to_string(), seq);

        let ctx = CommandContext::local(tmp.path(), "test", "test", false);
        let exit_code = execute_sequence(&config, "retry-seq", &ctx, &[]).unwrap();

        // Should succeed after 2 retries (counter goes: 1=fail, 2=fail, 3=pass)
        assert_eq!(exit_code, 0, "sequence should succeed after retries");
        let counter = std::fs::read_to_string(&counter_file).unwrap();
        assert_eq!(
            counter.trim(),
            "3",
            "check-step should have run 3 times (1 initial + 2 retries)"
        );
    }

    #[test]
    fn retry_fails_after_max_retries() {
        let tmp = TempDir::new().unwrap();

        let mut config = make_echo_config(&[("fail-step", "exit 1"), ("recovery", "echo ok")]);

        let seq = graft_common::SequenceDef {
            steps: vec!["fail-step".to_string()],
            description: None,
            args: vec![],
            on_step_fail: Some(graft_common::OnStepFail {
                step: "fail-step".to_string(),
                recovery: "recovery".to_string(),
                max: 2,
            }),
            checkpoint: None,
        };
        config.sequences.insert("max-retry-seq".to_string(), seq);

        let ctx = CommandContext::local(tmp.path(), "test", "test", false);
        let exit_code = execute_sequence(&config, "max-retry-seq", &ctx, &[]).unwrap();

        assert_ne!(exit_code, 0, "should fail after max retries");
    }

    #[test]
    fn recovery_failure_aborts_immediately() {
        let tmp = TempDir::new().unwrap();
        let out_file = tmp.path().join("count.txt");

        // fail-step always fails
        let fail_run = "exit 1".to_string();
        // recovery always fails
        let recovery_run = "exit 2".to_string();

        let mut config = make_echo_config(&[("fail-step", &fail_run), ("recovery", &recovery_run)]);

        let seq = graft_common::SequenceDef {
            steps: vec!["fail-step".to_string()],
            description: None,
            args: vec![],
            on_step_fail: Some(graft_common::OnStepFail {
                step: "fail-step".to_string(),
                recovery: "recovery".to_string(),
                max: 3,
            }),
            checkpoint: None,
        };
        config.sequences.insert("abort-seq".to_string(), seq);

        let ctx = CommandContext::local(tmp.path(), "test", "test", false);
        let exit_code = execute_sequence(&config, "abort-seq", &ctx, &[]).unwrap();

        // Recovery failed → should abort with recovery's exit code (2)
        assert_eq!(
            exit_code, 2,
            "should return recovery's exit code on recovery failure"
        );

        // Should not have retried (only 1 recovery attempt)
        let _ = out_file; // referenced to avoid unused warning
    }

    #[test]
    fn checkpoint_true_writes_checkpoint_json() {
        let tmp = TempDir::new().unwrap();

        let mut config = make_echo_config(&[("echo-step", "echo hello")]);

        let seq = graft_common::SequenceDef {
            steps: vec!["echo-step".to_string()],
            description: None,
            args: vec![],
            on_step_fail: None,
            checkpoint: Some(true),
        };
        config.sequences.insert("checkpoint-seq".to_string(), seq);

        let ctx = CommandContext::local(tmp.path(), "test", "test", false);
        let exit_code = execute_sequence(&config, "checkpoint-seq", &ctx, &[]).unwrap();
        assert_eq!(exit_code, 0);

        let checkpoint_file = tmp
            .path()
            .join(".graft")
            .join("run-state")
            .join("checkpoint.json");
        assert!(
            checkpoint_file.exists(),
            "checkpoint.json should be written when checkpoint: true"
        );

        let content = std::fs::read_to_string(&checkpoint_file).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed["phase"], "awaiting-review");
        assert_eq!(parsed["sequence"], "checkpoint-seq");
        assert!(parsed["created_at"].is_string());
        assert!(parsed["message"].is_string());
    }

    #[test]
    fn checkpoint_absent_does_not_write_checkpoint_json() {
        let tmp = TempDir::new().unwrap();

        let mut config = make_echo_config(&[("echo-step", "echo hello")]);

        let seq = graft_common::SequenceDef {
            steps: vec!["echo-step".to_string()],
            description: None,
            args: vec![],
            on_step_fail: None,
            checkpoint: None, // field absent
        };
        config
            .sequences
            .insert("no-checkpoint-seq".to_string(), seq);

        let ctx = CommandContext::local(tmp.path(), "test", "test", false);
        let exit_code = execute_sequence(&config, "no-checkpoint-seq", &ctx, &[]).unwrap();
        assert_eq!(exit_code, 0);

        let checkpoint_file = tmp
            .path()
            .join(".graft")
            .join("run-state")
            .join("checkpoint.json");
        assert!(
            !checkpoint_file.exists(),
            "checkpoint.json should NOT be written when checkpoint is absent"
        );
    }

    #[test]
    fn checkpoint_explicit_false_does_not_write_checkpoint_json() {
        let tmp = TempDir::new().unwrap();

        let mut config = make_echo_config(&[("echo-step", "echo hello")]);

        let seq = graft_common::SequenceDef {
            steps: vec!["echo-step".to_string()],
            description: None,
            args: vec![],
            on_step_fail: None,
            checkpoint: Some(false), // explicitly false
        };
        config
            .sequences
            .insert("no-checkpoint-seq".to_string(), seq);

        let ctx = CommandContext::local(tmp.path(), "test", "test", false);
        let exit_code = execute_sequence(&config, "no-checkpoint-seq", &ctx, &[]).unwrap();
        assert_eq!(exit_code, 0);

        let checkpoint_file = tmp
            .path()
            .join(".graft")
            .join("run-state")
            .join("checkpoint.json");
        assert!(
            !checkpoint_file.exists(),
            "checkpoint.json should NOT be written when checkpoint: false"
        );
    }

    #[test]
    fn checkpoint_not_written_on_failure() {
        let tmp = TempDir::new().unwrap();

        let mut config = make_echo_config(&[("fail-step", "exit 1")]);

        let seq = graft_common::SequenceDef {
            steps: vec!["fail-step".to_string()],
            description: None,
            args: vec![],
            on_step_fail: None,
            checkpoint: Some(true),
        };
        config
            .sequences
            .insert("fail-checkpoint-seq".to_string(), seq);

        let ctx = CommandContext::local(tmp.path(), "test", "test", false);
        let exit_code = execute_sequence(&config, "fail-checkpoint-seq", &ctx, &[]).unwrap();
        assert_ne!(exit_code, 0);

        let checkpoint_file = tmp
            .path()
            .join(".graft")
            .join("run-state")
            .join("checkpoint.json");
        assert!(
            !checkpoint_file.exists(),
            "checkpoint.json should NOT be written when sequence fails"
        );
    }

    // ── Crash resumability tests ─────────────────────────────────────────────

    /// Helper: write a synthetic sequence-state.json into the run-state dir.
    fn write_synthetic_state(
        run_state_dir: &std::path::Path,
        sequence: &str,
        step: &str,
        step_index: usize,
        step_count: usize,
        phase: &str,
    ) {
        std::fs::create_dir_all(run_state_dir).unwrap();
        let obj = serde_json::json!({
            "sequence": sequence,
            "step": step,
            "step_index": step_index,
            "step_count": step_count,
            "phase": phase,
        });
        let path = run_state_dir.join("sequence-state.json");
        std::fs::write(path, serde_json::to_string_pretty(&obj).unwrap()).unwrap();
    }

    /// Build a two-step ("step-a", "step-b") config + sequence that appends
    /// each step's name to `out_file`. Returns (config, `seq_name`).
    fn make_two_step_config(out_file: &std::path::Path) -> (GraftConfig, &'static str) {
        let run_a = format!("echo step-a >> {}", out_file.to_string_lossy());
        let run_b = format!("echo step-b >> {}", out_file.to_string_lossy());
        let mut config = make_echo_config(&[("step-a", &run_a), ("step-b", &run_b)]);
        let seq = graft_common::SequenceDef {
            steps: vec!["step-a".to_string(), "step-b".to_string()],
            description: None,
            args: vec![],
            on_step_fail: None,
            checkpoint: None,
        };
        config.sequences.insert("two-step".to_string(), seq);
        (config, "two-step")
    }

    #[test]
    fn resume_skips_completed_steps_when_state_is_running() {
        // step-a completed, step-b was running (killed at step_index=1) → skip step-a
        let tmp = TempDir::new().unwrap();
        let out_file = tmp.path().join("ran.txt");
        let run_state_dir = tmp.path().join(".graft").join("run-state");
        write_synthetic_state(&run_state_dir, "two-step", "step-b", 1, 2, "running");

        let (config, seq_name) = make_two_step_config(&out_file);
        let ctx = CommandContext::local(tmp.path(), "test", "test", false);
        let exit_code = execute_sequence(&config, seq_name, &ctx, &[]).unwrap();
        assert_eq!(exit_code, 0);

        let content = std::fs::read_to_string(&out_file).unwrap();
        assert!(
            !content.contains("step-a"),
            "step-a should be skipped (already completed)"
        );
        assert!(
            content.contains("step-b"),
            "step-b should execute on resume"
        );
    }

    #[test]
    fn resume_with_no_state_file_runs_all_steps() {
        // No sequence-state.json → fresh run, all steps execute
        let tmp = TempDir::new().unwrap();
        let out_file = tmp.path().join("ran.txt");

        let (config, seq_name) = make_two_step_config(&out_file);
        let ctx = CommandContext::local(tmp.path(), "test", "test", false);
        let exit_code = execute_sequence(&config, seq_name, &ctx, &[]).unwrap();
        assert_eq!(exit_code, 0);

        let content = std::fs::read_to_string(&out_file).unwrap();
        assert!(
            content.contains("step-a"),
            "step-a should run on fresh start"
        );
        assert!(
            content.contains("step-b"),
            "step-b should run on fresh start"
        );
    }

    #[test]
    fn resume_with_complete_phase_runs_all_steps() {
        // phase: "complete" → sequence already done → fresh run
        let tmp = TempDir::new().unwrap();
        let out_file = tmp.path().join("ran.txt");
        let run_state_dir = tmp.path().join(".graft").join("run-state");
        write_synthetic_state(&run_state_dir, "two-step", "", 1, 2, "complete");

        let (config, seq_name) = make_two_step_config(&out_file);
        let ctx = CommandContext::local(tmp.path(), "test", "test", false);
        let exit_code = execute_sequence(&config, seq_name, &ctx, &[]).unwrap();
        assert_eq!(exit_code, 0);

        let content = std::fs::read_to_string(&out_file).unwrap();
        assert!(
            content.contains("step-a"),
            "step-a should run after complete state"
        );
        assert!(
            content.contains("step-b"),
            "step-b should run after complete state"
        );
    }

    #[test]
    fn resume_with_failed_phase_runs_all_steps() {
        // phase: "failed" → sequence terminated cleanly → fresh run
        let tmp = TempDir::new().unwrap();
        let out_file = tmp.path().join("ran.txt");
        let run_state_dir = tmp.path().join(".graft").join("run-state");
        write_synthetic_state(&run_state_dir, "two-step", "step-a", 0, 2, "failed");

        let (config, seq_name) = make_two_step_config(&out_file);
        let ctx = CommandContext::local(tmp.path(), "test", "test", false);
        let exit_code = execute_sequence(&config, seq_name, &ctx, &[]).unwrap();
        assert_eq!(exit_code, 0);

        let content = std::fs::read_to_string(&out_file).unwrap();
        assert!(
            content.contains("step-a"),
            "step-a should run after failed state"
        );
        assert!(
            content.contains("step-b"),
            "step-b should run after failed state"
        );
    }

    #[test]
    fn resume_with_different_sequence_name_runs_all_steps() {
        // State belongs to a different sequence → fresh run
        let tmp = TempDir::new().unwrap();
        let out_file = tmp.path().join("ran.txt");
        let run_state_dir = tmp.path().join(".graft").join("run-state");
        write_synthetic_state(&run_state_dir, "other-seq", "step-a", 0, 2, "running");

        let (config, seq_name) = make_two_step_config(&out_file);
        let ctx = CommandContext::local(tmp.path(), "test", "test", false);
        let exit_code = execute_sequence(&config, seq_name, &ctx, &[]).unwrap();
        assert_eq!(exit_code, 0);

        let content = std::fs::read_to_string(&out_file).unwrap();
        assert!(
            content.contains("step-a"),
            "step-a should run (different sequence in state)"
        );
        assert!(
            content.contains("step-b"),
            "step-b should run (different sequence in state)"
        );
    }

    #[test]
    fn resume_with_retrying_phase_treats_same_as_running() {
        // phase: "retrying" at step_index=1 → skip step-a, restart from step-b
        let tmp = TempDir::new().unwrap();
        let out_file = tmp.path().join("ran.txt");
        let run_state_dir = tmp.path().join(".graft").join("run-state");
        write_synthetic_state(&run_state_dir, "two-step", "step-b", 1, 2, "retrying");

        let (config, seq_name) = make_two_step_config(&out_file);
        let ctx = CommandContext::local(tmp.path(), "test", "test", false);
        let exit_code = execute_sequence(&config, seq_name, &ctx, &[]).unwrap();
        assert_eq!(exit_code, 0);

        let content = std::fs::read_to_string(&out_file).unwrap();
        assert!(
            !content.contains("step-a"),
            "step-a should be skipped (retrying treated same as running)"
        );
        assert!(
            content.contains("step-b"),
            "step-b should execute on resume"
        );
    }
}
