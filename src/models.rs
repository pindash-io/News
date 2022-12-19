use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq)]
pub struct Folder {
    pub id: u64,
    pub name: String,
    pub sources: Option<Vec<Source>>,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq)]
pub struct Source {
    pub id: u64,
    pub name: String,
    pub url: String,
}
