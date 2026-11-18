use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{de::DeserializeOwned, Serialize};

use crate::models::{HistoryFile, Settings};

/// 所有应用数据都收敛到 ~/.dull_doc，用户输出目录独立。
pub fn data_dir() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or_else(|| "无法定位用户主目录".to_string())?;
    Ok(home.join(".dull_doc"))
}

pub fn ensure_data_dirs() -> Result<PathBuf, String> {
    let root = data_dir()?;
    // 本切片只落 settings/history 与临时、日志目录；runtimes 由后续 Python 引擎按需创建。
    for child in ["logs", "tmp"] {
        fs::create_dir_all(root.join(child)).map_err(|e| format!("创建应用数据目录失败: {e}"))?;
    }
    Ok(root)
}

pub fn expand_tilde(value: &str) -> PathBuf {
    if let Some(rest) = value.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(value)
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Option<T> {
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// 临时文件 + rename，避免半写 JSON 让下次启动无法读取。
fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {e}"))?;
    }
    let tmp = path.with_extension(format!("tmp-{}", uuid::Uuid::new_v4()));
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| format!("序列化失败: {e}"))?;
    {
        let mut file = fs::File::create(&tmp).map_err(|e| format!("写临时文件失败: {e}"))?;
        file.write_all(&bytes)
            .map_err(|e| format!("写临时文件失败: {e}"))?;
        file.sync_all()
            .map_err(|e| format!("同步临时文件失败: {e}"))?;
    }
    fs::rename(&tmp, path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        format!("提交文件失败: {e}")
    })
}

pub fn load_settings() -> Settings {
    let Ok(root) = data_dir() else {
        return Settings::default();
    };
    let mut settings = read_json::<Settings>(&root.join("settings.json")).unwrap_or_default();
    if settings.version == 0 {
        settings.version = 1;
    }
    if !matches!(settings.language.as_str(), "zh" | "en") {
        settings.language = "zh".into();
    }
    if !matches!(settings.theme.as_str(), "system" | "light" | "dark") {
        settings.theme = "system".into();
    }
    if settings.output_dir.trim().is_empty() {
        settings.output_dir = Settings::default().output_dir;
    }
    settings
}

pub fn save_settings(settings: &Settings) -> Result<Settings, String> {
    let mut next = settings.clone();
    next.version = 1;
    if !matches!(next.language.as_str(), "zh" | "en") {
        return Err("不支持的语言".into());
    }
    if !matches!(next.theme.as_str(), "system" | "light" | "dark") {
        return Err("不支持的主题".into());
    }
    if next.output_dir.trim().is_empty() {
        return Err("输出目录不能为空".into());
    }
    let root = ensure_data_dirs()?;
    write_json_atomic(&root.join("settings.json"), &next)?;
    Ok(next)
}

pub fn load_history() -> HistoryFile {
    let Ok(root) = data_dir() else {
        return HistoryFile {
            version: 1,
            tasks: vec![],
        };
    };
    let mut history = read_json::<HistoryFile>(&root.join("history.json")).unwrap_or(HistoryFile {
        version: 1,
        tasks: vec![],
    });
    if history.version == 0 {
        history.version = 1;
    }
    history.tasks.truncate(50);
    history
}

pub fn save_history(history: &HistoryFile) -> Result<(), String> {
    let root = ensure_data_dirs()?;
    write_json_atomic(&root.join("history.json"), history)
}

pub fn record_task(task: crate::models::HistoryTask) -> Result<(), String> {
    let mut history = load_history();
    history.tasks.insert(0, task);
    history.tasks.truncate(50);
    save_history(&history)
}

/// 批量重试后更新同一条历史，而不是新增重复记录。
pub fn upsert_task(task: crate::models::HistoryTask) -> Result<(), String> {
    let mut history = load_history();
    history.tasks.retain(|current| current.id != task.id);
    history.tasks.insert(0, task);
    history.tasks.truncate(50);
    save_history(&history)
}

pub fn clear_history() -> Result<(), String> {
    save_history(&HistoryFile {
        version: 1,
        tasks: vec![],
    })
}
