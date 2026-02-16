//! State query data structures.
use graft_common::state::StateResult;
use serde::{Deserialize, Serialize};

/// A state query definition from graft.yaml.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateQuery {
    pub name: String,
    pub run: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub deterministic: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
}

/// Get a summary string for display (type-specific formatting).
pub fn format_state_summary(result: &StateResult) -> String {
    // Try to format based on common query types
    if let Some(obj) = result.data.as_object() {
        // Writing metrics
        if let (Some(total_words), Some(words_today)) =
            (obj.get("total_words"), obj.get("words_today"))
        {
            return format!(
                "{} words total, {} today",
                total_words.as_u64().unwrap_or(0),
                words_today.as_u64().unwrap_or(0)
            );
        }

        // Task metrics
        if let (Some(open), Some(completed)) = (obj.get("open"), obj.get("completed")) {
            return format!(
                "{} open, {} done",
                open.as_u64().unwrap_or(0),
                completed.as_u64().unwrap_or(0)
            );
        }

        // Graph metrics
        if let (Some(broken), Some(orphaned)) = (obj.get("broken_links"), obj.get("orphaned")) {
            return format!(
                "{} broken links, {} orphans",
                broken.as_u64().unwrap_or(0),
                orphaned.as_u64().unwrap_or(0)
            );
        }

        // Recent activity
        if let Some(modified_today) = obj.get("modified_today") {
            return format!("{} modified today", modified_today.as_u64().unwrap_or(0));
        }
    }

    // Fallback: Generic JSON summary
    format!(
        "{} fields",
        result.data.as_object().map_or(0, serde_json::Map::len)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use graft_common::state::StateMetadata;
    use serde_json::json;

    #[test]
    fn test_writing_metrics_summary() {
        let result = StateResult {
            metadata: StateMetadata {
                query_name: "writing".to_string(),
                commit_hash: "abc123".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                command: "test".to_string(),
                deterministic: false,
            },
            data: json!({
                "total_words": 5000,
                "words_today": 250,
                "notes_created": 1,
                "notes_modified": 3,
                "date": "2026-02-14"
            }),
        };

        assert_eq!(format_state_summary(&result), "5000 words total, 250 today");
    }

    #[test]
    fn test_task_metrics_summary() {
        let result = StateResult {
            metadata: StateMetadata {
                query_name: "tasks".to_string(),
                commit_hash: "abc123".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                command: "test".to_string(),
                deterministic: true,
            },
            data: json!({
                "open": 59,
                "completed": 49,
                "total": 108
            }),
        };

        assert_eq!(format_state_summary(&result), "59 open, 49 done");
    }

    #[test]
    fn test_graph_metrics_summary() {
        let result = StateResult {
            metadata: StateMetadata {
                query_name: "graph".to_string(),
                commit_hash: "abc123".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                command: "test".to_string(),
                deterministic: true,
            },
            data: json!({
                "total_notes": 2019,
                "total_links": 4910,
                "broken_links": 2223,
                "orphaned": 463
            }),
        };

        assert_eq!(
            format_state_summary(&result),
            "2223 broken links, 463 orphans"
        );
    }

    #[test]
    fn test_generic_summary() {
        let result = StateResult {
            metadata: StateMetadata {
                query_name: "custom".to_string(),
                commit_hash: "abc123".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                command: "test".to_string(),
                deterministic: true,
            },
            data: json!({
                "foo": 1,
                "bar": 2,
                "baz": 3
            }),
        };

        assert_eq!(format_state_summary(&result), "3 fields");
    }
}
