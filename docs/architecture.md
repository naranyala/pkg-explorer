# Architecture

How PKG Explorer is put together: module map, threading model, data flow and the rules new code must follow.
Intent IDs refer to [PYRAMID-OF-INTENTS.md](../PYRAMID-OF-INTENTS.md).

---

## 1. Big picture

```
                     ┌────────────── eframe (egui 0.37) ──────────────┐
                     │  App::logic(ctx, frame)   ← called every frame │
                     │     • poll mpsc queue (Msg)                    │
                     │     • (future: job queue, config, timers)      │
                     │  App::ui(ui, frame)                           │
                     │     • Panel::top      → title, search, toggles │
                     │     • Panel::bottom   → status bar/spinner     │
                     │     • Panel::left     → virtualized list       │
                     │     • CentralPanel    → detail pane / extract  │
                     └───────┬──────────────────────────▲─────────────┘
                 spawn       │                          │ mpsc::Sender<Msg>
        ┌────────────────────▼───────────┐   ┌──────────┴──────────┐
        │ Scan worker (one thread)       │   │ Details worker      │
        │  for src in available_sources: │   │  src.details(pkg)   │
        │    src.list_packages() ────────┼──►│  keyed by kind:name │
        │    send(SourceDone)            │   └─────────────────────┘
        │  ctx.request_repaint()         │   ┌─────────────────────┐
        └────────────────────────────────┘   │ Extract worker      │
                                             │  archive? unpack    │
        ┌────────────────────────────────┐   │    : copy installed │
        │ Source trait (Send)            │   │  send(ExtractDone)  │
        │  rpm │ deb │ flatpak │ brew    │   └─────────────────────┘
        │  + which() auto-detection      │
        └────────────────────────────────┘
```

Two egui/eframe 0.37 entry points (the API changed in 0.37 — there is **no** `update(ctx)`):

| Method | Role in this app |
|--------|------------------|
| `App::logic(&mut self, ctx, frame)` | Drains the message queue. Runs even when the window is hidden (as long as a worker requested a repaint). |
| `App::ui(&mut self, ui, frame)` | Builds all widgets from the current state. Must stay pure — no blocking calls. |

Workers are plain `std::thread`s (no async runtime — every manager interaction is one blocking subprocess anyway) with `mpsc::channel::<Msg>()`. Each worker clones the `egui::Context` and calls `request_repaint()` after sending, so the frame wakes up immediately (`I.5.s3`).

---

## 2. Module map

| Module | Owns | Depends on |
|--------|------|-----------|
| `main.rs` | arg parsing → `cli::run()` or `eframe::run_native` (window 1280×820, min 900×600) | `app`, `cli` |
| `app.rs` | `PkgApp` state machine, all panels/widgets, message handling, headless tests | `extract`, `models`, `sources` |
| `models.rs` | `PkgKind`, `Package` (cheap list row), `PackageDetails` (lazy), `ScanResult` | — |
| `sources/mod.rs` | `Source` trait, `all_sources()` / `available_sources()`, `run(Command)` helper, `which()`, size formatting/parsing | `models` |
| `sources/{rpm,deb,flatpak,brew}.rs` | one backend each: detect, list, details | `sources::mod` |
| `extract.rs` | archive discovery, unpack commands, installed-file copy, path mapping | `models` |
| `cli.rs` | `--list` / `--extract` / `--help`, exit codes | `sources`, `extract` |

Layering rule: **`app.rs` knows about UI; `sources/*` know about CLIs; `extract.rs` knows about files.** No layer reaches across (e.g. sources never import egui).

---

## 3. Data model

```rust
enum PkgKind { Deb, Rpm, Flatpak, Brew }      // Ord → list sorting by kind

struct Package {                               // loaded for EVERYTHING (cheap)
    kind:    PkgKind,
    name:    String,
    version: String,                           // epoch stripped ("0:"-prefix)
    arch:    String,
    size:    Option<u64>,                      // bytes; None if manager doesn't say
    summary: String,
}

struct PackageDetails {                        // loaded LAZILY on selection
    description: String,
    homepage: String,
    maintainer: String,
    license: String,
    install_date: String,                      // display-ready string
    section: String,                           // rpm group / dpkg section / flatpak branch
    dependencies: Vec<String>,                 // normalized across managers
    files: Vec<PathBuf>,                       // absolute (deb/rpm/flatpak) or prefix-relative (brew)
    extra: Vec<(String, String)>,              // free-form rows (Cellar, tap, remote, …)
}
```

- Cache key: `"{kind:?}:{name}"` (`package_key()`). Selection re-clicks are no-ops; a failed load still caches a `PackageDetails` carrying the error text so the pane explains itself.
- `PkgKind: Ord` is what the fixed sort (`kind → name`) relies on; column sorting (Phase 2) will replace it with a configurable comparator.

### Message protocol

```rust
enum Msg {
    SourceDone  { source: String, result: ScanResult },   // per manager, streams in
    Details     { key: String,   result: anyhow::Result<PackageDetails> },
    ExtractDone { result: anyhow::Result<String> },
}
```

`ScanResult { packages, error: Option<String> }` — **both** may be present (e.g. brew fallback list + warning). The pump always extends the list, then records the error (`I.5.s6`: fail soft, report visibly).

---

## 4. Frame lifecycle

```
worker: list_packages() ──► tx.send(Msg::SourceDone) ──► ctx.request_repaint()
                                                                      │
eframe:  logic()  ◄────────────────────────────────────────────────────┘
           └─ rx.try_recv() loop → mutate PkgApp (sort, status, counters)
         ui()
           ├─ Panel::top      … enabled while !scanning, Refresh restarts scan
           ├─ Panel::bottom   spinner + status + error chip (hover = manager stderr)
           ├─ Panel::left     filtered_indices() → ScrollArea::show_rows (virtualized)
           └─ CentralPanel    details pane; start_details() only on real selection change
```

Selection rule: clicking a row sets `selected` and spawns the details worker; the worker is skipped when the cache already holds that key. Filters are computed **client-side every frame** (`filtered_indices()`) — search never re-triggers a scan (`I.1.b`).

---

## 5. Threading & ownership rules

1. Workers only own: cloned `Sender`, cloned `egui::Context`, and owned copies of `Package`/`PackageDetails`. No shared mutable state (`I.5.s3`).
2. `Source` trait objects cross threads ⇒ the trait is `Send` (`I.5.s2`).
3. The UI thread is the only writer of `PkgApp`; the queue is the only channel in.
4. Every `thread::spawn` sends *something* (`ExtractDone` even on error) — a lost message would freeze a spinner forever.
5. File caps in walkers (flatpak depth 12 / brew depth 14 / 50 000 entries) keep worst-case details bounded (`C.3`).

---

## 6. UI composition notes

- Panels use the unified `egui::Panel` API (`left/right/top/bottom`), taking `&mut Ui` in 0.37 — not the old `SidePanel/TopBottomPanel::show(ctx)`.
- Long lists (package rows, file rows) use `ScrollArea::show_rows` with measured `text_style_height` — only visible rows are laid out.
- The metadata block is a 2-column `Grid`; unknown fields from any manager land in `extra` rows so new backend info shows up without UI changes.
- Buttons that trigger workers capture a pre-cloned `egui::Context` (`let ctx = ui.ctx().clone()` at the top of `ui()`), because closures can't borrow `self` and `ui` ambiguously.
- `egui::Button::on_hover_text` doesn't exist — hover text comes from the `Response` of `ui.add_enabled(...)`.

---

## 7. Testing strategy (no display required)

`PkgApp::with_ctx(&egui::Context)` is the seam: production code uses `PkgApp::new(cc)`, tests build against a bare context.

```rust
let ctx = egui::Context::default();
let mut app = PkgApp::with_ctx(&ctx);          // spawns the real scan
let mut frame = eframe::Frame::_new_kittest(); // hidden eframe API for exactly this
app.logic(&ctx, &mut frame);                   // drain Msg queue
let out = ctx.run_ui(RawInput::default(), |ctx| {
    CentralPanel::default().show(ctx, |ui| app.ui(ui, &mut frame));
});
out.textures_delta.clear();                    // no GPU: drop texture updates on purpose
```

Two tests ship (`src/app.rs`):

| Test | Asserts |
|------|---------|
| `headless_ui_renders` | building every panel for N frames never panics |
| `scan_and_load_details` | with any manager present: scan yields packages, details of package #0 load with a non-empty file list |

Renderer-less frames panic in `epaint` if `TexturesDelta` is dropped unapplied — that's why `.clear()` is mandatory.

More: [development.md](development.md).

---

## 8. Design rules for new code (checklist)

- [ ] Blocking work (subprocess, file walk > ~10 ms) happens on a worker, never in `ui()` (`I.5.s3`).
- [ ] New manager = implement `Source` in one file + register in `all_sources()`; detection via `which()` (`I.5.s2`, `I.6.1.1`).
- [ ] Parse failures return `ScanResult { error }`, not panics; partial results still ship (`I.5.s6`).
- [ ] CLI and GUI must share the same code path for any capability (`I.5.s7`).
- [ ] Mutating actions (future) run behind `allow_mutations` config + confirmation + `pkexec`, never embedded sudo (`C.1`).
- [ ] New backlog work references an intent ID; if none fits, extend the pyramid first (`TODOS.md` hygiene rule).

---

## 9. Key dependencies

| Crate | Why |
|-------|-----|
| `eframe` 0.37 | window + egui renderer (glow), AccessKit a11y, storage |
| `rfd` 0.17 | native folder picker (GTK/XDG portal) for Extract |
| `anyhow` | error plumbing across threads |
| `serde` + `serde_json` | `brew info --json=v2` parsing; `PkgKind` serialization groundwork |

Everything else is `std` + subprocesses. See [backends.md](backends.md) for the exact external commands.
