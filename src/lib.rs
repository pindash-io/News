mod components;

// use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::watch::Sender;

pub use components::*;
pub mod db;
pub mod easymark;
pub mod models;
pub mod ui;
pub mod utils;
pub mod windows;

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    Create,
    Update,
    Read,
    Delete,
    Fetch,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Message {
    Normal,
    RefreshFolders,
    Feed(Action, models::Feed),
    Folder(Action, models::Folder),
}

#[derive(Debug)]
pub struct Store {
    pub sender: Sender<Message>,
    pub folders: Arc<RwLock<Vec<models::Folder>>>,
    // pub feeds: Arc<RwLock<HashMap<u64, Vec<models::Feed>>>>,
}

impl Store {
    pub fn new(sender: Sender<Message>, folders: Arc<RwLock<Vec<models::Folder>>>) -> Self {
        Self {
            sender,
            folders,
            // feeds: Arc::default(),
        }
    }
}
