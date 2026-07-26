mod data;

use std::cmp::Reverse;
use eframe::egui;
use data::{Patch, ZypperError};
use data::list_zypper_patches;


fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1040.0, 780.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Zypper patch browser",
        options,
        Box::new(|creation_context| {
            creation_context.egui_ctx.set_pixels_per_point(1.2);
            Ok(Box::new(RecordBrowserApp::new(Box::new(
                ZypperCommandData,
            ))))
        }),
    )
}

trait PatchData {
    fn patches(&self, include_all: bool) -> Result<Vec<Patch>, ZypperError>;
}

struct ZypperCommandData;

impl PatchData for ZypperCommandData {
    fn patches(&self, include_all: bool) -> Result<Vec<Patch>, ZypperError> {
        Ok(list_zypper_patches(include_all)?
            .update_status
            .map(|status| status.update_list.updates)
            .unwrap_or_default())
    }
}

struct RecordBrowserApp {
    source: Box<dyn PatchData>,
    patches: Vec<Patch>,
    selected_name: Option<String>,
    search: String,
    load_error: Option<String>,
    include_all: bool,
}

impl RecordBrowserApp {
    fn new(source: Box<dyn PatchData>) -> Self {
        let include_all = true;
        let (mut patches, load_error) = match source.patches(include_all) {
            Ok(patches) => (patches, None),
            Err(error) => (Vec::new(), Some(error.to_string())),
        };
        Self::sort_patches(&mut patches);
        let selected_name = patches.first().map(|patch| patch.name.clone());
        Self {
            source,
            patches,
            selected_name,
            search: String::new(),
            load_error,
            include_all,
        }
    }

    fn refresh(&mut self) {
        match self.source.patches(self.include_all) {
            Ok(patches) => {
                self.patches = patches;
                Self::sort_patches(&mut self.patches);
                self.load_error = None;
                if self.selected().is_none() {
                    self.selected_name = self.patches.first().map(|patch| patch.name.clone());
                }
            }
            Err(error) => self.load_error = Some(error.to_string()),
        }
    }

    fn selected(&self) -> Option<&Patch> {
        self.selected_name
            .as_ref()
            .and_then(|name| self.patches.iter().find(|patch| &patch.name == name))
    }

    fn category_count(&self, category: &str) -> usize {
        self.patches
            .iter()
            .filter(|patch| {
                patch
                    .category
                    .as_deref()
                    .is_some_and(|value| value.eq_ignore_ascii_case(category))
            })
            .count()
    }

    fn status_count(&self, status: &str) -> usize {
        self.patches
            .iter()
            .filter(|patch| {
                patch
                    .status
                    .as_deref()
                    .is_some_and(|value| value.eq_ignore_ascii_case(status))
            })
            .count()
    }

    fn sort_patches(patches: &mut [Patch]) {
        patches.sort_by_cached_key(|patch| {
            let is_needed = patch
                .status
                .as_deref()
                .is_some_and(|status| status.eq_ignore_ascii_case("needed"));
            let (name, number) = patch
                .name
                .rsplit_once('-')
                .unwrap_or((&patch.name,"0"));
            let number = number
                .parse::<u64>()
                .unwrap_or(0);
            (!is_needed, name.to_lowercase(), Reverse(number))
        });
    }
}

impl eframe::App for RecordBrowserApp {

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
       let ctx = ui.ctx().clone();

       egui::Panel::top("toolbar").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Patches");
                 ui.separator();
                ui.label(format!("{} total", self.patches.len()));
                ui.separator();
                ui.label(format!("{} security", self.category_count("security")));
                ui.separator();
                ui.label(format!(
                    "{} recommended",
                    self.category_count("recommended")
                ));
                ui.separator();
                ui.label(format!("{} optional", self.category_count("optional")));
                ui.separator();
                let needed_count = self.status_count("needed");
                let needed_label = format!("{needed_count} needed");
                if needed_count > 0 {
                    ui.colored_label(egui::Color32::RED, needed_label);
                } else {
                    ui.label(needed_label);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    if ui.button("Refresh").clicked() {
                        self.refresh();
                    }
                    if ui
                        .checkbox(&mut self.include_all, "Show all")
                        .changed()
                    {
                        self.refresh();
                    }
                    let is_dark = ui.visuals().dark_mode;
                    let mode_button = if is_dark { "Light" } else { "Dark" };
                    if ui.button(mode_button).clicked() {
                        if is_dark {
                            ctx.set_visuals(egui::Visuals::light());
                        } else {
                            ctx.set_visuals(egui::Visuals::dark());
                        }
                    }
                });
            });
        });

        egui::Panel::left("record_list")
            .resizable(true)
            .default_size(330.0)
            .show(ui, |ui| {
                ui.add(egui::TextEdit::singleline(&mut self.search).hint_text("Search patches..."));
                ui.add_space(8.0);

                if let Some(error) = &self.load_error {
                    ui.colored_label(egui::Color32::RED, error);
                    ui.add_space(8.0);
                }

                let query = self.search.to_lowercase();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for patch in &self.patches {
                        if !query.is_empty()
                            && !patch.name.to_lowercase().contains(&query)
                            && !patch
                            .category
                            .as_deref()
                            .unwrap_or_default()
                            .to_lowercase()
                            .starts_with(&query)
                            && !patch
                            .status
                            .as_deref()
                            .unwrap_or_default()
                            .to_lowercase()
                            .starts_with(&query)
                            && !patch.summary.to_lowercase().contains(&query)
                            && !patch.description.to_lowercase().contains(&query)
                        {
                            continue;
                        }

                        let selected = self.selected_name.as_deref() == Some(patch.name.as_str());
                        let response = ui.selectable_label(selected, &patch.name);
                        if response.clicked() {
                            self.selected_name = Some(patch.name.clone());
                        }
                        ui.horizontal(|ui| {
                            ui.label(patch.category.as_deref().unwrap_or("Uncategorized"));
                            ui.separator();
                            match &patch.status {
                                Some(status) => if status == "needed" {
                                    ui.colored_label(egui::Color32::RED, status)
                                } else {
                                    ui.label(status)
                                },
                                None => ui.label("Unknown"),
                            }

                        });
                        ui.label(&patch.summary);
                        ui.add_space(6.0);
                        ui.separator();
                    }
                });
            });

        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                if let Some(patch) = self.selected().cloned() {
                    ui.heading(&patch.name);
                    ui.separator();
                    ui.add_space(6.0);

                    egui::Grid::new("patch_details")
                        .num_columns(2)
                        .spacing([16.0, 8.0])
                        .show(ui, |ui| {
                            ui.strong("Name");
                            ui.label(&patch.name);
                            ui.end_row();
                            ui.strong("Kind");
                            ui.label(&patch.kind);
                            ui.end_row();
                            ui.strong("Edition");
                            ui.label(&patch.edition);
                            ui.end_row();
                            ui.strong("Architecture");
                            ui.label(&patch.arch);
                            ui.end_row();
                            ui.strong("Status");
                            ui.label(patch.status.as_deref().unwrap_or("Unknown"));
                            ui.end_row();
                            ui.strong("Category");
                            ui.label(patch.category.as_deref().unwrap_or("Uncategorized"));
                            ui.end_row();
                            ui.strong("Severity");
                            ui.label(patch.severity.as_deref().unwrap_or("Unspecified"));
                            ui.end_row();
                            ui.strong("Repository");
                            ui.hyperlink_to(&patch.source.alias, &patch.source.url);
                            ui.end_row();
                            ui.strong("Issue date");
                            ui.label(&patch.issue_date.date);
                            ui.end_row();
                        });

                    ui.add_space(16.0);
                    ui.strong("Summary");
                    ui.label(&patch.summary);
                    ui.add_space(12.0);
                    ui.strong("Description");
                    ui.label(&patch.description);
                    ui.add_space(12.0);
                    if !&patch.license.is_empty() {
                        ui.strong("License");
                        ui.label(&patch.license);
                    }
                    ui.add_space(12.0);
                    ui.strong("Issues");
                    ui.add_space(12.0);
                    for issue in &patch.issue_list.issue {
                        ui.horizontal(|ui| {
                            ui.label(&issue.issue_type);
                            ui.label(&issue.issue_id);
                        });
                        ui.hyperlink_to(&issue.title, &issue.href);
                        ui.add_space(10.0);
                    }
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.label("Select a patch to view its details.");
                    });
                }
            });
        });
    }
}
