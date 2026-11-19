//! 本地 JSONL 日志与脱敏诊断包。
//!
//! 日志只记录任务元数据（任务 ID、引擎、耗时、错误码），不写文件正文或密钥；
//! 日志级别与保留天数由设置控制，诊断包同样只包含这些元数据。

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};

use chrono::{Local, NaiveDate};
use serde_json::{json, Value};

use crate::models::{AppError, Settings};
use crate::storage;

static LEVEL: AtomicU8 = AtomicU8::new(2);

fn level_value(name: &str) -> u8 {
    match name {
        "error" => 0,
        "warn" => 1,
        "info" => 2,
        "debug" => 3,
        _ => 2,
    }
}

pub fn set_level(name: &str) {
    LEVEL.store(level_value(name), Ordering::Relaxed);
}

pub fn init(settings: &Settings) {
    set_level(&settings.log_level);
    cleanup(settings.log_retention_days);
    log(
        "info",
        "app.start",
        json!({ "version": env!("CARGO_PKG_VERSION"), "theme": settings.theme, "language": settings.language }),
    );
}

pub fn logs_dir() -> Result<PathBuf, String> {
    let root = storage::ensure_data_dirs()?;
    let dir = root.join("logs");
    fs::create_dir_all(&dir).map_err(|e| format!("创建日志目录失败: {e}"))?;
    Ok(dir)
}

/// 事件字段必须是可以 JSON 化的元数据；调用方不得传入文件内容。
pub fn log(level: &str, event: &str, fields: Value) {
    if level_value(level) > LEVEL.load(Ordering::Relaxed) {
        return;
    }
    let Ok(dir) = logs_dir() else { return };
    let now = Local::now();
    let file = dir.join(format!("{}.jsonl", now.format("%Y-%m-%d")));
    let mut line = json!({
        "ts": now.to_rfc3339(),
        "level": level,
        "event": event,
        "version": env!("CARGO_PKG_VERSION"),
    });
    if let (Some(target), Some(extra)) = (line.as_object_mut(), fields.as_object()) {
        for (key, value) in extra {
            target.insert(key.clone(), value.clone());
        }
    }
    if let Ok(mut handle) = OpenOptions::new().create(true).append(true).open(file) {
        let _ = writeln!(handle, "{line}");
    }
}

pub fn cleanup(retention_days: u32) {
    let Ok(dir) = logs_dir() else { return };
    let keep_after = Local::now().date_naive() - chrono::Duration::days(retention_days.max(1) as i64);
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("jsonl") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else { continue };
        if let Ok(date) = NaiveDate::parse_from_str(stem, "%Y-%m-%d") {
            if date < keep_after {
                let _ = fs::remove_file(path);
            }
        }
    }
}

fn redact_path(value: &str) -> String {
    let home = dirs::home_dir()
        .map(|home| home.to_string_lossy().into_owned())
        .unwrap_or_default();
    if !home.is_empty() && value.starts_with(&home) {
        value.replacen(&home, "~", 1)
    } else {
        value.to_string()
    }
}

fn diagnostics_json() -> Value {
    let settings = storage::load_settings();
    let history = storage::load_history();
    let recent = history
        .tasks
        .iter()
        .take(20)
        .map(|task| {
            json!({
                "id": task.id,
                "kind": if task.kind.is_empty() { "single" } else { &task.kind },
                "direction": task.direction,
                "engine": task.engine,
                "status": task.status,
                "errorCode": task.error_code,
                "durationMs": task.duration_ms,
                "input": redact_path(&task.input_path),
                "output": task.output_path.as_deref().map(redact_path),
            })
        })
        .collect::<Vec<_>>();
    json!({
        "schema": "manodoc.diagnostics.v1",
        "version": env!("CARGO_PKG_VERSION"),
        "platform": std::env::consts::OS,
        "settings": {
            "language": settings.language,
            "theme": settings.theme,
            "outputDir": redact_path(&settings.output_dir),
            "logLevel": settings.log_level,
            "logRetentionDays": settings.log_retention_days,
            "imageMode": settings.image_mode,
            "overwrite": settings.overwrite,
            "historyLimit": settings.history_limit,
            "engine": settings.engine,
        },
        "recentTasks": recent,
    })
}

/// 诊断包：diagnostics.json + 最近日志；不包含源文件、输出内容或密钥。
pub fn export_diagnostics(target: &Path) -> Result<String, AppError> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            AppError::new("E_DIAGNOSTICS", format!("无法创建诊断包目录: {e}"), "更换输出目录后重试")
        })?;
    }
    let file = File::create(target).map_err(|e| {
        AppError::new("E_DIAGNOSTICS", format!("无法创建诊断包: {e}"), "更换输出目录后重试")
    })?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let body = serde_json::to_vec_pretty(&diagnostics_json()).map_err(|e| {
        AppError::new("E_DIAGNOSTICS", format!("序列化诊断信息失败: {e}"), "重试")
    })?;
    zip.start_file("diagnostics.json", options)
        .map_err(|e| AppError::new("E_DIAGNOSTICS", format!("写入诊断包失败: {e}"), "重试"))?;
    zip.write_all(&body)
        .map_err(|e| AppError::new("E_DIAGNOSTICS", format!("写入诊断包失败: {e}"), "重试"))?;

    if let Ok(dir) = logs_dir() {
        let mut files = fs::read_dir(&dir)
            .map(|entries| {
                entries
                    .flatten()
                    .map(|entry| entry.path())
                    .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("jsonl"))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        files.sort();
        for path in files.into_iter().rev().take(5) {
            if let (Some(name), Ok(bytes)) = (path.file_name().and_then(|n| n.to_str()), fs::read(&path)) {
                if bytes.len() <= 2 * 1024 * 1024 {
                    let entry = format!("logs/{name}");
                    let _ = zip
                        .start_file(entry, options)
                        .and_then(|_| zip.write_all(&bytes).map_err(zip::result::ZipError::Io));
                }
            }
        }
    }

    zip.finish().map_err(|e| {
        AppError::new("E_DIAGNOSTICS", format!("完成诊断包失败: {e}"), "重试")
    })?;
    let _ = log("info", "diagnostics.export", json!({ "target": redact_path(&target.to_string_lossy()) }));
    Ok(target.to_string_lossy().into_owned())
}
