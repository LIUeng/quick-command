use crate::{errors::user_error, models::*, parser, search, store::Store};
use std::{collections::HashMap, fs, path::Path, process::Command, time::{SystemTime, UNIX_EPOCH}};
use tauri::{AppHandle, State};
use tauri_plugin_global_shortcut::GlobalShortcutExt;
use uuid::Uuid;

fn now() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() }

fn snapshot(data: &AppData) -> LauncherState {
    LauncherState { settings: data.settings.clone(), history: data.history.iter().take(30).cloned().collect(), indexed_directory_count: data.directories.len() }
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
        return Ok(QueryResponse { executable: None, directory_query: None, results: vec![], history: data.history.iter().take(30).cloned().collect(), can_create: false });
    }
    let parsed = parser::parse(&query)?;
    let directory_query = parsed.directory_arg_index.and_then(|index| parsed.args.get(index)).cloned();
    let results = directory_query.as_ref().map(|value| search::rank(value, &data.directories, 20)).unwrap_or_default();
    let can_create = directory_query.is_some() && results.is_empty() && data.settings.default_workspace.is_some();
    Ok(QueryResponse { executable: Some(parsed.executable), directory_query, results, history: vec![], can_create })
}

fn validate_target(target: &Path, settings: &Settings) -> Result<(), String> {
    let canonical = target.canonicalize().map_err(|_| format!("目录不存在: {}", target.display()))?;
    let allowed = settings.workspaces.iter().filter(|item| item.enabled).any(|workspace| {
        Path::new(&workspace.path).canonicalize().is_ok_and(|root| canonical.starts_with(root))
    });
    if allowed { Ok(()) } else { Err("目标目录不在已启用的工作区中".into()) }
}

fn launch(parsed: &ParsedCommand, target_path: Option<&str>) -> Result<Vec<String>, String> {
    let mut args = parsed.args.clone();
    if let (Some(index), Some(target)) = (parsed.directory_arg_index, target_path) {
        if index >= args.len() { return Err("目录参数位置无效".into()); }
        args[index] = target.to_string();
    }
    Command::new(&parsed.executable).args(&args).spawn().map_err(|error| {
        user_error(error, &format!("无法启动 {}，请确认该命令在应用 PATH 中可用", parsed.executable))
    })?;
    Ok(args)
}

fn record_success(data: &mut AppData, query: &str, parsed: &ParsedCommand, args: Vec<String>, target_path: Option<String>) {
    let timestamp = now();
    if let Some(target) = target_path.as_ref() {
        if let Some(record) = data.directories.iter_mut().find(|record| &record.path == target) {
            record.use_count += 1;
            record.last_used_at = Some(timestamp);
        }
    }
    data.history.insert(0, HistoryItem { id: Uuid::new_v4().to_string(), display_text: query.to_string(), executable: parsed.executable.clone(), args, target_path, executed_at: timestamp });
    data.history.truncate(200);
}

#[tauri::command]
pub fn execute_command(query: String, target_path: Option<String>, store: State<'_, Store>) -> Result<(), String> {
    let parsed = parser::parse(&query)?;
    let mut data = store.data.lock().map_err(|_| "无法更新应用状态")?;
    if let Some(target) = target_path.as_ref() { validate_target(Path::new(target), &data.settings)?; }
    let args = launch(&parsed, target_path.as_deref())?;
    record_success(&mut data, &query, &parsed, args, target_path);
    store.save(&data)
}

#[tauri::command]
pub fn create_and_execute(query: String, store: State<'_, Store>) -> Result<(), String> {
    let parsed = parser::parse(&query)?;
    let index = parsed.directory_arg_index.ok_or("当前命令没有可创建的目录参数")?;
    let name = parsed.args.get(index).ok_or("缺少项目名称")?;
    if name.is_empty() || name == "." || name == ".." || Path::new(name).components().count() != 1 { return Err("项目名称只能是单个安全目录名".into()); }
    let mut data = store.data.lock().map_err(|_| "无法更新应用状态")?;
    let root = data.settings.default_workspace.as_ref().ok_or("请先设置默认工作区")?;
    let root_path = Path::new(root).canonicalize().map_err(|_| "默认工作区不存在")?;
    let target = root_path.join(name);
    fs::create_dir(&target).map_err(|error| user_error(error, "创建目录失败，请检查目录名称和工作区权限"))?;
    let target_text = target.to_string_lossy().into_owned();
    data.directories.push(DirectoryRecord { path: target_text.clone(), name: name.clone(), use_count: 0, last_used_at: None });
    let args = match launch(&parsed, Some(&target_text)) {
        Ok(args) => args,
        Err(error) => { let _ = fs::remove_dir(&target); return Err(error); }
    };
    record_success(&mut data, &query, &parsed, args, Some(target_text));
    store.save(&data)
}

fn scan_root(root: &Path, max_depth: usize, output: &mut Vec<DirectoryRecord>) -> Result<(), String> {
    fn visit(path: &Path, depth: usize, max_depth: usize, output: &mut Vec<DirectoryRecord>) {
        if depth > max_depth { return; }
        let Ok(entries) = fs::read_dir(path) else { return; };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else { continue; };
            if !file_type.is_dir() { continue; }
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') || matches!(name.as_str(), "node_modules" | "target" | "dist" | "build") { continue; }
            let child = entry.path();
            output.push(DirectoryRecord { path: child.to_string_lossy().into_owned(), name, use_count: 0, last_used_at: None });
            visit(&child, depth + 1, max_depth, output);
        }
    }
    if !root.is_dir() { return Err(format!("工作区不存在: {}", root.display())); }
    visit(root, 1, max_depth, output);
    Ok(())
}

fn rebuild(data: &mut AppData) -> Result<(), String> {
    let old: HashMap<_, _> = data.directories.iter().map(|item| (item.path.clone(), (item.use_count, item.last_used_at))).collect();
    let mut directories = vec![];
    for workspace in data.settings.workspaces.iter().filter(|item| item.enabled) { scan_root(Path::new(&workspace.path), 4, &mut directories)?; }
    directories.sort_by(|left, right| left.path.cmp(&right.path));
    directories.dedup_by(|left, right| left.path == right.path);
    for item in &mut directories { if let Some((count, used)) = old.get(&item.path) { item.use_count = *count; item.last_used_at = *used; } }
    data.directories = directories;
    Ok(())
}

fn replace_shortcut(app: &AppHandle, previous: &str, next: &str) -> Result<(), String> {
    if previous == next { return Ok(()); }
    app.global_shortcut().unregister(previous)
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
pub fn save_settings(settings: Settings, app: AppHandle, store: State<'_, Store>) -> Result<LauncherState, String> {
    let mut data = store.data.lock().map_err(|_| "无法更新应用状态")?;
    let previous_shortcut = data.settings.shortcut.clone();
    let next_shortcut = settings.shortcut.clone();
    let mut next_data = data.clone();
    next_data.settings = settings;
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
pub fn reindex_workspaces(store: State<'_, Store>) -> Result<LauncherState, String> {
    let mut data = store.data.lock().map_err(|_| "无法更新应用状态")?;
    rebuild(&mut data)?;
    store.save(&data)?;
    Ok(snapshot(&data))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn rejects_target_outside_workspace() {
        let settings = Settings { shortcut: String::new(), default_workspace: None, workspaces: vec![] };
        assert!(validate_target(Path::new("/tmp"), &settings).is_err());
    }
}
