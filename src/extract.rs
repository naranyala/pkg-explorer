use crate::models::{Package, PackageDetails};
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn find_archive(pkg: &Package) -> Option<PathBuf> {
    match pkg.kind {
        crate::models::PkgKind::Deb => {
            let file = format!("{}_{}_{}.deb", pkg.name, pkg.version, pkg.arch);
            for dir in ["/var/cache/apt/archives", "/var/cache/apt"] {
                let p = Path::new(dir).join(&file);
                if p.is_file() {
                    return Some(p);
                }
            }
            None
        }
        crate::models::PkgKind::Rpm => {
            // Try cache dirs for an rpm whose name starts with name-version.
            let prefix = format!("{}-{}-{}", pkg.name, pkg.version, pkg.arch);
            let prefix2 = format!("{}-{}", pkg.name, pkg.version);
            for dir in [
                "/var/cache/dnf",
                "/var/cache/yum",
                "/var/cache/PackageKit",
                "/var/cache/apt/archives",
            ] {
                if let Some(p) = find_file_with_prefix(Path::new(dir), &prefix, "rpm") {
                    return Some(p);
                }
                if let Some(p) = find_file_with_prefix(Path::new(dir), &prefix2, "rpm") {
                    return Some(p);
                }
            }
            None
        }
        _ => None,
    }
}

fn find_file_with_prefix(dir: &Path, prefix: &str, ext: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    for e in entries.flatten() {
        let path = e.path();
        if path.is_dir() {
            if let Some(found) = find_file_with_prefix(&path, prefix, ext) {
                return Some(found);
            }
        } else if path.extension().and_then(|s| s.to_str()) == Some(ext) {
            let fname = path.file_name()?.to_string_lossy();
            if fname.starts_with(prefix) {
                return Some(path);
            }
        }
    }
    None
}

/// Extract `pkg` into `dest`. Returns a human-readable summary.
/// `progress` is called with (files_done, total_files_or_0, current_path).
pub fn extract(
    pkg: &Package,
    details: &PackageDetails,
    dest: &Path,
    mut progress: impl FnMut(usize, usize, &Path),
) -> anyhow::Result<String> {
    std::fs::create_dir_all(dest)?;

    if let Some(archive) = find_archive(pkg) {
        return match pkg.kind {
            crate::models::PkgKind::Deb => {
                let st = Command::new("dpkg-deb")
                    .args(["-x", &archive.to_string_lossy(), &dest.to_string_lossy()])
                    .status()
                    .map_err(|e| anyhow::anyhow!("dpkg-deb: {e}"))?;
                if !st.success() {
                    anyhow::bail!("dpkg-deb exited with {st}");
                }
                Ok(format!("Unpacked {} via dpkg-deb", archive.display()))
            }
            crate::models::PkgKind::Rpm => {
                let st = Command::new("sh")
                    .arg("-c")
                    .arg(format!(
                        "rpm2cpio '{}' | cpio -idmu --quiet -D '{}'",
                        shell_escape(&archive),
                        shell_escape(dest)
                    ))
                    .status()
                    .map_err(|e| anyhow::anyhow!("rpm2cpio/cpio: {e}"))?;
                if !st.success() {
                    anyhow::bail!("rpm2cpio/cpio exited with {st}");
                }
                Ok(format!("Unpacked {} via rpm2cpio+cpio", archive.display()))
            }
            _ => unreachable!(),
        };
    }

    // Fallback: copy the installed files from the live filesystem.
    copy_installed_files(pkg, details, dest, &mut progress)
}

fn copy_installed_files(
    pkg: &Package,
    details: &PackageDetails,
    dest: &Path,
    progress: &mut dyn FnMut(usize, usize, &Path),
) -> anyhow::Result<String> {
    let roots: Vec<PathBuf> = match pkg.kind {
        crate::models::PkgKind::Brew => {
            // Files in details are relative to Cellar/<name>/<ver>; prepend prefix.
            let prefix = brew_prefix();
            details.files.iter().map(|f| prefix.join(f)).collect()
        }
        _ => details.files.clone(),
    };

    let existing: Vec<&PathBuf> = roots.iter().filter(|p| p.exists()).collect();
    if existing.is_empty() {
        anyhow::bail!(
            "No cached archive and no installed files found for {} — cannot extract",
            pkg.name
        );
    }
    let total = existing.len();
    let mut copied = 0usize;
    let mut processed = 0usize;
    let mut failed: Vec<String> = Vec::new();
    for src in existing {
        let rel = match pkg.kind {
            crate::models::PkgKind::Brew => {
                src.strip_prefix(brew_prefix()).unwrap_or(src).to_path_buf()
            }
            _ => src.clone(),
        };
        let target = dest.join(rel.strip_prefix("/").unwrap_or(&rel));
        match copy_path(src, &target) {
            Ok(()) => copied += 1,
            Err(e) => failed.push(format!("{}: {e}", src.display())),
        }
        processed += 1;
        progress(processed, total, src);
    }
    if copied == 0 {
        anyhow::bail!(
            "copy failed: {}",
            failed.first().unwrap_or(&"unknown".into())
        );
    }
    let mut msg = format!(
        "Copied {copied}/{total} installed file(s)/dir(s) to {}",
        dest.display()
    );
    if !failed.is_empty() {
        msg.push_str(&format!(" ({} failed, first: {})", failed.len(), failed[0]));
    }
    Ok(msg)
}

fn copy_path(src: &Path, dst: &Path) -> std::io::Result<()> {
    if src.is_dir() {
        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let to = dst.join(entry.file_name());
            copy_path(&entry.path(), &to)?;
        }
        Ok(())
    } else {
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)?;
        }
        // Source files may be read-only (e.g. r-xr-xr-x); a previously copied
        // destination with those perms can't be opened for writing.
        if dst.exists() {
            let _ = std::fs::remove_file(dst);
        }
        std::fs::copy(src, dst)?;
        Ok(())
    }
}

fn brew_prefix() -> PathBuf {
    if let Ok(out) = Command::new("brew").arg("--prefix").output()
        && out.status.success()
    {
        return PathBuf::from(String::from_utf8_lossy(&out.stdout).trim());
    }
    PathBuf::from("/home/linuxbrew/.linuxbrew")
}

fn shell_escape(p: &Path) -> String {
    let s = p.to_string_lossy();
    if s.chars()
        .all(|c| c.is_alphanumeric() || "/._-+=:".contains(c))
    {
        s.into_owned()
    } else {
        format!("'{}'", s.replace('\'', r"'\''"))
    }
}
