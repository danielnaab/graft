//! Template rendering for command stdin.
//!
//! Supports rendering template files with Tera (Jinja2-compatible) engine.
//! Templates have access to built-in variables (`repo_path`, `repo_name`, etc.)
//! and state query results via the `state` namespace.

use crate::domain::StdinSource;
use crate::error::{GraftError, Result};
use std::collections::HashMap;
use std::path::Path;
use tera::Tera;

/// Context for template rendering, containing built-in and state variables.
pub struct TemplateContext {
    inner: tera::Context,
}

impl TemplateContext {
    /// Build a template context from built-in variables and state results.
    pub fn new(
        repo_path: &Path,
        commit_hash: &str,
        git_branch: &str,
        state_results: &HashMap<String, serde_json::Value>,
    ) -> Self {
        let mut ctx = tera::Context::new();

        // Built-in variables
        ctx.insert("repo_path", &repo_path.to_string_lossy().to_string());
        ctx.insert(
            "repo_name",
            &repo_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown"),
        );
        ctx.insert("commit_hash", commit_hash);
        ctx.insert("git_branch", git_branch);

        // State variables under the `state` namespace
        ctx.insert("state", state_results);

        Self { inner: ctx }
    }
}

/// Resolve a stdin source to rendered text.
///
/// For `StdinSource::Literal`, returns the text as-is.
/// For `StdinSource::Template`, reads the file and renders with Tera.
pub fn resolve_stdin(
    stdin: &StdinSource,
    base_dir: &Path,
    context: &TemplateContext,
) -> Result<String> {
    match stdin {
        StdinSource::Literal(text) => Ok(text.clone()),
        StdinSource::Template { path, engine } => {
            let engine_name = engine.as_deref().unwrap_or("tera");

            // Read template file
            let file_path = base_dir.join(path);
            let template_text = std::fs::read_to_string(&file_path).map_err(|e| {
                GraftError::CommandExecution(format!(
                    "Failed to read template file '{}': {e}",
                    file_path.display()
                ))
            })?;

            match engine_name {
                "none" => Ok(template_text),
                "tera" => render_template(&template_text, path, context),
                _ => Err(GraftError::Validation(format!(
                    "unsupported template engine: {engine_name}"
                ))),
            }
        }
    }
}

/// Render a template string with Tera.
pub fn render_template(
    template_text: &str,
    template_name: &str,
    context: &TemplateContext,
) -> Result<String> {
    let mut tera = Tera::default();
    tera.add_raw_template(template_name, template_text)
        .map_err(|e| {
            GraftError::CommandExecution(format!("Failed to parse template '{template_name}': {e}"))
        })?;

    tera.render(template_name, &context.inner).map_err(|e| {
        GraftError::CommandExecution(format!("Failed to render template '{template_name}': {e}"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_context() -> TemplateContext {
        let mut state = HashMap::new();
        state.insert(
            "coverage".to_string(),
            serde_json::json!({"percent_covered": 85.5, "total_lines": 1000}),
        );
        state.insert(
            "tasks".to_string(),
            serde_json::json!({"open": 3, "closed": 10}),
        );

        TemplateContext::new(
            Path::new("/home/user/my-project"),
            "abc123def456",
            "main",
            &state,
        )
    }

    #[test]
    fn render_builtin_variables() {
        let ctx = test_context();
        let result = render_template(
            "Repo: {{ repo_name }}, Branch: {{ git_branch }}",
            "test",
            &ctx,
        )
        .unwrap();
        assert_eq!(result, "Repo: my-project, Branch: main");
    }

    #[test]
    fn render_repo_path() {
        let ctx = test_context();
        let result = render_template("Path: {{ repo_path }}", "test", &ctx).unwrap();
        assert_eq!(result, "Path: /home/user/my-project");
    }

    #[test]
    fn render_commit_hash() {
        let ctx = test_context();
        let result = render_template("Commit: {{ commit_hash }}", "test", &ctx).unwrap();
        assert_eq!(result, "Commit: abc123def456");
    }

    #[test]
    fn render_state_variables() {
        let ctx = test_context();
        let result = render_template(
            "Coverage: {{ state.coverage.percent_covered }}%",
            "test",
            &ctx,
        )
        .unwrap();
        assert_eq!(result, "Coverage: 85.5%");
    }

    #[test]
    fn render_state_nested_access() {
        let ctx = test_context();
        let result = render_template("Open tasks: {{ state.tasks.open }}", "test", &ctx).unwrap();
        assert_eq!(result, "Open tasks: 3");
    }

    #[test]
    fn render_tera_conditional() {
        let ctx = test_context();
        let template = "{% if state.coverage.percent_covered > 80 %}GOOD{% else %}BAD{% endif %}";
        let result = render_template(template, "test", &ctx).unwrap();
        assert_eq!(result, "GOOD");
    }

    #[test]
    fn render_tera_loop() {
        let mut state = HashMap::new();
        state.insert(
            "items".to_string(),
            serde_json::json!({"list": ["a", "b", "c"]}),
        );
        let ctx = TemplateContext::new(Path::new("/tmp/repo"), "abc", "main", &state);

        let template = "{% for item in state.items.list %}{{ item }},{% endfor %}";
        let result = render_template(template, "test", &ctx).unwrap();
        assert_eq!(result, "a,b,c,");
    }

    #[test]
    fn render_invalid_template_syntax() {
        let ctx = test_context();
        let result = render_template("{{ unclosed", "test", &ctx);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to parse template"));
    }

    #[test]
    fn render_undefined_variable_fails() {
        let ctx = test_context();
        let result = render_template("{{ nonexistent_var }}", "test", &ctx);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to render template"));
    }

    #[test]
    fn resolve_literal_stdin() {
        let ctx = test_context();
        let stdin = StdinSource::Literal("hello world".to_string());
        let result = resolve_stdin(&stdin, Path::new("/tmp"), &ctx).unwrap();
        assert_eq!(result, "hello world");
    }

    #[test]
    fn resolve_template_stdin() {
        let dir = tempfile::tempdir().unwrap();
        let template_path = dir.path().join("test.md");
        std::fs::write(&template_path, "Hello {{ repo_name }}!").unwrap();

        let ctx = test_context();
        let stdin = StdinSource::Template {
            path: "test.md".to_string(),
            engine: None,
        };
        let result = resolve_stdin(&stdin, dir.path(), &ctx).unwrap();
        assert_eq!(result, "Hello my-project!");
    }

    #[test]
    fn resolve_template_with_none_engine() {
        let dir = tempfile::tempdir().unwrap();
        let template_path = dir.path().join("raw.md");
        std::fs::write(&template_path, "{{ not_evaluated }}").unwrap();

        let ctx = test_context();
        let stdin = StdinSource::Template {
            path: "raw.md".to_string(),
            engine: Some("none".to_string()),
        };
        let result = resolve_stdin(&stdin, dir.path(), &ctx).unwrap();
        assert_eq!(result, "{{ not_evaluated }}");
    }

    #[test]
    fn resolve_template_missing_file() {
        let ctx = test_context();
        let stdin = StdinSource::Template {
            path: "nonexistent.md".to_string(),
            engine: None,
        };
        let result = resolve_stdin(&stdin, &PathBuf::from("/tmp"), &ctx);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to read template file"));
    }

    #[test]
    fn resolve_template_with_state_data() {
        let dir = tempfile::tempdir().unwrap();
        let template_path = dir.path().join("report.md");
        std::fs::write(
            &template_path,
            "Coverage: {{ state.coverage.percent_covered }}% of {{ state.coverage.total_lines }} lines",
        )
        .unwrap();

        let ctx = test_context();
        let stdin = StdinSource::Template {
            path: "report.md".to_string(),
            engine: None,
        };
        let result = resolve_stdin(&stdin, dir.path(), &ctx).unwrap();
        assert_eq!(result, "Coverage: 85.5% of 1000 lines");
    }

    #[test]
    fn context_with_empty_state() {
        let state = HashMap::new();
        let ctx = TemplateContext::new(Path::new("/tmp/repo"), "abc", "main", &state);
        let result = render_template("Branch: {{ git_branch }}", "test", &ctx).unwrap();
        assert_eq!(result, "Branch: main");
    }
}
