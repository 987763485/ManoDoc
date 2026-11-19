//! 用户预设：内置预设只读，用户预设写入 `~/.manodoc/presets.json`。

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::batch::BatchConfig;
use crate::models::AppError;
use crate::storage;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub builtin: bool,
    pub config: BatchConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct PresetFile {
    #[serde(default)]
    version: u32,
    #[serde(default)]
    presets: Vec<Preset>,
}

pub fn builtin() -> Vec<Preset> {
    let standard = BatchConfig::default();
    let table = BatchConfig {
        preset: "table".into(),
        grade_tables: true,
        chunk_size: 800,
        overlap: 50,
        ..BatchConfig::default()
    };
    let corpus = BatchConfig {
        preset: "corpus".into(),
        chunk: true,
        metadata: true,
        corpus: true,
        chunk_size: 1200,
        overlap: 100,
        ..BatchConfig::default()
    };
    vec![
        Preset { id: "builtin-standard".into(), name: "标准文档".into(), builtin: true, config: standard },
        Preset { id: "builtin-table".into(), name: "表格清单".into(), builtin: true, config: table },
        Preset { id: "builtin-corpus".into(), name: "知识库".into(), builtin: true, config: corpus },
    ]
}

fn path() -> Result<std::path::PathBuf, String> {
    Ok(storage::data_dir()?.join("presets.json"))
}

fn read_user() -> Vec<Preset> {
    let Ok(file) = path() else { return vec![] };
    let Ok(bytes) = fs::read(file) else { return vec![] };
    let mut parsed = serde_json::from_slice::<PresetFile>(&bytes).unwrap_or_default();
    parsed.version = 1;
    parsed.presets.retain(|preset| !preset.builtin);
    parsed.presets
}

fn write_user(presets: &[Preset]) -> Result<(), String> {
    let file = path()?;
    let body = serde_json::to_vec_pretty(&PresetFile { version: 1, presets: presets.to_vec() })
        .map_err(|e| format!("序列化预设失败: {e}"))?;
    let temp = file.with_extension(format!("tmp-{}", uuid::Uuid::new_v4()));
    fs::write(&temp, body).map_err(|e| format!("写预设失败: {e}"))?;
    fs::rename(&temp, &file).map_err(|e| format!("提交预设失败: {e}"))
}

pub fn list() -> Vec<Preset> {
    let mut all = builtin();
    all.extend(read_user());
    all
}

fn validate(preset: &Preset) -> Result<(), AppError> {
    if preset.id.trim().is_empty() || preset.name.trim().is_empty() {
        return Err(AppError::new("E_PRESET", "预设缺少 id 或名称", "检查预设内容后重试"));
    }
    if preset.builtin {
        return Err(AppError::new("E_PRESET", "内置预设不可修改", "复制为自定义预设后再编辑"));
    }
    Ok(())
}

pub fn upsert(mut preset: Preset) -> Result<Vec<Preset>, AppError> {
    validate(&preset)?;
    preset.config = preset.config.normalized();
    let mut user = read_user();
    if let Some(existing) = user.iter_mut().find(|current| current.id == preset.id) {
        *existing = preset;
    } else {
        user.push(preset);
    }
    write_user(&user).map_err(|e| AppError::new("E_PRESET", e, "检查 ~/.manodoc 是否可写"))?;
    Ok(list())
}

pub fn delete(id: &str) -> Result<Vec<Preset>, AppError> {
    let mut user = read_user();
    user.retain(|preset| preset.id != id);
    write_user(&user).map_err(|e| AppError::new("E_PRESET", e, "检查 ~/.manodoc 是否可写"))?;
    Ok(list())
}

pub fn duplicate(id: &str) -> Result<Vec<Preset>, AppError> {
    let source = list()
        .into_iter()
        .find(|preset| preset.id == id)
        .ok_or_else(|| AppError::new("E_PRESET", "找不到要复制的预设", "刷新预设列表后重试"))?;
    let copy = Preset {
        id: format!("user-{}", uuid::Uuid::new_v4()),
        name: format!("{} 副本", source.name),
        builtin: false,
        config: source.config,
    };
    upsert(copy)
}

pub fn export_to(target: &Path) -> Result<String, AppError> {
    let body = serde_json::to_vec_pretty(&PresetFile { version: 1, presets: list() })
        .map_err(|e| AppError::new("E_PRESET", format!("序列化预设失败: {e}"), "重试"))?;
    fs::write(target, body).map_err(|e| {
        AppError::new("E_PRESET", format!("写入预设文件失败: {e}"), "更换目录后重试")
    })?;
    Ok(target.to_string_lossy().into_owned())
}

pub fn import_from(source: &Path) -> Result<Vec<Preset>, AppError> {
    let bytes = fs::read(source).map_err(|e| {
        AppError::new("E_PRESET", format!("读取预设文件失败: {e}"), "确认文件可读")
    })?;
    let parsed = serde_json::from_slice::<PresetFile>(&bytes).map_err(|e| {
        AppError::new("E_PRESET", format!("预设文件格式无效: {e}"), "选择 ManoDoc 导出的预设 JSON")
    })?;
    let mut user = read_user();
    for mut preset in parsed.presets {
        if preset.builtin {
            preset.id = format!("user-{}", uuid::Uuid::new_v4());
            preset.name = format!("{}（导入）", preset.name);
            preset.builtin = false;
        }
        preset.config = preset.config.normalized();
        if let Some(existing) = user.iter_mut().find(|current| current.id == preset.id) {
            *existing = preset;
        } else {
            user.push(preset);
        }
    }
    write_user(&user).map_err(|e| AppError::new("E_PRESET", e, "检查 ~/.manodoc 是否可写"))?;
    Ok(list())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_are_stable() {
        let presets = builtin();
        assert_eq!(presets.len(), 3);
        assert!(presets.iter().all(|preset| preset.builtin));
    }
}
