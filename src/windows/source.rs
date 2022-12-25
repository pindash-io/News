use std::{
    mem,
    ops::{Div, Sub},
};

use eframe::{egui, emath};

use crate::{
    models::{Folder, Source},
    Action, Message, Store,
};

use super::{View, Window};

#[derive(Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct AddWindow {
    url: String,
    name: String,
    folder: Folder,
    closed: bool,
    folders: Option<Vec<Folder>>,
}

impl AddWindow {
    pub const NAME: &'static str = "Add Source";
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
        mut data: Option<Message>,
    ) {
        // must
        if let Some(Message::RefreshFolders) = data.take() {
            if let Ok(reader) = store.folders.read() {
                self.folder = reader[0].clone_without_sources();
                self.folders = Some(reader.to_vec());
            }
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

impl View for AddWindow {
    fn ui(&mut self, ui: &mut egui::Ui, store: &Store) {
        ui.horizontal(|ui| {
            ui.add_sized((50., 24.), egui::Label::new("URL:"));
            ui.add(egui::TextEdit::singleline(&mut self.url).hint_text("Write source url"));
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
                .selected_text(self.folder.name.to_string())
                .show_ui(ui, |ui| {
                    let folder_id = &mut self.folder.id;
                    let folder_name = &mut self.folder.name;
                    self.folders.as_ref().map(move |folders| {
                        folders.iter().for_each(move |folder| {
                            if ui
                                .selectable_value(folder_id, folder.id, folder.name.to_string())
                                .changed()
                            {
                                *folder_name = folder.name.to_string();
                            }
                        });
                    });
                });
        });
        ui.end_row();

        ui.with_layout(
            egui::Layout::default().with_cross_align(emath::Align::RIGHT),
            move |ui| {
                ui.horizontal_wrapped(move |ui| {
                    if ui.button("Add").clicked() {
                        if self.url.is_empty() || self.name.is_empty() {
                            return;
                        }

                        let source = Source::new(
                            mem::replace(&mut self.url, "".to_string()),
                            mem::replace(&mut self.name, "".to_string()),
                            self.folder.id,
                        );
                        if let Err(e) = store.sender.send(Message::Source(Action::Create, source)) {
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
    source: Source,
    closed: bool,
}

impl DeleteWindow {
    pub const NAME: &'static str = "Delete Source";
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
        if let Some(Message::Source(_, source)) = data {
            self.source = source
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
            "Are you sure you want to delete the '{}' source?",
            self.source.name
        ));
        ui.end_row();

        ui.with_layout(
            egui::Layout::default().with_cross_align(emath::Align::RIGHT),
            move |ui| {
                ui.horizontal_wrapped(move |ui| {
                    if ui.button("Ok").clicked() {
                        if let Err(e) = store.sender.send(Message::Source(
                            Action::Delete,
                            mem::replace(&mut self.source, Source::default()),
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
    source: Source,
    folder: Folder,
    closed: bool,
    folders: Option<Vec<Folder>>,
}

impl EditWindow {
    pub const NAME: &'static str = "Edit Source";
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
        // must
        if let Some(Message::Source(_, source)) = data.take() {
            let folder_id = source.folder_id;
            self.source = source;
            if let Ok(reader) = store.folders.read() {
                self.folder = reader
                    .iter()
                    .find(|f| f.id == folder_id)
                    .or(Some(&reader[0]))
                    .cloned()
                    .unwrap();
                self.folders = Some(reader.to_vec());
            }
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

impl View for EditWindow {
    fn ui(&mut self, ui: &mut egui::Ui, store: &Store) {
        ui.horizontal(|ui| {
            ui.add_sized((50., 24.), egui::Label::new("URL:"));
            ui.add(egui::TextEdit::singleline(&mut self.source.url).hint_text("Write feed url"));
        });
        ui.end_row();
        ui.horizontal(|ui| {
            ui.add_sized((50., 24.), egui::Label::new("Name:"));
            ui.add(egui::TextEdit::singleline(&mut self.source.name).hint_text("Opional"));
        });
        ui.end_row();

        ui.horizontal(|ui| {
            ui.add_sized((50., 24.), egui::Label::new("Folder:"));
            egui::ComboBox::from_label("")
                .selected_text(self.folder.name.to_string())
                .show_ui(ui, |ui| {
                    let folder_id = &mut self.folder.id;
                    let folder_name = &mut self.folder.name;
                    self.folders.as_ref().map(move |folders| {
                        folders.iter().for_each(move |folder| {
                            if ui
                                .selectable_value(folder_id, folder.id, folder.name.to_string())
                                .changed()
                            {
                                *folder_name = folder.name.to_string();
                            }
                        });
                    });
                });
        });
        ui.end_row();

        ui.with_layout(
            egui::Layout::default().with_cross_align(emath::Align::RIGHT),
            move |ui| {
                ui.horizontal_wrapped(move |ui| {
                    if ui.button("Save").clicked() {
                        if self.source.url.is_empty() || self.source.name.is_empty() {
                            return;
                        }

                        let folder = mem::replace(&mut self.folder, Folder::default());
                        let mut source = mem::replace(&mut self.source, Source::default());
                        source.folder_id = folder.id;
                        if let Err(e) = store.sender.send(Message::Source(Action::Update, source)) {
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
