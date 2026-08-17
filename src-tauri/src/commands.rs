use crate::{
    command_catalog::{definition_for, ExecutionMode},
    errors::user_error,
    models::*,
    parser, presentation, search,
    store::Store,
    window_behavior::WindowBehavior,
};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, State};
use tauri_plugin_global_shortcut::GlobalShortcutExt;
use uuid::Uuid;

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn snapshot(data: &AppData) -> LauncherState {
    LauncherState {
        settings: data.settings.clone(),
        active_context: data.active_context.clone(),
        history: data.history.iter().take(30).cloned().collect(),
        indexed_directory_count: data.directories.len(),
    }
}

#[tauri::command]
pub fn get_launcher_state(store: State<'_, Store>) -> Result<LauncherState, String> {
    let data = store.data.lock().map_err(|_| "无法读取应用状态")?;
    Ok(snapshot(&data))
}

#[tauri::command]
pub fn search_projects(query: String, store: State<'_, Store>) -> Result<QueryResponse, String> {
    let data = store.data.lock().map_err(|_| "无法读取应用状态")?;
    if query.trim().is_empty() {
        return Ok(QueryResponse {
            executable: None,
            directory_query: None,
            results: vec![],
            actions: vec![],
            history: data.history.iter().take(30).cloned().collect(),
        });
    }
    let parsed = parser::parse(&query)?;
    let directory_query = parsed
        .directory_arg_index
        .and_then(|index| parsed.args.get(index))
        .cloned();
    let results = directory_query
        .as_ref()
        .filter(|value| is_safe_child_name(value))
        .map(|value| search::rank(value, &data.directories, 20))
        .unwrap_or_default();
    let actions = candidate_actions(&parsed, directory_query.as_deref());
    Ok(QueryResponse {
        executable: Some(parsed.executable),
        directory_query,
        results,
        actions,
        history: vec![],
    })
}

fn candidate_actions(parsed: &ParsedCommand, target: Option<&str>) -> Vec<CommandAction> {
    let Some(target) = target else { return vec![] };
    if parsed.executable != "code" || !is_safe_child_name(target) {
        return vec![];
    }

    vec![
        CommandAction {
            id: "open-file".into(),
            kind: CommandActionKind::OpenFile,
            label: "作为文件打开".into(),
            description: format!("选择工作区后使用 VS Code 打开 {target}"),
            requires_workspace: true,
        },
        CommandAction {
            id: "create-directory".into(),
            kind: CommandActionKind::CreateDirectory,
            label: "创建项目目录并打开".into(),
            description: format!("选择工作区后创建目录 {target}"),
            requires_workspace: true,
        },
    ]
}

fn is_safe_child_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.starts_with('-')
        && !name.ends_with('/')
        && Path::new(name).components().count() == 1
}

fn validate_target(target: &Path, settings: &Settings) -> Result<(), String> {
    let canonical = target
        .canonicalize()
        .map_err(|_| format!("目录不存在: {}", target.display()))?;
    let allowed = settings
        .workspaces
        .iter()
        .filter(|item| item.enabled)
        .any(|workspace| {
            Path::new(&workspace.path)
                .canonicalize()
                .is_ok_and(|root| canonical.starts_with(root))
        });
    if allowed {
        Ok(())
    } else {
        Err("目标目录不在已启用的工作区中".into())
    }
}

fn normalized_context(path: &str, settings: &Settings) -> Result<String, String> {
    let target = Path::new(path)
        .canonicalize()
        .map_err(|error| user_error(error, "所选上下文不存在或无法访问"))?;
    if !target.is_dir() {
        return Err("活动上下文必须是文件夹".into());
    }
    validate_target(&target, settings)?;
    Ok(target.to_string_lossy().into_owned())
}

fn resolve_cd_context(
    requested: &str,
    selected_path: Option<&str>,
    data: &AppData,
) -> Result<Option<String>, String> {
    let candidate = if let Some(selected) = selected_path {
        Path::new(selected).to_path_buf()
    } else if Path::new(requested).is_absolute() {
        Path::new(requested).to_path_buf()
    } else {
        let Some(context) = data.active_context.as_deref() else {
            return Ok(None);
        };
        Path::new(context).join(requested)
    };
    normalized_context(&candidate.to_string_lossy(), &data.settings).map(Some)
}

fn launch(parsed: &ParsedCommand, target_path: Option<&str>) -> Result<Vec<String>, String> {
    let mut args = parsed.args.clone();
    if let (Some(index), Some(target)) = (parsed.directory_arg_index, target_path) {
        if index >= args.len() {
            return Err("目录参数位置无效".into());
        }
        args[index] = target.to_string();
    }
    Command::new(&parsed.executable)
        .args(&args)
        .spawn()
        .map_err(|error| {
            user_error(
                error,
                &format!(
                    "无法启动 {}，请确认该命令在应用 PATH 中可用",
                    parsed.executable
                ),
            )
        })?;
    Ok(args)
}

fn record_success(
    data: &mut AppData,
    query: &str,
    parsed: &ParsedCommand,
    args: Vec<String>,
    target_path: Option<String>,
) {
    let timestamp = now();
    if let Some(target) = target_path.as_ref() {
        if let Some(record) = data
            .directories
            .iter_mut()
            .find(|record| &record.path == target)
        {
            record.use_count += 1;
            record.last_used_at = Some(timestamp);
        }
    }
    data.history.insert(
        0,
        HistoryItem {
            id: Uuid::new_v4().to_string(),
            display_text: query.to_string(),
            executable: parsed.executable.clone(),
            args,
            target_path,
            executed_at: timestamp,
        },
    );
    data.history.truncate(200);
}

#[tauri::command]
pub fn execute_command(
    query: String,
    target_path: Option<String>,
    store: State<'_, Store>,
) -> Result<CommandExecution, String> {
    let parsed = parser::parse(&query)?;
    let mut data = store.data.lock().map_err(|_| "无法更新应用状态")?;
    let definition = definition_for(&parsed.executable);
    if definition.execution_mode == ExecutionMode::Capture {
        return match presentation::execute(&parsed, &data)? {
            presentation::PresentationResolution::NeedsContext => {
                Ok(CommandExecution::NeedsContext {
                    message: "请选择工作区作为本次查看的目录上下文".into(),
                })
            }
            presentation::PresentationResolution::Ready(result) => {
                data.active_context = result.active_context;
                record_success(
                    &mut data,
                    &query,
                    &parsed,
                    result.effective_args,
                    Some(result.target_path),
                );
                store.save(&data)?;
                Ok(CommandExecution::Presented {
                    output: result.output,
                })
            }
        };
    }
    if definition.execution_mode == ExecutionMode::Internal && parsed.executable == "cd" {
        let requested = match parsed.args.as_slice() {
            [path] => path.as_str(),
            [] => return Err("请输入要切换到的目录名称或路径".into()),
            _ => return Err("cd 一次只能切换到一个目录".into()),
        };
        let Some(next_context) = resolve_cd_context(requested, target_path.as_deref(), &data)?
        else {
            return Ok(CommandExecution::NeedsContext {
                message: "请选择工作区作为 cd 的起始目录".into(),
            });
        };
        data.active_context = Some(next_context.clone());
        record_success(
            &mut data,
            &query,
            &parsed,
            vec![next_context.clone()],
            Some(next_context.clone()),
        );
        store.save(&data)?;
        return Ok(CommandExecution::ContextUpdated { path: next_context });
    }
    if definition.execution_mode == ExecutionMode::Internal {
        return Err(format!(
            "{} 尚未开放，请等待应用内安全确认流程完成",
            parsed.executable
        ));
    }
    if let Some(target) = target_path.as_ref() {
        validate_target(Path::new(target), &data.settings)?;
    }
    let args = launch(&parsed, target_path.as_deref())?;
    if let Some(target) = target_path.as_deref() {
        if Path::new(target).is_dir() {
            data.active_context = Some(normalized_context(target, &data.settings)?);
        }
    }
    record_success(&mut data, &query, &parsed, args, target_path);
    store.save(&data)?;
    Ok(CommandExecution::Launched)
}

fn selected_workspace(path: &str, settings: &Settings) -> Result<std::path::PathBuf, String> {
    let selected = Path::new(path)
        .canonicalize()
        .map_err(|error| user_error(error, "所选工作区不存在或无法访问"))?;
    let allowed = settings
        .workspaces
        .iter()
        .filter(|workspace| workspace.enabled)
        .any(|workspace| {
            Path::new(&workspace.path)
                .canonicalize()
                .is_ok_and(|configured| configured == selected)
        });
    if allowed {
        Ok(selected)
    } else {
        Err("所选目录不在已配置的工作区中".into())
    }
}

#[tauri::command]
pub fn execute_action(
    query: String,
    action_kind: CommandActionKind,
    workspace_path: String,
    store: State<'_, Store>,
) -> Result<(), String> {
    let parsed = parser::parse(&query)?;
    if parsed.executable != "code" {
        return Err("当前命令不支持该候选动作".into());
    }
    let index = parsed.directory_arg_index.ok_or("当前命令没有路径参数")?;
    let name = parsed.args.get(index).ok_or("缺少项目名称")?;
    if !is_safe_child_name(name) {
        return Err("目标名称只能是单个安全文件或目录名".into());
    }
    let mut data = store.data.lock().map_err(|_| "无法更新应用状态")?;
    let root_path = selected_workspace(&workspace_path, &data.settings)?;
    let target = root_path.join(name);
    let target_text = target.to_string_lossy().into_owned();

    let args = match action_kind {
        CommandActionKind::OpenFile => {
            if target.is_dir() {
                return Err("目标已经是文件夹，请选择项目结果或使用创建目录动作".into());
            }
            launch(&parsed, Some(&target_text))?
        }
        CommandActionKind::CreateDirectory => {
            if target.exists() {
                return Err("目标已经存在，请选择已有项目结果".into());
            }
            fs::create_dir(&target)
                .map_err(|error| user_error(error, "创建目录失败，请检查目录名称和工作区权限"))?;
            let args = match launch(&parsed, Some(&target_text)) {
                Ok(args) => args,
                Err(error) => {
                    let _ = fs::remove_dir(&target);
                    return Err(error);
                }
            };
            data.directories.push(DirectoryRecord {
                path: target_text.clone(),
                name: name.clone(),
                use_count: 0,
                last_used_at: None,
            });
            args
        }
    };
    data.active_context = Some(match action_kind {
        CommandActionKind::OpenFile => root_path.to_string_lossy().into_owned(),
        CommandActionKind::CreateDirectory => target_text.clone(),
    });
    record_success(&mut data, &query, &parsed, args, Some(target_text));
    store.save(&data)
}

#[tauri::command]
pub fn set_active_context(
    path: Option<String>,
    store: State<'_, Store>,
) -> Result<LauncherState, String> {
    let mut data = store.data.lock().map_err(|_| "无法更新应用状态")?;
    data.active_context = path
        .as_deref()
        .map(|value| normalized_context(value, &data.settings))
        .transpose()?;
    store.save(&data)?;
    Ok(snapshot(&data))
}

fn scan_root(
    root: &Path,
    max_depth: usize,
    output: &mut Vec<DirectoryRecord>,
) -> Result<(), String> {
    fn visit(path: &Path, depth: usize, max_depth: usize, output: &mut Vec<DirectoryRecord>) {
        if depth > max_depth {
            return;
        }
        let Ok(entries) = fs::read_dir(path) else {
            return;
        };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if !file_type.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.')
                || matches!(name.as_str(), "node_modules" | "target" | "dist" | "build")
            {
                continue;
            }
            let child = entry.path();
            output.push(DirectoryRecord {
                path: child.to_string_lossy().into_owned(),
                name,
                use_count: 0,
                last_used_at: None,
            });
            visit(&child, depth + 1, max_depth, output);
        }
    }
    if !root.is_dir() {
        return Err(format!("工作区不存在: {}", root.display()));
    }
    visit(root, 1, max_depth, output);
    Ok(())
}

fn rebuild(data: &mut AppData) -> Result<(), String> {
    let old: HashMap<_, _> = data
        .directories
        .iter()
        .map(|item| (item.path.clone(), (item.use_count, item.last_used_at)))
        .collect();
    let mut directories = vec![];
    for workspace in data.settings.workspaces.iter().filter(|item| item.enabled) {
        scan_root(Path::new(&workspace.path), 4, &mut directories)?;
    }
    directories.sort_by(|left, right| left.path.cmp(&right.path));
    directories.dedup_by(|left, right| left.path == right.path);
    for item in &mut directories {
        if let Some((count, used)) = old.get(&item.path) {
            item.use_count = *count;
            item.last_used_at = *used;
        }
    }
    data.directories = directories;
    Ok(())
}

fn normalize_workspaces(settings: &mut Settings) -> Result<(), String> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::with_capacity(settings.workspaces.len());

    for workspace in &settings.workspaces {
        let canonical = Path::new(&workspace.path)
            .canonicalize()
            .map_err(|error| user_error(error, "工作区不存在或无法访问"))?;
        if !canonical.is_dir() {
            return Err("工作区必须是文件夹".into());
        }
        if seen.insert(canonical.clone()) {
            normalized.push(Workspace {
                path: canonical.to_string_lossy().into_owned(),
                enabled: workspace.enabled,
            });
        }
    }

    let normalized_default = settings.default_workspace.as_ref().and_then(|path| {
        Path::new(path)
            .canonicalize()
            .ok()
            .map(|path| path.to_string_lossy().into_owned())
    });
    settings.default_workspace = normalized_default
        .filter(|path| normalized.iter().any(|workspace| &workspace.path == path));
    settings.workspaces = normalized;
    Ok(())
}

fn replace_shortcut(app: &AppHandle, previous: &str, next: &str) -> Result<(), String> {
    if previous == next {
        return Ok(());
    }
    app.global_shortcut()
        .unregister(previous)
        .map_err(|error| user_error(error, "无法更新快捷键，请重启应用后重试"))?;
    if let Err(error) = app.global_shortcut().register(next) {
        let _ = app.global_shortcut().register(previous);
        return Err(user_error(error, "快捷键无效或已被其他应用占用"));
    }
    Ok(())
}

fn restore_shortcut(app: &AppHandle, current: &str, previous: &str) {
    let _ = app.global_shortcut().unregister(current);
    let _ = app.global_shortcut().register(previous);
}

#[tauri::command]
pub fn save_settings(
    mut settings: Settings,
    app: AppHandle,
    store: State<'_, Store>,
) -> Result<LauncherState, String> {
    normalize_workspaces(&mut settings)?;
    let mut data = store.data.lock().map_err(|_| "无法更新应用状态")?;
    let previous_shortcut = data.settings.shortcut.clone();
    let next_shortcut = settings.shortcut.clone();
    let mut next_data = data.clone();
    next_data.settings = settings;
    if next_data
        .active_context
        .as_deref()
        .is_some_and(|path| normalized_context(path, &next_data.settings).is_err())
    {
        next_data.active_context = None;
    }
    rebuild(&mut next_data)?;

    replace_shortcut(&app, &previous_shortcut, &next_shortcut)?;

    if let Err(error) = store.save(&next_data) {
        if previous_shortcut != next_shortcut {
            restore_shortcut(&app, &next_shortcut, &previous_shortcut);
        }
        return Err(error);
    }
    *data = next_data;
    Ok(snapshot(&data))
}

#[tauri::command]
pub fn set_auto_hide_suspended(suspended: bool, behavior: State<'_, WindowBehavior>) {
    behavior.set_auto_hide_suspended(suspended);
}

#[tauri::command]
pub fn reindex_workspaces(store: State<'_, Store>) -> Result<LauncherState, String> {
    let mut data = store.data.lock().map_err(|_| "无法更新应用状态")?;
    rebuild(&mut data)?;
    store.save(&data)?;
    Ok(snapshot(&data))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_target_outside_workspace() {
        let settings = Settings {
            shortcut: String::new(),
            default_workspace: None,
            workspaces: vec![],
        };
        assert!(validate_target(Path::new("/tmp"), &settings).is_err());
    }

    #[test]
    fn normalizes_and_deduplicates_workspace_paths() {
        let mut settings = Settings {
            shortcut: String::new(),
            default_workspace: None,
            workspaces: vec![
                Workspace {
                    path: "/tmp".into(),
                    enabled: true,
                },
                Workspace {
                    path: "/private/tmp".into(),
                    enabled: true,
                },
            ],
        };

        normalize_workspaces(&mut settings).unwrap();
        assert_eq!(settings.workspaces.len(), 1);
        assert_eq!(settings.default_workspace, None);
    }

    #[test]
    fn code_plain_name_exposes_file_and_directory_actions() {
        let parsed = parser::parse("code example").unwrap();
        let actions = candidate_actions(&parsed, Some("example"));
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].kind, CommandActionKind::OpenFile);
        assert_eq!(actions[1].kind, CommandActionKind::CreateDirectory);
    }

    #[test]
    fn code_explicit_relative_path_does_not_offer_workspace_actions() {
        let parsed = parser::parse("code ./example").unwrap();
        assert!(candidate_actions(&parsed, Some("./example")).is_empty());
    }

    #[test]
    fn explicit_paths_are_not_project_search_terms() {
        assert!(!is_safe_child_name("./example"));
        assert!(!is_safe_child_name("../example"));
        assert!(!is_safe_child_name("/tmp/example"));
        assert!(!is_safe_child_name("-a"));
        assert!(is_safe_child_name("example"));
    }

    #[test]
    fn context_must_resolve_inside_an_enabled_workspace() {
        let settings = Settings {
            shortcut: String::new(),
            default_workspace: None,
            workspaces: vec![Workspace {
                path: "/tmp".into(),
                enabled: true,
            }],
        };
        assert!(normalized_context("/tmp", &settings).is_ok());

        let disabled = Settings {
            workspaces: vec![Workspace {
                path: "/tmp".into(),
                enabled: false,
            }],
            ..settings
        };
        assert!(normalized_context("/tmp", &disabled).is_err());
    }

    #[test]
    fn cd_resolves_relative_paths_and_rejects_workspace_escape() {
        let root = std::env::temp_dir().join(format!("quick-command-cd-{}", Uuid::new_v4()));
        let child = root.join("child");
        fs::create_dir_all(&child).unwrap();
        let mut data = AppData {
            settings: Settings {
                shortcut: String::new(),
                default_workspace: None,
                workspaces: vec![Workspace {
                    path: root.to_string_lossy().into_owned(),
                    enabled: true,
                }],
            },
            active_context: Some(root.to_string_lossy().into_owned()),
            ..AppData::default()
        };
        let canonical_root = root.canonicalize().unwrap();
        let canonical_child = child.canonicalize().unwrap();

        assert_eq!(
            resolve_cd_context("child", None, &data).unwrap(),
            Some(canonical_child.to_string_lossy().into_owned())
        );
        data.active_context = Some(child.to_string_lossy().into_owned());
        assert_eq!(
            resolve_cd_context("..", None, &data).unwrap(),
            Some(canonical_root.to_string_lossy().into_owned())
        );
        data.active_context = Some(root.to_string_lossy().into_owned());
        assert!(resolve_cd_context("..", None, &data).is_err());
        data.active_context = None;
        assert_eq!(resolve_cd_context("child", None, &data).unwrap(), None);

        fs::remove_dir_all(root).unwrap();
    }
}
