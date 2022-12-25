use std::{
    mem,
    ops::{Div, Sub},
};

use eframe::{egui, emath};

use crate::{models::Folder, Action, Message, Store};

use super::{View, Window};

#[derive(Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct AddWindow {
    name: String,
    closed: bool,
}

impl AddWindow {
    pub const NAME: &'static str = "Add Folder";
}

impl Window for AddWindow {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn show(
        &mut self,
        store: &Store,
        ctx: &egui::Context,
        open: &mut bool,
        size: egui::Vec2,
        _data: Option<Message>,
    ) {
        self.closed = false;
        egui::Window::new(self.name())
            .resizable(false)
            .default_width(280.0)
            .default_pos(size.sub(egui::vec2(280.0, 600.0)).div(2.0).to_pos2())
            .open(open)
            .show(ctx, |ui| self.ui(ui, store));
    }

    fn is_closed(&self) -> bool {
        self.closed
    }
}

impl View for AddWindow {
    fn ui(&mut self, ui: &mut egui::Ui, store: &Store) {
        ui.horizontal(|ui| {
            ui.add_sized((50., 24.), egui::Label::new("Name:"));
            ui.add(egui::TextEdit::singleline(&mut self.name).hint_text("Write folder name"));
        });
        ui.end_row();

        ui.with_layout(
            egui::Layout::default().with_cross_align(emath::Align::RIGHT),
            move |ui| {
                ui.horizontal_wrapped(move |ui| {
                    if ui.button("Add").clicked() {
                        if self.name.is_empty() {
                            return;
                        }
                        let mut folder = Folder::default();
                        folder.name = mem::replace(&mut self.name, "".to_string());
                        if let Err(e) = store.sender.send(Message::Folder(Action::Create, folder)) {
                            tracing::error!("{e}");
                        } else {
                            self.closed = true;
                        }
                    }
                });
            },
        );
    }
}

#[derive(Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct DeleteWindow {
    folder: Folder,
    closed: bool,
}

impl DeleteWindow {
    pub const NAME: &'static str = "Delete Folder";
}

impl Window for DeleteWindow {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn show(
        &mut self,
        store: &Store,
        ctx: &egui::Context,
        open: &mut bool,
        size: egui::Vec2,
        data: Option<Message>,
    ) {
        if let Some(Message::Folder(_, folder)) = data {
            self.folder = folder;
        }
        self.closed = false;
        egui::Window::new(self.name())
            .resizable(false)
            .default_width(280.0)
            .default_pos(size.sub(egui::vec2(280.0, 600.0)).div(2.0).to_pos2())
            .open(open)
            .show(ctx, |ui| self.ui(ui, store));
    }

    fn is_closed(&self) -> bool {
        self.closed
    }
}

impl View for DeleteWindow {
    fn ui(&mut self, ui: &mut egui::Ui, store: &Store) {
        ui.label(format!(
            "Are you sure you want to delete the '{}' folder?",
            self.folder.name
        ));
        ui.end_row();

        ui.with_layout(
            egui::Layout::default().with_cross_align(emath::Align::RIGHT),
            move |ui| {
                ui.horizontal_wrapped(move |ui| {
                    if ui.button("Ok").clicked() {
                        if let Err(e) = store.sender.send(Message::Folder(
                            Action::Delete,
                            mem::replace(&mut self.folder, Folder::default()),
                        )) {
                            tracing::error!("{e}");
                        } else {
                            self.closed = true;
                        }
                    }
                });
            },
        );
    }
}

#[derive(Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct EditWindow {
    folder: Folder,
    name: String,
    closed: bool,
}

impl EditWindow {
    pub const NAME: &'static str = "Rename Folder";
}

impl Window for EditWindow {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn show(
        &mut self,
        store: &Store,
        ctx: &egui::Context,
        open: &mut bool,
        size: egui::Vec2,
        mut data: Option<Message>,
    ) {
        if let Some(Message::Folder(_, folder)) = data.take() {
            self.name = folder.name.clone();
            self.folder = folder;
        }
        self.closed = false;
        egui::Window::new(self.name())
            .resizable(false)
            .collapsible(false)
            .default_width(280.0)
            .default_pos(size.sub(egui::vec2(280.0, 600.0)).div(2.0).to_pos2())
            .open(open)
            .show(ctx, |ui| self.ui(ui, store));
    }

    fn is_closed(&self) -> bool {
        self.closed
    }
}

impl View for EditWindow {
    fn ui(&mut self, ui: &mut egui::Ui, store: &Store) {
        ui.horizontal(|ui| {
            ui.add_sized((50., 24.), egui::Label::new("Name:"));
            ui.add(egui::TextEdit::singleline(&mut self.name).hint_text("Write folder name"));
        });
        ui.end_row();

        ui.with_layout(
            egui::Layout::default().with_cross_align(emath::Align::RIGHT),
            move |ui| {
                ui.horizontal_wrapped(move |ui| {
                    if ui.button("Rename").clicked() {
                        if self.name.is_empty() || self.name == self.folder.name {
                            return;
                        }
                        let mut folder = mem::replace(&mut self.folder, Folder::default());
                        folder.name = mem::replace(&mut self.name, "".to_string());
                        if let Err(e) = store.sender.send(Message::Folder(Action::Update, folder)) {
                            tracing::error!("{e}");
                        } else {
                            self.closed = true;
                        }
                    }
                });
            },
        );
    }
}
