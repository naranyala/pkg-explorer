# PYRAMID-OF-INTENTS — PKG Explorer

> Hierarchical design record for the **PKG Explorer** desktop app.
> Read top-down: every lower layer exists only to satisfy the layer above it.
> Every actionable work item in [`TODOS.md`](TODOS.md) traces back to an intent ID (`I.x.y`).
>
> Status legend: ✅ shipped in v0.1 · 🚧 next · 🔮 later · ⛔ explicitly out of scope

---

## Level 0 — Vision (the tip)

**One window to understand everything installed on this Linux machine — what it is, what it ships, where it lives, and how to get files out of it.**

Success test: a user who installs PKG Explorer never needs `rpm -qa | grep`, `dpkg -L`, `flatpak info`, or `brew list` again for routine questions.

---

## Level 1 — Outcome intents (the product promises)

| ID | Intent | Done when | Status |
|----|--------|-----------|--------|
| I.1 | **See everything at once** — every package from every installed manager in one searchable list | deb + rpm + flatpak + brew appear together with source badges | ✅ v0.1 (rpm/flatpak/brew verified live; deb activates on dpkg systems) |
| I.2 | **Understand one package deeply** — metadata, description, deps, exact file list | click → details pane answers "what is this and what does it own?" | ✅ v0.1 |
| I.3 | **Get files out safely** — extract a package's payload without touching the system | Extract → folder on disk; never needs root to *read* | ✅ v0.1 (cache-unpack + installed-copy fallback) |
| I.4 | **Trust the tool** — read-only by default; mutating actions are explicit, confirmed, and logged | no action modifies the system without a confirm dialog | 🚧 (v0.1 has no mutations at all — safe base) |
| I.5 | **Feel instant** — first paint < 300 ms; scan results stream in, UI never blocks | scan happens off-thread; list populates per source as results arrive | ✅ v0.1 |
| I.6 | **Run anywhere a package manager runs** — graceful degradation, no hard dependency on any one manager | missing manager = hidden/disabled source, never a crash | ✅ v0.1 |

---

## Level 2 — User intents (jobs to be done)

Personas:
- **U-A Curious Explorer** — "what is this weird thing eating 2 GB?"
- **U-B Investigator** — "which package ships `/usr/lib/libfoo.so.3`, and what else does it contain?"
- **U-C Migrator** — "I'm moving machines; give me these files as a folder/archive."
- **U-D Maintainer** — "verify, upgrade, remove — from one place, safely."
- **U-E Developer** — "diff two builds, inspect an ELF, dump file lists to CI."

| ID | User intent | Primary persona | Status |
|----|-------------|-----------------|--------|
| I.2.1 | Find a package by name/summary, fast, across all managers | A | ✅ |
| I.2.2 | Sort/group by size to find space hogs | A | 🚧 |
| I.2.3 | Answer "who owns this file/path?" (reverse lookup) | B | 🔮 |
| I.2.4 | Search file lists by glob/regex (`*.so`, `share/icons/*`) | B | 🚧 |
| I.3.1 | Extract to a chosen folder (current) | C | ✅ |
| I.3.2 | Extract to `.tar.gz` / re-pack a `.deb`/`.rpm` for transport | C | 🔮 |
| I.3.3 | Export machine-readable lists (`JSON`/`CSV`/`txt`) for scripts | C, E | 🚧 |
| I.4.1 | Install a package from a local `.deb`/`.rpm`/`.flatpakref` file | D | 🔮 |
| I.4.2 | Remove/uninstall with clear blast-radius preview (files that go away) | D | 🔮 |
| I.4.3 | See available updates per manager and trigger them | D | 🔮 |
| I.5.1 | Interact while a scan is running — search/select/refresh freely | all | ✅ |
| I.6.1 | On a system with only one manager (e.g. minimal Fedora) the app is still useful | all | ✅ |
| I.9.1 | Keyboard-driven flow: `/` search, arrows, Enter, `e` extract | B, E | 🚧 |
| I.9.2 | Works with screen readers (AccessKit is already in eframe) | a11y | 🔮 |

---

## Level 3 — Capability intents (feature domains)

### A. Explore (browse & find)

| ID | Intent | Design notes | Status |
|----|--------|--------------|--------|
| I.1.1 | Unified package list with source badges | virtualized rows: `kind  name`, version, size, summary on hover | ✅ |
| I.1.2 | Live search across name + summary | case-insensitive substring now; → later fuzzy ranking | ✅ (fuzzy 🔮) |
| I.1.3 | Per-source enable toggles | `deb/rpm/flatpak/brew` toggle chips in top bar | ✅ |
| I.1.4 | Sort by name / size / version / date | click column headers (needs a header row) | 🚧 |
| I.1.5 | Grouping mode: by manager / by origin(repo,tap,remote) / by size band | toolbar segment | 🔮 |
| I.2.5 | Derived views: "biggest 50", "recently installed", "orphans (deb)", "runtimes (flatpak)", "casks vs formulae" | saved filters | 🔮 |
| I.1.6 | Global stats strip: total packages, total installed size, per-source counts | status bar or dashboard tab | 🚧 |

### B. Inspect (understand one package)

| ID | Intent | Design notes | Status |
|----|--------|--------------|--------|
| I.2.1.1 | Info grid: version, arch, size, license, homepage, maintainer, install date, origin | two-column grid in detail pane | ✅ |
| I.2.1.2 | Full description with preserved paragraphs | done | ✅ |
| I.2.1.3 | Dependency list (expandable, per-manager semantics) | `Depends`/`Requires`/`runtime deps` normalized to one list | ✅ |
| I.2.1.4 | Complete file list, virtualized, with sizes & dir/file glyphs | sizes per file require walking (deb/rpm can: `rpm -ql --qf`? no → optional on demand) | 🚧 |
| I.2.1.5 | File list filtering (glob) + copy-to-clipboard + export | | 🚧 |
| I.2.1.6 | File preview pane: text / hex / image / ELF-headers / symlinks / permissions | `object` crate for ELF; hex viewer virtualized | 🔮 |
| I.2.1.7 | Reverse: "what package owns <path>" quick box (top bar) | calls manager per source | 🔮 |
| I.2.1.8 | Flatpak ref details: permissions, runtime, SDK, commit, origin remote | parse `flatpak info --show-metadata` | 🚧 |
| I.2.1.9 | Brew tap/links/receipt (`INSTALL_RECEIPT.json`) view | | 🔮 |
| I.4.4 | Verify integrity: `rpm -V`, `dpkg --verify`, flatpak `ostree fsck`-lite, brew doctor | read-only verification report | 🔮 |

### C. Extract (get data out)

| ID | Intent | Design notes | Status |
|----|--------|--------------|--------|
| I.3.1.1 | Extract via folder picker (rfd) | background job + status line | ✅ |
| I.3.1.2 | Prefer unpacking cached archive (`dpkg-deb -x`, `rpm2cpio\|cpio`); else copy installed files | current two-tier strategy | ✅ |
| I.3.1.3 | Progress: per-file counter + cancel button | job queue needs cancellation tokens | 🚧 |
| I.3.1.4 | Extract only selected subtrees of the file list | "extract checked…" | 🔮 |
| I.3.1.5 | Output as `tar.gz`/`zip` (`zip` crate or `tar`+`flate2`) | | 🔮 |
| I.3.1.6 | Batch extract (multi-select packages) | | 🔮 |
| I.3.1.7 | If archive not cached: download it first (`dnf download`, `apt download`, `brew fetch`) | read-only w.r.t. system; downloads to temp | 🚧 |

### D. Manage (mutate — future, gated)

| ID | Intent | Design notes | Status |
|----|--------|--------------|--------|
| I.4.1.1 | Install from file: `.deb`/`.rpm`/`.flatpakref`/`.local flatpak` | via `pkexec` polkit prompt; never silent root | 🔮 |
| I.4.2.1 | Uninstall: show file list as blast radius → typed confirm → run manager | dry-run mode first | 🔮 |
| I.4.3.1 | Updates tab: `dnf check-update` / `apt list --upgradable` / `flatpak remote-ls --updates` / `brew outdated` | refreshable list | 🔮 |
| I.4.x | **All of D behind an "Experimental" flag, default OFF**; app remains read-only otherwise | config `allow_mutations = false` | 🔮 |

### E. Compare & analyze

| ID | Intent | Design notes | Status |
|----|--------|--------------|--------|
| I.5.1.1 | Diff two packages' file lists (set diff) | multi-select 2 packages | 🔮 |
| I.5.1.2 | Diff two versions of same package (before/after upgrade) | | 🔮 |
| I.5.1.3 | Size analysis: treemap/bar of top offenders, per source | egui painting or table | 🔮 |

### F. Shell & platform

| ID | Intent | Design notes | Status |
|----|--------|--------------|--------|
| I.6.1.1 | Auto-detect managers (`which`), re-check on Refresh | `Source::is_available()` | ✅ |
| I.6.1.2 | Graceful per-source errors surfaced (not swallowed) | status bar error chip + hover detail | ✅ |
| I.6.1.3 | CLI twin for every GUI capability (scriptability) | `--list`, `--extract`, later `--info`, `--export`, `--json` | 🚧 |
| I.9.1.1 | Settings window: theme, font size, default extract dir, enabled sources, mutation flag | persisted to `~/.config/pkg-explorer/config.toml` | 🚧 |
| I.9.1.2 | Persist window geometry, last filters, favorites/recent | eframe storage or config file | 🚧 |
| I.9.1.3 | Theming: system/light/dark + accent | egui `Visuals` | 🚧 |
| I.9.1.4 | i18n skeleton (english-only strings today, `fluent` later) | 🔮 |
| I.9.1.5 | Structured logging (`tracing` → `~/.local/state/pkg-explorer/`) + "Report issue" bundle | 🔮 |
| I.9.1.6 | First-run tip overlay + command palette (`Ctrl+K`) | 🔮 |

---

## Level 4 — Interaction intents (screens & flows)

```
┌────────────────────────────────────────────────────────────────────┐
│ TOP BAR    title │ ⟳ │ [search…………] │ deb rpm flatpak brew │ stats │
├───────────────┬────────────────────────────────────────────────────┤
│ LIST (left)   │ DETAIL (right)                                    │
│ kind name     │ name  version · kind            [⬇ Extract…] [⋯]  │
│ ───────────── │ summary (italic)                                  │
│ rpm  foo 1.2  │ ┌ info grid ────────────────────────────────┐      │
│ flat bar 3.0  │ │ version / arch / size / license / origin  │      │
│ brew bat 0.26 │ └───────────────────────────────────────────┘      │
│ …(virtualized)│ ▸ description                                      │
│ filter: 1234  │ ▸ dependencies (24)                                │
│               │ ▸ files (1487)  [glob] [export] [copy]            │
├───────────────┴────────────────────────────────────────────────────┤
│ STATUS   ⟳ scanning rpm… │ extract: 45/120 … │ 2310 installed      │
└────────────────────────────────────────────────────────────────────┘
   Later: tabs  [Explore | Updates | Stats | Jobs]     ⌘K palette
```

| ID | Flow intent | Rules |
|----|-------------|-------|
| I.1.a | **Launch** → instant window, scan starts silently, rows stream in per source | never block on scan; Refresh is explicit |
| I.1.b | **Search-as-you-type** filters list instantly (client-side, no re-scan) | clear button; `/` focuses |
| I.1.c | **Select package** → details load async, spinner, cached per session | switching sources mid-load is safe (keyed cache) |
| I.3.a | **Extract** → folder dialog → background job → status progress → result summary | never overwrite the chosen dir silently: extract into `<dest>/<pkg>-<ver>/` |
| I.6.a | **Source error** → chip in status bar, hover shows manager stderr excerpt | scan still completes for healthy sources |
| I.9.a | **Cancel/quit during jobs** → jobs cancelled cleanly, no partial-file garbage (extract to temp, rename on success) 🚧 | |

Intent: layout must remain usable at 900×600 (min) and scale to 4K (text scaling setting).

---

## Level 5 — System intents (architecture)

```
                 ┌──────────── eframe App ─────────────┐
                 │  logic(): poll msgs · jobs · config │
                 │  ui():    panels (top/list/detail)  │
                 └───────┬──────────────────┬──────────┘
              commands   │                  │  mpsc::Receiver<Msg>
        ┌────────────────▼───────┐   ┌──────▼──────────────┐
        │ Scan worker (thread)   │   │ Details worker      │
        │ for src in sources:    │   │ Source::details()   │
        │   src.list_packages()  │   └─────────────────────┘
        └────────┬───────────────┘   ┌─────────────────────┐
                 │                   │ Extract worker      │
        ┌────────▼────────────────┐  │ archive? unpack     │
        │ Source trait (Send)     │  │   : copy installed  │
        │  rpm │ deb │ flatpak    │  └─────────────────────┘
        │  brew (fallback list)   │
        └─────────────────────────┘
```

| ID | System intent | Realization |
|----|---------------|-------------|
| I.5.s1 | **Shell-out, not native DB parsing** — managers' CLIs are the stable contract | `run(Command)` helper; parse TSV/JSON; unit-testable parsers |
| I.5.s2 | **One trait, N backends** — adding a manager = one file + detection | `trait Source: Send { kind, name, is_available, list_packages, details }` |
| I.5.s3 | **UI thread never blocks** — all manager calls off-thread | worker threads + `mpsc` + `ctx.request_repaint()` |
| I.5.s4 | **Data model minimal & owned** — `Package` (list row) vs `PackageDetails` (lazy, on select) | lazy details keeps scans cheap; details cached by `kind:name` key |
| I.5.s5 | **Extraction strategy resolution** — archive-in-cache → unpack; else copy from live FS | `extract::find_archive` + `copy_installed_files`, per-kind path remapping (brew prefix-relative, flatpak deploy-relative) |
| I.5.s6 | **Fail soft, report visibly** | per-source `error: Option<String>`; partial results still shown |
| I.5.s7 | **CLI and GUI share the core** | `cli.rs` calls same `sources`/`extract` — parity guaranteed |
| I.5.s8 | **Testability without a display** | `PkgApp::with_ctx()` + `Frame::_new_kittest()` headless harness tests |
| I.5.s9 | *future* **Job abstraction**: uniform queue (scan/details/extract/verify) with progress, cancel, dedup | 🚧 needed before Manage phase |
| I.5.s10 | *future* **Cache layer**: mtime-based, so Refresh is instant if nothing changed | 🔮 |
| I.5.s11 | *future* **Config object** (`serde` → TOML) shared by GUI, CLI, defaults | 🚧 |

Toolchain intents: `cargo build/clippy/test` all clean, zero warnings, on every change; MSRV = stable current (edition 2024); no unsafe.

---

## Level 6 — Constraints & quality intents

| ID | Constraint | Target |
|----|-----------|--------|
| C.1 | Safety | Read-only by default; no sudo embedded; secrets never logged; path traversal safe on extract (reject `..` components from archive listings) |
| C.2 | Performance | List 5k packages ≤ 2 s scan (rpm on this host: 2233 pkgs ≈ fast); virtualized list, 60 fps scroll; details ≤ 500 ms typical |
| C.3 | Memory | ≤ 150 MB RSS steady; file lists capped (50k rows) with overflow notice |
| C.4 | Portability | Linux only; x11 + wayland; works with ≥1 manager present; AlmaLinux/RHEL, Debian/Ubuntu, Fedora, Flatpak-enabled and Homebrew setups |
| C.5 | Dependencies | Few, pure-Rust where possible; only well-known crates (eframe, rfd, serde, anyhow…); system tools: manager CLIs + `cpio` (rpm path) |
| C.6 | A11y | Full keyboard nav; AccessKit via eframe; contrast AA in both themes |
| C.7 | Distribution | `.desktop` + icon; native rpm + deb + flatpak-packaged builds of the app; reproducible `cargo build --release` |
| C.8 | Honesty | Show exactly what the tool did (real paths, real counts); errors shown verbatim (truncated), never silently swallowed |

---

## Level 7 — Foundations (the rock)

- **Language/stack**: Rust 2024 · eframe/egui 0.37 (immediate-mode, `App::logic` + `App::ui`) · `Panel`/`CentralPanel` layout
- **Shipped modules**: `main.rs` (entry + CLI dispatch) · `app.rs` (UI + message pump) · `models.rs` · `sources/{mod,rpm,deb,flatpak,brew}.rs` · `extract.rs` · `cli.rs` — ~1.6k LOC
- **External contracts**: `dpkg-query/dpkg`, `rpm`, `flatpak`, `brew`, `dpkg-deb`, `rpm2cpio`, `cpio`, `rfd` (XDG portal dialogs)
- **Conventions**: no comments unless tricky; `anyhow` for errors; modules own their parsing; every new source implements the trait + a fallback path; every TODO in `TODOS.md` references an `I.x.y` here
- **Repo hygiene**: commit in small logical steps; keep `clippy --all-targets` clean; headless tests must stay green

---

## Anti-goals (what this must not become) ⛔

- Not a package *browser for remote repos* (no `dnf search` replacement) — installed state only, plus local-file install later.
- Not a daemon/service — single-shot GUI process.
- Not a package manager frontend that hides the CLI — every action shows the exact command it will run.
- Not Windows/macOS support (Linux only; brew = Linuxbrew).
- No telemetry, no network calls except explicit download/install actions.

---

## Traceability rule

`TODOS.md` items are grouped by phase and tagged `[I.x.y]`.
If a task can't be tied to an intent ID above → it doesn't belong in the backlog (or the pyramid needs a new intent first).
