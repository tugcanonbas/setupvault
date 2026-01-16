#!/usr/bin/env python3
import argparse
import datetime as dt
import os
import platform
import random
import sys
import uuid


def slugify(value: str) -> str:
    slug = []
    last_dash = False
    for ch in value:
        if ch.isalnum():
            slug.append(ch.lower())
            last_dash = False
        elif not last_dash:
            slug.append("-")
            last_dash = True
    return "".join(slug).strip("-") or "entry"


def yaml_quote(value: str) -> str:
    escaped = value.replace("\\", "\\\\").replace("\"", "\\\"")
    return f"\"{escaped}\""


def detect_system():
    system = platform.system().lower()
    if system.startswith("darwin"):
        os_name = "macos"
    elif system.startswith("windows"):
        os_name = "windows"
    else:
        os_name = "linux"
    arch = platform.machine().lower() or "unknown"
    return os_name, arch


def write_entry(path, entry):
    lines = []
    lines.append("---")
    lines.append(f"id: {entry['id']}")
    lines.append(f"title: {yaml_quote(entry['title'])}")
    lines.append(f"type: {entry['entry_type']}")
    lines.append(f"source: {yaml_quote(entry['source'])}")
    lines.append(f"cmd: {yaml_quote(entry['cmd'])}")
    lines.append("system:")
    lines.append(f"  os: {yaml_quote(entry['system']['os'])}")
    lines.append(f"  arch: {yaml_quote(entry['system']['arch'])}")
    lines.append(f"detected_at: {entry['detected_at']}")
    lines.append(f"status: {entry['status']}")
    lines.append("tags:")
    for tag in entry["tags"]:
        lines.append(f"  - {yaml_quote(tag)}")
    lines.append("---")
    lines.append("")
    lines.append("# Rationale")
    lines.append(entry["rationale"])
    lines.append("")
    lines.append("# Verification")
    lines.append(entry.get("verification", ""))
    lines.append("")
    with open(path, "w", encoding="utf-8") as f:
        f.write("\n".join(lines))


def entry_path(root, entry):
    type_dir = {
        "package": "packages",
        "config": "configs",
        "application": "applications",
        "script": "scripts",
        "other": "other",
    }[entry["entry_type"]]
    filename = f"{entry['source']}-{slugify(entry['title'])}-{entry['id']}.md"
    return os.path.join(root, "entries", type_dir, entry["source"], filename)


def write_yaml_list(path, items):
    lines = []
    for item in items:
        lines.append("-")
        lines.append(f"  id: {item['id']}")
        if item.get("path") is None:
            lines.append("  path: null")
        else:
            lines.append(f"  path: {yaml_quote(item['path'])}")
        lines.append(f"  title: {yaml_quote(item['title'])}")
        lines.append(f"  type: {item['entry_type']}")
        lines.append(f"  source: {yaml_quote(item['source'])}")
        lines.append(f"  cmd: {yaml_quote(item['cmd'])}")
        lines.append("  system:")
        lines.append(f"    os: {yaml_quote(item['system']['os'])}")
        lines.append(f"    arch: {yaml_quote(item['system']['arch'])}")
        lines.append(f"  detected_at: {item['detected_at']}")
        lines.append("  tags:")
        for tag in item["tags"]:
            lines.append(f"    - {yaml_quote(tag)}")
    with open(path, "w", encoding="utf-8") as f:
        f.write("\n".join(lines) + "\n")


def now_iso():
    return dt.datetime.now(dt.timezone.utc).isoformat()


def make_entry(system, spec, status="active"):
    return {
        "id": str(uuid.uuid4()),
        "title": spec["title"],
        "entry_type": spec["entry_type"],
        "source": spec["source"],
        "cmd": spec["cmd"],
        "system": {"os": system[0], "arch": system[1]},
        "detected_at": now_iso(),
        "status": status,
        "tags": spec["tags"],
        "rationale": spec["rationale"],
        "verification": spec.get("verification", ""),
    }


def make_detected(system, spec):
    return {
        "id": str(uuid.uuid4()),
        "path": spec.get("path"),
        "title": spec["title"],
        "entry_type": spec["entry_type"],
        "source": spec["source"],
        "cmd": spec["cmd"],
        "system": {"os": system[0], "arch": system[1]},
        "detected_at": now_iso(),
        "tags": spec["tags"],
    }


def build_demo_specs(os_name):
    base = [
        {
            "title": ".gitconfig",
            "entry_type": "config",
            "source": "dotfiles",
            "cmd": "open ~/.gitconfig",
            "tags": ["config", "git"],
            "rationale": "Consistent author identity and diff settings reduce review friction.",
        },
        {
            "title": ".zshrc",
            "entry_type": "config",
            "source": "dotfiles",
            "cmd": "open ~/.zshrc",
            "tags": ["shell", "config"],
            "rationale": "Shell settings keep tooling and aliases aligned.",
        },
        {
            "title": "CLI bootstrap script",
            "entry_type": "script",
            "source": "manual",
            "cmd": "./scripts/bootstrap.sh",
            "tags": ["bootstrap", "automation"],
            "rationale": "Single entrypoint reduces onboarding time for new machines.",
        },
        {
            "title": "README.md onboarding notes",
            "entry_type": "config",
            "source": "manual",
            "cmd": "open docs/guides/user-manual.md",
            "tags": ["documentation"],
            "rationale": "Keeps setup steps visible and reviewed during handoffs.",
        },
        {
            "title": "npm: typescript",
            "entry_type": "package",
            "source": "npm",
            "cmd": "npm install -g typescript",
            "tags": ["typescript", "tooling"],
            "rationale": "Global tsc keeps CLI scripts and build steps aligned.",
            "verification": "tsc --version",
        },
        {
            "title": "npm: eslint",
            "entry_type": "package",
            "source": "npm",
            "cmd": "npm install -g eslint",
            "tags": ["lint", "tooling"],
            "rationale": "Baseline linting keeps code quality consistent.",
            "verification": "eslint --version",
        },
        {
            "title": "npm: prettier",
            "entry_type": "package",
            "source": "npm",
            "cmd": "npm install -g prettier",
            "tags": ["formatting"],
            "rationale": "Shared formatting prevents noisy diffs.",
            "verification": "prettier --version",
        },
        {
            "title": "cargo: just",
            "entry_type": "package",
            "source": "cargo",
            "cmd": "cargo install just",
            "tags": ["task-runner", "cli"],
            "rationale": "One command surface for common project workflows.",
            "verification": "just --version",
        },
        {
            "title": "pip: black",
            "entry_type": "package",
            "source": "pip",
            "cmd": "pip install black",
            "tags": ["python", "formatting"],
            "rationale": "Predictable formatting avoids style diffs in shared scripts.",
            "verification": "black --version",
        },
    ]

    def package_entries(names, source, cmd_prefix, tags, rationale_prefix):
        specs = []
        for name in names:
            specs.append(
                {
                    "title": name,
                    "entry_type": "package",
                    "source": source,
                    "cmd": f"{cmd_prefix}{name}",
                    "tags": tags,
                    "rationale": f"{rationale_prefix} {name}.",
                }
            )
        return specs

    def app_entries(names, source, cmd_prefix, tags, rationale_prefix):
        specs = []
        for name in names:
            specs.append(
                {
                    "title": name,
                    "entry_type": "application",
                    "source": source,
                    "cmd": f"{cmd_prefix}{name}",
                    "tags": tags,
                    "rationale": f"{rationale_prefix} {name}.",
                }
            )
        return specs

    if os_name == "macos":
        brew_formulae = [
            "ripgrep",
            "jq",
            "bat",
            "fd",
            "fzf",
            "gh",
            "htop",
            "tree",
            "wget",
            "curl",
            "git",
            "git-lfs",
            "python",
            "node",
            "go",
            "rust",
            "cmake",
            "openssl@3",
            "sqlite",
            "postgresql",
            "redis",
            "docker",
            "docker-compose",
            "kubectl",
            "helm",
            "terraform",
            "awscli",
            "gcloud",
            "k9s",
            "pandoc",
            "ffmpeg",
            "tesseract",
            "graphviz",
            "imagemagick",
            "ngrok",
            "zstd",
            "xz",
            "rsync",
            "rclone",
            "openjdk",
            "gradle",
            "maven",
            "direnv",
            "shellcheck",
            "pre-commit",
            "poetry",
            "pipx",
            "uv",
        ]
        brew_casks = [
            "Visual Studio Code",
            "Slack",
            "Figma",
            "Postman",
            "Google Chrome",
            "Raycast",
            "Docker Desktop",
            "Notion",
            "Zoom",
            "Spotify",
            "Discord",
            "1Password",
            "Warp",
            "Obsidian",
            "Arc",
            "Alfred",
            "Rectangle",
            "Miro",
            "Notion Calendar",
            "Microsoft Teams",
            "Microsoft Word",
            "Microsoft Excel",
            "Microsoft PowerPoint",
            "GitHub Desktop",
            "Insomnia",
            "TablePlus",
            "Android Studio",
            "Xcode",
            "iTerm",
            "Chrome Canary",
            "Firefox",
            "Brave Browser",
            "Whimsical",
        ]
        mac_defaults = [
            "NSGlobalDomain",
            "com.apple.finder",
            "com.apple.dock",
            "com.apple.screencapture",
            "com.apple.trackpad",
            "com.apple.controlcenter",
            "com.apple.universalaccess",
            "com.apple.SoftwareUpdate",
            "com.apple.menuextra.clock",
            "com.apple.screensaver",
        ]

        specs = base
        specs += package_entries(
            brew_formulae,
            "homebrew",
            "brew install ",
            ["cli", "tooling"],
            "Installed via Homebrew to standardize",
        )
        specs += app_entries(
            brew_casks,
            "homebrew",
            "brew install --cask ",
            ["application"],
            "Installed via Homebrew cask for",
        )
        specs += [
            {
                "title": domain,
                "entry_type": "config",
                "source": "mac_defaults",
                "cmd": f"defaults read {domain}",
                "tags": ["config", "macos"],
                "rationale": f"Captures defaults for {domain} to keep UI behavior consistent.",
            }
            for domain in mac_defaults
        ]
        return specs

    if os_name == "linux":
        apt_packages = [
            "build-essential",
            "curl",
            "wget",
            "git",
            "git-lfs",
            "ripgrep",
            "jq",
            "fzf",
            "fd-find",
            "htop",
            "tree",
            "python3",
            "python3-pip",
            "nodejs",
            "npm",
            "docker.io",
            "docker-compose",
            "kubectl",
            "helm",
            "terraform",
            "sqlite3",
            "postgresql",
            "redis-server",
            "openssh-client",
            "rsync",
            "unzip",
            "zip",
            "make",
            "cmake",
            "clang",
            "openssl",
            "neovim",
        ]
        pacman_packages = [
            "base-devel",
            "git",
            "ripgrep",
            "jq",
            "fzf",
            "fd",
            "htop",
            "tree",
            "python",
            "nodejs",
            "npm",
            "docker",
            "docker-compose",
            "kubectl",
            "helm",
            "terraform",
            "postgresql",
            "redis",
            "sqlite",
            "openssh",
            "rsync",
            "cmake",
            "neovim",
        ]
        dnf_packages = [
            "git",
            "ripgrep",
            "jq",
            "fzf",
            "htop",
            "tree",
            "python3",
            "nodejs",
            "npm",
            "docker",
            "docker-compose",
            "kubectl",
            "helm",
            "terraform",
            "postgresql",
            "redis",
            "sqlite",
            "openssh",
            "rsync",
            "cmake",
            "neovim",
        ]
        flatpak_apps = [
            "org.mozilla.firefox",
            "com.slack.Slack",
            "com.visualstudio.code",
            "com.spotify.Client",
            "com.discordapp.Discord",
            "md.obsidian.Obsidian",
            "com.getpostman.Postman",
            "com.github.IsmaelMartinez.teams_for_linux",
            "com.google.Chrome",
            "io.dbeaver.DBeaverCommunity",
            "com.todoist.Todoist",
            "org.gnome.Calculator",
            "org.gnome.Terminal",
        ]
        snap_apps = [
            "postman",
            "spotify",
            "slack",
            "code",
            "discord",
            "chromium",
            "notion-snap",
            "zoom-client",
            "intellij-idea-community",
            "pycharm-community",
            "insomnia",
            "telegram-desktop",
        ]
        desktop_apps = [
            "Firefox",
            "Slack",
            "Figma",
            "Postman",
            "Discord",
            "Spotify",
            "Notion",
            "Zoom",
            "LibreOffice",
            "GIMP",
            "Inkscape",
        ]

        specs = base
        specs += package_entries(
            apt_packages,
            "apt",
            "sudo apt-get install ",
            ["cli", "tooling"],
            "Installed via apt to standardize",
        )
        specs += package_entries(
            dnf_packages,
            "dnf",
            "sudo dnf install ",
            ["cli", "tooling"],
            "Installed via dnf to standardize",
        )
        specs += package_entries(
            pacman_packages,
            "pacman",
            "sudo pacman -S ",
            ["cli", "tooling"],
            "Installed via pacman to standardize",
        )
        specs += app_entries(
            flatpak_apps,
            "flatpak",
            "flatpak install ",
            ["application"],
            "Installed via Flatpak for",
        )
        specs += app_entries(
            snap_apps,
            "snap",
            "sudo snap install ",
            ["application"],
            "Installed via Snap for",
        )
        specs += app_entries(
            desktop_apps,
            "applications",
            "gtk-launch ",
            ["application"],
            "Desktop application entry for",
        )
        return specs

    if os_name == "windows":
        winget_apps = [
            "Microsoft.PowerToys",
            "Microsoft.VisualStudioCode",
            "Microsoft.WindowsTerminal",
            "Git.Git",
            "Docker.DockerDesktop",
            "Postman.Postman",
            "Notion.Notion",
            "SlackTechnologies.Slack",
            "Zoom.Zoom",
            "Spotify.Spotify",
            "Discord.Discord",
            "Google.Chrome",
            "Mozilla.Firefox",
            "Figma.Figma",
            "GitHub.GitHubDesktop",
            "Microsoft.Teams",
            "Microsoft.OneDrive",
            "Microsoft.Edge",
            "Obsidian.Obsidian",
            "Insomnia.Insomnia",
            "TablePlus.TablePlus",
        ]
        choco_packages = [
            "nodejs",
            "python",
            "git",
            "7zip",
            "openssl.light",
            "curl",
            "wget",
            "jq",
            "ripgrep",
            "fzf",
            "docker-desktop",
            "kubectl",
            "helm",
            "terraform",
            "awscli",
            "azure-cli",
            "gcloudsdk",
            "make",
            "cmake",
            "neovim",
        ]
        scoop_packages = [
            "git",
            "ripgrep",
            "jq",
            "fzf",
            "nodejs",
            "python",
            "go",
            "rustup",
            "openssh",
            "curl",
            "wget",
            "7zip",
            "neovim",
            "make",
        ]
        store_apps = [
            "Spotify.Spotify",
            "Microsoft.PowerToys",
            "Microsoft.WindowsTerminal",
            "WhatsApp.WhatsApp",
            "Instagram.Instagram",
        ]
        program_files_apps = [
            "Visual Studio Code",
            "Slack",
            "Figma",
            "Postman",
            "Discord",
            "Spotify",
            "Notion",
            "Zoom",
            "GitHub Desktop",
            "Microsoft Teams",
            "Docker Desktop",
            "Google Chrome",
        ]

        specs = base
        specs += app_entries(
            winget_apps,
            "winget",
            "winget install --id ",
            ["application"],
            "Installed via winget for",
        )
        specs += package_entries(
            choco_packages,
            "chocolatey",
            "choco install ",
            ["cli", "tooling"],
            "Installed via Chocolatey to standardize",
        )
        specs += package_entries(
            scoop_packages,
            "scoop",
            "scoop install ",
            ["cli", "tooling"],
            "Installed via Scoop to standardize",
        )
        specs += app_entries(
            store_apps,
            "msstore",
            "winget install --id ",
            ["application"],
            "Installed via Microsoft Store for",
        )
        specs += app_entries(
            program_files_apps,
            "applications",
            "start ",
            ["application"],
            "Installed locally for",
        )
        return specs

    return base


def build_inbox_specs(os_name):
    base = [
        {
            "title": "gh",
            "entry_type": "package",
            "source": "homebrew" if os_name == "macos" else "apt",
            "cmd": "brew install gh" if os_name == "macos" else "sudo apt-get install gh",
            "tags": ["git", "cli"],
        },
        {
            "title": ".zshrc",
            "entry_type": "config",
            "source": "dotfiles",
            "cmd": "open ~/.zshrc",
            "tags": ["shell", "config"],
        },
        {
            "title": "Discord",
            "entry_type": "application",
            "source": "homebrew" if os_name == "macos" else "winget",
            "cmd": "brew install --cask discord" if os_name == "macos" else "winget install --id Discord.Discord",
            "tags": ["communication"],
        },
        {
            "title": "Notion",
            "entry_type": "application",
            "source": "homebrew" if os_name == "macos" else "winget",
            "cmd": "brew install --cask notion" if os_name == "macos" else "winget install --id Notion.Notion",
            "tags": ["notes"],
        },
    ]

    if os_name == "macos":
        return base + [
            {
                "title": "bat",
                "entry_type": "package",
                "source": "homebrew",
                "cmd": "brew install bat",
                "tags": ["cli", "preview"],
            },
            {
                "title": "Rectangle",
                "entry_type": "application",
                "source": "homebrew",
                "cmd": "brew install --cask rectangle",
                "tags": ["productivity"],
            },
            {
                "title": "Google Chrome",
                "entry_type": "application",
                "source": "homebrew",
                "cmd": "brew install --cask google-chrome",
                "tags": ["browser"],
            },
            {
                "title": "Warp",
                "entry_type": "application",
                "source": "homebrew",
                "cmd": "brew install --cask warp",
                "tags": ["terminal"],
            },
            {
                "title": "Raycast",
                "entry_type": "application",
                "source": "homebrew",
                "cmd": "brew install --cask raycast",
                "tags": ["productivity"],
            },
            {
                "title": "htop",
                "entry_type": "package",
                "source": "homebrew",
                "cmd": "brew install htop",
                "tags": ["monitoring"],
            },
        ]
    if os_name == "linux":
        return base + [
            {
                "title": "neovim",
                "entry_type": "package",
                "source": "pacman",
                "cmd": "sudo pacman -S neovim",
                "tags": ["editor"],
            },
            {
                "title": "postman",
                "entry_type": "application",
                "source": "snap",
                "cmd": "sudo snap install postman",
                "tags": ["api", "testing"],
            },
            {
                "title": "Firefox",
                "entry_type": "application",
                "source": "flatpak",
                "cmd": "flatpak install org.mozilla.firefox",
                "tags": ["browser"],
            },
            {
                "title": "htop",
                "entry_type": "package",
                "source": "apt",
                "cmd": "sudo apt-get install htop",
                "tags": ["monitoring"],
            },
            {
                "title": "docker",
                "entry_type": "package",
                "source": "apt",
                "cmd": "sudo apt-get install docker.io",
                "tags": ["containers"],
            },
        ]
    if os_name == "windows":
        return base + [
            {
                "title": "Microsoft PowerToys",
                "entry_type": "application",
                "source": "winget",
                "cmd": "winget install --id Microsoft.PowerToys",
                "tags": ["productivity"],
            },
            {
                "title": "Git",
                "entry_type": "package",
                "source": "scoop",
                "cmd": "scoop install git",
                "tags": ["git", "cli"],
            },
            {
                "title": "Node.js",
                "entry_type": "package",
                "source": "chocolatey",
                "cmd": "choco install nodejs -y",
                "tags": ["runtime"],
            },
            {
                "title": "Visual Studio Code",
                "entry_type": "application",
                "source": "winget",
                "cmd": "winget install --id Microsoft.VisualStudioCode",
                "tags": ["editor"],
            },
        ]
    return base


def build_fallback_inbox_specs():
    return [
        {
            "title": "git",
            "entry_type": "package",
            "source": "manual",
            "cmd": "git --version",
            "tags": ["git", "cli"],
        },
        {
            "title": "node",
            "entry_type": "package",
            "source": "manual",
            "cmd": "node --version",
            "tags": ["runtime"],
        },
        {
            "title": "python",
            "entry_type": "package",
            "source": "manual",
            "cmd": "python --version",
            "tags": ["runtime"],
        },
        {
            "title": "curl",
            "entry_type": "package",
            "source": "manual",
            "cmd": "curl --version",
            "tags": ["network", "cli"],
        },
        {
            "title": "fzf",
            "entry_type": "package",
            "source": "manual",
            "cmd": "fzf --version",
            "tags": ["cli", "search"],
        },
        {
            "title": "make",
            "entry_type": "package",
            "source": "manual",
            "cmd": "make --version",
            "tags": ["toolchain"],
        },
        {
            "title": "tmux",
            "entry_type": "package",
            "source": "manual",
            "cmd": "tmux -V",
            "tags": ["terminal"],
        },
        {
            "title": "zsh",
            "entry_type": "package",
            "source": "manual",
            "cmd": "zsh --version",
            "tags": ["shell"],
        },
    ]


def build_snoozed_specs(os_name):
    base = [
        {
            "title": "Zoom",
            "entry_type": "application",
            "source": "homebrew" if os_name == "macos" else "winget",
            "cmd": "brew install --cask zoom" if os_name == "macos" else "winget install --id Zoom.Zoom",
            "tags": ["communication"],
        },
    ]

    if os_name == "windows":
        return base + [
            {
                "title": "Spotify",
                "entry_type": "application",
                "source": "msstore",
                "cmd": "winget install --id Spotify.Spotify",
                "tags": ["media"],
            },
        ]
    if os_name == "linux":
        return base + [
            {
                "title": "Spotify",
                "entry_type": "application",
                "source": "flatpak",
                "cmd": "flatpak install com.spotify.Client",
                "tags": ["media"],
            },
        ]
    return base


def main():
    parser = argparse.ArgumentParser(description="Seed a demo SetupVault.")
    parser.add_argument("--vault", required=True, help="Path to the vault directory")
    parser.add_argument("--inbox", type=int, default=12, help="Inbox items to keep (max 15)")
    args = parser.parse_args()

    if args.inbox < 0 or args.inbox > 15:
        print("Inbox count must be between 0 and 15.", file=sys.stderr)
        sys.exit(1)

    root = os.path.abspath(os.path.expanduser(args.vault))
    entries_root = os.path.join(root, "entries")
    state_root = os.path.join(root, ".state")
    os.makedirs(entries_root, exist_ok=True)
    os.makedirs(state_root, exist_ok=True)

    system = detect_system()
    rng = random.Random(42)

    demo_specs = build_demo_specs(system[0])
    rng.shuffle(demo_specs)
    entries = [make_entry(system, spec) for spec in demo_specs]

    for entry in entries:
        path = entry_path(root, entry)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        write_entry(path, entry)

    inbox_specs = build_inbox_specs(system[0])
    rng.shuffle(inbox_specs)
    if len(inbox_specs) < args.inbox:
        fallback = build_fallback_inbox_specs()
        rng.shuffle(fallback)
        existing = {spec["title"] for spec in inbox_specs}
        for spec in fallback:
            if spec["title"] in existing:
                continue
            inbox_specs.append(spec)
            existing.add(spec["title"])
            if len(inbox_specs) >= args.inbox:
                break
    inbox_specs = inbox_specs[: args.inbox]
    inbox_items = [make_detected(system, spec) for spec in inbox_specs]
    write_yaml_list(os.path.join(state_root, "inbox.yaml"), inbox_items)

    snoozed_specs = build_snoozed_specs(system[0])
    snoozed_items = [make_detected(system, spec) for spec in snoozed_specs]
    write_yaml_list(os.path.join(state_root, "snoozed.yaml"), snoozed_items)

    print(f"Seeded vault at {root}")
    print(f"Entries: {len(entries)}")
    print(f"Inbox: {len(inbox_items)}")
    print(f"Snoozed: {len(snoozed_items)}")


if __name__ == "__main__":
    main()
