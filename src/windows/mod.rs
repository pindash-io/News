use eframe::egui;

use crate::{Message, Store};

pub mod folder;
pub mod source;

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
        data: Option<Message>,
    );

    /// status
    fn is_closed(&self) -> bool;
}
