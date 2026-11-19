//! Start page: new project, open, recently opened, recover.

use crate::app::App;
use crate::theme;
use egui::{RichText, Vec2};

impl App {
    pub(crate) fn start_page(&mut self, ui: &mut egui::Ui) {
        let w = ui.available_width().min(560.0);
        ui.vertical_centered(|ui| {
            ui.add_space((ui.available_height() * 0.12).clamp(16.0, 120.0));
            ui.label(RichText::new("Blueprint").size(40.0).strong().color(theme::BRAND));
            ui.label(
                RichText::new("Planning for the show: mic plots, setlists, patch plans and more.")
                    .color(theme::MUTED),
            );
            ui.add_space(26.0);
            ui.allocate_ui_with_layout(
                Vec2::new(w, 0.0),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.horizontal(|ui| {
                        let size = Vec2::new((w - 8.0) / 2.0, 44.0);
                        if ui
                            .add_sized(size, egui::Button::new(RichText::new("New project").strong()))
                            .on_hover_text("Cmd/Ctrl+N")
                            .clicked()
                        {
                            self.new_project();
                        }
                        if ui
                            .add_sized(size, egui::Button::new("Open project…"))
                            .on_hover_text("Cmd/Ctrl+O")
                            .clicked()
                        {
                            self.open(None);
                        }
                    });
                    ui.add_space(6.0);
                    if ui
                        .link(RichText::new("Try the demo musical \"The Lighthouse Keeper\"").color(theme::MUTED))
                        .on_hover_text("12 performers, 15 scenes, 16 transmitters, swaps and casts of the day")
                        .clicked()
                    {
                        self.open_demo();
                    }
                    if ui
                        .link(RichText::new("Try the demo band show \"Northbound Static\"").color(theme::MUTED))
                        .on_hover_text("A song library and two setlists: a club show and a festival set")
                        .clicked()
                    {
                        self.open_demo_band();
                    }
                    if self.untitled_recovery.is_some() {
                        ui.add_space(12.0);
                        egui::Frame::new()
                            .fill(theme::WARN.gamma_multiply(0.10))
                            .stroke(egui::Stroke::new(1.0, theme::WARN))
                            .corner_radius(6.0)
                            .inner_margin(egui::Margin::symmetric(12, 10))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.label("An untitled project was not saved last time.");
                                if ui.button("Recover it").clicked() {
                                    self.recover_untitled();
                                }
                            });
                    }
                    ui.add_space(20.0);
                    ui.label(RichText::new("RECENT").small().color(theme::MUTED));
                    ui.add_space(4.0);
                    let recent: Vec<_> = self
                        .settings
                        .recent
                        .iter()
                        .filter(|p| p.exists())
                        .cloned()
                        .collect();
                    if recent.is_empty() {
                        ui.label(RichText::new("Nothing opened yet.").color(theme::MUTED));
                    }
                    for p in recent {
                        let name = p
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("?")
                            .to_string();
                        let folder = p.parent().map(|d| d.display().to_string()).unwrap_or_default();
                        let resp = ui.add_sized(
                            [w, 44.0],
                            egui::Button::new(
                                RichText::new(format!("{name}\n{folder}")).size(13.5),
                            )
                            .wrap_mode(egui::TextWrapMode::Truncate),
                        );
                        if resp.on_hover_text(p.display().to_string()).clicked() {
                            self.open(Some(p.clone()));
                        }
                    }
                },
            );
        });
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
            ui.label(
                RichText::new(format!("Blueprint {}", env!("CARGO_PKG_VERSION")))
                    .small()
                    .color(theme::MUTED),
            );
        });
    }
}
