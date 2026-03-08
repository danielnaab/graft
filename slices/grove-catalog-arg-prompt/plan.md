---
status: done
created: 2026-03-06
---

# Pre-populate prompt for catalog commands that need arguments

## Story

Selecting a command from the `:catalog` picker runs it immediately with no
arguments. Commands that require arguments (e.g., `diagnose` expecting a prompt
argument) fail with an error. There's no opportunity to enter arguments before
execution, making the catalog picker a trap for those commands.

After this slice, selecting a command that has required arguments opens the
command line pre-populated with `:run <name> ` (cursor at end), ready for the
user to type arguments. Commands with no required args continue to execute
immediately as before.

## Approach

Three small changes:

1. **`Prompt::open_with(text)`** — new method that opens the command line with
   pre-filled text and cursor at end. Built on `TextBuffer::with_content()`,
   which exists but is currently gated behind `#[cfg(test)]`.

2. **`CliCommand::PopulatePrompt(String)`** — new variant that opens the prompt
   with pre-populated text instead of executing a command. Handled in
   `execute_cli_command()` by calling `self.prompt.open_with(text)`.

3. **Catalog action routing** — in `cmd_catalog()`, when building the actions
   list, look up each command name in `self.context.available_commands`. If the
   command has required args without defaults (and those args can't be auto-filled
   from focus), use `CliCommand::PopulatePrompt(format!("run {name} "))` instead
   of `CliCommand::Run(name, vec![])`. Sequences (names starting with "» ") and
   commands without required args keep the current `Run` action.

### Required-args check

A command needs prompting when it has at least one arg where
`arg.required && arg.default.is_none()`. This mirrors the validation logic
already in `cmd_run` (lines 912-917). Note: we intentionally don't check
`options_from` here — focus auto-fill happens at execution time in `cmd_run`,
so we should prompt even if `options_from` is set (the user may not have a
focused value).

### Why sequences are unaffected

Sequences are loaded from graft.yaml separately (via `load_sequences_into()`
at transcript.rs:1145-1159) and are NOT present in
`self.context.available_commands`. When the catalog action-building code looks
up `run_name` in `available_commands`, sequences won't be found, so they
fall through to the default `CliCommand::Run(run_name, vec![])` action.
No special-case needed.

### Prompt-already-active guard

`PopulatePrompt` handler in `execute_cli_command()` should check
`self.prompt.is_active()` before calling `open_with()`. If the prompt is
already open, the command is a no-op (avoids overwriting user input).

## Acceptance Criteria

- Selecting a command with required args from catalog opens prompt with `run <name> ` pre-filled
- Cursor is positioned at the end of the pre-filled text
- Selecting a command with no required args executes immediately (unchanged)
- Selecting a sequence executes immediately (unchanged)
- Commands with all optional args or args with defaults execute immediately (unchanged)
- The pre-populated prompt supports full editing, completion, and history
- `TextBuffer::with_content()` is available at runtime (not test-only)
- `cargo test && cargo clippy -- -D warnings && cargo fmt --check` passes

## Steps

- [x] **Ungear `TextBuffer::with_content()` and add `Prompt::open_with()`**
  - **Delivers** — prompt pre-population capability
  - **Done when** — `TextBuffer::with_content(s, cursor_pos)` has `#[cfg(test)]`
    removed; `Prompt::open_with(text: &str)` creates a `CommandLineState` with
    `TextBuffer::with_content(text, text.len())`; existing tests still pass
  - **Files** — `crates/grove-cli/src/tui/text_buffer.rs`,
    `crates/grove-cli/src/tui/prompt.rs`

- [x] **Add `CliCommand::PopulatePrompt` variant and handler**
  - **Delivers** — command dispatch for prompt pre-population
  - **Done when** — `CliCommand::PopulatePrompt(String)` variant exists in
    `command_line.rs`; `execute_cli_command()` handles it by calling
    `self.prompt.open_with(&text)`
  - **Files** — `crates/grove-cli/src/tui/command_line.rs`,
    `crates/grove-cli/src/tui/transcript.rs`

- [x] **Route catalog actions based on required args**
  - **Delivers** — catalog entries with required args open prompt instead of failing
  - **Done when** — in `cmd_catalog()`, for each command entry (not sequences),
    check `available_commands` for required args without defaults; if found, push
    `CliCommand::PopulatePrompt(format!("run {run_name} "))` instead of
    `CliCommand::Run(run_name, vec![])`; sequences always use `Run`
  - **Files** — `crates/grove-cli/src/tui/transcript.rs`
