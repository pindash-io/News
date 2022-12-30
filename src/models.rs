use serde::{Deserialize, Serialize};

pub use feed_rs::model::{Entry, FeedType, Person};

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq)]
pub struct Folder {
    pub id: u64,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feeds: Option<Vec<Feed>>,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq)]
pub struct Feed {
    pub id: u64,
    pub name: String,
    pub url: String,
    pub last_seen: i64,
    pub folder_id: u64,
    /// true: loading, false not loading
    #[serde(default)]
    pub status: bool,
    #[serde(default)]
    pub articles: Option<Vec<Article>>,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq)]
pub struct Author {
    pub id: u64,
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq)]
pub struct Article {
    pub id: u64,
    pub feed_id: u64,
    pub url: String,
    pub title: String,
    pub content: String,
    /// published
    pub created: i64,
    pub updated: i64,
    #[serde(default)]
    pub authors: Option<Vec<Author>>,
}

impl Feed {
    pub fn new(url: String, name: String, folder_id: u64) -> Self {
        Self {
            id: 0,
            name,
            url,
            folder_id,
            last_seen: 0,
            status: false,
            articles: None,
        }
    }
}

impl Folder {
    pub fn clone_without_feeds(&self) -> Self {
        Self {
            feeds: None,
            ..self.clone()
        }
    }
}
