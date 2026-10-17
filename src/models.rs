use std::path::PathBuf;

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, serde::Serialize, serde::Deserialize,
)]
pub enum PkgKind {
    Deb,
    Rpm,
    Flatpak,
    Brew,
}

impl PkgKind {
    pub fn label(self) -> &'static str {
        match self {
            PkgKind::Deb => "deb",
            PkgKind::Rpm => "rpm",
            PkgKind::Flatpak => "flatpak",
            PkgKind::Brew => "brew",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Package {
    pub kind: PkgKind,
    pub name: String,
    pub version: String,
    pub arch: String,
    /// Installed size in bytes, if known.
    pub size: Option<u64>,
    pub summary: String,
}

#[derive(Clone, Debug, Default)]
pub struct PackageDetails {
    pub description: String,
    pub homepage: String,
    pub maintainer: String,
    pub license: String,
    pub install_date: String,
    pub section: String,
    pub dependencies: Vec<String>,
    pub files: Vec<PathBuf>,
    /// Extra key/value rows shown in the info table.
    pub extra: Vec<(String, String)>,
}

/// Result of scanning one package manager.
pub struct ScanResult {
    pub packages: Vec<Package>,
    pub error: Option<String>,
}
