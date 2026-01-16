//! Change detection strategies for SetupVault.

use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

use chrono::Utc;
use sv_core::{CoreError, CoreResult, DetectedChange, Detector, EntryType, SystemInfo, Tag};

/// Detect Homebrew package changes.
#[derive(Debug, Default)]
pub struct BrewDetector;

impl BrewDetector {
    /// Create a new Homebrew detector.
    pub fn new() -> Self {
        Self
    }
}

impl Detector for BrewDetector {
    fn name(&self) -> &'static str {
        "homebrew"
    }

    fn scan(&self) -> CoreResult<Vec<DetectedChange>> {
        let system = default_system();
        let now = Utc::now();
        let package_tag = Tag::new("package")?;
        let app_tag = Tag::new("application")?;
        let mut changes = Vec::new();

        // Formulae
        if let Ok(output) = run_command("brew", &["list", "--formula"]) {
            for line in output.lines().map(str::trim).filter(|line| !line.is_empty()) {
                changes.push(DetectedChange {
                    id: uuid::Uuid::new_v4(),
                    path: None,
                    title: line.to_string(),
                    entry_type: EntryType::Package,
                    source: "homebrew".into(),
                    cmd: format!("brew install {line}"),
                    system: system.clone(),
                    detected_at: now,
                    tags: vec![package_tag.clone()],
                });
            }
        }

        // Casks
        if let Ok(output) = run_command("brew", &["list", "--cask"]) {
            for line in output.lines().map(str::trim).filter(|line| !line.is_empty()) {
                changes.push(DetectedChange {
                    id: uuid::Uuid::new_v4(),
                    path: None,
                    title: line.to_string(),
                    entry_type: EntryType::Application,
                    source: "homebrew".into(),
                    cmd: format!("brew install --cask {line}"),
                    system: system.clone(),
                    detected_at: now,
                    tags: vec![app_tag.clone()],
                });
            }
        }

        Ok(changes)
    }
}

/// Detect global npm package changes.
#[derive(Debug, Default)]
pub struct NpmDetector;

impl NpmDetector {
    /// Create a new npm detector.
    pub fn new() -> Self {
        Self
    }
}

impl Detector for NpmDetector {
    fn name(&self) -> &'static str {
        "npm"
    }

    fn scan(&self) -> CoreResult<Vec<DetectedChange>> {
        let output = run_command("npm", &["list", "-g", "--depth=0", "--parseable"])?;
        let system = default_system();
        let now = Utc::now();
        let tag = Tag::new("package")?;

        let mut lines = output.lines();
        let _root = lines.next();
        let mut changes = Vec::new();
        for line in lines.map(str::trim).filter(|line| !line.is_empty()) {
            let name = line
                .rsplit('/')
                .next()
                .unwrap_or(line)
                .to_string();
            changes.push(DetectedChange {
                id: uuid::Uuid::new_v4(),
                path: None,
                title: name.clone(),
                entry_type: EntryType::Package,
                source: "npm".into(),
                cmd: format!("npm install -g {name}"),
                system: system.clone(),
                detected_at: now,
                tags: vec![tag.clone()],
            });
        }
        Ok(changes)
    }
}

/// Detect cargo-installed crates.
#[derive(Debug, Default)]
pub struct CargoDetector;

impl CargoDetector {
    /// Create a new cargo detector.
    pub fn new() -> Self {
        Self
    }
}

impl Detector for CargoDetector {
    fn name(&self) -> &'static str {
        "cargo"
    }

    fn scan(&self) -> CoreResult<Vec<DetectedChange>> {
        let output = run_command("cargo", &["install", "--list"])?;
        let system = default_system();
        let now = Utc::now();
        let tag = Tag::new("package")?;

        let mut changes = Vec::new();
        for line in output.lines().map(str::trim) {
            if line.is_empty() || line.starts_with(" ") {
                continue;
            }
            let name = line.split_whitespace().next().unwrap_or(line).to_string();
            changes.push(DetectedChange {
                id: uuid::Uuid::new_v4(),
                path: None,
                title: name.clone(),
                entry_type: EntryType::Package,
                source: "cargo".into(),
                cmd: format!("cargo install {name}"),
                system: system.clone(),
                detected_at: now,
                tags: vec![tag.clone()],
            });
        }
        Ok(changes)
    }
}

/// Detect pip-installed packages.
#[derive(Debug, Default)]
pub struct PipDetector;

impl PipDetector {
    /// Create a new pip detector.
    pub fn new() -> Self {
        Self
    }
}

impl Detector for PipDetector {
    fn name(&self) -> &'static str {
        "pip"
    }

    fn scan(&self) -> CoreResult<Vec<DetectedChange>> {
        let output = run_command("pip", &["list", "--format=freeze"])?;
        let system = default_system();
        let now = Utc::now();
        let tag = Tag::new("package")?;

        let mut changes = Vec::new();
        for line in output.lines().map(str::trim) {
            if line.is_empty() {
                continue;
            }
            let name = line.split("==").next().unwrap_or(line).to_string();
            changes.push(DetectedChange {
                id: uuid::Uuid::new_v4(),
                path: None,
                title: name.clone(),
                entry_type: EntryType::Package,
                source: "pip".into(),
                cmd: format!("pip install {name}"),
                system: system.clone(),
                detected_at: now,
                tags: vec![tag.clone()],
            });
        }
        Ok(changes)
    }
}

/// Detect watched dotfile changes.
#[derive(Debug)]
pub struct DotfileDetector {
    paths: Vec<PathBuf>,
}

impl DotfileDetector {
    /// Create a dotfile detector with an explicit list of paths.
    pub fn new(paths: Vec<PathBuf>) -> Self {
        Self { paths }
    }

    /// Default dotfile paths for macOS and Linux environments.
    pub fn default_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join(".zshrc"));
            paths.push(home.join(".gitconfig"));
            paths.push(home.join(".vimrc"));
        }
        paths
    }
}

impl Detector for DotfileDetector {
    fn name(&self) -> &'static str {
        "dotfiles"
    }

    fn scan(&self) -> CoreResult<Vec<DetectedChange>> {
        let system = default_system();
        let now = Utc::now();
        let tag = Tag::new("config")?;
        let mut changes = Vec::new();
        for path in &self.paths {
            let title = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("dotfile")
                .to_string();
            changes.push(DetectedChange {
                id: uuid::Uuid::new_v4(),
                path: Some(path.display().to_string()),
                title,
                entry_type: EntryType::Config,
                source: "dotfiles".into(),
                cmd: format!("open {}", path.display()),
                system: system.clone(),
                detected_at: now,
                tags: vec![tag.clone()],
            });
        }
        Ok(changes)
    }
}

/// Detect macOS defaults changes.
#[derive(Debug, Default)]
pub struct MacDefaultsDetector;

impl MacDefaultsDetector {
    /// Create a new macOS defaults detector.
    pub fn new() -> Self {
        Self
    }
}

impl Detector for MacDefaultsDetector {
    fn name(&self) -> &'static str {
        "mac_defaults"
    }

    fn scan(&self) -> CoreResult<Vec<DetectedChange>> {
        if std::env::consts::OS != "macos" {
            return Ok(Vec::new());
        }
        let output = run_command("defaults", &["domains"])?;
        let system = default_system();
        let now = Utc::now();
        let tag = Tag::new("config")?;
        let domains = output
            .split(',')
            .map(str::trim)
            .filter(|domain| !domain.is_empty());

        let mut changes = Vec::new();
        for domain in domains {
            changes.push(DetectedChange {
                id: uuid::Uuid::new_v4(),
                path: None,
                title: domain.to_string(),
                entry_type: EntryType::Config,
                source: "mac_defaults".into(),
                cmd: format!("defaults read {domain}"),
                system: system.clone(),
                detected_at: now,
                tags: vec![tag.clone()],
            });
        }
        Ok(changes)
    }
}

/// Detect installed macOS applications.
#[derive(Debug, Default)]
pub struct AppDetector;

impl AppDetector {
    /// Create a new application detector.
    pub fn new() -> Self {
        Self
    }
}

impl Detector for AppDetector {
    fn name(&self) -> &'static str {
        "applications"
    }

    fn scan(&self) -> CoreResult<Vec<DetectedChange>> {
        if std::env::consts::OS != "macos" {
            return Ok(Vec::new());
        }

        let mut changes = Vec::new();
        let app_dir = std::path::Path::new("/Applications");
        
        if !app_dir.exists() {
            return Ok(Vec::new());
        }

        // Get list of brew casks to avoid duplicate attribution
        let brew_casks: std::collections::HashSet<String> = run_command("brew", &["list", "--cask"])
            .unwrap_or_default()
            .lines()
            .map(|s| normalize_name(s.trim()))
            .collect();

        let system = default_system();
        let now = Utc::now();
        let tag = Tag::new("application")?;

        if let Ok(entries) = std::fs::read_dir(app_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && path.extension().and_then(|s| s.to_str()) == Some("app") {
                    let name = path.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Unknown App")
                        .to_string();
                    
                    // Simple heuristic to check if it's a brew cask
                    let normalized_name = normalize_name(&name);
                    if brew_casks.contains(&normalized_name) {
                        continue;
                    }

                    changes.push(DetectedChange {
                        id: uuid::Uuid::new_v4(),
                        path: Some(path.display().to_string()),
                        title: name,
                        entry_type: EntryType::Application,
                        source: "applications".into(),
                        cmd: format!("open \"{}\"", path.display()),
                        system: system.clone(),
                        detected_at: now,
                        tags: vec![tag.clone()],
                    });
                }
            }
        }
        
        Ok(changes)
    }
}

/// Detect apt/dpkg installed packages.
#[derive(Debug, Default)]
pub struct AptDetector;

impl AptDetector {
    /// Create a new apt detector.
    pub fn new() -> Self {
        Self
    }
}

impl Detector for AptDetector {
    fn name(&self) -> &'static str {
        "apt"
    }

    fn scan(&self) -> CoreResult<Vec<DetectedChange>> {
        if std::env::consts::OS != "linux" {
            return Ok(Vec::new());
        }
        let output = run_command("dpkg-query", &["-W", "-f=${binary:Package}\n"])?;
        let system = default_system();
        let now = Utc::now();
        let tag = Tag::new("package")?;

        let changes = output
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(|name| DetectedChange {
                id: uuid::Uuid::new_v4(),
                path: None,
                title: name.to_string(),
                entry_type: EntryType::Package,
                source: "apt".into(),
                cmd: format!("sudo apt-get install {name}"),
                system: system.clone(),
                detected_at: now,
                tags: vec![tag.clone()],
            })
            .collect();

        Ok(changes)
    }
}

/// Detect dnf installed packages.
#[derive(Debug, Default)]
pub struct DnfDetector;

impl DnfDetector {
    /// Create a new dnf detector.
    pub fn new() -> Self {
        Self
    }
}

impl Detector for DnfDetector {
    fn name(&self) -> &'static str {
        "dnf"
    }

    fn scan(&self) -> CoreResult<Vec<DetectedChange>> {
        if std::env::consts::OS != "linux" {
            return Ok(Vec::new());
        }
        let output = run_command("dnf", &["list", "installed"])?;
        parse_rpm_list(&output, "dnf")
    }
}

/// Detect yum installed packages.
#[derive(Debug, Default)]
pub struct YumDetector;

impl YumDetector {
    /// Create a new yum detector.
    pub fn new() -> Self {
        Self
    }
}

impl Detector for YumDetector {
    fn name(&self) -> &'static str {
        "yum"
    }

    fn scan(&self) -> CoreResult<Vec<DetectedChange>> {
        if std::env::consts::OS != "linux" {
            return Ok(Vec::new());
        }
        let output = run_command("yum", &["list", "installed"])?;
        parse_rpm_list(&output, "yum")
    }
}

/// Detect pacman installed packages.
#[derive(Debug, Default)]
pub struct PacmanDetector;

impl PacmanDetector {
    /// Create a new pacman detector.
    pub fn new() -> Self {
        Self
    }
}

impl Detector for PacmanDetector {
    fn name(&self) -> &'static str {
        "pacman"
    }

    fn scan(&self) -> CoreResult<Vec<DetectedChange>> {
        if std::env::consts::OS != "linux" {
            return Ok(Vec::new());
        }
        let output = run_command("pacman", &["-Qq"])?;
        let system = default_system();
        let now = Utc::now();
        let tag = Tag::new("package")?;

        let changes = output
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(|name| DetectedChange {
                id: uuid::Uuid::new_v4(),
                path: None,
                title: name.to_string(),
                entry_type: EntryType::Package,
                source: "pacman".into(),
                cmd: format!("sudo pacman -S {name}"),
                system: system.clone(),
                detected_at: now,
                tags: vec![tag.clone()],
            })
            .collect();

        Ok(changes)
    }
}

/// Detect flatpak installed applications.
#[derive(Debug, Default)]
pub struct FlatpakDetector;

impl FlatpakDetector {
    /// Create a new flatpak detector.
    pub fn new() -> Self {
        Self
    }
}

impl Detector for FlatpakDetector {
    fn name(&self) -> &'static str {
        "flatpak"
    }

    fn scan(&self) -> CoreResult<Vec<DetectedChange>> {
        if std::env::consts::OS != "linux" {
            return Ok(Vec::new());
        }
        let output = run_command("flatpak", &["list", "--app", "--columns=application"])?;
        let system = default_system();
        let now = Utc::now();
        let tag = Tag::new("application")?;

        let changes = output
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(|name| DetectedChange {
                id: uuid::Uuid::new_v4(),
                path: None,
                title: name.to_string(),
                entry_type: EntryType::Application,
                source: "flatpak".into(),
                cmd: format!("flatpak install {name}"),
                system: system.clone(),
                detected_at: now,
                tags: vec![tag.clone()],
            })
            .collect();

        Ok(changes)
    }
}

/// Detect snap installed applications.
#[derive(Debug, Default)]
pub struct SnapDetector;

impl SnapDetector {
    /// Create a new snap detector.
    pub fn new() -> Self {
        Self
    }
}

impl Detector for SnapDetector {
    fn name(&self) -> &'static str {
        "snap"
    }

    fn scan(&self) -> CoreResult<Vec<DetectedChange>> {
        if std::env::consts::OS != "linux" {
            return Ok(Vec::new());
        }
        let output = run_command("snap", &["list"])?;
        let system = default_system();
        let now = Utc::now();
        let tag = Tag::new("application")?;

        let mut changes = Vec::new();
        for line in output.lines().map(str::trim) {
            if line.is_empty() || line.to_lowercase().starts_with("name") {
                continue;
            }
            let name = line.split_whitespace().next().unwrap_or(line);
            if name.is_empty() {
                continue;
            }
            changes.push(DetectedChange {
                id: uuid::Uuid::new_v4(),
                path: None,
                title: name.to_string(),
                entry_type: EntryType::Application,
                source: "snap".into(),
                cmd: format!("sudo snap install {name}"),
                system: system.clone(),
                detected_at: now,
                tags: vec![tag.clone()],
            });
        }

        Ok(changes)
    }
}

/// Detect winget packages.
#[derive(Debug, Default)]
pub struct WingetDetector;

impl WingetDetector {
    /// Create a new winget detector.
    pub fn new() -> Self {
        Self
    }
}

impl Detector for WingetDetector {
    fn name(&self) -> &'static str {
        "winget"
    }

    fn scan(&self) -> CoreResult<Vec<DetectedChange>> {
        if std::env::consts::OS != "windows" {
            return Ok(Vec::new());
        }
        let output = run_command("winget", &["list", "--source", "winget"])?;
        parse_winget_list(&output, "winget")
    }
}

/// Detect Microsoft Store packages via winget.
#[derive(Debug, Default)]
pub struct WingetStoreDetector;

impl WingetStoreDetector {
    /// Create a new winget store detector.
    pub fn new() -> Self {
        Self
    }
}

impl Detector for WingetStoreDetector {
    fn name(&self) -> &'static str {
        "msstore"
    }

    fn scan(&self) -> CoreResult<Vec<DetectedChange>> {
        if std::env::consts::OS != "windows" {
            return Ok(Vec::new());
        }
        let output = run_command("winget", &["list", "--source", "msstore"])?;
        parse_winget_list(&output, "msstore")
    }
}

/// Detect Chocolatey packages.
#[derive(Debug, Default)]
pub struct ChocolateyDetector;

impl ChocolateyDetector {
    /// Create a new Chocolatey detector.
    pub fn new() -> Self {
        Self
    }
}

impl Detector for ChocolateyDetector {
    fn name(&self) -> &'static str {
        "chocolatey"
    }

    fn scan(&self) -> CoreResult<Vec<DetectedChange>> {
        if std::env::consts::OS != "windows" {
            return Ok(Vec::new());
        }
        let output = run_command("choco", &["list", "-l"])?;
        let system = default_system();
        let now = Utc::now();
        let tag = Tag::new("package")?;

        let mut changes = Vec::new();
        for line in output.lines().map(str::trim) {
            if line.is_empty()
                || line.to_lowercase().starts_with("chocolatey")
                || line.to_lowercase().contains("packages installed")
            {
                continue;
            }
            let name = line.split_whitespace().next().unwrap_or(line);
            if name.is_empty() {
                continue;
            }
            changes.push(DetectedChange {
                id: uuid::Uuid::new_v4(),
                path: None,
                title: name.to_string(),
                entry_type: EntryType::Package,
                source: "chocolatey".into(),
                cmd: format!("choco install {name} -y"),
                system: system.clone(),
                detected_at: now,
                tags: vec![tag.clone()],
            });
        }

        Ok(changes)
    }
}

/// Detect Scoop packages.
#[derive(Debug, Default)]
pub struct ScoopDetector;

impl ScoopDetector {
    /// Create a new Scoop detector.
    pub fn new() -> Self {
        Self
    }
}

impl Detector for ScoopDetector {
    fn name(&self) -> &'static str {
        "scoop"
    }

    fn scan(&self) -> CoreResult<Vec<DetectedChange>> {
        if std::env::consts::OS != "windows" {
            return Ok(Vec::new());
        }
        let output = run_command("scoop", &["list"])?;
        let system = default_system();
        let now = Utc::now();
        let tag = Tag::new("package")?;

        let mut changes = Vec::new();
        for line in output.lines().map(str::trim) {
            if line.is_empty()
                || line.to_lowercase().starts_with("installed")
                || line.to_lowercase().starts_with("name")
            {
                continue;
            }
            let name = line.split_whitespace().next().unwrap_or(line);
            if name.is_empty() {
                continue;
            }
            changes.push(DetectedChange {
                id: uuid::Uuid::new_v4(),
                path: None,
                title: name.to_string(),
                entry_type: EntryType::Package,
                source: "scoop".into(),
                cmd: format!("scoop install {name}"),
                system: system.clone(),
                detected_at: now,
                tags: vec![tag.clone()],
            });
        }

        Ok(changes)
    }
}

/// Detect Windows applications installed under Program Files.
#[derive(Debug, Default)]
pub struct ProgramFilesDetector;

impl ProgramFilesDetector {
    /// Create a new Program Files detector.
    pub fn new() -> Self {
        Self
    }
}

impl Detector for ProgramFilesDetector {
    fn name(&self) -> &'static str {
        "program_files"
    }

    fn scan(&self) -> CoreResult<Vec<DetectedChange>> {
        if std::env::consts::OS != "windows" {
            return Ok(Vec::new());
        }
        let system = default_system();
        let now = Utc::now();
        let tag = Tag::new("application")?;

        let mut changes = Vec::new();
        let roots = [
            std::path::PathBuf::from(r"C:\Program Files"),
            std::path::PathBuf::from(r"C:\Program Files (x86)"),
        ];

        for root in roots {
            if let Ok(entries) = std::fs::read_dir(&root) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !path.is_dir() {
                        continue;
                    }
                    let name = path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Application")
                        .to_string();
                    changes.push(DetectedChange {
                        id: uuid::Uuid::new_v4(),
                        path: Some(path.display().to_string()),
                        title: name,
                        entry_type: EntryType::Application,
                        source: "applications".into(),
                        cmd: format!("start \"\" \"{}\"", path.display()),
                        system: system.clone(),
                        detected_at: now,
                        tags: vec![tag.clone()],
                    });
                }
            }
        }
        Ok(changes)
    }
}

/// Detect Linux desktop applications from .desktop files.
#[derive(Debug, Default)]
pub struct DesktopAppDetector;

impl DesktopAppDetector {
    /// Create a new desktop application detector.
    pub fn new() -> Self {
        Self
    }
}

impl Detector for DesktopAppDetector {
    fn name(&self) -> &'static str {
        "applications"
    }

    fn scan(&self) -> CoreResult<Vec<DetectedChange>> {
        if std::env::consts::OS != "linux" {
            return Ok(Vec::new());
        }

        let system = default_system();
        let now = Utc::now();
        let tag = Tag::new("application")?;
        let mut changes = Vec::new();

        let mut dirs = vec![PathBuf::from("/usr/share/applications")];
        if let Some(home) = dirs::home_dir() {
            dirs.push(home.join(".local/share/applications"));
        }

        for dir in dirs {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) != Some("desktop") {
                        continue;
                    }
                    let title = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Application")
                        .to_string();
                    let desktop_id = path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or(&title)
                        .to_string();
                    changes.push(DetectedChange {
                        id: uuid::Uuid::new_v4(),
                        path: Some(path.display().to_string()),
                        title,
                        entry_type: EntryType::Application,
                        source: "applications".into(),
                        cmd: format!("gtk-launch {desktop_id}"),
                        system: system.clone(),
                        detected_at: now,
                        tags: vec![tag.clone()],
                    });
                }
            }
        }

        Ok(changes)
    }
}

/// Build the default detector list for the current OS.
pub fn default_detectors() -> Vec<Arc<dyn Detector + Send + Sync>> {
    let os = std::env::consts::OS;
    let mut detectors: Vec<Arc<dyn Detector + Send + Sync>> = Vec::new();

    match os {
        "macos" => {
            detectors.push(Arc::new(BrewDetector::new()));
            detectors.push(Arc::new(NpmDetector::new()));
            detectors.push(Arc::new(CargoDetector::new()));
            detectors.push(Arc::new(PipDetector::new()));
            detectors.push(Arc::new(DotfileDetector::new(DotfileDetector::default_paths())));
            detectors.push(Arc::new(MacDefaultsDetector::new()));
            detectors.push(Arc::new(AppDetector::new()));
        }
        "linux" => {
            detectors.push(Arc::new(AptDetector::new()));
            detectors.push(Arc::new(DnfDetector::new()));
            detectors.push(Arc::new(YumDetector::new()));
            detectors.push(Arc::new(PacmanDetector::new()));
            detectors.push(Arc::new(FlatpakDetector::new()));
            detectors.push(Arc::new(SnapDetector::new()));
            detectors.push(Arc::new(DesktopAppDetector::new()));
            detectors.push(Arc::new(NpmDetector::new()));
            detectors.push(Arc::new(CargoDetector::new()));
            detectors.push(Arc::new(PipDetector::new()));
            detectors.push(Arc::new(DotfileDetector::new(DotfileDetector::default_paths())));
        }
        "windows" => {
            detectors.push(Arc::new(WingetDetector::new()));
            detectors.push(Arc::new(WingetStoreDetector::new()));
            detectors.push(Arc::new(ChocolateyDetector::new()));
            detectors.push(Arc::new(ScoopDetector::new()));
            detectors.push(Arc::new(ProgramFilesDetector::new()));
            detectors.push(Arc::new(NpmDetector::new()));
            detectors.push(Arc::new(CargoDetector::new()));
            detectors.push(Arc::new(PipDetector::new()));
        }
        _ => {
            detectors.push(Arc::new(NpmDetector::new()));
            detectors.push(Arc::new(CargoDetector::new()));
            detectors.push(Arc::new(PipDetector::new()));
        }
    }

    detectors
}

/// Run detectors concurrently using Tokio.
pub async fn run_detectors(
    detectors: Vec<std::sync::Arc<dyn Detector + Send + Sync>>,
) -> CoreResult<Vec<DetectedChange>> {
    let mut handles = Vec::new();
    for detector in detectors {
        handles.push(tokio::task::spawn_blocking(move || detector.scan()));
    }

    let mut all_changes = Vec::new();
    for handle in handles {
        let result = handle
            .await
            .map_err(|err| CoreError::Storage(err.to_string()))??;
        all_changes.extend(result);
    }
    Ok(all_changes)
}

fn default_system() -> SystemInfo {
    SystemInfo {
        os: std::env::consts::OS.into(),
        arch: std::env::consts::ARCH.into(),
    }
}

fn normalize_name(input: &str) -> String {
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

fn parse_rpm_list(output: &str, source: &str) -> CoreResult<Vec<DetectedChange>> {
    let system = default_system();
    let now = Utc::now();
    let tag = Tag::new("package")?;

    let mut changes = Vec::new();
    let mut started = false;
    for line in output.lines().map(str::trim) {
        if line.is_empty() {
            continue;
        }
        if line.to_lowercase().starts_with("installed") {
            started = true;
            continue;
        }
        if !started {
            if line.to_lowercase().starts_with("installed packages") {
                started = true;
            }
            continue;
        }
        let name_field = line.split_whitespace().next().unwrap_or(line);
        if name_field.is_empty() {
            continue;
        }
        let name = name_field.split('.').next().unwrap_or(name_field);
        changes.push(DetectedChange {
            id: uuid::Uuid::new_v4(),
            path: None,
            title: name.to_string(),
            entry_type: EntryType::Package,
            source: source.into(),
            cmd: format!("sudo {source} install {name}"),
            system: system.clone(),
            detected_at: now,
            tags: vec![tag.clone()],
        });
    }

    Ok(changes)
}

fn parse_winget_list(output: &str, source: &str) -> CoreResult<Vec<DetectedChange>> {
    let system = default_system();
    let now = Utc::now();
    let tag = Tag::new("application")?;

    let mut changes = Vec::new();
    let mut started = false;
    for line in output.lines() {
        let line = line.trim_end();
        if line.trim().is_empty() {
            continue;
        }
        if line.contains("---") {
            started = true;
            continue;
        }
        if line.to_lowercase().starts_with("name")
            && line.to_lowercase().contains("id")
        {
            continue;
        }
        if !started {
            continue;
        }

        let cols = split_columns(line);
        if cols.len() < 2 {
            continue;
        }
        let name = cols[0].clone();
        let id = cols[1].clone();
        let cmd = if !id.is_empty() {
            format!("winget install --id {id}")
        } else {
            format!("winget install {name}")
        };

        changes.push(DetectedChange {
            id: uuid::Uuid::new_v4(),
            path: None,
            title: name,
            entry_type: EntryType::Application,
            source: source.into(),
            cmd,
            system: system.clone(),
            detected_at: now,
            tags: vec![tag.clone()],
        });
    }

    Ok(changes)
}

fn split_columns(line: &str) -> Vec<String> {
    let mut columns = Vec::new();
    let mut current = String::new();
    let mut space_run = 0;

    for ch in line.chars() {
        if ch.is_whitespace() {
            space_run += 1;
            continue;
        }

        if space_run >= 2 {
            let trimmed = current.trim();
            if !trimmed.is_empty() {
                columns.push(trimmed.to_string());
            }
            current.clear();
        } else if space_run == 1 {
            current.push(' ');
        }
        space_run = 0;
        current.push(ch);
    }

    let trimmed = current.trim();
    if !trimmed.is_empty() {
        columns.push(trimmed.to_string());
    }

    columns
}

fn run_command(command: &str, args: &[&str]) -> CoreResult<String> {
    let output = Command::new(command).args(args).output();
    let output = match output {
        Ok(output) => output,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(String::new());
        }
        Err(err) => return Err(CoreError::Storage(err.to_string())),
    };

    if !output.status.success() {
        return Err(CoreError::Storage(format!(
            "{command} exited with status {}",
            output.status
        )));
    }

    String::from_utf8(output.stdout)
        .map_err(|err| CoreError::Storage(err.to_string()))
}
