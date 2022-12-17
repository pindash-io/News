use std::ops::Div;
use std::ops::Sub;

use eframe::egui;

mod components;

pub use components::*;
use eframe::egui::Label;
use eframe::emath::Align;

pub trait View {
    fn ui(&mut self, ui: &mut egui::Ui);
}

/// Something to view
pub trait Window {
    /// `&'static` so we can also use it as a key to store open/close state.
    fn name(&self) -> &'static str;

    /// Show windows, etc
    fn show(&mut self, ctx: &egui::Context, open: &mut bool, size: egui::Vec2);
}

#[derive(Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct WindowAddFeed {
    url: String,
    name: String,
}

impl Window for WindowAddFeed {
    fn name(&self) -> &'static str {
        "Add Feed"
    }

    fn show(&mut self, ctx: &egui::Context, open: &mut bool, size: egui::Vec2) {
        egui::Window::new(self.name())
            .resizable(false)
            .default_width(280.0)
            .default_pos(size.sub(egui::vec2(280.0, 600.0)).div(2.0).to_pos2())
            // .vscroll(false)
            .open(open)
            .show(ctx, |ui| self.ui(ui));
    }
}

impl View for WindowAddFeed {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // ui.label("URL:            ");
            ui.add_sized((50., 24.), Label::new("URL:"));
            ui.add(egui::TextEdit::singleline(&mut self.url).hint_text("Write feed url"));
        });
        ui.end_row();
        ui.horizontal(|ui| {
            // ui.label("Name:           ");
            ui.add_sized((50., 24.), Label::new("Name:"));
            ui.add(egui::TextEdit::singleline(&mut self.name).hint_text("Write feed name"));
        });
        ui.end_row();
        // ui.horizontal_wrapped(|ui| {
        //     // ui.label("Name:  ");
        //     ui.add_sized((50., 20.), Label::new("Name:"));
        //     ui.add(egui::TextEdit::singleline(&mut self.name).hint_text("Write feed url"));
        // });

        ui.horizontal(|ui| {
            // ui.label("Folder:");
            ui.add_sized((50., 24.), Label::new("Folder:"));
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
            egui::Layout::default().with_cross_align(Align::RIGHT),
            |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.button("Add");
                    // if ui.button("Cancel").clicked() {
                    // }
                });
            },
        );
    }
}

#[derive(Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct WindowAddFolder {
    name: String,
}

impl Window for WindowAddFolder {
    fn name(&self) -> &'static str {
        "Add Folder"
    }

    fn show(&mut self, ctx: &egui::Context, open: &mut bool, size: egui::Vec2) {
        egui::Window::new(self.name())
            .resizable(false)
            .default_width(280.0)
            .default_pos(size.sub(egui::vec2(280.0, 600.0)).div(2.0).to_pos2())
            // .vscroll(false)
            .open(open)
            .show(ctx, |ui| self.ui(ui));
    }
}

impl View for WindowAddFolder {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add_sized((50., 24.), Label::new("Name:"));
            ui.add(egui::TextEdit::singleline(&mut self.name).hint_text("Write folder name"));
        });
        ui.end_row();

        ui.with_layout(
            egui::Layout::default().with_cross_align(Align::RIGHT),
            |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.button("Add");
                    // if ui.button("Cancel").clicked() {}
                });
            },
        );
    }
}
