use std::{
    mem,
    ops::{Div, Sub},
};

use eframe::{egui, emath};
use serde::{Deserialize, Serialize};

use crate::{
    models::{Feed, Folder},
    Action, Message, Store,
};

use super::{View, Window};

#[derive(Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct AddWindow {
    url: String,
    name: String,
    folder: Folder,
    closed: bool,
    autofocus: bool,
    folders: Option<Vec<Folder>>,
}

impl AddWindow {
    pub const NAME: &'static str = "Add Feed";
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
            self.autofocus = true;
            if let Ok(reader) = store.folders.read() {
                self.folder = reader[0].clone_without_feeds();
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
            let resp =
                ui.add(egui::TextEdit::singleline(&mut self.url).hint_text("Write feed url"));
            if self.autofocus {
                self.autofocus = false;
                ui.memory_mut(|memory| {
                    memory.request_focus(resp.id);
                });
            }
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

                        let feed = Feed::new(
                            mem::replace(&mut self.url, "".to_string()),
                            mem::replace(&mut self.name, "".to_string()),
                            self.folder.id,
                        );
                        if let Err(e) = store.sender.send(Message::Feed(Action::Create, feed)) {
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

#[derive(Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct DeleteWindow {
    feed: Feed,
    closed: bool,
}

impl DeleteWindow {
    pub const NAME: &'static str = "Delete Feed";
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
        if let Some(Message::Feed(_, feed)) = data {
            self.feed = feed;
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
            "Are you sure you want to delete the '{}' feed?",
            self.feed.name
        ));
        ui.end_row();

        ui.with_layout(
            egui::Layout::default().with_cross_align(emath::Align::RIGHT),
            move |ui| {
                ui.horizontal_wrapped(move |ui| {
                    if ui.button("Ok").clicked() {
                        if let Err(e) = store.sender.send(Message::Feed(
                            Action::Delete,
                            mem::replace(&mut self.feed, Feed::default()),
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

#[derive(Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct EditWindow {
    feed: Feed,
    folder: Folder,
    closed: bool,
    autofocus: bool,
    folders: Option<Vec<Folder>>,
}

impl EditWindow {
    pub const NAME: &'static str = "Edit Feed";
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
        if let Some(Message::Feed(_, feed)) = data.take() {
            self.autofocus = true;
            let folder_id = feed.folder_id;
            self.feed = feed;
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
            let resp =
                ui.add(egui::TextEdit::singleline(&mut self.feed.url).hint_text("Write feed url"));
            if self.autofocus {
                self.autofocus = false;
                ui.memory_mut(|memory| {
                    memory.request_focus(resp.id);
                });
            }
        });
        ui.end_row();
        ui.horizontal(|ui| {
            ui.add_sized((50., 24.), egui::Label::new("Name:"));
            ui.add(egui::TextEdit::singleline(&mut self.feed.name).hint_text("Opional"));
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
                        if self.feed.url.is_empty() || self.feed.name.is_empty() {
                            return;
                        }

                        let folder = mem::replace(&mut self.folder, Folder::default());
                        let mut feed = mem::replace(&mut self.feed, Feed::default());
                        feed.folder_id = folder.id;
                        if let Err(e) = store.sender.send(Message::Feed(Action::Update, feed)) {
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
