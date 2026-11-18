use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use tauri::{AppHandle, Emitter, Manager};

use crate::batch::{self, BatchRequest, BatchResult, BatchScan};
use crate::engine::{self, routing::EngineChoice};
use crate::models::{
    AppError, ConversionResult, EngineInfo, FileInfo, HistoryTask, ProgressEvent, Settings,
};
use crate::ocr;
use crate::security::{canonical_input, prepare_output_dir};
use crate::storage;

#[derive(Default)]
pub struct AppState {
    pub cancelled: Mutex<HashSet<String>>,
    pub batch_cancels: Mutex<HashMap<String, Arc<AtomicBool>>>,
    pub batches: batch::BatchStore,
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
    let status = ocr::status();
    EngineInfo {
        anydoc_version: "0.2.4".into(),
        xberg_version: "1.4.0".into(),
        ready: true,
        ocr_ready: status.ready,
        ocr_model_dir: status.model_dir,
    }
}

#[tauri::command(rename_all = "camelCase")]
pub fn get_ocr_status() -> ocr::OcrStatus {
    ocr::status()
}

/// 模型下载只在用户点击后发生；失败返回可读错误，不重试、不切换镜像。
#[tauri::command(rename_all = "camelCase")]
pub async fn install_ocr_models(app: AppHandle) -> Result<ocr::OcrStatus, AppError> {
    tauri::async_runtime::spawn_blocking(move || ocr::install(&app))
        .await
        .map_err(|error| {
            AppError::new(
                "E_OCR_MODEL_DOWNLOAD",
                format!("模型安装线程失败: {error}"),
                "重启应用后重试",
            )
        })?
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
    let kind = engine::detect_input(&bytes, &input).ok_or_else(|| {
        AppError::new(
            "E_UNSUPPORTED_FORMAT",
            "无法识别该文件的格式",
            "请选择 docx、xlsx、pptx、rtf、epub、csv、文本型 PDF、PNG 或 JPEG",
        )
    })?;
    let name = input
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "未命名文件".into());
    let format = engine::input_format_label(&kind).to_string();
    Ok(FileInfo {
        name,
        path: input.to_string_lossy().into_owned(),
        size_bytes: bytes.len() as u64,
        direction: format!("{format} → Markdown"),
        format,
        is_image: matches!(kind, engine::InputKind::Image),
    })
}

#[tauri::command(rename_all = "camelCase")]
pub async fn convert_file(
    app: AppHandle,
    input_path: String,
    output_dir: Option<String>,
    task_id: String,
    engine_choice: Option<String>,
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
    let choice = EngineChoice::parse(engine_choice.as_deref().unwrap_or("auto"));

    emit_progress(&app, &task_id, "reading", 8);
    if take_cancelled(&app, &task_id) {
        return Err(cancelled_error());
    }
    emit_progress(&app, &task_id, "converting", 24);

    let input_for_task = input.clone();
    let progress_app = app.clone();
    let progress_task = task_id.clone();
    let extracted = tauri::async_runtime::spawn_blocking(move || {
        // 路由在需要切到 xberg 时回调进度，让界面从「转换」进入「OCR 识别」。
        engine::extract(&input_for_task, choice, |stage| {
            let (label, percent) = match stage {
                "ocr" => ("ocr", 46),
                "converting" => ("converting", 24),
                _ => ("converting", 30),
            };
            emit_progress(&progress_app, &progress_task, label, percent);
        })
    })
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
    // 历史参数保存引擎选择、实际引擎与回退原因，便于之后解释同一文件为何走 OCR。
    let params = serde_json::json!({
        "engineChoice": extracted.engine_choice,
        "engineUsed": extracted.engine,
        "fallbackReason": extracted.fallback_reason,
        "ocrPages": extracted.ocr_pages,
        "ocrEngine": extracted.ocr_engine,
        "method": extracted.method,
        "pages": extracted.pages
    });
    let task = HistoryTask {
        id: task_id.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
        input_path: input.to_string_lossy().into_owned(),
        input_name: input_name.clone(),
        direction: format!("{} → Markdown", extracted.format),
        engine: extracted.engine.clone(),
        status: "completed".into(),
        output_path: Some(output_path.to_string_lossy().into_owned()),
        error_code: None,
        stats: stats.clone(),
        duration_ms,
        kind: "single".into(),
        input_count: 1,
        input_paths: vec![input.to_string_lossy().into_owned()],
        output_dir: None,
        counts: None,
        params: Some(params),
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
        engine_used: extracted.engine,
        engine_choice: extracted.engine_choice,
        fallback_reason: extracted.fallback_reason,
        ocr_pages: extracted.ocr_pages,
        ocr_engine: extracted.ocr_engine,
        warnings: extracted.warnings,
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

#[tauri::command(rename_all = "camelCase")]
pub fn scan_batch_inputs(paths: Vec<String>, output_dir: String) -> Result<BatchScan, AppError> {
    batch::scan_inputs(&paths, &output_dir)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn start_batch(app: AppHandle, request: BatchRequest) -> Result<BatchResult, AppError> {
    tauri::async_runtime::spawn_blocking(move || batch::run(app, request))
        .await
        .map_err(|error| {
            AppError::new(
                "E_ENGINE_UNAVAILABLE",
                format!("批量线程失败: {error}"),
                "重启应用后重试",
            )
        })?
}

#[tauri::command(rename_all = "camelCase")]
pub fn cancel_batch(app: AppHandle, batch_id: String) {
    batch::cancel(&app, &batch_id);
}

#[tauri::command(rename_all = "camelCase")]
pub async fn retry_batch_file(
    app: AppHandle,
    batch_id: String,
    file_id: String,
) -> Result<BatchResult, AppError> {
    tauri::async_runtime::spawn_blocking(move || batch::retry_file(app, &batch_id, &file_id))
        .await
        .map_err(|error| {
            AppError::new(
                "E_ENGINE_UNAVAILABLE",
                format!("重试线程失败: {error}"),
                "重启应用后重试",
            )
        })?
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
