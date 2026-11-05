use crate::models::{Package, PkgKind};
use eframe::egui;
use std::collections::HashSet;

pub fn show(
    ui: &mut egui::Ui,
    root: &Package,
    dependencies: &[String],
    packages: &[Package],
) -> Option<Package> {
    let mut nodes = Vec::new();
    let mut seen = HashSet::new();
    for requirement in dependencies {
        let package = resolve(requirement, root.kind, packages);
        let key = package
            .as_ref()
            .map(|p| format!("{:?}:{}", p.kind, p.name))
            .unwrap_or_else(|| requirement.clone());
        if seen.insert(key) {
            nodes.push((requirement, package));
        }
    }

    let resolved = nodes.iter().filter(|(_, package)| package.is_some()).count();
    ui.horizontal_wrapped(|ui| {
        ui.label(format!("{} direct dependencies", dependencies.len()));
        ui.separator();
        ui.label(format!("{resolved} installed packages matched"));
    });
    ui.label("Select an installed node to explore its dependencies.");
    if nodes.is_empty() {
        ui.add_space(12.0);
        ui.weak("No dependency data is available for this package.");
        return None;
    }

    let width = ui.available_width().max(420.0);
    let height = (nodes.len() as f32 * 76.0 + 80.0).max(220.0);
    let mut selected = None;
    egui::ScrollArea::both()
        .id_salt("dependency_mindmap")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let (response, painter) = ui.allocate_painter(
                egui::vec2(width, height),
                egui::Sense::hover(),
            );
            let origin = response.rect.min;
            let root_rect = egui::Rect::from_min_size(
                origin + egui::vec2(16.0, height * 0.5 - 27.0),
                egui::vec2(190.0, 54.0),
            );
            let root_center = root_rect.center();
            let dep_x = 270.0;
            let dep_h = 56.0;
            let gap = 20.0;
            let list_height = nodes.len() as f32 * (dep_h + gap) - gap;
            let first_y = (height - list_height) * 0.5;
            let root_fill = ui.visuals().widgets.active.bg_fill;
            let node_fill = ui.visuals().widgets.inactive.bg_fill;
            let stroke = ui.visuals().widgets.noninteractive.fg_stroke;
            let text_color = ui.visuals().text_color();
            let muted = ui.visuals().weak_text_color();

            for (i, (_, package)) in nodes.iter().enumerate() {
                let rect = egui::Rect::from_min_size(
                    origin + egui::vec2(dep_x, first_y + i as f32 * (dep_h + gap)),
                    egui::vec2((width - dep_x - 20.0).max(200.0), dep_h),
                );
                painter.line_segment(
                    [egui::pos2(root_rect.right(), root_center.y), egui::pos2(rect.left(), rect.center().y)],
                    egui::Stroke::new(1.5, stroke.color),
                );
                let response = ui.interact(
                    rect,
                    ui.id().with(("dependency-node", i)),
                    if package.is_some() { egui::Sense::click() } else { egui::Sense::hover() },
                );
                painter.rect(
                    rect,
                    7.0,
                    if package.is_some() { node_fill } else { ui.visuals().faint_bg_color },
                    egui::Stroke::new(if response.hovered() { 2.0 } else { 1.0 }, stroke.color),
                    egui::StrokeKind::Inside,
                );
                let (title, subtitle) = match package {
                    Some(pkg) => (
                        format!("{}  ·  {}", pkg.kind.label(), pkg.name),
                        format!("{}  ·  {}", pkg.version, pkg.summary),
                    ),
                    None => ("External / unresolved requirement".to_string(), nodes[i].0.clone()),
                };
                painter.text(
                    rect.left_top() + egui::vec2(10.0, 17.0),
                    egui::Align2::LEFT_CENTER,
                    title,
                    egui::FontId::proportional(14.0),
                    text_color,
                );
                painter.text(
                    rect.left_top() + egui::vec2(10.0, 39.0),
                    egui::Align2::LEFT_CENTER,
                    subtitle,
                    egui::FontId::proportional(11.0),
                    muted,
                );
                if response.clicked() {
                    selected = package.clone();
                }
            }

            painter.rect(
                root_rect,
                7.0,
                root_fill,
                egui::Stroke::new(1.0, stroke.color),
                egui::StrokeKind::Inside,
            );
            painter.text(
                root_rect.center(),
                egui::Align2::CENTER_CENTER,
                format!("{}  ·  {}", root.kind.label(), root.name),
                egui::FontId::proportional(14.0),
                text_color,
            );
        });
    selected
}

fn resolve(requirement: &str, preferred_kind: PkgKind, packages: &[Package]) -> Option<Package> {
    for alternative in requirement.split('|') {
        let token = alternative
            .split_whitespace()
            .next()
            .unwrap_or("")
            .trim_matches(|c: char| c == ',' || c == '(' || c == ')');
        if token.is_empty() {
            continue;
        }
        let name = token.split_once(':').map_or(token, |(name, _)| name);
        if let Some(package) = packages
            .iter()
            .find(|package| package.kind == preferred_kind && package.name == name)
            .or_else(|| packages.iter().find(|package| package.name == name))
        {
            return Some(package.clone());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn package(kind: PkgKind, name: &str) -> Package {
        Package {
            kind,
            name: name.into(),
            version: "1.0".into(),
            arch: "x86_64".into(),
            size: None,
            summary: String::new(),
        }
    }

    #[test]
    fn resolves_debian_version_and_arch_suffixes() {
        let packages = vec![package(PkgKind::Deb, "libc6")];
        let result = resolve("libc6:amd64 (>= 2.35)", PkgKind::Deb, &packages);
        assert_eq!(result.map(|p| p.name), Some("libc6".into()));
    }

    #[test]
    fn resolves_installed_alternative() {
        let packages = vec![package(PkgKind::Deb, "bar")];
        let result = resolve("missing | bar (>= 1)", PkgKind::Deb, &packages);
        assert_eq!(result.map(|p| p.name), Some("bar".into()));
    }

    #[test]
    fn prefers_same_manager_when_names_collide() {
        let packages = vec![package(PkgKind::Rpm, "shared"), package(PkgKind::Deb, "shared")];
        let result = resolve("shared", PkgKind::Deb, &packages);
        assert_eq!(result.map(|p| p.kind), Some(PkgKind::Deb));
    }
}
