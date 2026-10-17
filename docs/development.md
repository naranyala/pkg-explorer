# Development guide

Workflow, conventions and how-to's for working on PKG Explorer.
Companion docs: [architecture.md](architecture.md) (how it's built), [backends.md](backends.md) (manager CLIs), [../TODOS.md](../TODOS.md) (what's next).

---

## 1. Toolchain & setup

- **Rust stable**, edition 2024 (installed dev machine: cargo 1.98.x via rustup)
- **OS**: Linux; development done on AlmaLinux 10 (GNOME/Wayland), but nothing is distro-specific
- **Build deps**: `pkg-config` + `gtk3` headers for `rfd` (folder dialogs)
  ```bash
  # RHEL/Fedora/Alma
  sudo dnf install gtk3-devel pkg-config
  # Debian/Ubuntu
  sudo apt install libgtk-3-dev pkg-config
  ```
- **Runtime tools** for full test coverage: at least one of `rpm`/`dpkg-query`/`flatpak`/`brew`; `cpio` for rpm-archive extraction
- No async runtime, no bindgen beyond GTK, no build scripts of our own

## 2. Daily commands

```bash
cargo run                    # debug build + launch GUI
cargo run --release          # release build + launch
cargo run -- --help          # CLI mode (note the `--`)

cargo build                  # fast compile check
cargo clippy --all-targets   # MUST stay warning-free (gate for every change)
cargo test                   # headless tests (~2 min: real scan + details of pkg #0)
cargo test headless           # just the fast UI test
cargo fmt                    # formatting (run before committing)
```

Pre-commit checklist (from `TODOS.md` hygiene rules):

1. `cargo fmt --check`
2. `cargo clippy --all-targets` → zero warnings
3. `cargo test` → green
4. If you touched `PkgKind` order or `kind_enabled` indexing → verify the toggle chips still map correctly
5. Docs updated (`backends.md` for CLI changes, `cli.md` for flag changes, pyramid/todos if scope changed)

## 3. Repo layout & ownership

```
src/main.rs        entry: CLI dispatch → gui::run        (keep thin)
src/cli.rs         flags, exit codes, output formatting
src/app.rs         PkgApp state + panels + Msg pump + tests   (biggest file; split UI helpers when it grows)
src/models.rs      data types shared everywhere
src/sources/       one file per manager + trait
src/extract.rs     extraction strategies
docs/  PYRAMID-OF-INTENTS.md  TODOS.md   ← design lives here, code follows
```

Design docs are **source of truth for intent** (`I.x.y` IDs); `TODOS.md` is the backlog; code implements both. A feature without an intent ID should not be built (rule from the pyramid).

## 4. Conventions

- **No comments** unless the code is genuinely tricky (project rule) — the docs carry the explanations instead.
- **Errors**: `anyhow` everywhere below the UI; at the UI boundary they become `Msg` payloads and status-bar text. Never `unwrap()` on external input (manager output, JSON, fs).
- **Subprocess discipline**: argument arrays only (`Command::new(...).args([...])`); the single `sh -c` in `extract.rs` is grandfathered and slated for replacement (Phase 5) — don't add more.
- **Blocking calls never in `ui()`** — spawn a thread, send a `Msg`, `request_repaint()`.
- **Names**: modules lowercase, `Source` methods match their role (`list_packages`, `details`), messages `PascalCase` variants.
- **UI text**: plain English strings inline (i18n extraction is a future concern).
- Testing seam: production constructs `PkgApp::new(cc)`; tests use `PkgApp::with_ctx(&ctx)`.

## 5. Testing strategy

| Layer | How | Where |
|-------|-----|-------|
| UI smoke | drive `logic()` + `ui()` against a bare `egui::Context` + `Frame::_new_kittest()`, clear `textures_delta` | `src/app.rs::tests` |
| Data pipeline | real scan → select package #0 → details must load with files | `src/app.rs::tests` |
| Parsers | fixture files of manager output (Phase 2: `tests/parsers.rs`) | planned |
| Extraction mapping | pure functions, no manager needed (Phase 2) | planned |
| CLI end-to-end | `cargo run -- --list` / `--extract` against the live system | manual / scripted |

Gotchas when writing UI tests:

- egui 0.37 renamed `Context::run` → **`run_ui`**.
- Dropping a frame's `TexturesDelta` without `.clear()` **panics** in epaint — always clear when there's no GPU.
- Tests use the real machine's managers — they scan 2000+ packages; keep them tolerant (assert `>0` when any source exists, skip when none).

## 6. How to change common things

### Add/tweak a UI element
Edit `src/app.rs`: `top_bar` / `status_bar` / `package_list` / `details_pane`.
- Need data not in `Package`/`PackageDetails`? Extend `models.rs`, populate in `sources/*`, or use the `extra: Vec<(String,String)>` rows for free-form display (no UI change needed for new metadata).
- Worker triggered from a widget: clone `ctx` first (`let ctx = ui.ctx().clone()`), then `thread::spawn` + `tx.send`.

### Add a manager backend
Follow the checklist at the bottom of [backends.md](backends.md).

### Change extraction behaviour
All strategies in `src/extract.rs`; keep the two-tier contract (archive → copy) and update [extraction.md](extraction.md).

### Add a CLI flag
`src/cli.rs` `run()` match — print usage on `--help`, keep stdout data / stderr diagnostics separation, exit 0/1, document in [cli.md](cli.md).

## 7. Debugging tips

- Manager command failing? Run it by hand exactly as the code does (`rpm -qa --qf …`, `brew info --json=v2`).
- GUI stuck on "Scanning…"? The scan thread panicked or a manager hangs — check stderr; each `SourceDone` should still arrive (a panic in a worker means a lost message; consider catching with `std::panic::catch_unwind` if we ever see it in the wild).
- Reproduce without a display: `cargo test` (headless harness) or CLI mode.
- Watch the message pump: temporary `eprintln!("{msg:?}")` in `poll_messages()`.

## 8. Performance budgets (from pyramid `C.2`/`C.3`)

| Budget | Target |
|--------|--------|
| First paint | < 300 ms (scan is off-thread, so this is just panel setup) |
| Full scan (rpm 2233 + flatpak 57 + brew ~25) | a few seconds, streaming — Refresh must not freeze UI |
| Package select → details | < 500 ms typical (file walk dominates for flatpak/brew) |
| Memory steady | ≤ 150 MB; file lists capped at 50 000 entries |

Measure before optimizing; `Instant::now()` around scan sections (planned `--verbose`, Phase 1).

## 9. Releasing

Not set up yet — Phase 6 in `TODOS.md` (rpm/deb/flatpak/AppImage builds of the app itself, CI, checksums). Until then: `cargo build --release` and copy the binary. Remember the license is still unspecified — pick one before shipping.
