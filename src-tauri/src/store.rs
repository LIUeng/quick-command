use crate::models::AppData;
use std::{fs, path::PathBuf, sync::Mutex};

pub struct Store { pub path: PathBuf, pub data: Mutex<AppData> }

impl Store {
    pub fn load(path: PathBuf) -> Result<Self, String> {
        let data = if path.exists() {
            serde_json::from_slice(&fs::read(&path).map_err(|error| error.to_string())?).map_err(|error| format!("状态文件损坏: {error}"))?
        } else { AppData::default() };
        Ok(Self { path, data: Mutex::new(data) })
    }

    pub fn save(&self, data: &AppData) -> Result<(), String> {
        if let Some(parent) = self.path.parent() { fs::create_dir_all(parent).map_err(|error| error.to_string())?; }
        let temporary = self.path.with_extension("tmp");
        fs::write(&temporary, serde_json::to_vec_pretty(data).map_err(|error| error.to_string())?).map_err(|error| error.to_string())?;
        fs::rename(temporary, &self.path).map_err(|error| error.to_string())
    }
}
