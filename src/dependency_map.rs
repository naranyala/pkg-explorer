use crate::models::Package;
use egui::{Align2, FontId, Rect, Sense, Stroke, StrokeKind, pos2, vec2};

pub fn render(
    ui: &mut egui::Ui,
    root: &Package,
    dependencies: &[String],
    packages: &[Package],
) -> Option<usize> {
    let mut nodes = Vec::new();
    for dep in dependencies {
        let dep_name = dep.clone();
        nodes.push((dep_name, resolve_first_match(dep, root.kind, packages)));
    }

    if nodes.is_empty() {
        return None;
    }

    ui.horizontal_wrapped(|ui| {
        ui.label(format!("{} direct dependencies", dependencies.len()));
        ui.separator();
        ui.label("installed package links");
    });

    let width = ui.available_width().max(420.0);
    let height = ((nodes.len().max(1) as f32) * 68.0 + 120.0).max(240.0);
    let (response, painter) = ui.allocate_painter(vec2(width, height), Sense::hover());
    let origin = response.rect.min;
    let root_rect = Rect::from_min_size(origin + vec2(16.0, height * 0.5 - 34.0), vec2(180.0, 68.0));
    let root_center = root_rect.center();
    let dep_x = 260.0;
    let dep_w = (width - dep_x - 18.0).max(220.0);
    let dep_h = 52.0;
    let gap = 18.0;
    let start_y = if nodes.len() == 1 {
        height * 0.5 - dep_h * 0.5
    } else {
        height * 0.5 - ((nodes.len() as f32 * (dep_h + gap) - gap) * 0.5)
    };

    let root_fill = ui.visuals().widgets.active.bg_fill;
    let node_fill = ui.visuals().widgets.inactive.bg_fill;
    let stroke = ui.visuals().widgets.noninteractive.fg_stroke;
    let text_color = ui.visuals().text_color();
    let muted = ui.visuals().weak_text_color();

    painter.rect(
        root_rect,
        8.0,
        root_fill,
        egui::Stroke::new(1.0, stroke.color),
        StrokeKind::Inside,
    );
    painter.text(
        root_rect.center(),
        Align2::CENTER_CENTER,
        format!("{}  ·  {}", root.kind.label(), root.name),
        FontId::proportional(14.0),
        text_color,
    );

    let mut selected = None;
    for (i, (dep, resolved_idx)) in nodes.iter().enumerate() {
        let rect = Rect::from_min_size(
            origin + vec2(dep_x, start_y + i as f32 * (dep_h + gap)),
            vec2(dep_w, dep_h),
        );
        painter.line_segment(
            [pos2(root_rect.right(), root_center.y), pos2(rect.left(), rect.center().y)],
            Stroke::new(1.5, stroke.color),
        );
        let response = ui.interact(rect, ui.id().with(("dep-node", i)), Sense::click());
        let fill = match resolved_idx {
            Some(_) => node_fill,
            None => ui.visuals().faint_bg_color,
        };
        painter.rect(
            rect,
            7.0,
            fill,
            egui::Stroke::new(if response.hovered() { 2.0 } else { 1.0 }, stroke.color),
            StrokeKind::Inside,
        );

        match resolved_idx {
            Some(index) => {
                let pkg = &packages[*index];
                let label = format!("{}  ·  {}", pkg.kind.label(), pkg.name);
                let subtitle = if pkg.summary.is_empty() {
                    pkg.version.clone()
                } else {
                    format!("{}  ·  {}", pkg.version, pkg.summary)
                };
                painter.text(
                    rect.left_top() + vec2(10.0, 16.0),
                    Align2::LEFT_CENTER,
                    label,
                    FontId::proportional(13.5),
                    text_color,
                );
                painter.text(
                    rect.left_top() + vec2(10.0, 38.0),
                    Align2::LEFT_CENTER,
                    subtitle,
                    FontId::proportional(10.5),
                    muted,
                );
                if response.clicked() {
                    selected = Some(*index);
                }
            }
            None => {
                painter.text(
                    rect.left_top() + vec2(10.0, 17.0),
                    Align2::LEFT_CENTER,
                    "External / unresolved",
                    FontId::proportional(12.5),
                    text_color,
                );
                painter.text(
                    rect.left_top() + vec2(10.0, 38.0),
                    Align2::LEFT_CENTER,
                    dep,
                    FontId::proportional(10.5),
                    muted,
                );
            }
        }
    }

    selected
}

fn resolve_first_match(dependency: &str, preferred_kind: crate::models::PkgKind, packages: &[Package]) -> Option<usize> {
    for candidate in dependency.split('|') {
        let name = normalize_name(candidate);
        if name.is_empty() {
            continue;
        }
        if let Some(index) = packages
            .iter()
            .position(|package| package.kind == preferred_kind && package.name == name)
        {
            return Some(index);
        }
        if let Some(index) = packages.iter().position(|package| package.name == name) {
            return Some(index);
        }
    }
    None
}

fn normalize_name(raw: &str) -> String {
    let trimmed = raw.trim();
    let trimmed = trimmed.trim_matches(|ch| matches!(ch, ')' | ']' | '(' | '[' | ',' | '"' | '\''));
    let candidate = trimmed
        .split_once(|ch: char| matches!(ch, ' ' | '\t' | '(' | '[' | '<' | '>' | '=' | ',' | ':' ))
        .map_or(trimmed, |(head, _)| head)
        .trim();
    candidate.trim_matches(|ch| matches!(ch, ')' | ']' | '(' | '[' | ',' | '"' | '\'' )).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::PkgKind;

    fn package(kind: PkgKind, name: &str) -> Package {
        Package {
            kind,
            name: name.to_string(),
            version: "1.0".to_string(),
            arch: "x86_64".to_string(),
            size: None,
            summary: String::new(),
        }
    }

    #[test]
    fn resolves_dependency_names_with_version_suffixes() {
        let packages = vec![package(PkgKind::Deb, "libc6")];
        let result = resolve_first_match("libc6:amd64 (>= 2.35)", PkgKind::Deb, &packages);
        assert_eq!(result, Some(0));
    }

    #[test]
    fn resolves_alternative_dependencies() {
        let packages = vec![package(PkgKind::Deb, "pkg-b")];
        let result = resolve_first_match("missing | pkg-b (>= 1)", PkgKind::Deb, &packages);
        assert_eq!(result, Some(0));
    }

    #[test]
    fn prefers_same_kind_when_names_overlap() {
        let packages = vec![package(PkgKind::Rpm, "shared"), package(PkgKind::Deb, "shared")];
        let result = resolve_first_match("shared", PkgKind::Deb, &packages);
        assert_eq!(result, Some(1));
    }
}

