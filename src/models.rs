use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq)]
pub struct Folder {
    pub id: u64,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sources: Option<Vec<Source>>,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq)]
pub struct Source {
    pub id: u64,
    pub name: String,
    pub url: String,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq)]
pub struct Feed {
    pub id: u64,
    pub source_id: u64,
    pub title: String,
    pub content: String,
    pub author: String,
    pub created: u64,
}
