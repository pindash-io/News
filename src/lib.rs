mod components;

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::watch::Sender;

pub use components::*;
pub mod db;
pub mod models;
pub mod ui;
pub mod windows;

#[derive(Clone, Debug, PartialEq)]
pub enum Message {
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
    /// (url, id)
    FetchFeedsBySource(String, u64),
}

#[derive(Debug)]
pub struct Store {
    pub sender: Sender<Message>,
    pub folders: Arc<RwLock<Vec<models::Folder>>>,
    pub feeds: Arc<RwLock<HashMap<u64, Vec<models::Feed>>>>,
}

impl Store {
    pub fn new(sender: Sender<Message>, folders: Arc<RwLock<Vec<models::Folder>>>) -> Self {
        Self {
            sender,
            folders,
            feeds: Arc::default(),
        }
    }
}
