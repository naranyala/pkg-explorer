# PKG Explorer

**One window to understand everything installed on your Linux machine** — deb, rpm, flatpak and Homebrew packages in a single searchable list: inspect metadata, dependencies and exact file lists, then extract any package's files to a folder.

A native desktop app written in Rust with [egui/eframe](https://github.com/emilk/egui). Read-only by default: it never modifies your system unless a future mutating feature is explicitly enabled (see [roadmap](TODOS.md)).

> **Status:** early v0.1 — core explore/inspect/extract loop works and is tested. See [PYRAMID-OF-INTENTS.md](PYRAMID-OF-INTENTS.md) for the full design and [TODOS.md](TODOS.md) for the roadmap.

---

## Features (v0.1, working today)

- **Unified package list** — all packages from every *installed* package manager, auto-detected:
  - `rpm` (dnf/yum/zypper systems)
  - `deb` (dpkg/apt systems)
  - `flatpak` (user + system installations)
  - `brew` (Linuxbrew formulae + casks)
- **Instant search** as you type (name + summary), with per-source toggle chips (deb / rpm / flatpak / brew)
- **Detail pane** on selection: info grid (version, arch, size, license, homepage, maintainer, install date…), full description, dependency list, and the package's complete file list (virtualized — comfortable with 50 000+ entries)
- **Extract** any package to a folder you choose:
  - prefers unpacking the cached archive (`dpkg-deb -x`, `rpm2cpio | cpio`)
  - falls back to copying the installed files from your live filesystem
  - works for flatpak deployments and brew Cellars without archives
- **Never blocks** — scans, detail loading and extraction all run on background threads; the UI stays interactive and results stream in per source
- **Visible errors** — if a manager fails (or isn't installed), the status bar shows a red chip with the real error; healthy sources still load
- **CLI twin** for scripting: `--list`, `--extract`, `--help`
- **Headless test suite** — UI and data pipeline tested without a display

```
┌──────────────────────────────────────────────────────────────────┐
│ PKG Explorer │ ⟳ │ [search……………] │ deb  rpm  flatpak  brew       │
├────────────────┬─────────────────────────────────────────────────┤
│ rpm  389-ds    │ 389-ds-base  1.4.10.6-12.module_el10.0   · rpm  │
│ flat org.gimp  │ ─────────────────────────────────────────────── │
│ brew bat 0.26  │ Summary · Version · Arch · Size · Homepage      │
│ …virtualized…  │ License · Maintainer · Installed · Cellar…      │
│ 2310 match(es) │ ▸ Description                                    │
│                │ ▸ Dependencies (24)                              │
│                │ ▸ Files (1487)                     [⬇ Extract…]  │
├────────────────┴─────────────────────────────────────────────────┤
│ ⟳ scanning flatpak…   │ 2310 installed                           │
└──────────────────────────────────────────────────────────────────┘
```

---

## Quick start

### Requirements

- **Linux** (x11 or Wayland), tested on AlmaLinux 10; any distro with at least one of the supported package managers
- **Rust toolchain** — recent stable with edition 2024 support ([rustup](https://rustup.rs))
- **Build deps for the GUI**: `gtk3` development files (used by the folder picker):
  - RHEL/Fedora: `sudo dnf install gtk3-devel pkg-config`
  - Debian/Ubuntu: `sudo apt install libgtk-3-dev pkg-config`
- **Runtime tools** (only what you actually use — all are optional):
  | Tool | Needed for |
  |------|-----------|
  | `rpm` | listing/inspecting rpm packages |
  | `rpm2cpio` + `cpio` | unpacking cached `.rpm` archives |
  | `dpkg-query`, `dpkg` | listing/inspecting deb packages |
  | `dpkg-deb` | unpacking cached `.deb` archives |
  | `flatpak` | listing/inspecting flatpaks |
  | `brew` | listing/inspecting Homebrew formulae & casks |

### Build & run

```bash
git clone <this-repo> pkg-explorer && cd pkg-explorer
cargo run --release        # builds once, then opens the window
```

That's it — the app scans automatically on launch and streams results in.

### Verify your build

```bash
cargo clippy --all-targets   # must be warning-free
cargo test                    # headless UI + scan/details tests
```

---

## Command line

The same core that drives the GUI is exposed for scripts:

```bash
# TSV list (kind, name, version, size, summary); counts go to stderr
pkg-explorer --list                 # every manager
pkg-explorer --list rpm             # one manager: deb|rpm|flatpak|brew

# Extract a package into <dest-dir>
pkg-explorer --extract rpm systemd /tmp/out
pkg-explorer --extract brew bat /tmp/out

pkg-explorer --help
```

Exit codes: `0` success · `1` error (message on stderr). Full reference: [docs/cli.md](docs/cli.md).

---

## Documentation

| Document | What's inside |
|----------|---------------|
| [PYRAMID-OF-INTENTS.md](PYRAMID-OF-INTENTS.md) | Full product design: vision → intents → architecture → constraints (IDs `I.x.y` / `C.x`) |
| [TODOS.md](TODOS.md) | Phased, traceable backlog — Phase 0 (shipped) … Phase 6 (release) |
| [docs/architecture.md](docs/architecture.md) | Module map, threading & message flow, data model, design rules |
| [docs/backends.md](docs/backends.md) | How each package-manager source detects, queries and parses its data |
| [docs/extraction.md](docs/extraction.md) | Extract strategy decision tree, path mapping, failure modes |
| [docs/cli.md](docs/cli.md) | CLI reference with formats and examples |
| [docs/development.md](docs/development.md) | Build/test workflow, conventions, "how to add a source" checklist |
| [docs/troubleshooting.md](docs/troubleshooting.md) | Known issues (incl. Homebrew JSON crash), FAQ, workarounds |

---

## How it works (short version)

1. **Shell-out, not database scraping** — the stable contract of every package manager is its CLI. One `Source` trait, four backends, each `list_packages()`/`details()` parsed from TSV/JSON output.
2. **UI never blocks** — worker threads send `Msg` results over an `mpsc` channel; `eframe::App::logic()` drains the queue every frame and requests a repaint.
3. **Details are lazy** — the cheap list row (`Package`) loads for everything; the expensive `PackageDetails` (description, deps, files) is fetched only for the package you click, cached per session.
4. **Extraction degrades gracefully** — cached archive → unpack it; no archive → copy the installed files with the package's own file list as the manifest.

Details: [docs/architecture.md](docs/architecture.md).

---

## Known limitations (v0.1)

- Homebrew's `brew info --json=v2` can crash on some systems (broken cask config) — PKG Explorer falls back to `brew list --versions` and shows the error in the status bar. Details: [docs/troubleshooting.md](docs/troubleshooting.md).
- Extracting writes directly into the folder you pick (a package subfolder is planned — Phase 1 in [TODOS.md](TODOS.md)).
- Extraction of a cached rpm archive needs `cpio` installed.
- No install/upgrade/remove actions yet — by design, the app is read-only for now.
- No settings/theme persistence yet (Phase 1).

---

## Project layout

```
src/
├── main.rs          entry point + CLI dispatch
├── app.rs           eframe app: panels, message pump, tests
├── models.rs        Package / PackageDetails / PkgKind
├── cli.rs           --list / --extract / --help
├── extract.rs       archive-or-copy extraction, path mapping
└── sources/
    ├── mod.rs       Source trait, command runner, size helpers
    ├── rpm.rs       rpm -qa / -qi / -ql …
    ├── deb.rs       dpkg-query / dpkg -L …
    ├── flatpak.rs   flatpak list / info …
    └── brew.rs      brew info --json=v2 (+ fallback) …
```

---

## Contributing

See [docs/development.md](docs/development.md). Golden rules: keep `cargo clippy --all-targets` and `cargo test` green; new backlog work must cite an intent ID from the pyramid.

## License

Not yet specified — add a `license` field to `Cargo.toml` and a `LICENSE` file before any release (tracked under Phase 6 of [TODOS.md](TODOS.md)).
