use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    pub path: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub shortcut: String,
    pub default_workspace: Option<String>,
    pub workspaces: Vec<Workspace>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            shortcut: "CommandOrControl+Shift+Space".into(),
            default_workspace: None,
            workspaces: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryRecord {
    pub path: String,
    pub name: String,
    pub use_count: u32,
    pub last_used_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    pub id: String,
    pub display_text: String,
    pub executable: String,
    pub args: Vec<String>,
    pub target_path: Option<String>,
    pub executed_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppData {
    pub version: u32,
    pub settings: Settings,
    pub directories: Vec<DirectoryRecord>,
    pub history: Vec<HistoryItem>,
}

impl Default for AppData {
    fn default() -> Self {
        Self { version: 1, settings: Settings::default(), directories: vec![], history: vec![] }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherState {
    pub settings: Settings,
    pub history: Vec<HistoryItem>,
    pub indexed_directory_count: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub path: String,
    pub name: String,
    pub score: f64,
    pub last_used_at: Option<u64>,
    pub use_count: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryResponse {
    pub executable: Option<String>,
    pub directory_query: Option<String>,
    pub results: Vec<SearchResult>,
    pub history: Vec<HistoryItem>,
    pub can_create: bool,
}

#[derive(Debug, Clone)]
pub struct ParsedCommand {
    pub executable: String,
    pub args: Vec<String>,
    pub directory_arg_index: Option<usize>,
}
