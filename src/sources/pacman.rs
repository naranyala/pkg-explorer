use super::{Source, run, which};
use crate::models::{Package, PackageDetails, PkgKind, ScanResult};
use std::process::Command;

pub struct PacmanSource;

impl Source for PacmanSource {
    fn kind(&self) -> PkgKind {
        PkgKind::Pacman
    }

    fn name(&self) -> &'static str {
        "pacman"
    }

    fn is_available(&self) -> bool {
        which("pacman")
    }

    fn list_packages(&self) -> ScanResult {
        match run(Command::new("pacman").args(["-Q"])) {
            Ok(output) => ScanResult {
                packages: parse_package_list(&output),
                error: None,
            },
            Err(error) => ScanResult {
                packages: vec![],
                error: Some(error.to_string()),
            },
        }
    }

    fn details(&self, pkg: &Package) -> anyhow::Result<PackageDetails> {
        let info = run(Command::new("pacman").args(["-Qi", "--", &pkg.name]))?;
        let files = run(Command::new("pacman").args(["-Ql", "--", &pkg.name]))?;
        let mut details = parse_details(&info);
        let prefix = format!("{} ", pkg.name);
        details.files = files
            .lines()
            .filter_map(|line| line.strip_prefix(&prefix))
            .filter(|path| !path.is_empty())
            .map(Into::into)
            .collect();
        Ok(details)
    }
}

fn parse_package_list(output: &str) -> Vec<Package> {
    let mut packages: Vec<Package> = output
        .lines()
        .filter_map(|line| {
            let (name, version) = line.trim().split_once(char::is_whitespace)?;
            let name = name.trim();
            let version = version.trim();
            if name.is_empty() || version.is_empty() {
                return None;
            }
            Some(Package {
                kind: PkgKind::Pacman,
                name: name.to_string(),
                version: version.to_string(),
                arch: String::new(),
                size: None,
                summary: String::new(),
            })
        })
        .collect();
    packages.sort_by(|a, b| a.name.cmp(&b.name));
    packages
}

fn parse_details(output: &str) -> PackageDetails {
    let mut details = PackageDetails::default();
    for line in output.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        match key.trim() {
            "Version" => details.extra.push(("Version".into(), value.into())),
            "Description" => details.description = value.into(),
            "Architecture" => details.extra.push(("Architecture".into(), value.into())),
            "URL" if value != "None" => details.homepage = value.into(),
            "Licenses" if value != "None" => details.license = value.into(),
            "Groups" if value != "None" => details.section = value.into(),
            "Depends On" if value != "None" => {
                details.dependencies = value.split_whitespace().map(str::to_string).collect();
            }
            "Optional Deps" if value != "None" => {
                details.dependencies.extend(value.split_whitespace().map(str::to_string));
            }
            "Install Date" if value != "None" => details.install_date = value.into(),
            "Installed Size" if value != "None" => details
                .extra
                .push(("Installed Size".into(), value.into())),
            "Packager" if value != "None" => details.maintainer = value.into(),
            "Repository" if value != "None" => details.extra.push(("Repository".into(), value.into())),
            _ => {}
        }
    }
    details
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_installed_package_rows() {
        let packages = parse_package_list("zlib 1:1.3.1-2\nbase 3.0-1\nmalformed\n");
        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0].name, "base");
        assert_eq!(packages[1].version, "1:1.3.1-2");
        assert_eq!(packages[0].kind, PkgKind::Pacman);
    }

    #[test]
    fn parses_pacman_metadata_and_dependencies() {
        let details = parse_details(
            "Name            : demo\nVersion         : 1.2-3\nDescription     : Example app\nArchitecture    : x86_64\nURL             : https://example.org\nLicenses        : MIT\nGroups          : None\nDepends On      : glibc>=2.38  zlib\nOptional Deps   : docs: Documentation\nInstall Date    : Mon 01 Jan 2024\nInstalled Size  : 4.20 MiB\nPackager        : Example Builder\n",
        );
        assert_eq!(details.description, "Example app");
        assert_eq!(details.homepage, "https://example.org");
        assert_eq!(details.license, "MIT");
        assert_eq!(details.dependencies, ["glibc>=2.38", "zlib", "docs:", "Documentation"]);
        assert_eq!(details.install_date, "Mon 01 Jan 2024");
        assert_eq!(details.maintainer, "Example Builder");
    }

    #[test]
    fn parses_file_list_only_for_requested_package() {
        let output = "demo /usr/bin/demo\ndemo /usr/share/doc/demo/readme\nother /usr/bin/other\n";
        let prefix = "demo ";
        let files: Vec<String> = output
            .lines()
            .filter_map(|line| line.strip_prefix(prefix))
            .map(str::to_string)
            .collect();
        assert_eq!(files, ["/usr/bin/demo", "/usr/share/doc/demo/readme"]);
    }
}
