use crate::extract;
use crate::models::{Package, PackageDetails, PkgKind, ScanResult};
use crate::sources::{self, Source};
use eframe::egui;
use std::sync::mpsc::{Receiver, Sender, channel};

enum Msg {
    SourceDone {
        source: String,
        result: ScanResult,
    },
    Details {
        key: String,
        result: anyhow::Result<PackageDetails>,
    },
    ExtractDone {
        result: anyhow::Result<String>,
    },
}

pub struct PkgApp {
    packages: Vec<Package>,
    scan_errors: Vec<(String, String)>,
    scanning: bool,
    pending_sources: usize,
    status: String,

    search: String,
    kind_enabled: [bool; 4],

    selected: Option<usize>,
    details: Option<(String, PackageDetails)>,
    details_loading: bool,

    extracting: bool,
    extract_status: Option<String>,

    tx: Sender<Msg>,
    rx: Receiver<Msg>,
}

impl PkgApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self::with_ctx(&cc.egui_ctx)
    }

    /// Build the app against an arbitrary [`egui::Context`] (used by tests).
    pub fn with_ctx(ctx: &egui::Context) -> Self {
        let (tx, rx) = channel();
        let mut app = Self {
            packages: vec![],
            scan_errors: vec![],
            scanning: false,
            pending_sources: 0,
            status: String::new(),
            search: String::new(),
            kind_enabled: [true; 4],
            selected: None,
            details: None,
            details_loading: false,
            extracting: false,
            extract_status: None,
            tx,
            rx,
        };
        app.start_scan(ctx);
        app
    }

    fn start_scan(&mut self, ctx: &egui::Context) {
        self.scanning = true;
        self.status = "Scanning installed packages…".into();
        self.scan_errors.clear();
        self.packages.clear();
        self.selected = None;
        self.details = None;

        let tx = self.tx.clone();
        let ctx = ctx.clone();
        let sources = sources::available_sources();
        self.pending_sources = sources.len();
        std::thread::spawn(move || {
            for src in sources {
                let source = src.name().to_string();
                let result = src.list_packages();
                let _ = tx.send(Msg::SourceDone { source, result });
                ctx.request_repaint();
            }
        });
    }

    fn start_details(&mut self, pkg: &Package, ctx: &egui::Context) {
        let key = package_key(pkg);
        if self
            .details
            .as_ref()
            .map(|(k, _)| *k == key)
            .unwrap_or(false)
        {
            return;
        }
        self.details = None;
        self.details_loading = true;

        let mut source: Option<Box<dyn Source>> = None;
        for s in sources::all_sources() {
            if s.kind() == pkg.kind {
                source = Some(s);
                break;
            }
        }
        let Some(src) = source else {
            self.details_loading = false;
            return;
        };
        let pkg = pkg.clone();
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = src.details(&pkg);
            let _ = tx.send(Msg::Details { key, result });
            ctx.request_repaint();
        });
    }

    fn start_extract(&mut self, ctx: &egui::Context) {
        let Some(idx) = self.selected else { return };
        let Some((_, details)) = &self.details else {
            return;
        };
        if self.extracting {
            return;
        }
        let Some(dest) = rfd::FileDialog::new()
            .set_title("Choose extraction destination")
            .pick_folder()
        else {
            return;
        };

        let pkg = self.packages[idx].clone();
        let details = details.clone();
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        self.extracting = true;
        self.extract_status = Some(format!("Extracting {}…", pkg.name));
        std::thread::spawn(move || {
            let result = extract::extract(&pkg, &details, &dest, |_done, _total, _path| {});
            let _ = tx.send(Msg::ExtractDone { result });
            ctx.request_repaint();
        });
    }

    fn poll_messages(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                Msg::SourceDone { source, result } => {
                    if let Some(err) = result.error {
                        self.scan_errors.push((source, err));
                    }
                    self.packages.extend(result.packages);
                    self.packages
                        .sort_by(|a, b| a.kind.cmp(&b.kind).then_with(|| a.name.cmp(&b.name)));
                    self.status = format!("{} packages", self.packages.len());
                    self.pending_sources = self.pending_sources.saturating_sub(1);
                    if self.pending_sources == 0 {
                        self.scanning = false;
                    }
                }
                Msg::Details { key, result } => {
                    self.details_loading = false;
                    match result {
                        Ok(d) => self.details = Some((key, d)),
                        Err(e) => {
                            self.details = Some((
                                key,
                                PackageDetails {
                                    description: format!("Failed to load details: {e}"),
                                    ..Default::default()
                                },
                            ));
                            self.status = format!("details error: {e}");
                        }
                    }
                }
                Msg::ExtractDone { result } => {
                    self.extracting = false;
                    self.extract_status = Some(match result {
                        Ok(s) => format!("✓ {s}"),
                        Err(e) => format!("✗ extract failed: {e}"),
                    });
                }
            }
        }
    }

    fn filtered_indices(&self) -> Vec<usize> {
        let q = self.search.to_lowercase();
        self.packages
            .iter()
            .enumerate()
            .filter(|(_, p)| self.kind_enabled[p.kind as usize])
            .filter(|(_, p)| {
                q.is_empty()
                    || p.name.to_lowercase().contains(&q)
                    || p.summary.to_lowercase().contains(&q)
            })
            .map(|(i, _)| i)
            .collect()
    }

    // ---------- UI sections ----------

    fn top_bar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.add_space(6.0);
        let mut rescan = false;
        ui.horizontal(|ui| {
            ui.heading("PKG Explorer");
            ui.separator();
            ui.add_enabled_ui(!self.scanning, |ui| {
                if ui
                    .button("⟳ Refresh")
                    .on_hover_text("Re-scan installed packages")
                    .clicked()
                {
                    rescan = true;
                }
            });
            ui.add(
                egui::TextEdit::singleline(&mut self.search)
                    .hint_text("Search name or summary…")
                    .desired_width(320.0),
            );
            if !self.search.is_empty() && ui.button("✕").clicked() {
                self.search.clear();
            }
            ui.separator();
            for (i, kind) in [PkgKind::Deb, PkgKind::Rpm, PkgKind::Flatpak, PkgKind::Brew]
                .into_iter()
                .enumerate()
            {
                ui.toggle_value(&mut self.kind_enabled[i], kind.label());
            }
        });
        ui.add_space(4.0);
        if rescan {
            self.start_scan(ctx);
        }
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if self.scanning || self.details_loading || self.extracting {
                ui.spinner();
            }
            ui.label(&self.status);
            if let Some(s) = self.extract_status.clone() {
                ui.separator();
                ui.label(s);
            }
            if !self.scan_errors.is_empty() {
                let tip = self
                    .scan_errors
                    .iter()
                    .map(|(k, e)| format!("{k}: {e}"))
                    .collect::<Vec<_>>()
                    .join("\n");
                ui.separator();
                ui.colored_label(
                    ui.visuals().error_fg_color,
                    format!("{} source error(s)", self.scan_errors.len()),
                )
                .on_hover_text(tip);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!("{} installed", self.packages.len()));
            });
        });
        ui.add_space(4.0);
    }

    fn package_list(&mut self, ui: &mut egui::Ui, filtered: &[usize], ctx: &egui::Context) {
        ui.add_space(4.0);
        ui.label(format!("{} match(es)", filtered.len()));
        ui.separator();
        let row_h = ui.text_style_height(&egui::TextStyle::Body);
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show_rows(ui, row_h + 6.0, filtered.len(), |ui, range| {
                for row in range {
                    let idx = filtered[row];
                    let p = self.packages[idx].clone();
                    let selected = self.selected == Some(idx);
                    let resp = ui.selectable_label(
                        selected,
                        egui::RichText::new(format!("{}  {}", p.kind.label(), p.name)).monospace(),
                    );
                    if resp.clicked() {
                        self.selected = Some(idx);
                        self.start_details(&p, ctx);
                    }
                    resp.on_hover_text(format!(
                        "{} {}\n{}\n{}",
                        p.name,
                        p.version,
                        sources::format_size(p.size),
                        p.summary
                    ));
                }
            });
    }

    fn details_pane(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let Some(idx) = self.selected else {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new(if self.scanning {
                        "Scanning…"
                    } else {
                        "Select a package on the left"
                    })
                    .weak(),
                );
            });
            return;
        };
        if idx >= self.packages.len() {
            self.selected = None;
            return;
        }
        let p = self.packages[idx].clone();

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.heading(p.name.clone());
            ui.monospace(&p.version);
            ui.label(format!("· {}", p.kind.label()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let btn = ui
                    .add_enabled(
                        !self.extracting && self.details.is_some(),
                        egui::Button::new("⬇ Extract…"),
                    )
                    .on_hover_text(
                        "Unpack .deb/.rpm from package cache if available, else copy the installed files",
                    );
                if btn.clicked() {
                    self.start_extract(ctx);
                }
            });
        });
        if !p.summary.is_empty() {
            ui.label(egui::RichText::new(&p.summary).italics());
        }
        ui.add_space(6.0);

        if self.details_loading {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Loading details…");
            });
            return;
        }
        let Some((_, d)) = &self.details else {
            ui.label("No details available.");
            return;
        };

        ui.add_space(4.0);
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("meta")
                    .num_columns(2)
                    .spacing([16.0, 4.0])
                    .show(ui, |ui| {
                        if !d.section.is_empty() {
                            ui.label(egui::RichText::new("Summary").strong());
                            ui.label(&d.section);
                            ui.end_row();
                        }
                        if !p.summary.is_empty() {
                            ui.label(egui::RichText::new("Short").strong());
                            ui.label(&p.summary);
                            ui.end_row();
                        }
                        ui.label(egui::RichText::new("Version").strong());
                        ui.monospace(&p.version);
                        ui.end_row();
                        ui.label(egui::RichText::new("Arch").strong());
                        ui.label(&p.arch);
                        ui.end_row();
                        ui.label(egui::RichText::new("Size").strong());
                        ui.label(sources::format_size(p.size));
                        ui.end_row();
                        if !d.homepage.is_empty() {
                            ui.label(egui::RichText::new("Homepage").strong());
                            ui.hyperlink(&d.homepage);
                            ui.end_row();
                        }
                        if !d.maintainer.is_empty() {
                            ui.label(egui::RichText::new("Maintainer").strong());
                            ui.label(&d.maintainer);
                            ui.end_row();
                        }
                        if !d.license.is_empty() {
                            ui.label(egui::RichText::new("License").strong());
                            ui.label(&d.license);
                            ui.end_row();
                        }
                        if !d.install_date.is_empty() {
                            ui.label(egui::RichText::new("Installed").strong());
                            ui.label(&d.install_date);
                            ui.end_row();
                        }
                        for (k, v) in &d.extra {
                            ui.label(egui::RichText::new(k).strong());
                            ui.label(v);
                            ui.end_row();
                        }
                    });

                if !d.description.is_empty() {
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("Description").strong());
                    ui.label(&d.description);
                }

                if !d.dependencies.is_empty() {
                    ui.add_space(8.0);
                    ui.collapsing(format!("Dependencies ({})", d.dependencies.len()), |ui| {
                        egui::ScrollArea::vertical()
                            .max_height(200.0)
                            .show(ui, |ui| {
                                for dep in &d.dependencies {
                                    ui.label(dep);
                                }
                            });
                    });
                }

                if !d.files.is_empty() {
                    ui.add_space(8.0);
                    ui.collapsing(format!("Files ({})", d.files.len()), |ui| {
                        let row_h = ui.text_style_height(&egui::TextStyle::Monospace);
                        egui::ScrollArea::vertical()
                            .max_height(360.0)
                            .auto_shrink([false, false])
                            .show_rows(ui, row_h + 2.0, d.files.len(), |ui, range| {
                                for i in range {
                                    ui.monospace(d.files[i].display().to_string());
                                }
                            });
                    });
                }
                ui.add_space(24.0);
            });
    }
}

fn package_key(p: &Package) -> String {
    format!("{:?}:{}", p.kind, p.name)
}

impl eframe::App for PkgApp {
    fn logic(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_messages();
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        egui::Panel::top("top").show(ui, |ui| self.top_bar(ui, &ctx));
        egui::Panel::bottom("bottom").show(ui, |ui| self.status_bar(ui));

        let filtered = self.filtered_indices();
        egui::Panel::left("list")
            .exact_size(420.0)
            .resizable(true)
            .show(ui, |ui| self.package_list(ui, &filtered, &ctx));

        egui::CentralPanel::default().show(ui, |ui| self.details_pane(ui, &ctx));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::App;
    use std::time::{Duration, Instant};

    fn pump(app: &mut PkgApp, ctx: &egui::Context, frames: usize) {
        let mut frame = eframe::Frame::_new_kittest();
        for _ in 0..frames {
            app.logic(ctx, &mut frame);
            let mut output = ctx.run_ui(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| app.ui(ui, &mut frame));
            });
            // No real renderer in tests: drop texture updates intentionally.
            output.textures_delta.clear();
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    #[test]
    fn headless_ui_renders() {
        let ctx = egui::Context::default();
        let mut app = PkgApp::with_ctx(&ctx);
        pump(&mut app, &ctx, 10);
        assert!(!app.scanning || app.pending_sources > 0 || !app.packages.is_empty());
    }

    #[test]
    fn scan_and_load_details() {
        if sources::available_sources().is_empty() {
            return;
        }
        let ctx = egui::Context::default();
        let mut app = PkgApp::with_ctx(&ctx);
        let deadline = Instant::now() + Duration::from_secs(180);
        while app.scanning && Instant::now() < deadline {
            app.poll_messages();
            std::thread::sleep(Duration::from_millis(100));
        }
        assert!(
            !app.packages.is_empty(),
            "expected packages from available managers"
        );

        let pkg = app.packages[0].clone();
        app.start_details(&pkg, &ctx);
        let deadline = Instant::now() + Duration::from_secs(60);
        while app.details.is_none() && Instant::now() < deadline {
            app.poll_messages();
            std::thread::sleep(Duration::from_millis(100));
        }
        let (_, d) = app.details.as_ref().expect("details should load");
        assert!(!d.files.is_empty(), "expected file list for {}", pkg.name);
        pump(&mut app, &ctx, 5);
    }
}
