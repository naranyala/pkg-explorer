use super::{Source, run, which};
use crate::models::{Package, PackageDetails, PkgKind, ScanResult};
use std::path::PathBuf;
use std::process::Command;

pub struct BrewSource;

impl Source for BrewSource {
    fn kind(&self) -> PkgKind {
        PkgKind::Brew
    }
    fn name(&self) -> &'static str {
        "brew (Homebrew)"
    }
    fn is_available(&self) -> bool {
        which("brew")
    }

    fn list_packages(&self) -> ScanResult {
        match run(Command::new("brew").args(["info", "--json=v2"])) {
            Ok(out) => {
                let json: serde_json::Value = match serde_json::from_str(&out) {
                    Ok(v) => v,
                    Err(e) => return fallback_list(format!("parse brew json: {e}")),
                };
                let mut packages = Vec::new();

                if let Some(arr) = json.get("formulae").and_then(|v| v.as_array()) {
                    for f in arr {
                        let name = str_of(f, "name");
                        let installed = f.get("installed").and_then(|v| v.as_array());
                        let Some(inst) = installed.and_then(|a| a.first()) else {
                            continue;
                        };
                        let version = str_of(inst, "version");
                        let summary = str_of(f, "desc");
                        packages.push(Package {
                            kind: PkgKind::Brew,
                            name,
                            version,
                            arch: "brew".into(),
                            size: None,
                            summary,
                        });
                    }
                }
                if let Some(arr) = json.get("casks").and_then(|v| v.as_array()) {
                    for c in arr {
                        let name = str_of(c, "token");
                        let version = match c.get("version").and_then(|v| v.as_str()) {
                            Some(v) => v.to_string(),
                            None => continue,
                        };
                        let desc = str_of(c, "desc");
                        packages.push(Package {
                            kind: PkgKind::Brew,
                            name,
                            version,
                            arch: "cask".into(),
                            size: None,
                            summary: desc,
                        });
                    }
                }
                if packages.is_empty() {
                    return fallback_list(
                        "brew info --json=v2 returned no installed packages".into(),
                    );
                }
                packages.sort_by(|a, b| a.name.cmp(&b.name));
                ScanResult {
                    packages,
                    error: None,
                }
            }
            Err(e) => fallback_list(short_error(&e.to_string())),
        }
    }

    fn details(&self, pkg: &Package) -> anyhow::Result<PackageDetails> {
        let mut d = PackageDetails::default();
        let mut meta_err: Option<anyhow::Error> = None;

        match run(Command::new("brew").args(["info", "--json=v2", &pkg.name])).and_then(|out| {
            serde_json::from_str::<serde_json::Value>(&out)
                .map_err(|e| anyhow::anyhow!("parse brew json: {e}"))
        }) {
            Ok(json) => {
                let obj = json
                    .get("formulae")
                    .and_then(|v| v.as_array())
                    .and_then(|a| a.first())
                    .or_else(|| {
                        json.get("casks")
                            .and_then(|v| v.as_array())
                            .and_then(|a| a.first())
                    });

                if let Some(f) = obj {
                    d.description = str_of(f, "desc");
                    d.homepage = str_of(f, "homepage");
                    d.license = str_of(f, "license");
                    if let Some(deps) = f.get("dependencies").and_then(|v| v.as_array()) {
                        d.dependencies = deps
                            .iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect();
                    }
                    for key in ["name", "fullName", "tap", "versioned_formulae"] {
                        if let Some(v) = f.get(key) {
                            let s = match v {
                                serde_json::Value::String(s) => s.clone(),
                                serde_json::Value::Array(a) => a
                                    .iter()
                                    .filter_map(|x| x.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", "),
                                _ => continue,
                            };
                            if !s.is_empty() {
                                d.extra.push((key.to_string(), s));
                            }
                        }
                    }
                    if let Some(inst) = f
                        .get("installed")
                        .and_then(|v| v.as_array())
                        .and_then(|a| a.first())
                        && let Some(t) = inst.get("time").and_then(|v| v.as_i64())
                        && let Ok(dt) = chrono_long(t)
                    {
                        d.install_date = dt;
                    }
                }
            }
            Err(e) => meta_err = Some(e),
        }

        let prefix = homebrew_prefix();
        let cellars = cellar_dirs(&pkg.name);
        for root in &cellars {
            d.extra
                .push(("Install dir".into(), root.display().to_string()));
            let mut size = 0u64;
            collect_files(root, &prefix, &mut d.files, &mut size, 0);
            if size > 0 {
                d.extra
                    .push(("Installed size".into(), super::format_size(Some(size))));
            }
        }
        d.files.sort();

        if d.files.is_empty() && d.description.is_empty() {
            return Err(meta_err
                .unwrap_or_else(|| anyhow::anyhow!("no metadata or files for {}", pkg.name)));
        }
        if let Some(e) = meta_err
            && d.description.is_empty()
        {
            d.description = format!("brew metadata unavailable: {}", short_error(&e.to_string()));
        }
        Ok(d)
    }
}

/// Fall back to `brew list --versions` when `brew info --json=v2` fails.
fn fallback_list(primary_error: String) -> ScanResult {
    let mut packages = Vec::new();
    let mut errors = vec![primary_error];
    for (args, arch) in [
        (&["list", "--formula", "--versions"][..], "brew"),
        (&["list", "--cask", "--versions"][..], "cask"),
    ] {
        match run(Command::new("brew").args(args)) {
            Ok(out) => {
                for line in out.lines() {
                    let mut it = line.split_whitespace();
                    let (Some(name), Some(version)) = (it.next(), it.next()) else {
                        continue;
                    };
                    packages.push(Package {
                        kind: PkgKind::Brew,
                        name: name.to_string(),
                        version: version.to_string(),
                        arch: arch.to_string(),
                        size: None,
                        summary: String::new(),
                    });
                }
            }
            Err(e) => errors.push(format!("brew {}: {}", args[1], short_error(&e.to_string()))),
        }
    }
    if packages.is_empty() {
        return ScanResult {
            packages: vec![],
            error: Some(errors.join("; ")),
        };
    }
    packages.sort_by(|a, b| a.name.cmp(&b.name));
    ScanResult {
        packages,
        error: Some(format!(
            "using fallback list: {}",
            short_error(&errors.join("; "))
        )),
    }
}

/// Keep only the first couple of lines of a noisy Homebrew traceback.
fn short_error(msg: &str) -> String {
    msg.lines()
        .take(2)
        .filter(|l| !l.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn str_of(v: &serde_json::Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string()
}

fn cellar_dirs(name: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let base = homebrew_prefix().join("Cellar").join(name);
    if base.is_dir()
        && let Ok(versions) = std::fs::read_dir(&base)
    {
        for v in versions.flatten() {
            out.push(v.path());
        }
    }
    let caskroom = homebrew_prefix().join("Caskroom").join(name);
    if caskroom.is_dir() {
        out.push(caskroom);
    }
    out
}

fn homebrew_prefix() -> PathBuf {
    static PREFIX: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    PREFIX
        .get_or_init(|| {
            run(Command::new("brew").args(["--prefix"]))
                .map(|out| PathBuf::from(out.trim()))
                .unwrap_or_else(|_| PathBuf::from("/home/linuxbrew/.linuxbrew"))
        })
        .clone()
}

fn collect_files(
    dir: &std::path::Path,
    root: &std::path::Path,
    files: &mut Vec<PathBuf>,
    size: &mut u64,
    depth: usize,
) {
    if depth > 14 || files.len() > 50_000 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if let Ok(rel) = path.strip_prefix(root) {
            files.push(rel.to_path_buf());
        }
        match entry.file_type() {
            Ok(t) if t.is_dir() => collect_files(&path, root, files, size, depth + 1),
            Ok(t) if t.is_file() => {
                if let Ok(md) = entry.metadata() {
                    *size += md.len();
                }
            }
            _ => {}
        }
    }
}

/// Format a unix timestamp without pulling in chrono.
fn chrono_long(ts: i64) -> Result<String, ()> {
    let days = ts.div_euclid(86400);
    let secs = ts.rem_euclid(86400);
    let (y, m, d) = civil_from_days(days)?;
    Ok(format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}:{:02} UTC",
        secs / 3600,
        (secs % 3600) / 60,
        secs % 60
    ))
}

/// Howard Hinnant's civil_from_days.
fn civil_from_days(z: i64) -> Result<(i64, u32, u32), ()> {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    Ok((if m <= 2 { y + 1 } else { y }, m, d))
}
