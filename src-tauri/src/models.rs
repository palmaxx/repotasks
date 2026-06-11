use serde::{Deserialize, Serialize};

/// An imported repo as tracked in the central index (`projects.json`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub path: String,
    pub color: String,
    #[serde(default)]
    pub pinned: bool,
    pub added_at: String,
}

/// The sidecar `.repotasks.json` written into each imported repo.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoConfig {
    pub id: String,
    pub name: String,
    pub color: String,
    pub created_at: String,
    pub note_file: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntryKind {
    Note,
    Todo,
}

/// A single parsed line from a `NOTES.md` file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub kind: EntryKind,
    pub text: String,
    /// Meaningful for todos; always false for notes.
    pub done: bool,
    pub timestamp: Option<String>,
    /// 0-based line index in the file, used for in-place rewrites.
    pub line: usize,
}
