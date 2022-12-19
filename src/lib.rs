mod components;

use std::sync::{Arc, RwLock};
use tokio::sync::watch::Sender;

pub use components::*;
pub mod db;
pub mod models;
pub mod windows;

#[derive(Clone, Debug, PartialEq)]
pub enum Messge {
    Normal,
    /// (url, name, folder id)
    NewSource(String, String, u64),
    /// (name)
    NewFolder(String),
    /// (name, id)
    DeleteFolder(String, u64),
    /// (name, id)
    RenameFolder(String, u64),
    ///
    RefreshFolders,
    /// (name, id, folder id)
    DeleteSource(String, u64, u64),
    /// (url, name, id, folder id, prev folder id)
    EditSource(String, String, u64, u64, Option<u64>),
}

#[derive(Debug)]
pub struct Store {
    pub sender: Sender<Messge>,
    pub folders: Arc<RwLock<Vec<models::Folder>>>,
}

impl Store {
    pub fn new(sender: Sender<Messge>, folders: Arc<RwLock<Vec<models::Folder>>>) -> Self {
        Self { sender, folders }
    }
}
