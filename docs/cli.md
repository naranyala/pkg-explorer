# CLI reference

`pkg-explorer` doubles as a command-line tool — same code paths as the GUI (`I.5.s7`), so anything the GUI can list or extract, scripts can too.

```
pkg-explorer                  # launch the desktop app
pkg-explorer --help           # usage
pkg-explorer --list [KIND]    # TSV package list
pkg-explorer --extract <KIND> <name> <dest-dir>
```

`KIND` ∈ `deb` | `rpm` | `flatpak` | `brew` (case-insensitive; `homebrew` accepted as an alias for `brew`).

Exit codes: **0** success · **1** error (message on `stderr`). Unknown flags also exit 1 after printing usage.

---

## `--list [KIND]`

Scans the selected manager(s) — all four when `KIND` is omitted — and prints one package per line to **stdout**:

```
<kind>\t<name>\t<version>\t<size>\t<summary>
```

- Tab-separated, so `cut -f2`, `column -t -s$'\t'`, and pandas/`read_csv(sep='\t')` all work.
- `<size>` is already human-formatted (`12.3 MiB`, `—` when the manager doesn't report a size). Raw bytes are not (yet) available — a `--json` flag with numbers is planned (Phase 1/2, `TODOS.md`).
- `<summary>` may be empty (notably brew fallback listings).
- Diagnostics — source errors, the final `# <n> packages` count — go to **stderr**, so stdout stays pipe-clean.

### Examples

```bash
# biggest rpm packages (size sorts fine within one unit family, but prefer this)
pkg-explorer --list rpm | sort -t$'\t' -k5 -h -r | head

# all package names, any manager
pkg-explorer --list | cut -f2 | sort -u

# bash completion-ish check: is flatpak in use?
pkg-explorer --list flatpak | head -3

# JSON-friendly: dump raw fields
pkg-explorer --list | awk -F'\t' '{printf "%s %s %s\n", $1, $2, $3}'
```

### Real output (AlmaLinux 10 host)

```
rpm	Goose	1.50.0-1	613.8 MiB	Goose App
rpm	ModemManager	1.22.0-7.el10	4.4 MiB	Mobile broadband modem management service
flatpak	ai.storyteller.artcraft-launcher	0.4.1	28.1 MiB	Install, update and launch the Craft App Suite · stable
brew	bat	0.26.1	—	
```

---

## `--extract <KIND> <name> <dest-dir>`

Resolves `<name>` against the installed list of `<KIND>`, loads its details (file manifest), then extracts:

1. if a cached `.deb`/`.rpm` archive is found → unpack it (`dpkg-deb -x` / `rpm2cpio | cpio`);
2. otherwise copy the installed files into `<dest-dir>`.

`<dest-dir>` is created if missing. Files are written **directly into it** (paths like `<dest>/usr/bin/…` for rpm/deb) — currently no per-package subfolder (planned, Phase 1).

**Progress** prints to stderr, one line per top-level entry:

```
[1/26] /home/linuxbrew/.linuxbrew/Cellar/bat/0.26.1/.brew
[2/26] /home/linuxbrew/.linuxbrew/Cellar/bat/0.26.1/.brew/bat.rb
…
```

The final summary goes to **stdout** on success:

```
Copied 26/26 installed file(s)/dir(s) to /tmp/out
```

or a non-zero exit with `error: …` on stderr.

### Examples

```bash
pkg-explorer --extract rpm systemd /tmp/systemd-payload
pkg-explorer --extract deb coreutils /tmp/deb-out
pkg-explorer --extract flatpak org.gimp.GIMP /tmp/gimp-files
pkg-explorer --extract brew bat /tmp/bat

# chain it
dest=$(mktemp -d) && pkg-explorer --extract rpm zsh "$dest" && grep -r "zshrc" "$dest"
```

### Typical errors

| Message | Cause | Fix |
|---------|-------|-----|
| `package <name> not found` | not installed under that manager (name must match exactly, no globs yet) | check with `--list <kind>` |
| `no source for …` / `<tool> not available` | manager not on this machine | use another KIND |
| `No cached archive and no installed files found …` | empty manifest and no archive | details load failed earlier; retry/select a package whose details render in the GUI |
| `… exited with status: …` (`dpkg-deb`/`cpio`) | archive corrupt or tool missing | install `cpio`/`dpkg-deb`; inspect stderr line above |

---

## `--help`

```
pkg-explorer — GUI by default; CLI helpers:
  pkg-explorer --list [deb|rpm|flatpak|brew]
  pkg-explorer --extract <deb|rpm|flatpak|brew> <name> <dest-dir>
```

---

## Not yet available (planned)

From [TODOS.md](../TODOS.md) Phases 1–3:

- `--info <KIND> <name>` — print full details (text or `--json`)
- `--export <KIND> <name> <file>` — file list as `txt`/`csv`/`json`
- global `--json` — machine-readable output everywhere (numbers, not human sizes)
- `--verbose` — scan timings

Until they exist, parse `--list` output or use the GUI.

---

## Scripting notes

- The CLI **detects managers the same way as the GUI** — a machine without `brew` simply skips it.
  - `--list <kind>` for a manager that isn't installed prints nothing for that kind and exits **0** (with `# 0 packages` on stderr) — it is *not* an error.
  - `--extract <kind> …` for a manager that isn't installed exits **1** with `error: <kind> not available` — you asked for a specific package, so silence would be wrong.
- Brew may emit `using fallback list: brew failed: …` on stderr while still listing packages (known upstream JSON crash — see [troubleshooting.md](troubleshooting.md)). Check exit code, not stderr, for success.
- Keep `dest-dir` on a filesystem with room — extraction copies real bytes; big flatpaks can be multi-GB.
