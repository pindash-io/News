use std::collections::BTreeSet;
use std::ops::{Div, Sub};

use eframe::{egui, emath};

mod components;

pub use components::*;

pub trait View {
    fn ui(&mut self, ui: &mut egui::Ui);
}

/// Something to view
pub trait Window {
    /// `&'static` so we can also use it as a key to store open/close state.
    fn name(&self) -> &'static str;

    /// Show windows, etc
    fn show(&mut self, ctx: &egui::Context, open: &mut bool, size: egui::Vec2);

    /// status
    fn is_closed(&self) -> bool;
}

#[derive(Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct WindowAddFeed {
    url: String,
    name: String,
    closed: bool,
}

impl WindowAddFeed {
    pub const NAME: &'static str = "Add Feed";
}

impl Window for WindowAddFeed {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn show(&mut self, ctx: &egui::Context, open: &mut bool, size: egui::Vec2) {
        self.closed = false;
        egui::Window::new(self.name())
            .resizable(false)
            .default_width(280.0)
            .default_pos(size.sub(egui::vec2(280.0, 600.0)).div(2.0).to_pos2())
            // .vscroll(false)
            .open(open)
            .show(ctx, |ui| self.ui(ui));
    }

    fn is_closed(&self) -> bool {
        self.closed
    }
}

impl View for WindowAddFeed {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add_sized((50., 24.), egui::Label::new("URL:"));
            ui.add(egui::TextEdit::singleline(&mut self.url).hint_text("Write feed url"));
        });
        ui.end_row();
        ui.horizontal(|ui| {
            ui.add_sized((50., 24.), egui::Label::new("Name:"));
            ui.add(egui::TextEdit::singleline(&mut self.name).hint_text("Opional"));
        });
        ui.end_row();

        ui.horizontal(|ui| {
            ui.add_sized((50., 24.), egui::Label::new("Folder:"));
            egui::ComboBox::from_label("")
                // .selected_text(format!("{:?}", radio))
                .show_ui(ui, |ui| {
                    // ui.selectable_value(radio, Enum::First, "First");
                    // ui.selectable_value(radio, Enum::Second, "Second");
                    // ui.selectable_value(radio, Enum::Third, "Third");
                });
            // });
        });
        ui.end_row();

        ui.with_layout(
            egui::Layout::default().with_cross_align(emath::Align::RIGHT),
            |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui.button("Add").clicked() {
                        self.closed = true;
                        tracing::info!("add feed");
                    }
                });
            },
        );
    }
}

#[derive(Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct WindowAddFolder {
    name: String,
    closed: bool,
}

impl WindowAddFolder {
    pub const NAME: &'static str = "Add Folder";
}

impl Window for WindowAddFolder {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn show(&mut self, ctx: &egui::Context, open: &mut bool, size: egui::Vec2) {
        self.closed = false;
        egui::Window::new(self.name())
            .resizable(false)
            .default_width(280.0)
            .default_pos(size.sub(egui::vec2(280.0, 600.0)).div(2.0).to_pos2())
            .open(open)
            .show(ctx, |ui| self.ui(ui));
    }

    fn is_closed(&self) -> bool {
        self.closed
    }
}

impl View for WindowAddFolder {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add_sized((50., 24.), egui::Label::new("Name:"));
            ui.add(egui::TextEdit::singleline(&mut self.name).hint_text("Write folder name"));
        });
        ui.end_row();

        ui.with_layout(
            egui::Layout::default().with_cross_align(emath::Align::RIGHT),
            |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui.button("Add").clicked() {
                        self.closed = true;
                        tracing::info!("add folder")
                    }
                });
            },
        );
    }
}
