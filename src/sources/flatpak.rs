use super::{Source, parse_human_size, run, which};
use crate::models::{Package, PackageDetails, PkgKind, ScanResult};
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct FlatpakSource;

impl Source for FlatpakSource {
    fn kind(&self) -> PkgKind {
        PkgKind::Flatpak
    }
    fn name(&self) -> &'static str {
        "flatpak"
    }
    fn is_available(&self) -> bool {
        which("flatpak")
    }

    fn list_packages(&self) -> ScanResult {
        let cols = "application,version,branch,arch,installation,size,description";
        match run(Command::new("flatpak").args(["list", "--columns", cols])) {
            Ok(out) => {
                let packages: Vec<Package> = out
                    .lines()
                    .filter(|l| !l.trim().is_empty())
                    .filter_map(|line| {
                        let mut it = line.split('\t');
                        let name = it.next()?.to_string();
                        let version = it.next().unwrap_or("").to_string();
                        let branch = it.next().unwrap_or("").to_string();
                        let arch = it.next().unwrap_or("").to_string();
                        let install = it.next().unwrap_or("").to_string();
                        let size = it.next().and_then(parse_human_size);
                        let summary = it.next().unwrap_or("").to_string();
                        let summary = if summary.is_empty() {
                            format!("{branch} · {install}")
                        } else {
                            format!("{summary} · {branch}")
                        };
                        Some(Package {
                            kind: PkgKind::Flatpak,
                            name,
                            version,
                            arch,
                            size,
                            summary,
                        })
                    })
                    .collect();
                ScanResult {
                    packages,
                    error: None,
                }
            }
            Err(e) => ScanResult {
                packages: vec![],
                error: Some(e.to_string()),
            },
        }
    }

    fn details(&self, pkg: &Package) -> anyhow::Result<PackageDetails> {
        let info = run(Command::new("flatpak").args(["info", &pkg.name]))?;
        let mut d = PackageDetails::default();
        for line in info.lines() {
            if let Some((k, v)) = line.split_once(": ") {
                match k {
                    "License" => d.license = v.to_string(),
                    "Homepage" | "Home page" => d.homepage = v.to_string(),
                    "Description" => d.description = v.to_string(),
                    "Installed size" => d.extra.push((
                        "Installed size".into(),
                        super::format_size(parse_human_size(v)),
                    )),
                    _ => d.extra.push((k.to_string(), v.to_string())),
                }
            }
        }
        if let Ok(loc) = run(Command::new("flatpak").args(["info", "--show-location", &pkg.name])) {
            let root = PathBuf::from(loc.trim());
            d.extra
                .push(("Location".into(), root.display().to_string()));
            let files_dir = root.join("files");
            if files_dir.is_dir() {
                list_dir(&files_dir, &mut d.files, 0);
                d.files.sort();
            }
        }
        Ok(d)
    }
}

fn list_dir(dir: &Path, out: &mut Vec<PathBuf>, depth: usize) {
    if depth > 12 || out.len() > 50_000 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        out.push(path.clone());
        if path.is_dir() {
            list_dir(&path, out, depth + 1);
        }
    }
}
