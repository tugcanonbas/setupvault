//! Filesystem-backed persistence for the SetupVault.

use std::fs;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use walkdir::WalkDir;

use sv_core::{
    CoreError, CoreResult, DetectedChange, Entry, EntryStatus, EntryType, Rationale, SystemInfo, Tag,
    VaultRepository,
};

/// Default directory name for the vault.
pub const VAULT_DIR_NAME: &str = "setupvault";

const CONFIG_FILE_NAME: &str = "config.yaml";

/// Filesystem-backed vault repository.
#[derive(Debug, Clone)]
pub struct FsVault {
    root: PathBuf,
}

impl FsVault {
    /// Create a new filesystem vault rooted at the provided path.
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Get the root path of the vault.
    pub fn path(&self) -> &std::path::Path {
        &self.root
    }

    /// Resolve the default vault path (~/.setupvault).
    pub fn default_path() -> CoreResult<PathBuf> {
        if let Some(dir) = dirs::home_dir() {
            return Ok(dir.join(format!(".{VAULT_DIR_NAME}")));
        }
        Err(CoreError::Storage(
            "unable to determine a default vault path".into(),
        ))
    }

    /// Check if the vault exists at the root path.
    pub fn exists(&self) -> bool {
        self.root.exists() && self.entries_root().exists()
    }

    /// Initialize the vault structure.
    pub fn init(&self) -> CoreResult<()> {
        if self.exists() {
            return Ok(());
        }
        fs::create_dir_all(self.entries_root())
            .map_err(|err| CoreError::Storage(err.to_string()))?;
        fs::create_dir_all(self.state_root())
            .map_err(|err| CoreError::Storage(err.to_string()))?;
        Ok(())
    }

    fn entries_root(&self) -> PathBuf {
        self.root.join("entries")
    }

    fn state_root(&self) -> PathBuf {
        self.root.join(".state")
    }

    fn inbox_path(&self) -> PathBuf {
        self.state_root().join("inbox.yaml")
    }

    fn snoozed_path(&self) -> PathBuf {
        self.state_root().join("snoozed.yaml")
    }

    fn detector_snapshot_path(&self, source: &str) -> PathBuf {
        self.state_root().join("detectors").join(format!("{source}.yaml"))
    }

    fn entry_dir(entry_type: &EntryType, source: &str) -> PathBuf {
        let type_dir = match entry_type {
            EntryType::Package => "packages",
            EntryType::Config => "configs",
            EntryType::Application => "applications",
            EntryType::Script => "scripts",
            EntryType::Other => "other",
        };
        PathBuf::from(type_dir).join(source)
    }

    fn entry_file_name(entry: &Entry) -> String {
        let slug = slugify(&entry.title);
        format!("{}-{}-{}.md", entry.source, slug, entry.id)
    }

    fn entry_path(&self, entry: &Entry) -> PathBuf {
        self.entries_root()
            .join(Self::entry_dir(&entry.entry_type, &entry.source))
            .join(Self::entry_file_name(entry))
    }

    fn find_entry_path(&self, id: Uuid) -> CoreResult<Option<PathBuf>> {
        let entries_root = self.entries_root();
        if !entries_root.exists() {
            return Ok(None);
        }
        for entry in WalkDir::new(&entries_root).into_iter().filter_map(Result::ok) {
            if !entry.file_type().is_file() {
                continue;
            }
            if entry.path().extension().and_then(|ext| ext.to_str()) != Some("md") {
                continue;
            }
            let contents = fs::read_to_string(entry.path())
                .map_err(|err| CoreError::Storage(err.to_string()))?;
            if let Ok(frontmatter) = parse_frontmatter(&contents) {
                if frontmatter.id == id {
                    return Ok(Some(entry.into_path()));
                }
            }
        }
        Ok(None)
    }

    /// Load the current inbox queue from disk.
    pub fn load_inbox(&self) -> CoreResult<Vec<DetectedChange>> {
        let path = self.inbox_path();
        if !path.exists() {
            return Ok(Vec::new());
        }
        let contents = fs::read_to_string(&path)
            .map_err(|err| CoreError::Storage(err.to_string()))?;
        serde_yaml::from_str(&contents).map_err(|err| CoreError::Storage(err.to_string()))
    }

    /// Persist the inbox queue to disk.
    pub fn save_inbox(&self, changes: &[DetectedChange]) -> CoreResult<()> {
        let path = self.inbox_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| CoreError::Storage(err.to_string()))?;
        }
        let contents = serde_yaml::to_string(changes)
            .map_err(|err| CoreError::Storage(err.to_string()))?;
        fs::write(path, contents).map_err(|err| CoreError::Storage(err.to_string()))?;
        Ok(())
    }

    /// Add a new item to the inbox queue.
    pub fn add_inbox_item(&self, item: DetectedChange) -> CoreResult<()> {
        let mut changes = self.load_inbox()?;
        changes.push(item);
        self.save_inbox(&changes)
    }

    /// Remove a single inbox item by id.
    pub fn remove_inbox_item(&self, id: Uuid) -> CoreResult<()> {
        let mut changes = self.load_inbox()?;
        changes.retain(|change| change.id != id);
        self.save_inbox(&changes)
    }

    /// Load snoozed changes from disk.
    pub fn load_snoozed(&self) -> CoreResult<Vec<DetectedChange>> {
        let path = self.snoozed_path();
        if !path.exists() {
            return Ok(Vec::new());
        }
        let contents = fs::read_to_string(&path)
            .map_err(|err| CoreError::Storage(err.to_string()))?;
        serde_yaml::from_str(&contents).map_err(|err| CoreError::Storage(err.to_string()))
    }

    /// Persist snoozed changes to disk.
    pub fn save_snoozed(&self, changes: &[DetectedChange]) -> CoreResult<()> {
        let path = self.snoozed_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| CoreError::Storage(err.to_string()))?;
        }
        let contents = serde_yaml::to_string(changes)
            .map_err(|err| CoreError::Storage(err.to_string()))?;
        fs::write(path, contents).map_err(|err| CoreError::Storage(err.to_string()))?;
        Ok(())
    }

    /// Move an inbox item into the snoozed list.
    pub fn snooze_inbox_item(&self, id: Uuid) -> CoreResult<()> {
        let mut inbox = self.load_inbox()?;
        let mut snoozed = self.load_snoozed()?;
        if let Some(position) = inbox.iter().position(|change| change.id == id) {
            snoozed.push(inbox.remove(position));
            self.save_snoozed(&snoozed)?;
            self.save_inbox(&inbox)?;
        }
        Ok(())
    }

    /// Move a snoozed item back into the inbox.
    pub fn unsnooze_item(&self, id: Uuid) -> CoreResult<()> {
        let mut inbox = self.load_inbox()?;
        let mut snoozed = self.load_snoozed()?;
        if let Some(position) = snoozed.iter().position(|change| change.id == id) {
            inbox.push(snoozed.remove(position));
            self.save_snoozed(&snoozed)?;
            self.save_inbox(&inbox)?;
        }
        Ok(())
    }

    /// Remove a snoozed item from the list.
    pub fn remove_snoozed_item(&self, id: Uuid) -> CoreResult<()> {
        let mut snoozed = self.load_snoozed()?;
        snoozed.retain(|change| change.id != id);
        self.save_snoozed(&snoozed)
    }

    /// Load the last detector snapshot for a source.
    pub fn load_detector_snapshot(&self, source: &str) -> CoreResult<Vec<DetectedChange>> {
        let path = self.detector_snapshot_path(source);
        if !path.exists() {
            return Ok(Vec::new());
        }
        let contents = fs::read_to_string(&path)
            .map_err(|err| CoreError::Storage(err.to_string()))?;
        serde_yaml::from_str(&contents).map_err(|err| CoreError::Storage(err.to_string()))
    }

    /// Persist the detector snapshot for a source.
    pub fn save_detector_snapshot(&self, source: &str, changes: &[DetectedChange]) -> CoreResult<()> {
        let path = self.detector_snapshot_path(source);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| CoreError::Storage(err.to_string()))?;
        }
        let contents = serde_yaml::to_string(changes)
            .map_err(|err| CoreError::Storage(err.to_string()))?;
        fs::write(path, contents).map_err(|err| CoreError::Storage(err.to_string()))?;
        Ok(())
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct VaultConfig {
    path: Option<String>,
}

fn config_path() -> CoreResult<PathBuf> {
    if let Some(dir) = dirs::config_dir() {
        return Ok(dir.join(VAULT_DIR_NAME).join(CONFIG_FILE_NAME));
    }
    Err(CoreError::Storage(
        "unable to determine config directory".into(),
    ))
}

pub fn load_config() -> CoreResult<VaultConfig> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(VaultConfig::default());
    }
    let contents = fs::read_to_string(&path)
        .map_err(|err| CoreError::Storage(err.to_string()))?;
    serde_yaml::from_str(&contents).map_err(|err| CoreError::Storage(err.to_string()))
}

pub fn save_config(config: &VaultConfig) -> CoreResult<()> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| CoreError::Storage(err.to_string()))?;
    }
    let contents = serde_yaml::to_string(config)
        .map_err(|err| CoreError::Storage(err.to_string()))?;
    fs::write(path, contents).map_err(|err| CoreError::Storage(err.to_string()))?;
    Ok(())
}

pub fn set_config_path(path: &std::path::Path) -> CoreResult<()> {
    let config = VaultConfig {
        path: Some(path.to_string_lossy().to_string()),
    };
    save_config(&config)
}

pub fn resolve_vault_path() -> CoreResult<PathBuf> {
    if let Ok(value) = std::env::var("SETUPVAULT_PATH") {
        if !value.trim().is_empty() {
            return Ok(PathBuf::from(value));
        }
    }

    let config = load_config()?;
    if let Some(path) = config.path {
        if !path.trim().is_empty() {
            return Ok(PathBuf::from(path));
        }
    }

    FsVault::default_path()
}

impl VaultRepository for FsVault {
    fn list(&self) -> CoreResult<Vec<Entry>> {
        let entries_root = self.entries_root();
        if !entries_root.exists() {
            return Ok(Vec::new());
        }
        let mut entries = Vec::new();
        for entry in WalkDir::new(&entries_root).into_iter().filter_map(Result::ok) {
            if !entry.file_type().is_file() {
                continue;
            }
            if entry.path().extension().and_then(|ext| ext.to_str()) != Some("md") {
                continue;
            }
            let contents = fs::read_to_string(entry.path())
                .map_err(|err| CoreError::Storage(err.to_string()))?;
            let parsed = parse_entry(&contents)?;
            entries.push(parsed);
        }
        Ok(entries)
    }

    fn get(&self, id: Uuid) -> CoreResult<Option<Entry>> {
        let Some(path) = self.find_entry_path(id)? else {
            return Ok(None);
        };
        let contents = fs::read_to_string(&path)
            .map_err(|err| CoreError::Storage(err.to_string()))?;
        let entry = parse_entry(&contents)?;
        Ok(Some(entry))
    }

    fn create(&self, entry: &Entry) -> CoreResult<()> {
        let path = self.entry_path(entry);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| CoreError::Storage(err.to_string()))?;
        }
        let content = render_entry(entry)?;
        fs::write(path, content).map_err(|err| CoreError::Storage(err.to_string()))?;
        Ok(())
    }

    fn update(&self, entry: &Entry) -> CoreResult<()> {
        let existing = self.find_entry_path(entry.id)?;
        let path = existing.unwrap_or_else(|| self.entry_path(entry));
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| CoreError::Storage(err.to_string()))?;
        }
        let content = render_entry(entry)?;
        fs::write(path, content).map_err(|err| CoreError::Storage(err.to_string()))?;
        Ok(())
    }

    fn delete(&self, id: Uuid) -> CoreResult<()> {
        let Some(path) = self.find_entry_path(id)? else {
            return Ok(());
        };
        fs::remove_file(path).map_err(|err| CoreError::Storage(err.to_string()))?;
        Ok(())
    }
}

impl FsVault {
    /// Remove an entry and restore it to the inbox.
    pub fn restore_to_inbox(&self, id: Uuid) -> CoreResult<()> {
        let Some(entry) = self.get(id)? else {
            return Ok(());
        };
        
        let change = DetectedChange {
            id: Uuid::new_v4(), // Assign new ID for inbox instance
            path: None, // Path info is lost in Entry conversion unfortunately, or could be inferred
            title: entry.title,
            entry_type: entry.entry_type,
            source: entry.source,
            cmd: entry.cmd,
            system: entry.system,
            detected_at: entry.detected_at,
            tags: entry.tags,
        };

        self.delete(id)?;
        self.add_inbox_item(change)?;
        Ok(())
    }
}

/// Render an entry into Markdown with YAML frontmatter.
pub fn render_entry_markdown(entry: &Entry) -> CoreResult<String> {
    render_entry(entry)
}

#[derive(Debug, Deserialize, Serialize)]
struct Frontmatter {
    id: Uuid,
    title: String,
    #[serde(rename = "type")]
    entry_type: EntryType,
    source: String,
    cmd: String,
    system: SystemInfo,
    detected_at: DateTime<Utc>,
    status: EntryStatus,
    tags: Vec<String>,
}

fn render_entry(entry: &Entry) -> CoreResult<String> {
    let frontmatter = Frontmatter {
        id: entry.id,
        title: entry.title.clone(),
        entry_type: entry.entry_type.clone(),
        source: entry.source.clone(),
        cmd: entry.cmd.clone(),
        system: entry.system.clone(),
        detected_at: entry.detected_at,
        status: entry.status.clone(),
        tags: entry.tags.iter().map(|tag| tag.as_str().to_string()).collect(),
    };
    let yaml = serde_yaml::to_string(&frontmatter)
        .map_err(|err| CoreError::Storage(err.to_string()))?;
    let mut content = String::new();
    content.push_str("---\n");
    content.push_str(&yaml);
    content.push_str("---\n\n");
    content.push_str("# Rationale\n");
    content.push_str(entry.rationale.as_str());
    content.push_str("\n\n# Verification\n");
    if let Some(verification) = &entry.verification {
        content.push_str(verification);
    }
    content.push('\n');
    Ok(content)
}

fn parse_entry(contents: &str) -> CoreResult<Entry> {
    let frontmatter = parse_frontmatter(contents)?;
    let body = parse_body(contents)?;
    let rationale = extract_section(&body, "Rationale")
        .ok_or_else(|| CoreError::Storage("missing rationale section".into()))?;
    let rationale = Rationale::new(rationale)?;
    let verification = extract_section(&body, "Verification");

    let tags = frontmatter
        .tags
        .into_iter()
        .map(Tag::new)
        .collect::<CoreResult<Vec<_>>>()?;

    Entry::new(
        frontmatter.id,
        frontmatter.title,
        frontmatter.entry_type,
        frontmatter.source,
        frontmatter.cmd,
        frontmatter.system,
        frontmatter.detected_at,
        frontmatter.status,
        tags,
        rationale,
        verification,
    )
}

fn parse_frontmatter(contents: &str) -> CoreResult<Frontmatter> {
    let (frontmatter, _) = split_frontmatter(contents)?;
    serde_yaml::from_str(frontmatter).map_err(|err| CoreError::Storage(err.to_string()))
}

fn parse_body(contents: &str) -> CoreResult<String> {
    let (_, body) = split_frontmatter(contents)?;
    Ok(body.to_string())
}

fn split_frontmatter(contents: &str) -> CoreResult<(&str, &str)> {
    let header = "---\n";
    if !contents.starts_with(header) {
        return Err(CoreError::Storage("missing frontmatter header".into()));
    }

    let marker = "\n---\n";
    let remainder = &contents[header.len()..];
    let end = remainder
        .find(marker)
        .ok_or_else(|| CoreError::Storage("unterminated frontmatter".into()))?;

    let frontmatter = &remainder[..end];
    let body_start = end + marker.len();
    let body = &remainder[body_start..];
    Ok((frontmatter.trim_end(), body.trim_start()))
}

fn extract_section(body: &str, heading: &str) -> Option<String> {
    let mut lines = body.lines();
    while let Some(line) = lines.next() {
        if line.trim() == format!("# {heading}") {
            let mut section = Vec::new();
            for line in lines.by_ref() {
                if line.trim_start().starts_with("# ") {
                    break;
                }
                section.push(line);
            }
            return Some(section.join("\n").trim().to_string());
        }
    }
    None
}

fn slugify(input: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = false;
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            slug.push('-');
            last_dash = true;
        }
    }
    slug.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn round_trip_entry() {
        let temp = TempDir::new().expect("temp dir");
        let vault = FsVault::new(temp.path().to_path_buf());
        let entry = Entry::new(
            Uuid::new_v4(),
            "jq",
            EntryType::Package,
            "homebrew",
            "brew install jq",
            SystemInfo {
                os: "macos".into(),
                arch: "arm64".into(),
            },
            Utc::now(),
            EntryStatus::Active,
            vec![Tag::new("cli").unwrap()],
            Rationale::new("json parsing").unwrap(),
            Some("jq --version".into()),
        )
        .unwrap();

        vault.create(&entry).expect("create entry");
        let fetched = vault.get(entry.id).expect("get entry");
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().title, "jq");
    }
}
