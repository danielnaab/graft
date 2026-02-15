///! State query data structures.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

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

/// Metadata for a state query result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateMetadata {
    pub query_name: String,
    pub commit_hash: String,
    pub timestamp: String, // ISO 8601 format
    pub command: String,
    pub deterministic: bool,
}

impl StateMetadata {
    /// Parse timestamp as DateTime.
    pub fn timestamp_parsed(&self) -> Option<DateTime<Utc>> {
        DateTime::parse_from_rfc3339(&self.timestamp)
            .ok()
            .map(|dt| dt.with_timezone(&Utc))
    }

    /// Get human-readable time ago string.
    pub fn time_ago(&self) -> String {
        match self.timestamp_parsed() {
            Some(ts) => {
                let now = Utc::now();
                let duration = now.signed_duration_since(ts);

                if duration.num_seconds() < 60 {
                    "just now".to_string()
                } else if duration.num_minutes() < 60 {
                    let mins = duration.num_minutes();
                    format!("{}m ago", mins)
                } else if duration.num_hours() < 24 {
                    let hours = duration.num_hours();
                    format!("{}h ago", hours)
                } else {
                    let days = duration.num_days();
                    format!("{}d ago", days)
                }
            }
            None => "unknown".to_string(),
        }
    }
}

/// A state query result with data and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateResult {
    pub metadata: StateMetadata,
    pub data: Value,
}

impl StateResult {
    /// Get a summary string for display (type-specific formatting).
    pub fn summary(&self) -> String {
        // Try to format based on common query types
        if let Some(obj) = self.data.as_object() {
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
        format!("{} fields", self.data.as_object().map_or(0, |o| o.len()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_time_ago_formats_correctly() {
        let metadata = StateMetadata {
            query_name: "test".to_string(),
            commit_hash: "abc123".to_string(),
            timestamp: Utc::now().to_rfc3339(),
            command: "test".to_string(),
            deterministic: true,
        };

        let time_ago = metadata.time_ago();
        assert_eq!(time_ago, "just now");
    }

    #[test]
    fn test_writing_metrics_summary() {
        let result = StateResult {
            metadata: StateMetadata {
                query_name: "writing".to_string(),
                commit_hash: "abc123".to_string(),
                timestamp: Utc::now().to_rfc3339(),
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

        assert_eq!(result.summary(), "5000 words total, 250 today");
    }

    #[test]
    fn test_task_metrics_summary() {
        let result = StateResult {
            metadata: StateMetadata {
                query_name: "tasks".to_string(),
                commit_hash: "abc123".to_string(),
                timestamp: Utc::now().to_rfc3339(),
                command: "test".to_string(),
                deterministic: true,
            },
            data: json!({
                "open": 59,
                "completed": 49,
                "total": 108
            }),
        };

        assert_eq!(result.summary(), "59 open, 49 done");
    }

    #[test]
    fn test_graph_metrics_summary() {
        let result = StateResult {
            metadata: StateMetadata {
                query_name: "graph".to_string(),
                commit_hash: "abc123".to_string(),
                timestamp: Utc::now().to_rfc3339(),
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

        assert_eq!(result.summary(), "2223 broken links, 463 orphans");
    }
}
