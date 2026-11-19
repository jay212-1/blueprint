//! The patch pages: inputs, outputs, devices, lines, cables and stands.

use crate::app::{App, Page};
use crate::theme;
use bp_core::patch::{check, PatchProblem};
use bp_core::patch_grid::{PatchGrid, PatchSheet};
use egui::RichText;
use std::sync::Arc;

impl App {
    /// The checks of the patch, worked out once per change.
    pub fn patch_problems(&mut self) -> Arc<Vec<PatchProblem>> {
        let Some(doc) = self.doc.as_ref() else { return Arc::new(Vec::new()) };
        match &self.patch_cache {
            Some((g, v)) if *g == doc.generation => v.clone(),
            _ => {
                let v = Arc::new(check(&doc.project));
                self.patch_cache = Some((doc.generation, v.clone()));
                v
            }
        }
    }

    pub(crate) fn patch_page(&mut self, ui: &mut egui::Ui, sheet: PatchSheet) {
        let problems = self.patch_problems();
        ui.heading(sheet.title());
        ui.label(RichText::new(sheet.help()).small().color(theme::MUTED));
        let devices = self.doc.as_ref().map_or(0, |d| d.project.patch.devices.len());
        if devices == 0 && matches!(sheet, PatchSheet::Inputs | PatchSheet::Outputs | PatchSheet::Lines) {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Start with the devices: stageboxes, desks, recorders.").color(theme::WARN));
                if ui.small_button("Go to Devices").clicked() {
                    self.page = Page::Patch(PatchSheet::Devices);
                }
            });
        }
        if matches!(sheet, PatchSheet::Inputs | PatchSheet::Outputs) && !problems.is_empty() {
            let n = problems.len();
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{n} problem{} in the patch", if n == 1 { "" } else { "s" })).color(theme::WARN));
                if ui.small_button("Show checks").clicked() {
                    self.page = Page::Checks;
                }
            });
        }
        ui.add_space(6.0);
        let grid = PatchGrid { sheet, problems: &problems };
        let hint = match sheet {
            PatchSheet::Inputs => "No inputs yet. Click \"Add input\" or paste an input list from a spreadsheet (Ch, Name, Source, Mic …).",
            PatchSheet::Outputs => "No outputs yet. Click \"Add output\": PA, wedges, IEMs, recording and broadcast feeds.",
            PatchSheet::Devices => "No devices yet. Click \"Add device\" or paste a list (Name, Kind, Location, Model, Inputs, Outputs).",
            PatchSheet::Lines => "No lines yet. Click \"Add line\" for a multicore or a network link between two devices.",
            PatchSheet::Pack => "Nothing to pack yet: the list is counted from the inputs, outputs and lines.",
        };
        self.table(ui, &grid, sheet.noun(), hint);
    }
}
