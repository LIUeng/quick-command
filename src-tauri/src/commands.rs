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
    path::{Component, Path, PathBuf},
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

fn remove_history_item(data: &mut AppData, history_id: &str) -> bool {
    let previous_len = data.history.len();
    data.history.retain(|item| item.id != history_id);
    data.history.len() != previous_len
}

#[tauri::command]
pub fn delete_history_item(
    history_id: String,
    store: State<'_, Store>,
) -> Result<LauncherState, String> {
    let mut data = store.data.lock().map_err(|_| "无法更新应用状态")?;
    let mut next_data = data.clone();
    if remove_history_item(&mut next_data, &history_id) {
        store.save(&next_data)?;
        *data = next_data;
    }
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
    if parsed.executable != "code" || safe_relative_project_path(target).is_none() {
        return vec![];
    }

    let directory_intent = target.ends_with('/');
    let mut actions = Vec::with_capacity(if directory_intent { 1 } else { 2 });
    if !directory_intent {
        actions.push(CommandAction {
            id: "open-file".into(),
            kind: CommandActionKind::OpenFile,
            label: "作为文件打开".into(),
            description: format!("选择工作区后使用 VS Code 打开 {target}"),
            requires_workspace: true,
        });
    }
    actions.push(CommandAction {
        id: "create-directory".into(),
        kind: CommandActionKind::CreateDirectory,
        label: "创建项目目录并打开".into(),
        description: format!("选择工作区后创建目录 {target}"),
        requires_workspace: true,
    });
    actions
}

fn is_safe_child_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.starts_with('-')
        && !name.ends_with('/')
        && Path::new(name).components().count() == 1
}

fn safe_relative_project_path(value: &str) -> Option<PathBuf> {
    if value.is_empty() || value.starts_with('-') || Path::new(value).is_absolute() {
        return None;
    }
    let without_trailing_separator = value.strip_suffix('/').unwrap_or(value);
    if without_trailing_separator.is_empty()
        || without_trailing_separator
            .split('/')
            .any(|component| component.is_empty() || component == "." || component == "..")
    {
        return None;
    }
    let path = Path::new(value);
    if path
        .components()
        .all(|component| matches!(component, Component::Normal(_)))
    {
        Some(path.to_path_buf())
    } else {
        None
    }
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

struct DirectoryCreationPlan {
    target: std::path::PathBuf,
    workspace: std::path::PathBuf,
    name: String,
}

fn resolve_mkdir_plan(
    requested: &str,
    data: &AppData,
) -> Result<Option<DirectoryCreationPlan>, String> {
    if requested.is_empty() || requested.starts_with('-') {
        return Err("请输入有效的目录路径；mkdir 暂不支持命令选项".into());
    }
    let candidate = if Path::new(requested).is_absolute() {
        Path::new(requested).to_path_buf()
    } else {
        let Some(context) = data.active_context.as_deref() else {
            return Ok(None);
        };
        Path::new(context).join(requested)
    };
    if candidate.exists() {
        return Err("目标目录已经存在，无需重复创建".into());
    }
    let name = candidate
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty() && *value != "." && *value != "..")
        .ok_or("目录名称无效，请输入明确的目录名称")?
        .to_string();
    let parent = candidate.parent().ok_or("无法确定目标目录的父目录")?;
    let canonical_parent = parent
        .canonicalize()
        .map_err(|error| user_error(error, "父目录不存在或无法访问；mkdir 暂不自动创建多级目录"))?;
    if !canonical_parent.is_dir() {
        return Err("目标路径的父级不是文件夹".into());
    }
    let workspace = data
        .settings
        .workspaces
        .iter()
        .filter(|workspace| workspace.enabled)
        .filter_map(|workspace| Path::new(&workspace.path).canonicalize().ok())
        .filter(|root| canonical_parent.starts_with(root))
        .max_by_key(|root| root.components().count())
        .ok_or("目标目录必须位于已启用的工作区中")?;
    let target = canonical_parent.join(&name);
    if target.exists() {
        return Err("目标目录已经存在，无需重复创建".into());
    }
    Ok(Some(DirectoryCreationPlan {
        target,
        workspace,
        name,
    }))
}

fn apply_mkdir_plan(
    plan: DirectoryCreationPlan,
    query: &str,
    parsed: &ParsedCommand,
    store: &Store,
    data: &mut AppData,
) -> Result<CommandExecution, String> {
    fs::create_dir(&plan.target)
        .map_err(|error| user_error(error, "创建目录失败，请检查名称和父目录权限"))?;
    let target_text = plan.target.to_string_lossy().into_owned();
    let mut next_data = data.clone();
    if !next_data
        .directories
        .iter()
        .any(|directory| directory.path == target_text)
    {
        next_data.directories.push(DirectoryRecord {
            path: target_text.clone(),
            name: plan.name,
            use_count: 0,
            last_used_at: None,
        });
    }
    record_success(
        &mut next_data,
        query,
        parsed,
        vec![target_text.clone()],
        Some(target_text.clone()),
        HistoryActionKind::CreateDirectory,
    );
    if let Err(error) = store.save(&next_data) {
        let _ = fs::remove_dir(&plan.target);
        return Err(error);
    }
    *data = next_data;
    Ok(CommandExecution::OperationCompleted {
        title: "目录已创建".into(),
        message: "目录已加入项目索引，可以继续使用 code 或 cd。".into(),
        path: target_text,
    })
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
    action_kind: HistoryActionKind,
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
            action: Some(HistoryAction { kind: action_kind }),
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
                let action_kind = match &result.output {
                    PresentationOutput::Directory { .. } => HistoryActionKind::ListDirectory,
                    PresentationOutput::TextFile { .. } => HistoryActionKind::ReadTextFile,
                };
                data.active_context = result.active_context;
                record_success(
                    &mut data,
                    &query,
                    &parsed,
                    result.effective_args,
                    Some(result.target_path),
                    action_kind,
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
            HistoryActionKind::ChangeContext,
        );
        store.save(&data)?;
        return Ok(CommandExecution::ContextUpdated { path: next_context });
    }
    if definition.execution_mode == ExecutionMode::Internal && parsed.executable == "mkdir" {
        let requested = match parsed.args.as_slice() {
            [path] => path.as_str(),
            [] => return Err("请输入要创建的目录名称或路径".into()),
            _ => return Err("mkdir 当前一次只能创建一个目录".into()),
        };
        let Some(plan) = resolve_mkdir_plan(requested, &data)? else {
            return Ok(CommandExecution::NeedsContext {
                message: "请选择工作区作为 mkdir 的起始目录".into(),
            });
        };
        return Ok(CommandExecution::Confirmation {
            confirmation: OperationConfirmation {
                kind: OperationKind::CreateDirectory,
                title: format!("创建目录 {}", plan.name),
                description: "确认后将在所选工作区内创建这个目录。".into(),
                target_path: plan.target.to_string_lossy().into_owned(),
                workspace_path: plan.workspace.to_string_lossy().into_owned(),
                paths_to_create: vec![plan.target.to_string_lossy().into_owned()],
            },
        });
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
    let action_kind = if target_path.is_some() {
        HistoryActionKind::OpenProject
    } else {
        HistoryActionKind::LaunchCommand
    };
    record_success(&mut data, &query, &parsed, args, target_path, action_kind);
    store.save(&data)?;
    Ok(CommandExecution::Launched)
}

#[tauri::command]
pub fn confirm_operation(
    query: String,
    operation_kind: OperationKind,
    target_path: String,
    workspace_path: String,
    store: State<'_, Store>,
) -> Result<CommandExecution, String> {
    let parsed = parser::parse(&query)?;
    let mut data = store.data.lock().map_err(|_| "无法更新应用状态")?;
    match operation_kind {
        OperationKind::CreateDirectory => {
            if parsed.executable != "mkdir" {
                return Err("当前确认信息与命令不匹配，请重新执行命令".into());
            }
            let requested = match parsed.args.as_slice() {
                [path] => path.as_str(),
                _ => return Err("mkdir 当前一次只能创建一个目录".into()),
            };
            let plan = resolve_mkdir_plan(requested, &data)?
                .ok_or("活动上下文已变化，请重新选择工作区并执行命令")?;
            if plan.target != Path::new(&target_path)
                || plan.workspace != selected_workspace(&workspace_path, &data.settings)?
            {
                return Err("目录目标或工作区已变化，请重新确认后再创建".into());
            }
            apply_mkdir_plan(plan, &query, &parsed, &store, &mut data)
        }
        OperationKind::CreateProjectDirectoryAndOpen => {
            if parsed.executable != "code" {
                return Err("当前确认信息与命令不匹配，请重新执行命令".into());
            }
            let index = parsed.directory_arg_index.ok_or("当前命令没有路径参数")?;
            let requested = parsed.args.get(index).ok_or("缺少项目路径")?;
            let root = selected_workspace(&workspace_path, &data.settings)?;
            let plan = resolve_project_creation_plan(&root, requested)?;
            if plan.target != Path::new(&target_path) {
                return Err("项目目标已变化，请重新确认后再创建".into());
            }
            apply_project_creation_plan(plan, &query, &parsed, &store, &mut data)
        }
    }
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

#[derive(Debug)]
struct ProjectCreationPlan {
    root: PathBuf,
    target: PathBuf,
    missing_directories: Vec<PathBuf>,
}

fn resolve_workspace_relative_target(
    workspace_root: &Path,
    relative: &Path,
) -> Result<PathBuf, String> {
    let root = workspace_root
        .canonicalize()
        .map_err(|error| user_error(error, "所选工作区不存在或无法访问"))?;
    let components: Vec<_> = relative.components().collect();
    let mut current = root.clone();
    for (index, component) in components.iter().enumerate() {
        let Component::Normal(name) = component else {
            return Err("项目路径无效，请使用不含 . 或 .. 的相对路径".into());
        };
        let candidate = current.join(name);
        match fs::symlink_metadata(&candidate) {
            Ok(_) => {
                let canonical = candidate
                    .canonicalize()
                    .map_err(|error| user_error(error, "项目路径无法访问"))?;
                if !canonical.starts_with(&root) {
                    return Err("项目路径不能通过符号链接离开所选工作区".into());
                }
                if index + 1 < components.len() && !canonical.is_dir() {
                    return Err("项目路径中的已有部分不是文件夹".into());
                }
                current = canonical;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                current = candidate;
                for remaining in &components[index + 1..] {
                    let Component::Normal(name) = remaining else {
                        return Err("项目路径无效，请使用不含 . 或 .. 的相对路径".into());
                    };
                    current.push(name);
                }
                break;
            }
            Err(error) => return Err(user_error(error, "无法检查项目路径，请确认工作区权限")),
        }
    }
    Ok(current)
}

fn resolve_project_creation_plan(
    workspace_root: &Path,
    requested: &str,
) -> Result<ProjectCreationPlan, String> {
    let relative = safe_relative_project_path(requested)
        .ok_or("项目路径无效，请使用不含 . 或 .. 的相对路径")?;
    let root = workspace_root
        .canonicalize()
        .map_err(|error| user_error(error, "所选工作区不存在或无法访问"))?;
    if !root.is_dir() {
        return Err("所选工作区必须是文件夹".into());
    }

    let mut current = root.clone();
    let mut missing_directories = Vec::new();
    let mut encountered_missing = false;
    for component in relative.components() {
        let Component::Normal(name) = component else {
            return Err("项目路径无效，请使用不含 . 或 .. 的相对路径".into());
        };
        let candidate = current.join(name);
        if encountered_missing {
            current = candidate;
            missing_directories.push(current.clone());
            continue;
        }
        match fs::symlink_metadata(&candidate) {
            Ok(_) => {
                let canonical = candidate
                    .canonicalize()
                    .map_err(|error| user_error(error, "项目路径中的目录无法访问"))?;
                if !canonical.starts_with(&root) {
                    return Err("项目路径不能通过符号链接离开所选工作区".into());
                }
                if !canonical.is_dir() {
                    return Err("项目路径中的已有部分不是文件夹".into());
                }
                current = canonical;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                encountered_missing = true;
                current = candidate;
                missing_directories.push(current.clone());
            }
            Err(error) => {
                return Err(user_error(error, "无法检查项目目录，请确认工作区权限"));
            }
        }
    }
    if missing_directories.is_empty() {
        return Err("目标项目目录已经存在，请直接选择并打开".into());
    }
    Ok(ProjectCreationPlan {
        root,
        target: current,
        missing_directories,
    })
}

fn rollback_created_directories(created: &[PathBuf]) {
    for path in created.iter().rev() {
        let _ = fs::remove_dir(path);
    }
}

fn apply_project_creation_plan(
    plan: ProjectCreationPlan,
    query: &str,
    parsed: &ParsedCommand,
    store: &Store,
    data: &mut AppData,
) -> Result<CommandExecution, String> {
    let mut created = Vec::with_capacity(plan.missing_directories.len());
    for path in &plan.missing_directories {
        if let Err(error) = fs::create_dir(path) {
            rollback_created_directories(&created);
            return Err(user_error(
                error,
                "创建项目目录失败，请检查名称和工作区权限",
            ));
        }
        let canonical = match path.canonicalize() {
            Ok(canonical) if canonical.starts_with(&plan.root) => canonical,
            _ => {
                created.push(path.clone());
                rollback_created_directories(&created);
                return Err("创建后的项目路径超出所选工作区，操作已取消".into());
            }
        };
        created.push(canonical);
    }

    let target_text = plan.target.to_string_lossy().into_owned();
    let args = match launch(parsed, Some(&target_text)) {
        Ok(args) => args,
        Err(error) => {
            rollback_created_directories(&created);
            return Err(error);
        }
    };

    let mut next_data = data.clone();
    for path in &created {
        let path_text = path.to_string_lossy().into_owned();
        if next_data
            .directories
            .iter()
            .any(|directory| directory.path == path_text)
        {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        next_data.directories.push(DirectoryRecord {
            path: path_text,
            name: name.to_string(),
            use_count: 0,
            last_used_at: None,
        });
    }
    next_data.active_context = Some(target_text.clone());
    record_success(
        &mut next_data,
        query,
        parsed,
        args,
        Some(target_text),
        HistoryActionKind::CreateDirectoryAndOpen,
    );
    store.save(&next_data)?;
    *data = next_data;
    Ok(CommandExecution::Launched)
}

#[tauri::command]
pub fn execute_action(
    query: String,
    action_kind: CommandActionKind,
    workspace_path: String,
    store: State<'_, Store>,
) -> Result<CommandExecution, String> {
    let parsed = parser::parse(&query)?;
    if parsed.executable != "code" {
        return Err("当前命令不支持该候选动作".into());
    }
    let index = parsed.directory_arg_index.ok_or("当前命令没有路径参数")?;
    let name = parsed.args.get(index).ok_or("缺少项目路径")?;
    let relative =
        safe_relative_project_path(name).ok_or("项目路径无效，请使用不含 . 或 .. 的相对路径")?;
    let mut data = store.data.lock().map_err(|_| "无法更新应用状态")?;
    let root_path = selected_workspace(&workspace_path, &data.settings)?;
    let target = resolve_workspace_relative_target(&root_path, &relative)?;
    let target_text = target.to_string_lossy().into_owned();

    if action_kind == CommandActionKind::CreateDirectory {
        let plan = resolve_project_creation_plan(&root_path, name)?;
        return Ok(CommandExecution::Confirmation {
            confirmation: OperationConfirmation {
                kind: OperationKind::CreateProjectDirectoryAndOpen,
                title: format!("创建项目 {}", relative.display()),
                description: format!(
                    "确认后将创建 {} 个目录，并使用 VS Code 打开最终目录。",
                    plan.missing_directories.len()
                ),
                target_path: plan.target.to_string_lossy().into_owned(),
                workspace_path: plan.root.to_string_lossy().into_owned(),
                paths_to_create: plan
                    .missing_directories
                    .iter()
                    .map(|path| path.to_string_lossy().into_owned())
                    .collect(),
            },
        });
    }

    let args = match action_kind {
        CommandActionKind::OpenFile => {
            if target.is_dir() {
                return Err("目标已经是文件夹，请选择项目结果或使用创建目录动作".into());
            }
            launch(&parsed, Some(&target_text))?
        }
        CommandActionKind::CreateDirectory => unreachable!(),
    };
    data.active_context = Some(root_path.to_string_lossy().into_owned());
    record_success(
        &mut data,
        &query,
        &parsed,
        args,
        Some(target_text),
        HistoryActionKind::OpenFile,
    );
    store.save(&data)?;
    Ok(CommandExecution::Launched)
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
    fn code_nested_relative_path_exposes_workspace_actions() {
        let parsed = parser::parse("code x-pro/test01").unwrap();
        let actions = candidate_actions(&parsed, Some("x-pro/test01"));
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].kind, CommandActionKind::OpenFile);
        assert_eq!(actions[1].kind, CommandActionKind::CreateDirectory);
    }

    #[test]
    fn code_trailing_slash_exposes_only_directory_creation() {
        let parsed = parser::parse("code x-pro/test01/").unwrap();
        let actions = candidate_actions(&parsed, Some("x-pro/test01/"));
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].kind, CommandActionKind::CreateDirectory);
    }

    #[test]
    fn explicit_paths_are_not_project_search_terms() {
        assert!(!is_safe_child_name("./example"));
        assert!(!is_safe_child_name("../example"));
        assert!(!is_safe_child_name("/tmp/example"));
        assert!(!is_safe_child_name("-a"));
        assert!(is_safe_child_name("example"));
        assert!(safe_relative_project_path("x-pro/test01").is_some());
        assert!(safe_relative_project_path("x-pro/test01/").is_some());
        assert!(safe_relative_project_path("./example").is_none());
        assert!(safe_relative_project_path("x-pro/./example").is_none());
        assert!(safe_relative_project_path("../example").is_none());
        assert!(safe_relative_project_path("x-pro/../example").is_none());
        assert!(safe_relative_project_path("/tmp/example").is_none());
    }

    #[test]
    fn nested_project_plan_tracks_all_missing_directories() {
        let root = std::env::temp_dir().join(format!("quick-command-project-{}", Uuid::new_v4()));
        fs::create_dir(&root).unwrap();

        let plan = resolve_project_creation_plan(&root, "x-pro/test01").unwrap();
        let canonical_root = root.canonicalize().unwrap();
        assert_eq!(plan.target, canonical_root.join("x-pro/test01"));
        assert_eq!(
            plan.missing_directories,
            vec![
                canonical_root.join("x-pro"),
                canonical_root.join("x-pro/test01")
            ]
        );

        fs::create_dir(root.join("existing")).unwrap();
        let plan = resolve_project_creation_plan(&root, "existing/test01").unwrap();
        assert_eq!(
            plan.missing_directories,
            vec![canonical_root.join("existing/test01")]
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn nested_project_plan_rejects_existing_file_component() {
        let root = std::env::temp_dir().join(format!("quick-command-project-{}", Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("file"), "content").unwrap();

        assert!(resolve_project_creation_plan(&root, "file/test01").is_err());

        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn nested_project_plan_rejects_symlink_escape() {
        use std::os::unix::fs::symlink;

        let root = std::env::temp_dir().join(format!("quick-command-project-{}", Uuid::new_v4()));
        let outside =
            std::env::temp_dir().join(format!("quick-command-outside-{}", Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        fs::create_dir(&outside).unwrap();
        symlink(&outside, root.join("escape")).unwrap();

        assert!(resolve_project_creation_plan(&root, "escape/test01").is_err());

        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(outside).unwrap();
    }

    #[test]
    fn rollback_removes_only_new_empty_directories() {
        let root = std::env::temp_dir().join(format!("quick-command-project-{}", Uuid::new_v4()));
        let parent = root.join("parent");
        let child = parent.join("child");
        fs::create_dir_all(&child).unwrap();

        rollback_created_directories(&[parent.clone(), child]);
        assert!(!parent.exists());

        fs::create_dir_all(parent.join("child")).unwrap();
        fs::write(parent.join("child/keep.txt"), "content").unwrap();
        rollback_created_directories(&[parent.clone(), parent.join("child")]);
        assert!(parent.join("child/keep.txt").is_file());

        fs::remove_dir_all(root).unwrap();
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

    #[test]
    fn mkdir_previews_only_targets_inside_existing_workspace_parents() {
        let root = std::env::temp_dir().join(format!("quick-command-mkdir-{}", Uuid::new_v4()));
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
            active_context: Some(child.to_string_lossy().into_owned()),
            ..AppData::default()
        };

        let plan = resolve_mkdir_plan("example", &data).unwrap().unwrap();
        assert_eq!(plan.target, child.canonicalize().unwrap().join("example"));
        assert!(resolve_mkdir_plan("nested/missing", &data).is_err());
        data.active_context = Some(root.to_string_lossy().into_owned());
        assert!(resolve_mkdir_plan("../outside", &data).is_err());
        data.active_context = None;
        assert!(resolve_mkdir_plan("example", &data).unwrap().is_none());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn mkdir_confirmation_creates_and_records_the_directory() {
        let root = std::env::temp_dir().join(format!("quick-command-create-{}", Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
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
        let plan = resolve_mkdir_plan("example", &data).unwrap().unwrap();
        let target = plan.target.clone();
        let store = Store {
            path: root.join("state.json"),
            data: std::sync::Mutex::new(AppData::default()),
        };
        let parsed = parser::parse("mkdir example").unwrap();

        assert!(matches!(
            apply_mkdir_plan(plan, "mkdir example", &parsed, &store, &mut data).unwrap(),
            CommandExecution::OperationCompleted { .. }
        ));
        assert!(target.is_dir());
        assert_eq!(data.history.len(), 1);
        assert_eq!(
            data.history[0].action.as_ref().map(|action| action.kind),
            Some(HistoryActionKind::CreateDirectory)
        );
        assert!(data
            .directories
            .iter()
            .any(|item| item.path == target.to_string_lossy()));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn mkdir_rolls_back_when_state_persistence_fails() {
        let root = std::env::temp_dir().join(format!("quick-command-rollback-{}", Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
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
        let plan = resolve_mkdir_plan("example", &data).unwrap().unwrap();
        let target = plan.target.clone();
        let invalid_state_path = root.join("state-dir");
        fs::create_dir(&invalid_state_path).unwrap();
        let store = Store {
            path: invalid_state_path,
            data: std::sync::Mutex::new(AppData::default()),
        };
        let parsed = parser::parse("mkdir example").unwrap();

        assert!(apply_mkdir_plan(plan, "mkdir example", &parsed, &store, &mut data).is_err());
        assert!(!target.exists());
        assert!(data.history.is_empty());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn deleting_history_preserves_directory_frecency() {
        let mut data = AppData {
            directories: vec![DirectoryRecord {
                path: "/tmp/example".into(),
                name: "example".into(),
                use_count: 7,
                last_used_at: Some(42),
            }],
            history: vec![
                HistoryItem {
                    id: "remove".into(),
                    display_text: "code example".into(),
                    executable: "code".into(),
                    args: vec!["/tmp/example".into()],
                    target_path: Some("/tmp/example".into()),
                    action: None,
                    executed_at: 42,
                },
                HistoryItem {
                    id: "keep".into(),
                    display_text: "ls".into(),
                    executable: "ls".into(),
                    args: vec!["/tmp".into()],
                    target_path: Some("/tmp".into()),
                    action: None,
                    executed_at: 41,
                },
            ],
            ..AppData::default()
        };

        assert!(remove_history_item(&mut data, "remove"));
        assert_eq!(data.history.len(), 1);
        assert_eq!(data.history[0].id, "keep");
        assert_eq!(data.directories[0].use_count, 7);
        assert_eq!(data.directories[0].last_used_at, Some(42));
        assert!(!remove_history_item(&mut data, "missing"));
    }
}
