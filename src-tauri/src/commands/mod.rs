use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::Instant;

use tauri::{AppHandle, Emitter, Manager};

use crate::engine;
use crate::models::{
    AppError, ConversionResult, EngineInfo, FileInfo, HistoryTask, ProgressEvent, Settings,
};
use crate::security::{canonical_input, prepare_output_dir};
use crate::storage;

#[derive(Default)]
pub struct AppState {
    pub cancelled: Mutex<HashSet<String>>,
}

#[tauri::command(rename_all = "camelCase")]
pub fn get_settings() -> Settings {
    storage::load_settings()
}

#[tauri::command(rename_all = "camelCase")]
pub fn save_settings(settings: Settings) -> Result<Settings, AppError> {
    storage::save_settings(&settings)
        .map_err(|e| AppError::new("E_SETTINGS_WRITE", e, "检查 ~/.dull_doc 是否可写后重试"))
}

#[tauri::command(rename_all = "camelCase")]
pub fn get_app_data_dir() -> Result<String, AppError> {
    storage::data_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(|e| AppError::new("E_APP_DATA", e, "检查用户主目录权限"))
}

#[tauri::command(rename_all = "camelCase")]
pub fn get_engine_info() -> EngineInfo {
    EngineInfo {
        anydoc_version: "0.2.4".into(),
        ready: true,
    }
}

#[tauri::command(rename_all = "camelCase")]
pub fn inspect_file(path: String) -> Result<FileInfo, AppError> {
    let input = canonical_input(&path)?;
    let bytes = fs::read(&input).map_err(|e| {
        AppError::new(
            "E_IO",
            format!("读取文件失败: {e}"),
            "确认文件仍存在并具有读取权限",
        )
    })?;
    let format = match engine::detect_format(&bytes, &input) {
        Some(format) => engine::format_label(format).to_string(),
        None => {
            return Err(AppError::new(
                "E_UNSUPPORTED_FORMAT",
                "无法识别该文件的格式",
                "请选择 docx、xlsx、pptx、rtf、epub、csv 或文本型 PDF",
            ))
        }
    };
    let name = input
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "未命名文件".into());
    Ok(FileInfo {
        name,
        path: input.to_string_lossy().into_owned(),
        size_bytes: bytes.len() as u64,
        direction: format!("{format} → Markdown"),
        format,
    })
}

#[tauri::command(rename_all = "camelCase")]
pub async fn convert_file(
    app: AppHandle,
    input_path: String,
    output_dir: Option<String>,
    task_id: String,
) -> Result<ConversionResult, AppError> {
    let started = Instant::now();
    let input = canonical_input(&input_path)?;
    let input_name = input
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "未命名文件".into());
    let settings = storage::load_settings();
    let target_dir = output_dir.unwrap_or(settings.output_dir);
    let output_root = prepare_output_dir(&target_dir)?;

    emit_progress(&app, &task_id, "reading", 8);
    if take_cancelled(&app, &task_id) {
        return Err(cancelled_error());
    }
    emit_progress(&app, &task_id, "converting", 24);

    let input_for_task = input.clone();
    let extracted = tauri::async_runtime::spawn_blocking(move || engine::extract(&input_for_task))
        .await
        .map_err(|e| {
            AppError::new(
                "E_ENGINE_UNAVAILABLE",
                format!("转换线程失败: {e}"),
                "重启应用后重试",
            )
        })??;

    if take_cancelled(&app, &task_id) {
        return Err(cancelled_error());
    }
    emit_progress(&app, &task_id, "writing", 82);

    let stats = engine::stats(&extracted.markdown);
    let output_path = write_output(&output_root, &input, &extracted.markdown)?;
    emit_progress(&app, &task_id, "writing", 96);

    let duration_ms = started.elapsed().as_millis();
    let task = HistoryTask {
        id: task_id.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
        input_path: input.to_string_lossy().into_owned(),
        input_name: input_name.clone(),
        direction: format!("{} → Markdown", extracted.format),
        engine: "anydoc".into(),
        status: "completed".into(),
        output_path: Some(output_path.to_string_lossy().into_owned()),
        error_code: None,
        stats: stats.clone(),
        duration_ms,
    };
    let _ = storage::record_task(task);
    let _ = app.emit(
        "conversion-progress",
        ProgressEvent {
            task_id: task_id.clone(),
            stage: "done".into(),
            percent: 100,
        },
    );

    Ok(ConversionResult {
        task_id,
        output_name: output_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        output_path: output_path.to_string_lossy().into_owned(),
        stats,
        duration_ms,
    })
}

#[tauri::command(rename_all = "camelCase")]
pub fn cancel_conversion(app: AppHandle, task_id: String) {
    if let Ok(mut set) = app.state::<AppState>().cancelled.lock() {
        set.insert(task_id);
    }
}

#[tauri::command(rename_all = "camelCase")]
pub fn list_history() -> Vec<HistoryTask> {
    storage::load_history().tasks
}

#[tauri::command(rename_all = "camelCase")]
pub fn clear_history() -> Result<(), AppError> {
    storage::clear_history()
        .map_err(|e| AppError::new("E_HISTORY_WRITE", e, "检查 ~/.dull_doc 是否可写"))
}

#[tauri::command(rename_all = "camelCase")]
pub fn open_path(path: String, reveal: Option<bool>) -> Result<(), AppError> {
    let target = PathBuf::from(&path);
    if !target.exists() {
        return Err(AppError::new(
            "E_NOT_FOUND",
            "路径不存在",
            "确认输出文件仍在原位置",
        ));
    }
    let mut command = Command::new("open");
    if reveal.unwrap_or(false) && target.is_file() {
        command.arg("-R");
    }
    command.arg(&target);
    let status = command.status().map_err(|e| {
        AppError::new(
            "E_OPEN_FAILED",
            format!("打开失败: {e}"),
            "请在 Finder 中手动打开",
        )
    })?;
    if status.success() {
        Ok(())
    } else {
        Err(AppError::new(
            "E_OPEN_FAILED",
            "系统未能打开该路径",
            "请在 Finder 中手动打开",
        ))
    }
}

fn write_output(root: &Path, input: &Path, markdown: &str) -> Result<PathBuf, AppError> {
    let stem = input
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "output".into());
    for index in 0..1000 {
        let name = if index == 0 {
            format!("{stem}.md")
        } else {
            format!("{stem}-{index}.md")
        };
        let candidate = root.join(name);
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(mut file) => {
                if let Err(error) = file.write_all(markdown.as_bytes()) {
                    let _ = fs::remove_file(&candidate);
                    return Err(AppError::new(
                        "E_OUTPUT_PERMISSION",
                        format!("写入输出失败: {error}"),
                        "检查磁盘空间与目录权限后重试",
                    ));
                }
                return Ok(candidate);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(AppError::new(
                    "E_OUTPUT_PERMISSION",
                    format!("无法创建输出文件: {error}"),
                    "更换可写目录后重试",
                ))
            }
        }
    }
    Err(AppError::new(
        "E_OUTPUT_PERMISSION",
        "同名文件过多，无法生成输出",
        "更换输出目录或清理同名文件",
    ))
}

fn take_cancelled(app: &AppHandle, task_id: &str) -> bool {
    app.state::<AppState>()
        .cancelled
        .lock()
        .map(|mut set| set.remove(task_id))
        .unwrap_or(false)
}

fn cancelled_error() -> AppError {
    AppError::new("E_CANCELLED", "转换已取消", "可以重新选择文件或再次开始")
}

fn emit_progress(app: &AppHandle, task_id: &str, stage: &str, percent: u8) {
    let _ = app.emit(
        "conversion-progress",
        ProgressEvent {
            task_id: task_id.to_string(),
            stage: stage.to_string(),
            percent,
        },
    );
}
