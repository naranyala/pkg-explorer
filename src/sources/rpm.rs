use super::{Source, run, which};
use crate::models::{Package, PackageDetails, PkgKind, ScanResult};
use std::process::Command;

pub struct RpmSource;

impl Source for RpmSource {
    fn kind(&self) -> PkgKind {
        PkgKind::Rpm
    }
    fn name(&self) -> &'static str {
        "rpm (dnf/yum)"
    }
    fn is_available(&self) -> bool {
        which("rpm")
    }

    fn list_packages(&self) -> ScanResult {
        let fmt = "%{NAME}\t%{EPOCHNUM}:%{VERSION}-%{RELEASE}\t%{ARCH}\t%{SIZE}\t%{SUMMARY}\n";
        match run(Command::new("rpm").args(["-qa", "--qf", fmt])) {
            Ok(out) => {
                let mut packages: Vec<Package> = out
                    .lines()
                    .filter(|l| !l.trim().is_empty())
                    .filter_map(|line| {
                        let mut it = line.split('\t');
                        let name = it.next()?.to_string();
                        let version = it.next()?.to_string();
                        let version = version.strip_prefix("0:").unwrap_or(&version).to_string();
                        let arch = it.next().unwrap_or("").to_string();
                        let size = it.next().and_then(|s| s.parse().ok());
                        let summary = it.next().unwrap_or("").to_string();
                        Some(Package {
                            kind: PkgKind::Rpm,
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
        let info = run(Command::new("rpm").args(["-qi", &pkg.name])).unwrap_or_default();
        let deps = run(Command::new("rpm").args(["-qR", &pkg.name])).unwrap_or_default();
        let files = run(Command::new("rpm").args(["-ql", &pkg.name])).unwrap_or_default();

        let mut d = PackageDetails {
            dependencies: deps
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(String::from)
                .collect(),
            files: files
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(Into::into)
                .collect(),
            ..Default::default()
        };
        for line in info.lines() {
            if line.starts_with(' ') && !d.description.is_empty() {
                d.description.push('\n');
                d.description.push_str(line.trim_start());
                continue;
            }
            let Some((k, v)) = line.split_once(':') else {
                continue;
            };
            let v = v.trim();
            match k.trim() {
                "Name" => {}
                "Version" | "Release" | "Architecture" | "Group" | "License" | "Vendor"
                | "Packager" | "Build Host" | "Relocations" => {
                    d.extra.push((k.trim().to_string(), v.to_string()));
                }
                "Size" => d
                    .extra
                    .push(("Installed Size".into(), super::format_size(v.parse().ok()))),
                "URL" => d.homepage = v.to_string(),
                "Summary" => d.section = v.to_string(),
                "Description" => {
                    if !d.description.is_empty() {
                        d.description.push('\n');
                    }
                    d.description.push_str(v);
                }
                "Maintainer" | "Author" => d.maintainer = v.to_string(),
                "Install Date" => d.install_date = v.to_string(),
                _ => {}
            }
        }
        Ok(d)
    }
}
