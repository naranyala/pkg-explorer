# Package-manager backends

Each backend implements `Source` in `src/sources/` and shells out to a manager's CLI. List operations should stay cheap; expensive package details are loaded only after selection. Parser tests use captured output and do not require the manager to be installed.

## Detection

| Source | Probe | Common systems |
|---|---|---|
| rpm | `rpm` | Fedora, RHEL, AlmaLinux, openSUSE, Mageia |
| deb | `dpkg-query` | Debian, Ubuntu and derivatives |
| pacman | `pacman` | Arch Linux and derivatives |
| flatpak | `flatpak` | Cross-distro app/runtime installations |
| brew | `brew` | Linuxbrew |

A detected manager does not by itself guarantee complete distribution support. For example, the RPM backend covers installed RPM inventory on openSUSE but does not implement zypper-specific update or repository workflows.

## RPM

List: `rpm -qa --qf '%{NAME}\t%{EPOCHNUM}:%{VERSION}-%{RELEASE}\t%{ARCH}\t%{SIZE}\t%{SUMMARY}\n'`.
Details query `rpm -qi`, `rpm -qR`, and `rpm -ql`. Epoch `0:` is removed from displayed versions. Failed detail commands are surfaced as errors.

## Debian packages

List: `dpkg-query -W -f '${Package}\t${Version}\t${Architecture}\t${Installed-Size}\t${Summary}\n'`.
`Installed-Size` is KiB. Details use `dpkg-query` and `dpkg -L`; file-list errors are surfaced.

## Arch packages (`src/sources/pacman.rs`)

List: `pacman -Q` emits one installed package and version per line. Size is currently unavailable in the list and left unknown. Details use `pacman -Qi -- <name>` for metadata and dependency fields and `pacman -Ql -- <name>` for paths. Optional dependencies are included as raw dependency entries, with their descriptions retained as manager output provides them. The backend is read-only and uses argument arrays.

The fixture tests cover installed-list parsing, details fields, dependency extraction, and package-scoped file-list parsing. Arch has not yet been smoke-tested on a live system; do not treat that as completed validation.

## Flatpak

List: `flatpak list --columns application,version,branch,arch,installation,size,description`. Details use `flatpak info` and walk the deployment's `files/` directory. Permission/runtime metadata parsing remains planned.

## Homebrew (Linuxbrew)

Primary list/details use `brew info --json=v2`; listing falls back to `brew list --formula --versions` and `brew list --cask --versions` if the JSON command fails. Files are walked from Cellar/Caskroom.

## Cross-source and adding a backend

The shared `Package` / `PackageDetails` types keep manager-specific values in `extra`. A new backend should add a `PkgKind`, parser fixtures, source detection/registration, CLI kind support, a UI source toggle, and documented command behavior. Extraction support must be assessed separately; listing and inspecting does not imply that archive extraction is implemented.

Still planned: zypper-specific behavior, apk, xbps, Nix profiles, Gentoo Portage, eopkg, and Snap. Nix profiles need a separate model for profile links and store paths rather than assuming ordinary filesystem package ownership.
