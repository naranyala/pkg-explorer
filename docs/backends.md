# Package-manager backends

Each backend is a `Source` impl in `src/sources/`. All of them **shell out to the manager's CLI** — that's the stable contract (`I.5.s1`). This document records exactly what runs, how output is parsed, and which quirks are handled.

Common contract (see `src/sources/mod.rs`):

```rust
trait Source: Send {
    fn kind(&self) -> PkgKind;
    fn name(&self) -> &'static str;                 // shown on errors
    fn is_available(&self) -> bool;                 // which(<cli>)
    fn list_packages(&self) -> ScanResult;          // cheap; must not panic
    fn details(&self, &Package) -> anyhow::Result<PackageDetails>;
}
```

`run(&mut Command)` captures stdout, and on non-zero exit returns `Err("<program> failed: <stderr>")` — stderr is what eventually shows in the status-bar error chip.

---

## Detection

| Source | Probe | Rationale |
|--------|-------|-----------|
| rpm | `which rpm` | present on Fedora/RHEL/openSUSE/Mageia |
| deb | `which dpkg-query` | Debian/Ubuntu (avoids plain `dpkg` name collisions) |
| flatpak | `which flatpak` | can coexist with any of the above |
| brew | `which brew` | Linuxbrew in `/home/linuxbrew/.linuxbrew` or `/usr/local` |

Unavailable sources are **omitted entirely** (`available_sources()`), so a minimal Fedora box shows just rpm + whatever else exists (`I.6.1.1`). Detection is re-evaluated on every Refresh.

> Note: `which()` itself shells out to the `which` binary — fine on all mainstream distros, but worth knowing if you target a system without coreutils-style `which`.

---

## rpm (`src/sources/rpm.rs`)

### List
```
rpm -qa --qf '%{NAME}\t%{EPOCHNUM}:%{VERSION}-%{RELEASE}\t%{ARCH}\t%{SIZE}\t%{SUMMARY}\n'
```
- One TSV row per package; `%{EPOCHNUM}` yields `0` when no epoch — the `0:` prefix is stripped so rows look like `1.2.3-4.el10`.
- `%{SIZE}` is **bytes** → used directly as `Package.size`.
- Rows sorted by name after collection.

### Details
```
rpm -qi <name>     # metadata block
rpm -qR <name>     # requirements → dependencies
rpm -ql <name>     # file list (absolute paths)
```
- `rpm -qi` parsing: `Key : value` lines; **continuation lines start with a space** and are appended to the description (handled).
- `Size` is converted with `format_size()` (bytes → KiB/MiB/… display).
- Multi-line/`(none)` values tolerated; unknown keys are ignored rather than failing.

### Quirks handled
- Epoch normalization (never displays `0:`).
- Description wrapping (`-qi` indents continuation lines).

---

## deb (`src/sources/deb.rs`)

### List
```
dpkg-query -W -f '${Package}\t${Version}\t${Architecture}\t${Installed-Size}\t${Summary}\n'
```
- **`Installed-Size` is in KiB** per dpkg convention → multiplied by 1024 to get bytes (a classic off-by-1024 bug if missed).
- Rows sorted by name.

### Details
```
dpkg-query -W -f <multi-field template> <name>   # one shot: status, size, maintainer,
                                                 # homepage, section, depends, description
dpkg -L <name>                                  # file list (absolute paths)
```
- Custom template terminated by `===END===` so parsing is unambiguous.
- Description block: `Description: <summary>` then **continuation lines indented by a space** — collected until the next `key:` field.
- `Depends` split on `,` (versions and alternatives kept verbatim inside each item).
- `Installed-Size` converted KiB → bytes.

### Notes
- Lists *all* dpkg states, including config-files-only leftovers (`deinstall ok config-files`) — v0.1 does not filter those; a future "orphans" view will (`I.2.5`).

---

## flatpak (`src/sources/flatpak.rs`)

### List
```
flatpak list --columns application,version,branch,arch,installation,size,description
```
- TSV. Columns map to: `name, version, branch(→section), arch, installation, size, summary`.
- `size` is a **human string** (`"1.2 GB"`) → `parse_human_size()` (KiB/MiB/GiB + bare B, tolerant of `kB`/`KiB`/trailing `B`).
- Summary falls back to `"{branch} · {installation}"` when the description column is empty.
- Covers user **and** system installations (default `flatpak list` behaviour).

### Details
```
flatpak info <app-id>                 # key: value metadata
flatpak info --show-location <id>      # deployment dir
```
- `Installed size` (human string) parsed into the size helpers again.
- **File list** is not a CLI feature — it walks `<deployment>/files/` recursively:
  - paths stored **absolute** (consistent with rpm/deb, and what extraction expects),
  - caps: depth 12, 50 000 entries (then silently stops — UI shows the count it got).

### Quirks handled
- Location may be under `/var/lib/flatpak/…` (system) or `~/.local/share/flatpak/…` (user) — both work because we never hardcode it.

---

## brew (`src/sources/brew.rs`)

### List (primary path)
```
brew info --json=v2
```
- `formulae[]`: only entries with `installed[]` non-empty (i.e. actually installed); version taken from `installed[0].version`; `desc` → summary.
- `casks[]`: `token`/`version`/`desc`, arch-tagged `"cask"`.

### List (fallback path) — **important**
`brew info --json=v2` is a single monolithic command that can die on one broken cask (observed upstream crash: `key not found: :prefpanedir` in `cask/config.rb`). When that happens:

```
brew list --formula --versions
brew list --cask --versions
```
- Parses `name version…` per line; summaries empty.
- Returns **packages + error together**: `ScanResult.error = "using fallback list: brew failed: …"` (first two lines of the traceback only — `short_error()`).
- The GUI shows this as the red status chip while the list still populates (`I.5.s6`). Same fallback triggers if JSON parses but contains zero installed packages.

### Details
```
brew info --json=v2 <name>     # best effort — can fail the same way
```
- Metadata (`desc`, `homepage`, `license`, `dependencies`, `name/fullName/tap/versioned_formulae`, install time) filled when JSON succeeds.
- **File list always comes from disk**, not the CLI:
  - `brew --prefix` (cached in a `OnceLock`; fallback `/home/linuxbrew/.linuxbrew`),
  - walks `Cellar/<name>/*` and `Caskroom/<name>/`,
  - paths stored **relative to the brew prefix** (`Cellar/bat/0.26.1/bin/bat`),
  - also sums bytes for an "Installed size" extra row,
  - caps: depth 14, 50 000 entries.
- If metadata failed **and** nothing is on disk → `Err` (surfaces as "details: …" in status). If disk walk found files → details succeed with an explanatory description line.

### Why prefix-relative?
Because extraction re-joins `prefix + rel` (see [extraction.md](extraction.md)). Keeping brew paths relative also makes the Files list readable instead of repeating `/home/linuxbrew/.linuxbrew/` on every row.

---

## Cross-source normalization

| Field | rpm | deb | flatpak | brew |
|-------|-----|-----|---------|------|
| `version` | `1.2.3-4.el10` (epoch stripped) | verbatim | ref version | formula version |
| `size` | bytes `%{SIZE}` | KiB×1024 | human string→bytes | None in list; summed from Cellar walk in details |
| `arch` | `%{ARCH}` | `${Architecture}` | arch column | `"brew"` / `"cask"` |
| `summary` | `%{SUMMARY}` | `${Summary}` | description, else branch·installation | `desc`, empty on fallback |
| `files` | absolute (`rpm -ql`) | absolute (`dpkg -L`) | absolute (walk of `files/`) | prefix-relative (walk) |
| `dependencies` | `rpm -qR` lines | `Depends` split `,` | — (runtime shown via extra rows) | `dependencies[]` |

Rows with different semantics are normalized only where the UI needs them; everything manager-specific goes into `details.extra` as `("Key", "value")` rows so nothing is lost.

---

## Adding a new backend (checklist)

1. Create `src/sources/<name>.rs` with `pub struct <Name>Source;`.
2. Implement `Source`: `kind()` (add a `PkgKind` variant + label), `name()`, `is_available()` via `which("<main-cli>")`, `list_packages()`, `details()`.
3. Respect the contracts: cheap list (one command, no walks), lazy details, `ScanResult.error` for soft failures, no panics, `Send`.
4. Register it in `all_sources()` (`src/sources/mod.rs`).
5. Keep `cli.rs` working: `parse_kind()` gets a new arm (`--list <kind>`, `--extract <kind> …`).
6. Add the toggle-chip position (UI array `kind_enabled` is indexed by `PkgKind as usize` — keep enum order in sync!).
7. Add a parser fixture test (Phase 2 will standardize these in `tests/parsers.rs`).
8. Document the commands + quirks in **this file**, and claim the intent `[I.5.s2]` in `TODOS.md`.

Candidates on the parking lot: `pacman`, `zypper`, `apk`, `nix profile`.
