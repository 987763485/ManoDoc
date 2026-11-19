use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
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
use crate::logging;
use crate::ocr;
use crate::preflight;
use crate::presets::{self, Preset};
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
    let saved = storage::save_settings(&settings)
        .map_err(|e| AppError::new("E_SETTINGS_WRITE", e, "检查 ~/.manodoc 是否可写后重试"))?;
    logging::set_level(&saved.log_level);
    logging::cleanup(saved.log_retention_days);
    logging::log(
        "info",
        "settings.save",
        serde_json::json!({ "logLevel": saved.log_level, "imageMode": saved.image_mode, "engine": saved.engine }),
    );
    Ok(saved)
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
    let sidecars = engine::sidecar_status();
    EngineInfo {
        anydoc_version: "0.2.4".into(),
        xberg_version: "1.4.0".into(),
        ready: true,
        ocr_ready: status.ready,
        ocr_model_dir: status.model_dir,
        pandoc_ready: sidecars.pandoc.ready,
        pandoc_version: sidecars.pandoc.version,
        typst_ready: sidecars.typst.ready,
        typst_version: sidecars.typst.version,
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
    let targets = engine::export::targets_for(&kind)
        .into_iter()
        .map(|target| target.id().to_string())
        .collect::<Vec<_>>();
    let default_target = targets.first().cloned().unwrap_or_else(|| "markdown".into());
    let default_label = if default_target == "markdown" {
        "Markdown"
    } else {
        engine::export::targets_for(&kind)
            .first()
            .map(|target| target.label())
            .unwrap_or("Markdown")
    };
    let estimated_engine = match kind {
        engine::InputKind::Image => "xberg",
        engine::InputKind::Markdown | engine::InputKind::Html => "pandoc",
        engine::InputKind::Document(anydoc::Format::Pdf) => "auto（anydoc → xberg）",
        engine::InputKind::Document(_) => "anydoc",
    };
    Ok(FileInfo {
        name,
        path: input.to_string_lossy().into_owned(),
        size_bytes: bytes.len() as u64,
        direction: format!("{format} → {default_label}"),
        format,
        is_image: matches!(kind, engine::InputKind::Image),
        targets,
        default_target,
        page_count: if matches!(kind, engine::InputKind::Document(anydoc::Format::Pdf)) {
            preflight::page_count(&input)
        } else {
            None
        },
        estimated_engine: estimated_engine.into(),
    })
}

#[tauri::command(rename_all = "camelCase")]
pub async fn convert_file(
    app: AppHandle,
    input_path: String,
    output_dir: Option<String>,
    task_id: String,
    engine_choice: Option<String>,
    target_format: Option<String>,
    no_text_policy: Option<String>,
    force_ocr: Option<bool>,
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
    let force_ocr = force_ocr.unwrap_or(false);

    let bytes = fs::read(&input).map_err(|error| {
        AppError::new(
            "E_IO",
            format!("读取文件失败: {error}"),
            "确认文件仍存在并具有读取权限",
        )
    })?;
    let kind = engine::detect_input(&bytes, &input).ok_or_else(|| {
        AppError::new(
            "E_UNSUPPORTED_FORMAT",
            "无法识别该文件的格式",
            "请选择 docx、xlsx、pptx、rtf、epub、csv、PDF、Markdown、HTML、PNG 或 JPEG",
        )
    })?;
    let supported = engine::export::targets_for(&kind);
    let target = target_format
        .as_deref()
        .and_then(engine::TargetFormat::parse)
        .unwrap_or(supported[0]);
    if !supported.contains(&target) {
        return Err(AppError::new(
            "E_UNSUPPORTED_DIRECTION",
            format!("不支持 {} → {}", engine::input_format_label(&kind), target.label()),
            "在转换页选择可用目标格式",
        ));
    }
    let source_label = engine::input_format_label(&kind).to_string();
    let direction = format!("{source_label} → {}", target.label());

    // 单文件「跳过并继续」：把无文本页从临时副本中删除，转换其余页面。
    // 预检与 lopdf 都是阻塞操作，且内部会对 xberg 调用 block_on；必须放到
    // spawn_blocking，不能直接占用 async 命令所在的 tokio 线程。
    let policy = no_text_policy.unwrap_or_else(|| "ask".into());
    let mut effective_input = input.clone();
    let mut skipped_pages: Vec<u32> = Vec::new();
    let mut stripped_temp: Option<PathBuf> = None;
    if policy == "skip" && matches!(kind, engine::InputKind::Document(anydoc::Format::Pdf)) {
        let source = input.clone();
        let prepared = tauri::async_runtime::spawn_blocking(move || {
            let check = preflight::inspect(&source).ok()?;
            if check.pages_without_text.is_empty()
                || check.pages_without_text.len() >= check.page_count as usize
            {
                return None;
            }
            let temp = storage::data_dir()
                .ok()?
                .join("tmp")
                .join(format!("skip-{}.pdf", uuid::Uuid::new_v4()));
            preflight::strip_pages(&source, &check.pages_without_text, &temp).ok()?;
            Some((temp, check.pages_without_text))
        })
        .await
        .ok()
        .flatten();
        if let Some((temp, pages)) = prepared {
            skipped_pages = pages;
            effective_input = temp.clone();
            stripped_temp = Some(temp);
        }
    }

    emit_progress(&app, &task_id, "reading", 8);
    if take_cancelled(&app, &task_id) {
        return Err(cancelled_error());
    }
    emit_progress(&app, &task_id, "converting", 24);

    let input_for_task = effective_input.clone();
    let output_root_for_task = output_root.clone();
    // 跳过无文本页时转换的是临时副本，输出名与资产目录仍以原始文件名为准。
    let output_name = input
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned());
    let progress_app = app.clone();
    let progress_task = task_id.clone();
    let exported = tauri::async_runtime::spawn_blocking(move || {
        engine::export::convert(
            &input_for_task,
            &kind,
            target,
            choice,
            force_ocr,
            &output_root_for_task,
            output_name.as_deref(),
            |stage| {
                let (label, percent) = match stage {
                    "ocr" => ("ocr", 46),
                    "rendering" => ("rendering", 64),
                    "writing" => ("writing", 88),
                    _ => ("converting", 24),
                };
                emit_progress(&progress_app, &progress_task, label, percent);
            },
        )
    })
    .await
    .map_err(|e| {
        AppError::new(
            "E_ENGINE_UNAVAILABLE",
            format!("转换线程失败: {e}"),
            "重启应用后重试",
        )
    })??;
    if let Some(temp) = stripped_temp {
        let _ = fs::remove_file(temp);
    }

    if take_cancelled(&app, &task_id) {
        let _ = fs::remove_file(&exported.output_path);
        return Err(cancelled_error());
    }
    emit_progress(&app, &task_id, "writing", 96);

    let mut warnings = exported.warnings.clone();
    if !skipped_pages.is_empty() {
        warnings.push(format!(
            "已跳过无文本页 {:?}；这些页面未写入输出",
            skipped_pages
        ));
    }
    let stats = engine::stats(&exported.stats_markdown);
    let output_path = exported.output_path.clone();
    let duration_ms = started.elapsed().as_millis();
    // 历史参数保存目标格式、引擎选择、实际引擎与回退原因。
    let params = serde_json::json!({
        "target": exported.target.id(),
        "engineChoice": choice.as_str(),
        "engineUsed": exported.engine,
        "sourceEngine": exported.source_engine,
        "fallbackReason": exported.fallback_reason,
        "ocrPages": exported.ocr_pages,
        "ocrEngine": exported.ocr_engine,
        "forceOcr": force_ocr
    });
    let task = HistoryTask {
        id: task_id.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
        input_path: input.to_string_lossy().into_owned(),
        input_name: input_name.clone(),
        direction: direction.clone(),
        engine: exported.engine.clone(),
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
    logging::log(
        "info",
        "task.completed",
        serde_json::json!({
            "taskId": task_id,
            "engine": exported.engine,
            "target": exported.target.id(),
            "durationMs": duration_ms,
            "chars": stats.chars,
        }),
    );
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
        target: exported.target.label().into(),
        direction,
        engine_used: exported.engine,
        engine_choice: choice.as_str().into(),
        fallback_reason: exported.fallback_reason,
        ocr_pages: exported.ocr_pages,
        ocr_engine: exported.ocr_engine,
        warnings,
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
        .map_err(|e| AppError::new("E_HISTORY_WRITE", e, "检查 ~/.manodoc 是否可写"))
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

#[tauri::command(rename_all = "camelCase")]
pub async fn preflight_pdf(path: String) -> Result<preflight::PreflightResult, AppError> {
    tauri::async_runtime::spawn_blocking(move || {
        let input = canonical_input(&path)?;
        preflight::inspect(&input)
    })
    .await
    .map_err(|error| {
        AppError::new(
            "E_PREFLIGHT",
            format!("预检线程失败: {error}"),
            "重试或复制诊断信息",
        )
    })?
}

#[tauri::command(rename_all = "camelCase")]
pub fn get_logs_dir() -> Result<String, AppError> {
    logging::logs_dir()
        .map(|path| path.to_string_lossy().into_owned())
        .map_err(|error| AppError::new("E_LOG", error, "检查 ~/.manodoc 是否可写"))
}

#[tauri::command(rename_all = "camelCase")]
pub fn export_diagnostics(target: String) -> Result<String, AppError> {
    let path = storage::expand_tilde(&target);
    logging::export_diagnostics(&path)
}

#[tauri::command(rename_all = "camelCase")]
pub fn list_presets() -> Vec<Preset> {
    presets::list()
}

#[tauri::command(rename_all = "camelCase")]
pub fn save_preset(preset: Preset) -> Result<Vec<Preset>, AppError> {
    presets::upsert(preset)
}

#[tauri::command(rename_all = "camelCase")]
pub fn delete_preset(id: String) -> Result<Vec<Preset>, AppError> {
    presets::delete(&id)
}

#[tauri::command(rename_all = "camelCase")]
pub fn duplicate_preset(id: String) -> Result<Vec<Preset>, AppError> {
    presets::duplicate(&id)
}

#[tauri::command(rename_all = "camelCase")]
pub fn export_presets(target: String) -> Result<String, AppError> {
    presets::export_to(&storage::expand_tilde(&target))
}

#[tauri::command(rename_all = "camelCase")]
pub fn import_presets(path: String) -> Result<Vec<Preset>, AppError> {
    let source = canonical_input(&path)?;
    presets::import_from(&source)
}

#[tauri::command(rename_all = "camelCase")]
pub fn path_kind(path: String) -> Result<String, AppError> {
    let meta = std::fs::metadata(&path).map_err(|error| {
        AppError::new("E_IO", format!("无法访问路径: {error}"), "确认路径仍存在")
    })?;
    Ok(if meta.is_dir() { "directory" } else { "file" }.into())
}
