use crate::extract;
use crate::models::PkgKind;
use crate::sources;

pub fn run(args: &[String]) {
    match args.first().map(String::as_str) {
        Some("--list") => {
            let kind_filter = args
                .get(1)
                .map(|s| parse_kind(s).unwrap_or_else(|e| fail(&e)));
            let mut n = 0;
            for src in sources::available_sources() {
                if let Some(k) = kind_filter
                    && src.kind() != k
                {
                    continue;
                }
                let res = src.list_packages();
                if let Some(err) = res.error {
                    eprintln!("{}: {err}", src.name());
                }
                for p in &res.packages {
                    println!(
                        "{}\t{}\t{}\t{}\t{}",
                        p.kind.label(),
                        p.name,
                        p.version,
                        sources::format_size(p.size),
                        p.summary
                    );
                    n += 1;
                }
            }
            eprintln!("# {n} packages");
            std::process::exit(0);
        }
        Some("--extract") => {
            if args.len() != 4 {
                fail("usage: pkg-explorer --extract <deb|rpm|flatpak|brew> <name> <dest-dir>");
            }
            let kind = parse_kind(&args[1]).unwrap_or_else(|e| fail(&e));
            let name = &args[2];
            let dest = std::path::PathBuf::from(&args[3]);

            let src = sources::all_sources()
                .into_iter()
                .find(|s| s.kind() == kind)
                .unwrap_or_else(|| fail(&format!("no source for {kind:?}")));
            if !src.is_available() {
                fail(&format!("{} not available", src.name()));
            }
            let pkg = src
                .list_packages()
                .packages
                .into_iter()
                .find(|p| p.name == *name)
                .unwrap_or_else(|| fail(&format!("package {name} not found")));
            let details = src.details(&pkg).unwrap_or_else(|e| fail(&e.to_string()));
            match extract::extract(&pkg, &details, &dest, |done, total, path| {
                eprintln!("[{done}/{total}] {}", path.display());
            }) {
                Ok(msg) => {
                    println!("{msg}");
                    std::process::exit(0);
                }
                Err(e) => fail(&e.to_string()),
            }
        }
        Some("--help") | Some("-h") => {
            println!("pkg-explorer — GUI by default; CLI helpers:");
            println!("  pkg-explorer --list [deb|rpm|flatpak|brew]");
            println!("  pkg-explorer --extract <deb|rpm|flatpak|brew> <name> <dest-dir>");
            std::process::exit(0);
        }
        _ => fail("try: pkg-explorer --help"),
    }
}

fn parse_kind(s: &str) -> Result<PkgKind, String> {
    match s.to_ascii_lowercase().as_str() {
        "deb" => Ok(PkgKind::Deb),
        "rpm" => Ok(PkgKind::Rpm),
        "flatpak" => Ok(PkgKind::Flatpak),
        "brew" | "homebrew" => Ok(PkgKind::Brew),
        other => Err(format!("unknown kind '{other}'")),
    }
}

fn fail(msg: &str) -> ! {
    eprintln!("error: {msg}");
    std::process::exit(1);
}
