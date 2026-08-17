use crate::{errors::user_error, models::AppData};
use std::{fs, path::PathBuf, sync::Mutex};

pub struct Store {
    pub path: PathBuf,
    pub data: Mutex<AppData>,
}

impl Store {
    pub fn load(path: PathBuf) -> Result<Self, String> {
        let data = if path.exists() {
            let bytes = fs::read(&path).map_err(|error| user_error(error, "无法读取应用数据"))?;
            serde_json::from_slice(&bytes)
                .map_err(|error| user_error(error, "应用数据无法解析，请重置应用数据后重试"))?
        } else {
            AppData::default()
        };
        Ok(Self {
            path,
            data: Mutex::new(data),
        })
    }

    pub fn save(&self, data: &AppData) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| user_error(error, "无法创建应用数据目录"))?;
        }
        let temporary = self.path.with_extension("tmp");
        let bytes = serde_json::to_vec_pretty(data)
            .map_err(|error| user_error(error, "无法生成应用数据"))?;
        fs::write(&temporary, bytes).map_err(|error| user_error(error, "无法保存应用数据"))?;
        fs::rename(temporary, &self.path).map_err(|error| user_error(error, "无法完成应用数据保存"))
    }
}
