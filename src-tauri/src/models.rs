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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum HistoryActionKind {
    LaunchCommand,
    OpenProject,
    OpenFile,
    CreateDirectoryAndOpen,
    ListDirectory,
    ReadTextFile,
    ChangeContext,
    CreateDirectory,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HistoryAction {
    pub kind: HistoryActionKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    pub id: String,
    pub display_text: String,
    pub executable: String,
    pub args: Vec<String>,
    pub target_path: Option<String>,
    #[serde(default)]
    pub action: Option<HistoryAction>,
    pub executed_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppData {
    pub version: u32,
    pub settings: Settings,
    #[serde(default)]
    pub active_context: Option<String>,
    pub directories: Vec<DirectoryRecord>,
    pub history: Vec<HistoryItem>,
}

impl Default for AppData {
    fn default() -> Self {
        Self {
            version: 1,
            settings: Settings::default(),
            active_context: None,
            directories: vec![],
            history: vec![],
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherState {
    pub settings: Settings,
    pub active_context: Option<String>,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CommandActionKind {
    OpenFile,
    CreateDirectory,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum OperationKind {
    CreateDirectory,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationConfirmation {
    pub kind: OperationKind,
    pub title: String,
    pub description: String,
    pub target_path: String,
    pub workspace_path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandAction {
    pub id: String,
    pub kind: CommandActionKind,
    pub label: String,
    pub description: String,
    pub requires_workspace: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryResponse {
    pub executable: Option<String>,
    pub directory_query: Option<String>,
    pub results: Vec<SearchResult>,
    pub actions: Vec<CommandAction>,
    pub history: Vec<HistoryItem>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PresentationEntryKind {
    Directory,
    File,
    Symlink,
    Other,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresentationEntry {
    pub name: String,
    pub path: String,
    pub kind: PresentationEntryKind,
    pub size: Option<u64>,
    pub modified_at: Option<u64>,
    pub hidden: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum PresentationOutput {
    Directory {
        path: String,
        entries: Vec<PresentationEntry>,
        directory_count: usize,
        file_count: usize,
        hidden_count: usize,
        detailed: bool,
        truncated: bool,
    },
    TextFile {
        path: String,
        name: String,
        content: String,
        size: u64,
        line_count: usize,
        language: Option<String>,
    },
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum CommandExecution {
    Launched,
    NeedsContext {
        message: String,
    },
    ContextUpdated {
        path: String,
    },
    Confirmation {
        confirmation: OperationConfirmation,
    },
    OperationCompleted {
        title: String,
        message: String,
        path: String,
    },
    Presented {
        output: PresentationOutput,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_history_without_action_metadata_still_loads() {
        let json = r#"{
            "id":"legacy",
            "displayText":"code example",
            "executable":"code",
            "args":["example"],
            "targetPath":null,
            "executedAt":1
        }"#;
        let item: HistoryItem = serde_json::from_str(json).unwrap();
        assert_eq!(item.action, None);
    }
}

#[derive(Debug, Clone)]
pub struct ParsedCommand {
    pub executable: String,
    pub args: Vec<String>,
    pub directory_arg_index: Option<usize>,
}
