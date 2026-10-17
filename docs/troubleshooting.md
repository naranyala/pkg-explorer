# Troubleshooting & FAQ

Known issues, their explanations, and workarounds. Found something new? Add it here (and a failing test before the fix — project rule).

---

## Known issues

### 1. Homebrew: `brew info --json=v2` crashes → "using fallback list"

**Symptom** (status bar red chip / CLI stderr):

```
brew (Homebrew): using fallback list: brew failed: Error: key not found: :prefpanedir
/home/linuxbrew/.linuxbrew/Homebrew/Library/Homebrew/cask/config.rb:225:in `Hash#fetch' …
```

**What happens**: PKG Explorer's primary brew listing uses `brew info --json=v2`, a single monolithic Homebrew command. On this system it dies inside cask config resolution (`:prefpanedir` missing — an upstream Homebrew bug triggered by the installed cask set / configuration).

**Effect**: the app **still lists** brew packages — it automatically falls back to `brew list --formula --versions` + `brew list --cask --versions` (name + version only, no summaries). Package details may be reduced (metadata command can fail the same way), but file lists and extraction still work because they come from walking `Cellar/`/`Caskroom/` on disk.

**Workarounds**:
- Live with it — list + extract work.
- Try `brew doctor` / upgrade Homebrew; check whether a specific cask with a broken `prefpanedir` stanza is the culprit (`brew info --json=v2` by hand to reproduce).
- Track: the error text is shown verbatim (first two lines) in the status-bar chip — hover it for the full message.

### 2. `xkbcommon: ERROR: couldn't find a Compose file for locale "en_US.UTF-8"`

**Symptom**: one line on stderr every launch.

**Cause**: your locale/Compose setup (common in minimal or non-GNOME setups). It comes from the windowing stack, not from PKG Explorer's code.

**Effect**: keyboard compose sequences (dead keys) may not work; the app itself runs fine.

**Fix** (optional): install the compose data for your locale, e.g. `sudo dnf install xkbcommon-x11` / ensure `/usr/share/X11/locale/en_US.UTF-8/Compose` exists, or run under a locale the system actually ships (`LANG=C.UTF-8` for testing).

### 3. No folder dialog opens on "⬇ Extract…"

**Cause**: `rfd` needs GTK at runtime (build needs `gtk3-devel`); on pure-Wayland sessions it may go through the XDG portal instead.

**Checks**:
```bash
pkg-config --exists gtk+-3.0 && echo gtk-ok     # build-time
ls $XDG_RUNTIME_DIR/bus                          # session bus present?
```
**Fix**: install `gtk3` (runtime + devel), or run from a full desktop session rather than a bare compositor.

### 4. Extract fails: `rpm2cpio/cpio: …` or `dpkg-deb: … not found`

**Cause**: archive-unpack path needs `cpio` (rpm) or `dpkg-deb` (deb). They're separate packages on some distros.

**Fix**: `sudo dnf install cpio` / `sudo apt install cpio dpkg` — or accept the copy-path fallback: extraction automatically falls back to copying installed files when **no archive is found**; it does *not* fall back when an archive was found but the tool is missing (error tells you which tool).

### 5. Extract says `No cached archive and no installed files found`

**Cause**: the package has no cached archive in the known cache dirs **and** its file manifest is empty (details failed earlier, or the package owns no files, e.g. some metapackages).

**Fix**: open the package in the GUI first — if the Files section is empty/missing there, details loading failed (see status-bar error). For metapackages with genuinely no files, there's nothing to extract.

### 6. `error: package <name> not found` (CLI)

**Cause**: exact-name mismatch, or wrong KIND. Names are case-sensitive and must match the manager's name (flatpak = full app id, brew = formula/cask token).

**Fix**: `pkg-explorer --list <kind> | grep -i <fragment>` and use the exact name. Glob extraction is not implemented yet.

### 7. Extracted tree looks duplicated / files re-copied

**Expected (v0.1)**: manifests contain directories *and* their children, so the copy loop visits both — directory first (recursively), then each child again. Idempotent, slightly wasteful; performance-neutral at typical sizes. Dedup is a possible Phase-2 cleanup.

### 8. Partial extraction left a half-filled destination

**Cause**: v0.1 writes directly into the chosen folder; killing the app mid-copy leaves whatever was written so far. Also see §4: a mid-run tool failure aborts the archive path but the copy path continues and reports `Copied X/Y (Z failed…)`.

**Fix**: delete the destination and re-run when it matters. Cancel-safe staging (temp dir + rename) and a Cancel button are Phase 1 items (`[I.9.a]`, `[I.3.1.3]`).

### 9. `cargo build` fails on `rfd`/GTK sys crates

Missing build deps — see [development.md](development.md) §1 (`gtk3-devel`, `pkg-config`).

### 10. Tests fail / hang on a machine with no package managers

`scan_and_load_details` scans the real system. With zero managers available it returns early (passes); with managers present it can take ~1–2 minutes — that's the real `brew info`/`rpm -qa` cost, not a hang. See the test's 180 s deadline.

### 11. SELinux denial when extracting (RHEL-family)

Copying package files into unusual destinations (e.g. under `~/.config`) can hit policy edges on enforcing SELinux systems. The app never needs root — if `cp: Permission denied`-style errors appear, check `ausearch -m avc -ts recent`. Choosing `$HOME/Downloads` or `/tmp` as destination avoids most cases.

---

## FAQ

**Is it safe? Does it need root?**
Read-only: it only runs manager *query* commands (`rpm -qa`, `dpkg-query`, `flatpak info/list`, `brew info/list`) plus extract-time file reads. No sudo, no mutations, no telemetry, no network beyond what `brew`/`flatpak` do internally when invoked. Mutating features (install/remove) are deliberately deferred and will be opt-in behind a config flag (`allow_mutations`, default off) with polkit prompts.

**Why shell out instead of parsing rpmdb/dpkg status files natively?**
The CLI surface is each manager's stable, documented contract; native DBs (rpmdb BDB/sqlite, dpkg status format) are version-fragile. Design intent `I.5.s1` — rationale in [architecture.md](architecture.md) and the [pyramid](../PYRAMID-OF-INTENTS.md).

**Where do extracted files go?**
Directly under the folder you pick (v0.1), preserving the package's own paths: rpm/deb → `dest/usr/...`; flatpak → `dest/var/lib/flatpak/...`; brew → `dest/Cellar/...`. A `dest/<pkg>-<ver>/` subfolder is planned (Phase 1).

**Why is brew size `—` but flatpak/rpm show sizes?**
Homebrew doesn't report installed sizes in the data we parse (the Cellar walk *does* compute one — it appears in the details pane as "Installed size", not in the list row). rpm/dpkg report sizes directly; flatpak publishes human-readable sizes.

**Can it install, update or remove packages?**
Not yet — v0.1 is browse/inspect/extract only. See Phase 5 of [TODOS.md](../TODOS.md) for the gated design.

**Which manager should I use for X?**
Not what this tool does — it shows *what is installed*, per manager, side by side. Remote search stays with `dnf search`/`apt`/`flatpak search`/`brew search` (anti-goal in the pyramid).

**Does it work on Arch (pacman) / openSUSE (zypper) / Alpine (apk)?**
The UI works (they'd simply have no sources), but no backend exists yet — they're on the parking lot in `TODOS.md`. Adding one is a ~150-line file + registration; checklist in [backends.md](backends.md).

**How do I report a bug?**
Include: distro + version, `pkg-explorer --list` output (stderr too), the exact manager command you ran by hand, and what you expected. Log/report bundling (`tracing`) is planned (Phase 4).

---

## Getting more detail

```bash
# what exactly does the app call? (run the same commands by hand)
rpm -qa --qf '%{NAME}\t%{EPOCHNUM}:%{VERSION}-%{RELEASE}\t%{ARCH}\t%{SIZE}\t%{SUMMARY}\n' | head
flatpak list --columns application,version,branch,arch,installation,size,description | head
brew info --json=v2 > /tmp/brew.json ; head -c 400 /tmp/brew.json

# stderr in GUI mode shows worker panics if any
pkg-explorer 2>&1 | tee /tmp/pkg-explorer.log
```
