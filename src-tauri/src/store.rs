use crate::{
    errors::user_error,
    models::{AppData, CURRENT_DATA_VERSION},
};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

pub struct Store {
    pub path: PathBuf,
    pub data: Mutex<AppData>,
}

#[derive(Debug)]
enum StateReadError {
    Io(std::io::Error),
    Invalid,
    FutureVersion(u32),
}

impl Store {
    pub fn load(path: PathBuf) -> Result<Self, String> {
        let (mut data, notice) = load_with_recovery(&path)?;
        data.startup_notice = notice;
        Ok(Self {
            path,
            data: Mutex::new(data),
        })
    }

    pub fn save(&self, data: &AppData) -> Result<(), String> {
        let mut current = data.clone();
        migrate(&mut current)?;
        write_state(&self.path, &current, true)
    }
}

fn load_with_recovery(path: &Path) -> Result<(AppData, Option<String>), String> {
    if !path.exists() {
        return restore_without_primary(path);
    }

    match read_state(path) {
        Ok((data, migrated)) => {
            if migrated {
                write_state(path, &data, true)?;
            }
            Ok((data, None))
        }
        Err(StateReadError::FutureVersion(version)) => Err(future_version_message(version)),
        Err(StateReadError::Io(error)) => Err(user_error(error, "无法读取应用数据")),
        Err(StateReadError::Invalid) => recover_invalid_primary(path),
    }
}

fn restore_without_primary(path: &Path) -> Result<(AppData, Option<String>), String> {
    let backup = backup_path(path);
    if !backup.exists() {
        return Ok((AppData::default(), None));
    }
    match read_state(&backup) {
        Ok((data, _)) => {
            write_state(path, &data, false)?;
            Ok((
                data,
                Some("主状态文件缺失，已从最近的有效备份恢复。".into()),
            ))
        }
        Err(StateReadError::FutureVersion(version)) => Err(future_version_message(version)),
        Err(StateReadError::Io(error)) => Err(user_error(error, "无法读取应用数据备份")),
        Err(StateReadError::Invalid) => {
            let preserved = preserve_invalid_file(&backup)?;
            let data = AppData::default();
            write_state(path, &data, false)?;
            Ok((
                data,
                Some(format!(
                    "应用数据备份已损坏，已保留为 {preserved}，当前使用空白状态。"
                )),
            ))
        }
    }
}

fn recover_invalid_primary(path: &Path) -> Result<(AppData, Option<String>), String> {
    let preserved = preserve_invalid_file(path)?;
    let backup = backup_path(path);
    if backup.exists() {
        match read_state(&backup) {
            Ok((data, _)) => {
                write_state(path, &data, false)?;
                return Ok((
                    data,
                    Some(format!(
                        "应用数据已损坏，原文件已保留为 {preserved}，并从最近备份恢复。"
                    )),
                ));
            }
            Err(StateReadError::FutureVersion(version)) => {
                return Err(future_version_message(version));
            }
            Err(StateReadError::Io(error)) => {
                return Err(user_error(error, "无法读取应用数据备份"));
            }
            Err(StateReadError::Invalid) => {}
        }
    }

    let data = AppData::default();
    write_state(path, &data, false)?;
    Ok((
        data,
        Some(format!(
            "应用数据已损坏且没有有效备份，原文件已保留为 {preserved}，当前使用空白状态。"
        )),
    ))
}

fn read_state(path: &Path) -> Result<(AppData, bool), StateReadError> {
    let bytes = fs::read(path).map_err(StateReadError::Io)?;
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| StateReadError::Invalid)?;
    let previous_version = value
        .get("version")
        .and_then(serde_json::Value::as_u64)
        .and_then(|version| u32::try_from(version).ok())
        .ok_or(StateReadError::Invalid)?;
    if previous_version > CURRENT_DATA_VERSION {
        return Err(StateReadError::FutureVersion(previous_version));
    }
    let mut data: AppData = serde_json::from_value(value).map_err(|_| StateReadError::Invalid)?;
    migrate(&mut data).map_err(|_| StateReadError::Invalid)?;
    Ok((data, previous_version != CURRENT_DATA_VERSION))
}

fn migrate(data: &mut AppData) -> Result<(), String> {
    if data.version > CURRENT_DATA_VERSION {
        return Err(future_version_message(data.version));
    }
    while data.version < CURRENT_DATA_VERSION {
        match data.version {
            1 => data.version = 2,
            _ => return Err("应用数据版本无效，无法自动迁移".into()),
        }
    }
    Ok(())
}

fn write_state(path: &Path, data: &AppData, preserve_previous: bool) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| user_error(error, "无法创建应用数据目录"))?;
    }
    if preserve_previous && path.exists() {
        match read_state(path) {
            Ok(_) => {
                fs::copy(path, backup_path(path))
                    .map_err(|error| user_error(error, "无法备份现有应用数据"))?;
            }
            Err(StateReadError::FutureVersion(version)) => {
                return Err(future_version_message(version));
            }
            Err(StateReadError::Io(error)) => {
                return Err(user_error(error, "无法读取现有应用数据"));
            }
            Err(StateReadError::Invalid) => {
                return Err("现有应用数据已损坏，请重启应用以进入恢复流程".into());
            }
        }
    }

    let temporary = temporary_path(path);
    let bytes =
        serde_json::to_vec_pretty(data).map_err(|error| user_error(error, "无法生成应用数据"))?;
    fs::write(&temporary, bytes).map_err(|error| user_error(error, "无法保存应用数据"))?;
    fs::rename(&temporary, path).map_err(|error| user_error(error, "无法完成应用数据保存"))
}

fn backup_path(path: &Path) -> PathBuf {
    path.with_extension("backup.json")
}

fn temporary_path(path: &Path) -> PathBuf {
    path.with_extension("tmp")
}

fn preserve_invalid_file(path: &Path) -> Result<String, String> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("state");
    let preserved = path.with_file_name(format!("{stem}.corrupt-{timestamp}.json"));
    fs::copy(path, &preserved).map_err(|error| user_error(error, "无法保留损坏的应用数据"))?;
    Ok(preserved
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("损坏数据备份")
        .to_string())
}

fn future_version_message(version: u32) -> String {
    format!("应用数据来自更新版本（数据版本 {version}），请升级 Quick Command 后再打开")
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn test_root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("quick-command-store-{name}-{}", Uuid::new_v4()))
    }

    fn write_json(path: &Path, data: &AppData) {
        fs::write(path, serde_json::to_vec_pretty(data).unwrap()).unwrap();
    }

    #[test]
    fn migrates_version_one_data_and_rewrites_the_primary_file() {
        let root = test_root("migration");
        fs::create_dir(&root).unwrap();
        let path = root.join("state.json");
        let mut legacy = AppData::default();
        legacy.version = 1;
        legacy.settings.shortcut = "Control+Space".into();
        write_json(&path, &legacy);

        let store = Store::load(path.clone()).unwrap();
        let data = store.data.lock().unwrap();
        assert_eq!(data.version, CURRENT_DATA_VERSION);
        assert_eq!(data.settings.shortcut, "Control+Space");
        drop(data);
        let persisted: AppData = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        assert_eq!(persisted.version, CURRENT_DATA_VERSION);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn recovers_a_corrupt_primary_from_the_last_valid_backup() {
        let root = test_root("backup-recovery");
        fs::create_dir(&root).unwrap();
        let path = root.join("state.json");
        fs::write(&path, b"not json").unwrap();
        let mut backup = AppData::default();
        backup.settings.shortcut = "Alt+Space".into();
        write_json(&backup_path(&path), &backup);

        let store = Store::load(path.clone()).unwrap();
        let data = store.data.lock().unwrap();
        assert_eq!(data.settings.shortcut, "Alt+Space");
        assert!(data
            .startup_notice
            .as_deref()
            .is_some_and(|notice| notice.contains("从最近备份恢复")));
        assert!(fs::read_dir(&root).unwrap().flatten().any(|entry| entry
            .file_name()
            .to_string_lossy()
            .starts_with("state.corrupt-")));
        drop(data);
        assert!(read_state(&path).is_ok());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn restores_a_missing_primary_from_the_last_valid_backup() {
        let root = test_root("missing-primary");
        fs::create_dir(&root).unwrap();
        let path = root.join("state.json");
        let mut backup = AppData::default();
        backup.settings.shortcut = "Command+2".into();
        write_json(&backup_path(&path), &backup);

        let store = Store::load(path.clone()).unwrap();
        let data = store.data.lock().unwrap();
        assert_eq!(data.settings.shortcut, "Command+2");
        assert!(data
            .startup_notice
            .as_deref()
            .is_some_and(|notice| notice.contains("主状态文件缺失")));
        drop(data);
        assert!(path.is_file());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn preserves_corrupt_data_and_starts_with_defaults_without_a_backup() {
        let root = test_root("default-recovery");
        fs::create_dir(&root).unwrap();
        let path = root.join("state.json");
        fs::write(&path, b"not json").unwrap();

        let store = Store::load(path.clone()).unwrap();
        let data = store.data.lock().unwrap();
        assert_eq!(data.version, CURRENT_DATA_VERSION);
        assert!(data.settings.workspaces.is_empty());
        assert!(data
            .startup_notice
            .as_deref()
            .is_some_and(|notice| notice.contains("没有有效备份")));
        assert!(fs::read_dir(&root).unwrap().flatten().any(|entry| entry
            .file_name()
            .to_string_lossy()
            .starts_with("state.corrupt-")));
        drop(data);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn refuses_to_overwrite_data_from_a_future_version() {
        let root = test_root("future");
        fs::create_dir(&root).unwrap();
        let path = root.join("state.json");
        fs::write(
            &path,
            format!(
                "{{\"version\":{},\"futureOnlyField\":true}}",
                CURRENT_DATA_VERSION + 1
            ),
        )
        .unwrap();
        let original = fs::read(&path).unwrap();

        let error = Store::load(path.clone()).err().unwrap();
        assert!(error.contains("来自更新版本"));
        assert_eq!(fs::read(path).unwrap(), original);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn save_keeps_the_previous_valid_state_as_a_backup() {
        let root = test_root("save-backup");
        fs::create_dir(&root).unwrap();
        let path = root.join("state.json");
        let mut first = AppData::default();
        first.settings.shortcut = "Control+1".into();
        write_json(&path, &first);
        let store = Store::load(path.clone()).unwrap();
        let mut second = first;
        second.settings.shortcut = "Control+2".into();

        store.save(&second).unwrap();
        let (backup, _) = read_state(&backup_path(&path)).unwrap();
        let (primary, _) = read_state(&path).unwrap();
        assert_eq!(backup.settings.shortcut, "Control+1");
        assert_eq!(primary.settings.shortcut, "Control+2");

        fs::remove_dir_all(root).unwrap();
    }
}
