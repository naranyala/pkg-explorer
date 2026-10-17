pub mod brew;
pub mod deb;
pub mod flatpak;
pub mod rpm;

use crate::models::{Package, PackageDetails, PkgKind, ScanResult};
use std::process::{Command, Output};

pub trait Source: Send {
    fn kind(&self) -> PkgKind;
    fn name(&self) -> &'static str;
    /// True if the manager's CLI is usable on this system.
    fn is_available(&self) -> bool;
    fn list_packages(&self) -> ScanResult;
    fn details(&self, pkg: &Package) -> anyhow::Result<PackageDetails>;
}

pub fn all_sources() -> Vec<Box<dyn Source>> {
    vec![
        Box::new(rpm::RpmSource),
        Box::new(deb::DebSource),
        Box::new(flatpak::FlatpakSource),
        Box::new(brew::BrewSource),
    ]
}

pub fn available_sources() -> Vec<Box<dyn Source>> {
    all_sources()
        .into_iter()
        .filter(|s| s.is_available())
        .collect()
}

/// Run a command, returning stdout (lossy). Err on non-zero exit.
pub fn run(cmd: &mut Command) -> anyhow::Result<String> {
    let out: Output = cmd
        .output()
        .map_err(|e| anyhow::anyhow!("spawn {:?}: {e}", cmd))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        anyhow::bail!("{} failed: {}", program_name(cmd), err.trim());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

pub fn which(program: &str) -> bool {
    Command::new("which")
        .arg(program)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn program_name(cmd: &Command) -> String {
    cmd.get_program().to_string_lossy().into_owned()
}

/// Parse a flatpak-style human size ("1.2 MB", "350 kB", "12.5 KiB") into bytes.
pub fn parse_human_size(s: &str) -> Option<u64> {
    let s = s.trim();
    let s = s.strip_suffix('B').unwrap_or(s).trim();
    let mut parts = s.split_whitespace();
    let num: f64 = parts.next()?.parse().ok()?;
    let unit = parts.next().unwrap_or("B").to_ascii_lowercase();
    let mult = match unit.as_str() {
        "b" => 1.0,
        "k" | "kb" | "kib" => 1024.0,
        "m" | "mb" | "mib" => 1024.0 * 1024.0,
        "g" | "gb" | "gib" => 1024.0 * 1024.0 * 1024.0,
        _ => return None,
    };
    Some((num * mult) as u64)
}

pub fn format_size(bytes: Option<u64>) -> String {
    let Some(b) = bytes else { return "—".into() };
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut v = b as f64;
    let mut i = 0;
    while v >= 1024.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{b} {}", UNITS[i])
    } else {
        format!("{v:.1} {}", UNITS[i])
    }
}
