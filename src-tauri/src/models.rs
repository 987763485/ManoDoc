use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 前端与 Rust 之间的统一错误结构。code 用于程序分支，message/nextStep 面向用户。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: String,
    pub message: String,
    pub detail: Option<Value>,
    pub next_step: String,
}

impl AppError {
    pub fn new(code: &str, message: impl Into<String>, next_step: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
            detail: None,
            next_step: next_step.into(),
        }
    }

    pub fn with_detail(mut self, detail: Value) -> Self {
        self.detail = Some(detail);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub version: u32,
    pub language: String,
    pub theme: String,
    pub output_dir: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: 1,
            language: "zh".into(),
            theme: "system".into(),
            output_dir: "~/Documents/DullDoc Output".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TaskStats {
    pub chars: usize,
    pub headings: usize,
    pub table_rows: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BatchCounts {
    pub success: usize,
    pub skipped: usize,
    pub failed: usize,
    pub warnings: usize,
    pub chunks: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryTask {
    pub id: String,
    pub created_at: String,
    pub input_path: String,
    pub input_name: String,
    pub direction: String,
    pub engine: String,
    pub status: String,
    pub output_path: Option<String>,
    pub error_code: Option<String>,
    pub stats: TaskStats,
    pub duration_ms: u128,
    // 批量记录与单文件记录共用 history.json；旧记录缺省时按单文件处理。
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub input_count: usize,
    #[serde(default)]
    pub input_paths: Vec<String>,
    #[serde(default)]
    pub output_dir: Option<String>,
    #[serde(default)]
    pub counts: Option<BatchCounts>,
    #[serde(default)]
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HistoryFile {
    pub version: u32,
    #[serde(default)]
    pub tasks: Vec<HistoryTask>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub format: String,
    pub direction: String,
    pub is_image: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversionResult {
    pub task_id: String,
    pub output_path: String,
    pub output_name: String,
    pub stats: TaskStats,
    pub duration_ms: u128,
    pub engine_used: String,
    pub engine_choice: String,
    pub fallback_reason: Option<String>,
    pub ocr_pages: Vec<u32>,
    pub ocr_engine: Option<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEvent {
    pub task_id: String,
    pub stage: String,
    pub percent: u8,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineInfo {
    pub anydoc_version: String,
    pub xberg_version: String,
    pub ready: bool,
    pub ocr_ready: bool,
    pub ocr_model_dir: String,
}
