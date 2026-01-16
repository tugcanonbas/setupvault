use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use chrono::Utc;
use uuid::Uuid;

use sv_core::{
    DetectedChange, Entry, EntryStatus, EntryType, Rationale, SystemInfo, Tag, VaultRepository,
};
use sv_detectors::{default_detectors, run_detectors};
use sv_fs::{render_entry_markdown, resolve_vault_path, set_config_path, FsVault};

#[derive(Parser)]
#[command(name = "sv", version, about = "SetupVault CLI")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Initialize the vault.
    Init {
        /// Optional path to initialize the vault at.
        #[arg(long)]
        path: Option<String>,
    },
    /// Capture a change and require rationale.
    Capture {
        /// Optional title for quick capture.
        title: Option<String>,
        /// Provide rationale without an interactive prompt.
        #[arg(long)]
        rationale: String,
        /// Entry type for capture.
        #[arg(long, value_enum, default_value = "other")]
        entry_type: EntryTypeArg,
        /// Source label for the entry.
        #[arg(long, default_value = "manual")]
        source: String,
        /// Reproduction command.
        #[arg(long)]
        cmd: Option<String>,
        /// Tags for the entry.
        #[arg(long)]
        tag: Vec<String>,
        /// Optional verification guidance.
        #[arg(long)]
        verification: Option<String>,
    },
    /// List detected changes waiting for action.
    Inbox {
        /// Refresh the inbox by running detectors.
        #[arg(long)]
        refresh: bool,
    },
    /// Approve a detected change by id.
    Approve {
        id: String,
        #[arg(long)]
        rationale: String,
        #[arg(long)]
        tag: Vec<String>,
        #[arg(long)]
        verification: Option<String>,
    },
    /// Snooze a detected change by id.
    Snooze { id: String },
    /// Ignore a detected change by id.
    Ignore { id: String },
    /// Restore a snoozed change to the inbox.
    Unsnooze { id: String },
    /// List entries in the vault.
    List,
    /// Show a single entry by id.
    Show { id: String },
    /// Search entries by query.
    Search { query: String },
    /// Export entries to a directory.
    Export { path: String },
}

#[derive(Clone, ValueEnum)]
enum EntryTypeArg {
    Package,
    Config,
    Application,
    Script,
    Other,
}

impl From<EntryTypeArg> for EntryType {
    fn from(value: EntryTypeArg) -> Self {
        match value {
            EntryTypeArg::Package => EntryType::Package,
            EntryTypeArg::Config => EntryType::Config,
            EntryTypeArg::Application => EntryType::Application,
            EntryTypeArg::Script => EntryType::Script,
            EntryTypeArg::Other => EntryType::Other,
        }
    }
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();

    let command = match cli.command {
        Some(c) => c,
        None => return sv_tui::run(),
    };

    if let Command::Init { path } = &command {
        let path = path
            .clone()
            .map(std::path::PathBuf::from)
            .unwrap_or(FsVault::default_path()?);
        let vault = FsVault::new(path.clone());
        vault.init().context("failed to initialize vault")?;
        set_config_path(&path)?;
        println!("Vault initialized at {}", path.display());
        return Ok(());
    }

    let vault = FsVault::new(resolve_vault_path()?);
    if !vault.exists() {
        return Err(anyhow!(
            "SetupVault is not initialized. Run `setupvault init` to get started."
        ));
    }

    match command {
        Command::Capture {
            title,
            rationale,
            entry_type,
            source,
            cmd,
            tag,
            verification,
        } => capture_entry(
            &vault,
            title,
            rationale,
            entry_type.into(),
            source,
            cmd,
            tag,
            verification,
        ),
        Command::Inbox { refresh } => inbox(&vault, refresh),
        Command::Approve {
            id,
            rationale,
            tag,
            verification,
        } => approve(&vault, &id, rationale, tag, verification),
        Command::Snooze { id } => snooze(&vault, &id),
        Command::Ignore { id } => ignore(&vault, &id),
        Command::Unsnooze { id } => unsnooze(&vault, &id),
        Command::List => list_entries(&vault),
        Command::Show { id } => show_entry(&vault, &id),
        Command::Search { query } => search_entries(&vault, &query),
        Command::Export { path } => export_entries(&vault, &path),
        Command::Init { .. } => unreachable!("handled above"),
    }
}

#[allow(clippy::too_many_arguments)]
fn capture_entry(
    vault: &FsVault,
    title: Option<String>,
    rationale: String,
    entry_type: EntryType,
    source: String,
    cmd: Option<String>,
    tags: Vec<String>,
    verification: Option<String>,
) -> Result<()> {
    let title = title.unwrap_or_else(|| "Untitled".to_string());
    let rationale = Rationale::new(rationale).context("invalid rationale")?;
    let tags = parse_tags(tags)?;
    let cmd = cmd.unwrap_or_else(|| "manual entry".to_string());
    let entry = Entry::new(
        Uuid::new_v4(),
        title,
        entry_type,
        source,
        cmd,
        SystemInfo {
            os: std::env::consts::OS.into(),
            arch: std::env::consts::ARCH.into(),
        },
        Utc::now(),
        EntryStatus::Active,
        tags,
        rationale,
        verification,
    )
    .context("invalid entry")?;

    vault.create(&entry).context("failed to write entry")?;
    Ok(())
}

fn inbox(vault: &FsVault, refresh: bool) -> Result<()> {
    if refresh {
        let detectors = default_detectors();

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .context("failed to initialize runtime")?;
        let changes = runtime
            .block_on(run_detectors(detectors))
            .context("detector run failed")?;

        let mut inbox = vault.load_inbox().context("failed to load inbox")?;
        let mut new_changes = Vec::new();
        for (source, group) in group_by_source(&changes) {
            let previous = vault.load_detector_snapshot(&source)?;
            let diff = diff_changes(&previous, &group);
            vault.save_detector_snapshot(&source, &group)?;
            new_changes.extend(diff);
        }

        if !new_changes.is_empty() {
            append_unique(&mut inbox, new_changes);
            vault.save_inbox(&inbox).context("failed to save inbox")?;
        }
    }

    let inbox = vault.load_inbox().context("failed to load inbox")?;
    if inbox.is_empty() {
        return Ok(());
    }

    for change in inbox {
        println!(
            "{}\t{}\t{}\t{}",
            change.id, change.title, change.source, change.cmd
        );
    }
    Ok(())
}

fn approve(
    vault: &FsVault,
    id: &str,
    rationale: String,
    tags: Vec<String>,
    verification: Option<String>,
) -> Result<()> {
    let id = Uuid::parse_str(id).context("invalid id")?;
    let inbox = vault.load_inbox().context("failed to load inbox")?;
    let change = inbox
        .into_iter()
        .find(|change| change.id == id)
        .ok_or_else(|| anyhow!("change not found"))?;

    if let Some(path) = change.path.as_ref() {
        if let Ok(contents) = std::fs::read_to_string(path) {
            if sv_utils::contains_potential_secret(&contents) {
                eprintln!("warning: potential secret detected in {path}");
            }
        }
    }

    let entry = Entry::new(
        Uuid::new_v4(),
        change.title,
        change.entry_type,
        change.source,
        change.cmd,
        change.system,
        change.detected_at,
        EntryStatus::Active,
        parse_tags(tags)?,
        Rationale::new(rationale)?,
        verification,
    )?;

    vault.create(&entry).context("failed to write entry")?;
    vault.remove_inbox_item(id).context("failed to update inbox")?;
    Ok(())
}

fn snooze(vault: &FsVault, id: &str) -> Result<()> {
    let id = Uuid::parse_str(id).context("invalid id")?;
    vault.snooze_inbox_item(id).context("failed to snooze")?;
    Ok(())
}

fn ignore(vault: &FsVault, id: &str) -> Result<()> {
    let id = Uuid::parse_str(id).context("invalid id")?;
    vault.remove_inbox_item(id).context("failed to ignore")?;
    Ok(())
}

fn unsnooze(vault: &FsVault, id: &str) -> Result<()> {
    let id = Uuid::parse_str(id).context("invalid id")?;
    vault.unsnooze_item(id).context("failed to unsnooze")?;
    Ok(())
}

fn list_entries(vault: &FsVault) -> Result<()> {
    let entries = vault.list().context("failed to list entries")?;
    for entry in entries {
        println!("{}\t{}\t{}", entry.id, entry.title, entry.source);
    }
    Ok(())
}

fn show_entry(vault: &FsVault, id: &str) -> Result<()> {
    let id = Uuid::parse_str(id).context("invalid id")?;
    let entry = vault.get(id).context("failed to get entry")?;
    if let Some(entry) = entry {
        let markdown = render_entry_markdown(&entry).context("failed to render entry")?;
        println!("{markdown}");
    }
    Ok(())
}

fn search_entries(vault: &FsVault, query: &str) -> Result<()> {
    let entries = vault.list().context("failed to list entries")?;
    let query = query.to_lowercase();
    for entry in entries.into_iter().filter(|entry| {
        entry.title.to_lowercase().contains(&query)
            || entry
                .tags
                .iter()
                .any(|tag| tag.as_str().to_lowercase().contains(&query))
            || entry.rationale.as_str().to_lowercase().contains(&query)
    }) {
        println!("{}\t{}\t{}", entry.id, entry.title, entry.source);
    }
    Ok(())
}

fn export_entries(vault: &FsVault, path: &str) -> Result<()> {
    let target = std::path::PathBuf::from(path);
    if !target.exists() {
        std::fs::create_dir_all(&target).context("failed to create export directory")?;
    }

    let entries = vault.list().context("failed to list entries")?;
    for entry in entries {
        let file_name = sanitize_export_filename(&entry.title, entry.id);
        let dest = target.join(file_name);
        let content = render_entry_markdown(&entry).context("failed to render entry")?;
        std::fs::write(dest, content).context("failed to export entry")?;
    }
    Ok(())
}

fn parse_tags(tags: Vec<String>) -> Result<Vec<Tag>> {
    tags.into_iter()
        .map(|tag| Tag::new(tag).map_err(|err| anyhow!(err.to_string())))
        .collect()
}

fn sanitize_export_filename(title: &str, id: Uuid) -> String {
    let slug = slugify(title);
    let slug = if slug.is_empty() { "entry" } else { slug.as_str() };
    format!("{slug}-{id}.md")
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

fn diff_changes(previous: &[DetectedChange], current: &[DetectedChange]) -> Vec<DetectedChange> {
    let previous_keys: std::collections::HashSet<_> = previous
        .iter()
        .map(|change| (change.source.clone(), change.title.clone()))
        .collect();
    current
        .iter()
        .filter(|change| !previous_keys.contains(&(change.source.clone(), change.title.clone())))
        .cloned()
        .collect()
}

fn append_unique(target: &mut Vec<DetectedChange>, incoming: Vec<DetectedChange>) {
    let mut seen: std::collections::HashSet<_> = target
        .iter()
        .map(|change| (change.source.clone(), change.title.clone()))
        .collect();
    for change in incoming {
        let key = (change.source.clone(), change.title.clone());
        if seen.insert(key) {
            target.push(change);
        }
    }
}

fn group_by_source(
    changes: &[DetectedChange],
) -> std::collections::BTreeMap<String, Vec<DetectedChange>> {
    let mut map = std::collections::BTreeMap::new();
    for change in changes {
        map.entry(change.source.clone())
            .or_insert_with(Vec::new)
            .push(change.clone());
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_help_snapshot() {
        let mut cmd = Cli::command();
        let mut buffer = Vec::new();
        cmd.write_long_help(&mut buffer).expect("help output");
        let help = String::from_utf8(buffer).expect("utf8 help");
        insta::assert_snapshot!(help);
    }
}
