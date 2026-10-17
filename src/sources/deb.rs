use super::{Source, run, which};
use crate::models::{Package, PackageDetails, PkgKind, ScanResult};
use std::process::Command;

pub struct DebSource;

impl Source for DebSource {
    fn kind(&self) -> PkgKind {
        PkgKind::Deb
    }
    fn name(&self) -> &'static str {
        "deb (dpkg/apt)"
    }
    fn is_available(&self) -> bool {
        which("dpkg-query")
    }

    fn list_packages(&self) -> ScanResult {
        let fmt = "${Package}\t${Version}\t${Architecture}\t${Installed-Size}\t${Summary}\n";
        match run(Command::new("dpkg-query").args(["-W", "-f", fmt])) {
            Ok(out) => {
                let mut packages: Vec<Package> = out
                    .lines()
                    .filter(|l| !l.trim().is_empty())
                    .filter_map(|line| {
                        let mut it = line.split('\t');
                        let name = it.next()?.to_string();
                        let version = it.next()?.to_string();
                        let arch = it.next().unwrap_or("").to_string();
                        // dpkg Installed-Size is in KiB
                        let size = it
                            .next()
                            .and_then(|s| s.parse::<u64>().ok())
                            .map(|kib| kib * 1024);
                        let summary = it.next().unwrap_or("").to_string();
                        Some(Package {
                            kind: PkgKind::Deb,
                            name,
                            version,
                            arch,
                            size,
                            summary,
                        })
                    })
                    .collect();
                packages.sort_by(|a, b| a.name.cmp(&b.name));
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
        let fmt = concat!(
            "Package: ${Package}\n",
            "Status: ${Status}\n",
            "Version: ${Version}\n",
            "Architecture: ${Architecture}\n",
            "Installed-Size: ${Installed-Size}\n",
            "Maintainer: ${Maintainer}\n",
            "Homepage: ${Homepage}\n",
            "Section: ${Section}\n",
            "Depends: ${Depends}\n",
            "Description: ${Description}\n",
            "===END===\n"
        );
        let out = run(Command::new("dpkg-query").args(["-W", "-f", fmt, &pkg.name]))?;

        let mut d = PackageDetails::default();
        let mut in_desc = false;
        for line in out.lines() {
            if line == "===END===" {
                break;
            }
            if in_desc {
                if line.starts_with(' ') || line.starts_with('\t') {
                    if !d.description.is_empty() {
                        d.description.push('\n');
                    }
                    d.description.push_str(line.trim_start());
                    continue;
                }
                in_desc = false;
            }
            if let Some((k, v)) = line.split_once(": ") {
                match k {
                    "Maintainer" => d.maintainer = v.to_string(),
                    "Homepage" => d.homepage = v.to_string(),
                    "Section" => d.section = v.to_string(),
                    "Description" => {
                        d.description = v.to_string();
                        in_desc = true;
                    }
                    "Depends" => {
                        d.dependencies = v.split(',').map(|s| s.trim().to_string()).collect()
                    }
                    "Installed-Size" => {
                        d.extra.push((
                            "Installed Size".into(),
                            super::format_size(v.parse::<u64>().ok().map(|k| k * 1024)),
                        ));
                    }
                    "Status" | "Package" | "Version" | "Architecture" => {
                        d.extra.push((k.to_string(), v.to_string()));
                    }
                    _ => {}
                }
            }
        }

        if let Ok(list) = run(Command::new("dpkg").args(["-L", &pkg.name])) {
            d.files = list
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(Into::into)
                .collect();
        }
        Ok(d)
    }
}
