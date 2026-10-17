# Extraction — how "⬇ Extract…" gets files out of a package

Implementation: `src/extract.rs`. Public entry:

```rust
extract(pkg, details, dest, progress) -> anyhow::Result<String>
```

Runs on a background worker; `progress(done, total, path)` is a callback (the CLI prints it, the GUI currently ignores it — a real progress bar is Phase 1 `[I.3.1.3]`).

---

## 1. Strategy decision tree

```
extract(pkg, details, dest)
│
├─ find_archive(pkg) hits a cached .deb / .rpm ?
│    ├─ deb   → dpkg-deb -x <archive> <dest>          [exact archive bytes]
│    └─ rpm   → sh -c "rpm2cpio <archive> | cpio -idmu --quiet -D <dest>"
│
└─ else (flatpak, brew, deb/rpm without cache)
     └─ copy_installed_files():  manifest = details.files
          for each path that still exists → copy_path(src, dest/…)
```

**Archive wins** because it reconstructs the original payload (correct empty dirs, `%ghost` handling for rpm, no dependence on the live FS). The copy path is the universal fallback and the *only* option for flatpak and brew (`I.3.1.2`).

### Archive search locations (hardcoded today)

| Kind | Searched (recursive) |
|------|----------------------|
| deb  | `/var/cache/apt/archives`, `/var/cache/apt` |
| rpm  | `/var/cache/dnf`, `/var/cache/yum`, `/var/cache/PackageKit`, `/var/cache/apt/archives` |

Matching: filename must start with `<name>-<version>-<arch>` (rpm also tries `<name>-<version>`). Nested cache dirs (dnf's per-repo `packages/` folders) are traversed.

Typical reality: **package caches are empty on updated systems** — so on most days you'll get the copy path. Planned improvement: fetch-on-demand via `dnf download` / `apt-get download` / `brew fetch` first (`I.3.1.7`).

### External tools required

| Path | Tools |
|------|-------|
| deb archive | `dpkg-deb` |
| rpm archive | `rpm2cpio`, `cpio` (invoked through one `sh -c` — the **only** shell string in the codebase; Phase 5 will replace it with a piped spawn for argument-safety) |
| copy path | none (pure Rust + std::fs) |

---

## 2. Where files land (path mapping)

`dest` is the folder the user picked (rfd dialog / CLI argument). Current behaviour v0.1: files go **directly under `dest`** (a `<dest>/<pkg>-<ver>/` subfolder is planned, Phase 1 `[I.9.a]`).

| Source of manifest | Example manifest path | Written as |
|--------------------|----------------------|------------|
| deb (`dpkg -L`) | `/usr/bin/foo` | `dest/usr/bin/foo` (leading `/` stripped) |
| rpm (`rpm -ql`) | `/usr/bin/foo` | `dest/usr/bin/foo` |
| flatpak (walk) | `/var/lib/flatpak/app/<id>/…/files/bin/x` | `dest/var/lib/flatpak/app/…/files/bin/x` |
| brew (walk) | `Cellar/bat/0.26.1/bin/bat` (prefix-relative) | `dest/Cellar/bat/0.26.1/bin/bat` after re-joining `brew --prefix` |

So the output preserves the *package's own view of the filesystem* — extracted rpms/debs are rooted like `/`, which makes `diff -r` against the live system or a chroot-style inspection straightforward.

### brew special case

```rust
roots = details.files.map(|rel| brew_prefix().join(rel))   // absolutize for reading
rel   = src.strip_prefix(brew_prefix())                     // de-absolutize for writing
```
Both directions go through the same prefix (cached `OnceLock`), so a stale prefix would break in one consistent place. `Caskroom/<name>` paths are handled the same way.

---

## 3. copy_path() semantics

```rust
copy_path(src, dst):
  if src.is_dir()   → create_dir_all(dst); recurse into entries
  if src is file    → create_dir_all(parent); if dst exists: remove it; fs::copy(src, dst)
```

Details that matter:

- **Read-only destinations**: `fs::copy` preserves source mode — a `r-xr-xr-x` binary copied once becomes unwritable, so re-running extraction would fail with `EACCES` on open. Fix: existing destination files are removed before copying. (Found by real-world testing of brew `bat`.)
- **Dir permissions** are *not* copied (`create_dir_all` defaults 755+umask) — keeps previously copied trees traversable.
- **Symlinks**: `fs::copy` follows them (materializes the target content). Symlink *entries* in a walked tree are seen by `read_dir` and copied as their targets. Consequence: an absolute symlink like `libfoo.so → /lib/x86_64/libfoo.so.2` pulls in target bytes instead of recreating the link — acceptable for inspection, documented limitation (preview/relink is future work).
- **Double visits**: manifests contain both a directory and its children (walk pushes dirs too). The dir copy already copies the children, then each child is copied again — idempotent and cheap for typical sizes, paid for simplicity.

---

## 4. Progress, partial failure, errors

```rust
processed/copy ok  → progress(processed, total, path); count copied
copy error         → recorded in failed[] as "<path>: <io error>"; loop CONTINUES
nothing copied     → bail!("copy failed: <first error>")
some failed        → Ok("Copied 26/26 …" / "… (2 failed, first: …)")
```

- One unreadable file no longer aborts a whole extraction (`C.8`: report honestly).
- The final summary string is exactly what the status bar shows (✓ green / ✗ red via `ExtractDone`).
- Current v0.1 has **no cancel button and no temp-dir staging** — both are Phase 1 (`[I.3.1.3]`, `[I.9.a]`). Until then, a cancelled/killed app can leave a partially copied tree at `dest`.

---

## 5. Safety (`C.1`)

| Check | Status |
|-------|--------|
| Never runs as root; no sudo anywhere | ✅ |
| Destination chosen explicitly by the user | ✅ |
| Path traversal (`..`, absolute entries) from **archive** payloads | ⚠️ `dpkg-deb`/`rpm2cpio\|cpio` write whatever the archive contains — today we trust distro caches (root-owned dirs). Hardening audit scheduled Phase 1 `[C.1]` |
| Path traversal from **manifests** | manifests are absolute `/…` or prefix-relative `Cellar/…`; `strip_prefix("/")` normalizes. A hypothetical malicious manifest with `../` would escape `dest` — same audit applies |
| Overwriting user files in `dest` | files with existing names are replaced (see read-only note above). Planned: extract into a package subfolder `[I.9.a]` |

---

## 6. Failure modes → what the user sees

| Situation | Result |
|-----------|--------|
| Archive found, `dpkg-deb`/`cpio` missing | ✗ `dpkg-deb: No such file…` / `rpm2cpio/cpio: …` in status bar |
| No archive, manifest empty (e.g. metadata load failed earlier) | ✗ `No cached archive and no installed files found for <pkg> — cannot extract` |
| Some files vanished since install (removed by admin) | partial success: `Copied 1480/1487 … (7 failed, first: /usr/…: No such file)` |
| Destination not writable (e.g. `/usr`) | ✗ first `create_dir_all` permission error |
| Everything failed | ✗ `copy failed: <first error>` |

The GUI shows `✓ <summary>` or `✗ extract failed: <error>` in the status bar; the CLI prints the summary to stdout (exit 0) or the error to stderr (exit 1).

---

## 7. Testing

- End-to-end verified during v0.1 development: flatpak copy (60 paths), brew copy (26/26 after the read-only fix), rpm archive pipeline (`rpm2cpio | cpio` sanity run against a cached `.rpm`).
- Unit tests for pure mapping logic (brew prefix-join/strip, dest joining, read-only overwrite) are a Phase 2 item (`TODOS.md` → "extract path-mapping unit tests") — they don't need any package manager installed.
- Manual test recipe:
  ```bash
  cargo run --release -- --extract brew bat /tmp/out
  find /tmp/out -type f | head
  cargo run --release -- --extract rpm <installed-pkg-with-cached-rpm> /tmp/out2
  ```
