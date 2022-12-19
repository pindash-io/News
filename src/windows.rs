use std::mem;
use std::ops::{Div, Sub};

use eframe::{egui, emath};

use crate::models::Source;
use crate::{models, Messge, Store};

pub trait View {
    fn ui(&mut self, ui: &mut egui::Ui, store: &Store);
}

/// Something to view
pub trait Window {
    /// `&'static` so we can also use it as a key to store open/close state.
    fn name(&self) -> &'static str;

    /// Show windows, etc
    fn show(
        &mut self,
        store: &Store,
        ctx: &egui::Context,
        open: &mut bool,
        size: egui::Vec2,
        data: Option<Messge>,
    );

    /// status
    fn is_closed(&self) -> bool;
}

#[derive(Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct WindowAddFeed {
    url: String,
    name: String,
    folder_name: String,
    folder_id: u64,
    closed: bool,
    folders: Option<Vec<models::Folder>>,
}

impl WindowAddFeed {
    pub const NAME: &'static str = "Add Feed";
}

impl Window for WindowAddFeed {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn show(
        &mut self,
        store: &Store,
        ctx: &egui::Context,
        open: &mut bool,
        size: egui::Vec2,
        mut data: Option<Messge>,
    ) {
        // must
        if let Some(Messge::RefreshFolders) = data.take() {
            if let Ok(reader) = store.folders.read() {
                self.folder_id = reader[0].id;
                self.folder_name = reader[0].name.to_string();
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

impl View for WindowAddFeed {
    fn ui(&mut self, ui: &mut egui::Ui, store: &Store) {
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
                .selected_text(self.folder_name.to_string())
                .show_ui(ui, |ui| {
                    let folder_id = &mut self.folder_id;
                    let folder_name = &mut self.folder_name;
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
                        let url = mem::replace(&mut self.url, "".to_string());
                        let name = mem::replace(&mut self.name, "".to_string());

                        if url.is_empty() || name.is_empty() {
                            return;
                        }

                        if let Err(e) =
                            store
                                .sender
                                .send(Messge::NewSource(url, name, self.folder_id))
                        {
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

    fn show(
        &mut self,
        store: &Store,
        ctx: &egui::Context,
        open: &mut bool,
        size: egui::Vec2,
        data: Option<Messge>,
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

impl View for WindowAddFolder {
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
                        let name = mem::replace(&mut self.name, "".to_string());
                        if name.is_empty() {
                            return;
                        }
                        if let Err(e) = store.sender.send(Messge::NewFolder(name)) {
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
pub struct WindowDeleteFolder {
    folder: models::Folder,
    closed: bool,
}

impl WindowDeleteFolder {
    pub const NAME: &'static str = "Delete Folder";
}

impl Window for WindowDeleteFolder {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn show(
        &mut self,
        store: &Store,
        ctx: &egui::Context,
        open: &mut bool,
        size: egui::Vec2,
        data: Option<Messge>,
    ) {
        if let Some(Messge::DeleteFolder(name, id)) = data {
            self.folder = models::Folder {
                id,
                name,
                sources: None,
            };
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

impl View for WindowDeleteFolder {
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
                        if let Err(e) = store.sender.send(Messge::DeleteFolder(
                            self.folder.name.to_string(),
                            self.folder.id,
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
pub struct WindowRenameFolder {
    folder: models::Folder,
    name: String,
    closed: bool,
}

impl WindowRenameFolder {
    pub const NAME: &'static str = "Rename Folder";
}

impl Window for WindowRenameFolder {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn show(
        &mut self,
        store: &Store,
        ctx: &egui::Context,
        open: &mut bool,
        size: egui::Vec2,
        mut data: Option<Messge>,
    ) {
        if let Some(Messge::RenameFolder(name, id)) = data.take() {
            self.name = name.clone();
            self.folder = models::Folder {
                id,
                name,
                sources: None,
            };
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

impl View for WindowRenameFolder {
    fn ui(&mut self, ui: &mut egui::Ui, store: &Store) {
        // ui.label(format!("Rename “{}” to", self.folder.name));
        // ui.end_row();

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
                        let name = mem::replace(&mut self.name, "".to_string());
                        if name.is_empty() || name == self.folder.name {
                            return;
                        }
                        if let Err(e) = store
                            .sender
                            .send(Messge::RenameFolder(name, self.folder.id))
                        {
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
pub struct WindowDeleteSource {
    source: models::Source,
    folder_id: u64,
    closed: bool,
}

impl WindowDeleteSource {
    pub const NAME: &'static str = "Delete Source";
}

impl Window for WindowDeleteSource {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn show(
        &mut self,
        store: &Store,
        ctx: &egui::Context,
        open: &mut bool,
        size: egui::Vec2,
        data: Option<Messge>,
    ) {
        if let Some(Messge::DeleteSource(name, id, folder_id)) = data {
            self.folder_id = folder_id;
            self.source = models::Source {
                id,
                name,
                url: "".to_string(),
            };
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

impl View for WindowDeleteSource {
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
                        if let Err(e) = store.sender.send(Messge::DeleteSource(
                            self.source.name.to_string(),
                            self.source.id,
                            self.folder_id,
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
pub struct WindowEditSource {
    source: models::Source,
    folder: models::Folder,
    prev_folder_id: u64,
    closed: bool,
    folders: Option<Vec<models::Folder>>,
}

impl WindowEditSource {
    pub const NAME: &'static str = "Edit Source";
}

impl Window for WindowEditSource {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn show(
        &mut self,
        store: &Store,
        ctx: &egui::Context,
        open: &mut bool,
        size: egui::Vec2,
        mut data: Option<Messge>,
    ) {
        // must
        if let Some(Messge::EditSource(url, name, id, folder_id, _)) = data.take() {
            self.source = models::Source { id, name, url };
            if let Ok(reader) = store.folders.read() {
                self.folder = reader
                    .iter()
                    .find(|f| f.id == folder_id)
                    .or(Some(&reader[0]))
                    .cloned()
                    .unwrap();
                self.prev_folder_id = self.folder.id;
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

impl View for WindowEditSource {
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
                        let url = mem::replace(&mut self.source.url, "".to_string());
                        let name = mem::replace(&mut self.source.name, "".to_string());

                        if url.is_empty() || name.is_empty() {
                            return;
                        }

                        if let Err(e) = store.sender.send(Messge::EditSource(
                            url,
                            name,
                            self.source.id,
                            self.folder.id,
                            if self.prev_folder_id == self.folder.id {
                                None
                            } else {
                                Some(self.prev_folder_id)
                            },
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
