mod components;

use std::sync::{mpsc::Sender, Arc, RwLock};

pub use components::*;
pub mod models;
pub mod windows;

#[derive(Clone, Debug, PartialEq)]
pub enum Messge {
    /// (url, name, folder id)
    NewSource(String, String, u64),
    /// (name)
    NewFolder(String),
    /// (name, id)
    DeleteFolder(String, u64),
    /// (name, id)
    RenameFolder(String, u64),
}

#[derive(Debug, Clone)]
pub struct Store {
    pub sender: Sender<Messge>,
    pub folders: Arc<RwLock<Vec<models::Folder>>>,
}

impl Store {
    pub fn new(sender: Sender<Messge>, folders: Arc<RwLock<Vec<models::Folder>>>) -> Self {
        Self { sender, folders }
    }
}
