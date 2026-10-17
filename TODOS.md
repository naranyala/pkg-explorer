# TODOS — PKG Explorer

> Actionable backlog. Every item cites an intent in [`PYRAMID-OF-INTENTS.md`](PYRAMID-OF-INTENTS.md).
> Rule: `[ ]` → `[x]` only when `cargo clippy --all-targets` is warning-free, `cargo test` is green,
> and the item's acceptance check actually ran (GUI items: manual pass on a real session).
>
> Phases are ordered by dependency, not by calendar. Ship = tag a version.

---

## Phase 0 — v0.1 foundation ✅ (shipped)

- [x] `[I.1]` Unified list: rpm + flatpak + brew sources auto-detected, deb included (verified where dpkg exists)
- [x] `[I.2.1]` Search-as-you-type over name + summary; per-source toggle chips
- [x] `[I.2.1.1..3]` Detail pane: info grid, description, dependencies
- [x] `[I.2.1.4]` File list per package (virtualized), lazy-loaded on select
- [x] `[I.3.1.1..2]` Extract: rfd folder picker; cached `.deb`/`.rpm` unpack else installed-copy (flatpak + brew copy verified; rpm2cpio pipeline verified)
- [x] `[I.5.s3]` Background workers (scan/details/extract) + mpsc + repaint
- [x] `[I.6.1.2]` Per-source errors surfaced in status bar (brew JSON crash → visible, fallback list works)
- [x] `[I.6.1.3]` CLI: `--list [kind]`, `--extract <kind> <name> <dest>`
- [x] `[I.5.s8]` Headless tests: UI render + real scan/details
- [x] `[C.7-precursor]` `cargo clippy --all-targets` clean; `cargo test` green

---

## Phase 1 — Harden v0.1 → v0.2 (polish, ship)

- [ ] `[I.9.a]` Extract into `<dest>/<pkg>-<ver>/` subfolder; never spill loose into user-chosen dir → `src/app.rs`, `src/extract.rs`
- [ ] `[I.9.a]` Extract to temp dir + atomic rename on success; delete temp on cancel/failure
- [ ] `[I.3.1.3]` Real progress bar + **Cancel** button for extract jobs (needs `Arc<AtomicBool>` token) → status bar job widget
- [ ] `[I.9.1.2]` Persist window geometry + search/kind filters (eframe storage or TOML) → `src/config.rs` (new)
- [ ] `[I.9.1.3]` Theme setting: system / light / dark, applied via `ctx.set_visuals()`
- [ ] `[I.9.1.1]` Settings dialog (⚙ in top bar): theme, font scale, default extract dir, enabled sources
- [ ] `[I.1.b]` Keyboard: `/` focus search · `↑↓` move selection · `Enter` open · `e` extract · `Ctrl+R` refresh → `src/app.rs` input handling
- [ ] `[I.1.a]` Per-source scan progress text ("scanning rpm… ✓, flatpak… ✓") instead of single spinner
- [ ] `[I.6.1.3]` CLI: `--info <kind> <name>` (text/JSON), `--export <kind> <dest>` file lists, global `--json` flag → `src/cli.rs`
- [ ] `[C.1]` Path-safety audit on extract: strip `..`/absolute components when unpacking archives
- [ ] `[C.7]` `.desktop` file + 512² SVG/PNG icon + `assets/` + install instructions (README)
- [ ] `[C.2]` Perf pass: measure scan time on 5k packages; add `Instant` timings behind `--verbose`
- [ ] `[C.3]` File-list cap: show "showing first 50,000" note when truncated (currently silent)
- [ ] README: screenshots, feature matrix, CLI usage, build deps (gtk3 for rfd)
- [ ] Git: initial commit, then one commit per completed phase item

---

## Phase 2 — Explore & Inspect depth → v0.3

- [ ] `[I.1.4]` Sortable list headers: name / version / size / source (click-to-sort, asc/desc) → replace `sort_by` fixed order
- [ ] `[I.1.6]` Stats strip: total count + total installed size per source (compute during scan; sizes often missing → sum what's known)
- [ ] `[I.2.4]` File-list glob filter box + copy-all + "copy visible" → detail pane toolbar
- [ ] `[I.1.3]` Derivative saved filters: Biggest 50 · Recently installed · Casks · Flatpak runtimes · Deb orphans
- [ ] `[I.2.1.8]` Flatpak: parse `flatpak info --show-metadata` (permissions, runtime, sdk, origin, commit)
- [ ] `[I.2.1.4]` Per-file sizes for rpm/deb where obtainable; brew/flatpak already walked → size column in file list
- [ ] `[I.1.b]` Fuzzy ranking (prefix > substring > subsequence) + match-highlight in list rows
- [ ] `[I.5.s9]` Job manager refactor: unified queue with progress/cancel/dedup (scan, details, extract share it) → `src/jobs.rs` (new)
- [ ] `[I.5.s11]` Config file `~/.config/pkg-explorer/config.toml` (`serde` + `toml` crate), settings dialog reads/writes it
- [ ] `[I.6.1.3]` `pkg-explorer --help` polish: examples + man-style output
- [ ] Tests: parser fixtures (rpm/dpkg TSV samples, brew JSON, flatpak TSV) → `tests/parsers.rs` with checked-in fixture files
- [ ] Tests: extract path-mapping unit tests (brew prefix-rel, flatpak deploy-rel, rpm abs) — pure functions, no manager needed

---

## Phase 3 — Extract & export power → v0.4

- [ ] `[I.3.1.7]` "Fetch archive first" when not cached: `dnf download --destdir`, `apt-get download`, `brew fetch` → then archive path
- [ ] `[I.3.1.5]` Export as `.tar.gz` / `.zip` (`tar` + `flate2` / `zip` crates) instead of only raw folder
- [ ] `[I.3.1.4]` Extract checked subtree: checkboxes in file list → extract only selection
- [ ] `[I.3.1.6]` Multi-select packages in list (Shift/Ctrl) → batch extract into one tree
- [ ] `[I.3.1.3]` Progress shows bytes + files; ETA for large packages
- [ ] `[I.2.4]` Export file lists: `txt` / `csv` / `json` via rfd save dialog or CLI `--export`
- [ ] `[I.2.1.6]` File preview pane (modal): text (first 1 MiB) · hex dump (virtualized 64-byte rows) · image (egui texture) · ELF header summary (`object` crate) · symlink target · mode/owner
- [ ] `[I.2.3]` Reverse lookup box: "who owns path?" → queries each source (`rpm -qf`, `dpkg -S`, brew `ls --formula`, flatpak path scan)
- [ ] `[C.1]` Verify mode: `rpm -V` / `dpkg --verify` → per-file ✅/❌ report table → `[I.4.4]`

---

## Phase 4 — Updates & analysis → v0.5

- [ ] `[I.4.3.1]` Updates tab: `dnf check-update` · `apt list --upgradable` · `flatpak remote-ls --updates` · `brew outdated` (each behind source detection)
- [ ] `[I.5.1.3]` Size analysis view: top-50 bar chart (egui painter), per-source totals, treemap optional
- [ ] `[I.5.1.1]` Diff two packages' file lists (select 2 → set-diff panel: only-in-A / only-in-B / both)
- [ ] `[I.5.1.2]` Version diff of same package across snapshots (export JSON at two times, diff later)
- [ ] `[I.9.1.6]` Command palette `Ctrl+K`: actions + package jump
- [ ] `[I.9.1.5]` `tracing` + rotating file log; "Copy diagnostics" button (versions, sources, errors)
- [ ] Stats dashboard tab: history of total sizes (appends a JSON snapshot per scan) — only if it stays cheap

---

## Phase 5 — Manage (mutating, experimental) → v0.6

> Gate: everything behind `allow_mutations = false` config + visible "Experimental" badge.
> Every action previews the exact command; runs via `pkexec` (polkit), never embedded sudo. `[C.1]`

- [ ] `[I.4.1.1]` Install local file: `.deb` (`apt install ./x.deb`), `.rpm` (`dnf install ./x.rpm`), `.flatpakref`, `.flatpak` bundle
- [ ] `[I.4.2.1]` Uninstall: blast-radius file list → typed package-name confirm → run → stream output into Jobs pane
- [ ] `[I.4.x]` Jobs pane (bottom tab): live stdout/stderr of management commands, cancel where safe
- [ ] `[I.4.3.1]` Apply updates from Updates tab (selected rows → same job pipeline)
- [ ] `[C.1]` Security review: argument arrays only (never shell strings) for pkexec paths; no env passthrough; audit `shell_escape` usages (rpm2cpio pipeline is the only `sh -c` — replace with spawned pipe chain: `rpm2cpio` stdout → `cpio` stdin)
- [ ] Tests: dry-run flag asserting exact command lines (no execution)

---

## Phase 6 — Packaging & release → v1.0

- [ ] `[C.7]` Build native `.rpm` (on this AlmaLinux host) and `.deb` (in container) of the app itself
- [ ] `[C.7]` Flatpak manifest (`org.pkgexplorer.PkgExplorer`, flathub-compatible org.freedesktop.Platform) + `flatpak-builder` smoke test
- [ ] `[C.7]` AppImage as portable fallback
- [ ] CI: GitHub Actions — fmt + clippy + test matrix (ubuntu, fedora container for rpm-source tests), artifact upload
- [ ] Release script: version bump, tag, `cargo build --release --locked`, checksums, changelog generation
- [ ] `.desktop` + icon polish: startup WM class, app id for Wayland, file-association hooks for opening `.deb`/`.rpm` (Phase 5 flow)
- [ ] README final: screenshots (Wayland-safe capture), feature matrix, FAQ, troubleshooting (brew JSON crash note)

---

## Ongoing hygiene (never "done")

- [ ] Keep `cargo clippy --all-targets` and `cargo test` green on every change `[C.7-adjacent]`
- [ ] When adding a package manager (pacman/zypper/apk/nix): implement `Source` + fallback-less error path + detection + test fixture `[I.5.s2, I.6.1.1]`
- [ ] Keep intent↔todo traceability: new backlog item without an `I.x` → extend the pyramid first `[I.5.s7]`
- [ ] Review deps quarterly; no new dep without need `[C.5]`
- [ ] Bug triage: every bug report gets a failing test before the fix `[C.8]`

---

## Backlog parking lot (ideas, no intent yet — promote deliberately)

- Hex/thumbnail preview in a detached always-on-top window
- `pacman` / `zypper` / `apk` / `nix profile` sources
- Compare against a remote repo to detect locally modified/removed files (rpm does this natively: `rpm -Va`)
- Package screenshots/metadata from AppStream catalogs (`/usr/share/metainfo`) for flatpak/deb polish
- Dark/light follows GNOME portal appearance setting automatically
- Flatpak "unused runtimes" cleanup assistant (still read-only: report only)
