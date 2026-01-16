//! Core domain entities, rules, and traits for SetupVault.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Result type for core operations.
pub type CoreResult<T> = Result<T, CoreError>;

/// Errors returned by core validation and domain rules.
#[derive(Debug, Error)]
pub enum CoreError {
    /// Returned when a validation rule is violated.
    #[error("validation error: {0}")]
    Validation(String),
    /// Returned when repository operations fail.
    #[error("storage error: {0}")]
    Storage(String),
}

/// A user-provided explanation for why a change exists.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Rationale {
    text: String,
}

impl Rationale {
    /// Create a new rationale, rejecting empty or whitespace-only values.
    pub fn new(text: impl Into<String>) -> CoreResult<Self> {
        let text = text.into();
        if text.trim().is_empty() {
            return Err(CoreError::Validation("rationale cannot be empty".into()));
        }
        Ok(Self { text })
    }

    /// Access the rationale text.
    pub fn as_str(&self) -> &str {
        &self.text
    }
}

/// A label used to group or filter entries.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct Tag {
    value: String,
}

impl Tag {
    /// Create a new tag, rejecting empty or whitespace-only values.
    pub fn new(value: impl Into<String>) -> CoreResult<Self> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(CoreError::Validation("tag cannot be empty".into()));
        }
        Ok(Self { value })
    }

    /// Access the tag value.
    pub fn as_str(&self) -> &str {
        &self.value
    }
}

/// Supported entry categories.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EntryType {
    /// Package manager installs.
    Package,
    /// Configuration file changes.
    Config,
    /// Installed software applications.
    Application,
    /// Script or automation entries.
    Script,
    /// Catch-all for other changes.
    Other,
}

/// The current lifecycle status of an entry.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EntryStatus {
    /// Actively tracked entry.
    Active,
    /// Deferred for later review.
    Snoozed,
    /// Explicitly ignored or discarded.
    Ignored,
}

/// System metadata to help reproduce environments.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct SystemInfo {
    /// Operating system identifier.
    pub os: String,
    /// Architecture identifier.
    pub arch: String,
}

/// A persisted record in the SetupVault.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Entry {
    /// Unique identifier for the entry.
    pub id: Uuid,
    /// Human-readable title.
    pub title: String,
    /// Entry category.
    pub entry_type: EntryType,
    /// Detector source, such as homebrew or npm.
    pub source: String,
    /// Exact command to reproduce the change.
    pub cmd: String,
    /// System metadata for reproducibility.
    pub system: SystemInfo,
    /// Timestamp when the change was detected.
    pub detected_at: DateTime<Utc>,
    /// Current lifecycle status.
    pub status: EntryStatus,
    /// Optional tags for grouping and search.
    pub tags: Vec<Tag>,
    /// Required user rationale.
    pub rationale: Rationale,
    /// Optional verification guidance.
    pub verification: Option<String>,
}

impl Entry {
    /// Create a new entry, validating required fields.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: Uuid,
        title: impl Into<String>,
        entry_type: EntryType,
        source: impl Into<String>,
        cmd: impl Into<String>,
        system: SystemInfo,
        detected_at: DateTime<Utc>,
        status: EntryStatus,
        tags: Vec<Tag>,
        rationale: Rationale,
        verification: Option<String>,
    ) -> CoreResult<Self> {
        let title = title.into();
        if title.trim().is_empty() {
            return Err(CoreError::Validation("title cannot be empty".into()));
        }
        let source = source.into();
        if source.trim().is_empty() {
            return Err(CoreError::Validation("source cannot be empty".into()));
        }
        let cmd = cmd.into();
        if cmd.trim().is_empty() {
            return Err(CoreError::Validation("cmd cannot be empty".into()));
        }

        Ok(Self {
            id,
            title,
            entry_type,
            source,
            cmd,
            system,
            detected_at,
            status,
            tags,
            rationale,
            verification,
        })
    }
}

/// A change detected by a detector before user approval.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct DetectedChange {
    /// Unique identifier for the detected change.
    pub id: Uuid,
    /// Optional path associated with the change (e.g., a dotfile).
    pub path: Option<String>,
    /// Human-readable title.
    pub title: String,
    /// Entry category.
    pub entry_type: EntryType,
    /// Detector source.
    pub source: String,
    /// Exact command to reproduce the change.
    pub cmd: String,
    /// System metadata.
    pub system: SystemInfo,
    /// Timestamp when the change was detected.
    pub detected_at: DateTime<Utc>,
    /// Suggested tags.
    pub tags: Vec<Tag>,
}

/// Repository abstraction for reading and writing entries.
pub trait VaultRepository {
    /// Fetch a list of all entries.
    fn list(&self) -> CoreResult<Vec<Entry>>;
    /// Fetch a single entry by id.
    fn get(&self, id: Uuid) -> CoreResult<Option<Entry>>;
    /// Create a new entry.
    fn create(&self, entry: &Entry) -> CoreResult<()>;
    /// Update an existing entry.
    fn update(&self, entry: &Entry) -> CoreResult<()>;
    /// Delete an entry by id.
    fn delete(&self, id: Uuid) -> CoreResult<()>;
}

/// Detector interface for scanning system changes.
pub trait Detector {
    /// Return the detector name.
    fn name(&self) -> &'static str;
    /// Scan for changes and return detected changes.
    fn scan(&self) -> CoreResult<Vec<DetectedChange>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rationale_rejects_empty() {
        let result = Rationale::new("   ");
        assert!(matches!(result, Err(CoreError::Validation(_))));
    }

    #[test]
    fn tag_rejects_empty() {
        let result = Tag::new("");
        assert!(matches!(result, Err(CoreError::Validation(_))));
    }

    #[test]
    fn entry_requires_non_empty_fields() {
        let rationale = Rationale::new("needed for json parsing").unwrap();
        let system = SystemInfo {
            os: "macos".into(),
            arch: "arm64".into(),
        };

        let entry = Entry::new(
            Uuid::new_v4(),
            "jq",
            EntryType::Package,
            "homebrew",
            "brew install jq",
            system,
            Utc::now(),
            EntryStatus::Active,
            Vec::new(),
            rationale,
            None,
        );

        assert!(entry.is_ok());
    }
}
