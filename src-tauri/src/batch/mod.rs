use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager};

use crate::chunk::{self, Chunk, ChunkContext};
use crate::clean::{self, CleanOptions};
use crate::corpus::{self, CorpusFile, CorpusMeta};
use crate::engine;
use crate::models::{AppError, BatchCounts, HistoryTask, TaskStats};
use crate::storage;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchConfig {
    pub preset: String,
    pub engine: String,
    pub concurrency: usize,
    pub clean_headers: bool,
    pub clean_cjk: bool,
    pub grade_tables: bool,
    pub chunk: bool,
    pub metadata: bool,
    pub corpus: bool,
    pub chunk_size: usize,
    pub overlap: usize,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            preset: "standard".into(),
            engine: "auto".into(),
            concurrency: default_concurrency(),
            clean_headers: true,
            clean_cjk: true,
            grade_tables: true,
            chunk: true,
            metadata: true,
            corpus: true,
            chunk_size: 1200,
            overlap: 50,
        }
    }
}

impl BatchConfig {
    fn normalized(mut self) -> Self {
        self.concurrency = self.concurrency.clamp(1, 8);
        self.chunk_size = self.chunk_size.clamp(500, 4000);
        self.overlap = self.overlap.min(300);
        self
    }

    fn clean_options(&self) -> CleanOptions {
        CleanOptions {
            headers: self.clean_headers,
            cjk: self.clean_cjk,
            tables: self.grade_tables,
        }
    }

    fn params(&self) -> Value {
        json!({
            "preset": self.preset,
            "engine": self.engine,
            "concurrency": self.concurrency,
            "cleanHeaders": self.clean_headers,
            "cleanCjk": self.clean_cjk,
            "gradeTables": self.grade_tables,
            "chunk": self.chunk,
            "metadata": self.metadata,
            "corpus": self.corpus,
            "chunkSize": self.chunk_size,
            "overlap": self.overlap
        })
    }
}

pub fn default_concurrency() -> usize {
    let cpus = std::thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(2);
    (cpus / 2).clamp(1, 4).min(8)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchInput {
    pub id: String,
    pub path: String,
    pub name: String,
    pub relative: String,
    pub format: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchScan {
    pub files: Vec<BatchInput>,
    pub skipped: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchRequest {
    pub batch_id: String,
    pub name: Option<String>,
    pub files: Vec<BatchInput>,
    pub config: BatchConfig,
    pub output_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchFileResult {
    pub id: String,
    pub path: String,
    pub name: String,
    pub relative: String,
    pub format: String,
    pub hash: String,
    pub status: String,
    pub engine: String,
    #[serde(default)]
    pub engine_choice: String,
    #[serde(default)]
    pub fallback_reason: Option<String>,
    #[serde(default)]
    pub ocr_pages: Vec<u32>,
    #[serde(default)]
    pub ocr_engine: Option<String>,
    pub table_grade: Option<String>,
    pub warnings: Vec<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub next_step: Option<String>,
    pub chars: usize,
    pub headings: usize,
    pub table_rows: usize,
    pub chunks: usize,
    pub duration_ms: u64,
    pub output_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchResult {
    pub batch_id: String,
    pub name: String,
    pub status: String,
    pub package_dir: String,
    pub files: Vec<BatchFileResult>,
    pub counts: BatchCounts,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchProgress {
    pub batch_id: String,
    pub file_id: Option<String>,
    pub status: String,
    pub completed: usize,
    pub total: usize,
    pub percent: u8,
}

#[derive(Debug, Clone)]
pub struct StoredBatch {
    pub result: BatchResult,
    pub config: BatchConfig,
    pub inputs: Vec<BatchInput>,
    pub package_dir: PathBuf,
}

#[derive(Default)]
pub struct BatchStore {
    pub sessions: Mutex<HashMap<String, StoredBatch>>,
}

pub fn scan_inputs(paths: &[String], output_dir: &str) -> Result<BatchScan, AppError> {
    let output = storage::expand_tilde(output_dir);
    let output = output.canonicalize().unwrap_or(output);
    let mut seen = HashSet::new();
    let mut files = Vec::new();
    let mut skipped = Vec::new();

    for raw in paths {
        let path = Path::new(raw).canonicalize().map_err(|error| {
            AppError::new(
                "E_IO",
                format!("无法访问输入路径: {error}"),
                "确认文件或文件夹仍然存在",
            )
        })?;
        if path.starts_with(&output) {
            skipped.push(format!("跳过输出目录自身: {}", path.display()));
            continue;
        }
        if path.is_file() {
            let root = path.parent().unwrap_or(Path::new("/")).to_path_buf();
            add_file(&path, &root, &mut seen, &mut files, &mut skipped)?;
        } else if path.is_dir() {
            walk_dir(&path, &path, &output, &mut seen, &mut files, &mut skipped)?;
        } else {
            skipped.push(format!("跳过非普通文件: {}", path.display()));
        }
    }

    files.sort_by(|left, right| left.relative.cmp(&right.relative));
    Ok(BatchScan { files, skipped })
}

fn walk_dir(
    root: &Path,
    dir: &Path,
    output: &Path,
    seen: &mut HashSet<String>,
    files: &mut Vec<BatchInput>,
    skipped: &mut Vec<String>,
) -> Result<(), AppError> {
    let entries = fs::read_dir(dir).map_err(|error| {
        AppError::new(
            "E_IO",
            format!("无法读取目录 {}: {error}", dir.display()),
            "检查文件夹权限",
        )
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| {
            AppError::new("E_IO", format!("读取目录项失败: {error}"), "重试扫描")
        })?;
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().into_owned();
        if file_name.starts_with('.') {
            continue;
        }
        let metadata = fs::symlink_metadata(&path).map_err(|error| {
            AppError::new(
                "E_IO",
                format!("无法读取 {}: {error}", path.display()),
                "确认路径权限",
            )
        })?;
        if metadata.file_type().is_symlink() {
            skipped.push(format!("跳过符号链接: {}", path.display()));
            continue;
        }
        if metadata.is_dir() {
            if path
                .canonicalize()
                .map(|p| p.starts_with(output))
                .unwrap_or(false)
            {
                continue;
            }
            walk_dir(root, &path, output, seen, files, skipped)?;
        } else if metadata.is_file() {
            add_file(&path, root, seen, files, skipped)?;
        }
    }
    Ok(())
}

fn add_file(
    path: &Path,
    root: &Path,
    seen: &mut HashSet<String>,
    files: &mut Vec<BatchInput>,
    skipped: &mut Vec<String>,
) -> Result<(), AppError> {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let key = canonical.to_string_lossy().into_owned();
    if !seen.insert(key.clone()) {
        return Ok(());
    }
    let kind = engine::routing::detect_path(&canonical);
    let Some(kind) = kind else {
        skipped.push(format!("跳过不支持的文件: {}", path.display()));
        return Ok(());
    };
    let relative = canonical
        .strip_prefix(root)
        .unwrap_or(&canonical)
        .to_string_lossy()
        .replace('\\', "/");
    let mut relative = relative;
    let base = relative.clone();
    let mut index = 1;
    while files.iter().any(|file| file.relative == relative) {
        relative = add_suffix(&base, index);
        index += 1;
    }
    let name = canonical
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "未命名文件".into());
    let size_bytes = fs::metadata(&canonical).map(|meta| meta.len()).unwrap_or(0);
    files.push(BatchInput {
        id: short_hash(&key),
        path: canonical.to_string_lossy().into_owned(),
        name,
        relative,
        format: engine::input_format_label(&kind).to_string(),
        size_bytes,
    });
    Ok(())
}

fn add_suffix(relative: &str, index: usize) -> String {
    let path = Path::new(relative);
    let stem = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".into());
    let ext = path
        .extension()
        .map(|ext| format!(".{}", ext.to_string_lossy()))
        .unwrap_or_default();
    if let Some(parent) = path.parent() {
        if parent.as_os_str().is_empty() {
            format!("{stem}-{index}{ext}")
        } else {
            format!("{}/{stem}-{index}{ext}", parent.to_string_lossy())
        }
    } else {
        format!("{stem}-{index}{ext}")
    }
}

pub fn run(app: AppHandle, request: BatchRequest) -> Result<BatchResult, AppError> {
    let config = request.config.clone().normalized();
    let output_root = crate::security::prepare_output_dir(&request.output_dir)?;
    let batch_name = sanitize_name(request.name.unwrap_or_else(|| "corpus".into()));
    let package_dir = unique_package(&output_root, &batch_name);
    let temp = output_root.join(format!(".dulldoc-tmp-{}", uuid::Uuid::new_v4()));
    let mirror_root = temp.join("mirror");
    fs::create_dir_all(&mirror_root).map_err(|error| {
        AppError::new(
            "E_OUTPUT_PERMISSION",
            format!("无法创建临时语料目录: {error}"),
            "检查输出目录权限",
        )
    })?;

    let cancel = Arc::new(AtomicBool::new(false));
    if let Some(state) = app.try_state::<crate::commands::AppState>() {
        if let Ok(mut map) = state.batch_cancels.lock() {
            map.insert(request.batch_id.clone(), cancel.clone());
        }
    }

    let started = Instant::now();
    let total = request.files.len();
    let queue = Arc::new(Mutex::new(VecDeque::from(
        (0..total).collect::<Vec<usize>>(),
    )));
    let results: Arc<Mutex<Vec<Option<BatchFileResult>>>> = Arc::new(Mutex::new(vec![None; total]));
    let chunk_store: Arc<Mutex<HashMap<String, Vec<Chunk>>>> = Arc::new(Mutex::new(HashMap::new()));
    let completed = Arc::new(AtomicUsize::new(0));
    let workers = config.concurrency.min(total.max(1));

    std::thread::scope(|scope| {
        for _ in 0..workers {
            let app = app.clone();
            let queue = queue.clone();
            let results = results.clone();
            let chunk_store = chunk_store.clone();
            let cancel = cancel.clone();
            let completed = completed.clone();
            let files = &request.files;
            let config = &config;
            let mirror_root = &mirror_root;
            let batch_id = request.batch_id.clone();
            scope.spawn(move || loop {
                if cancel.load(Ordering::Relaxed) {
                    break;
                }
                let index = match queue.lock() {
                    Ok(mut queue) => queue.pop_front(),
                    Err(_) => None,
                };
                let Some(index) = index else { break };
                let input = &files[index];
                emit_progress(
                    &app,
                    &batch_id,
                    Some(&input.id),
                    "converting",
                    completed.load(Ordering::Relaxed),
                    total,
                );
                let (result, chunks) = process_file(input, config, mirror_root, &cancel);
                let file_status = result.status.clone();
                if !chunks.is_empty() {
                    if let Ok(mut store) = chunk_store.lock() {
                        store.insert(input.id.clone(), chunks);
                    }
                }
                if let Ok(mut slot) = results.lock() {
                    slot[index] = Some(result);
                }
                let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
                emit_progress(&app, &batch_id, Some(&input.id), &file_status, done, total);
            });
        }
    });

    if let Some(state) = app.try_state::<crate::commands::AppState>() {
        if let Ok(mut map) = state.batch_cancels.lock() {
            map.remove(&request.batch_id);
        }
    }

    let mut file_results: Vec<BatchFileResult> = match results.lock() {
        Ok(results) => request
            .files
            .iter()
            .enumerate()
            .map(|(index, input)| {
                results
                    .get(index)
                    .cloned()
                    .flatten()
                    .unwrap_or_else(|| cancelled_input(input))
            })
            .collect(),
        Err(_) => request.files.iter().map(cancelled_input).collect(),
    };

    let cancelled = cancel.load(Ordering::Relaxed);
    let duration_ms = started.elapsed().as_millis() as u64;
    if cancelled {
        let _ = fs::remove_dir_all(&temp);
        let counts = counts(&file_results);
        let result = BatchResult {
            batch_id: request.batch_id.clone(),
            name: batch_name,
            status: "cancelled".into(),
            package_dir: String::new(),
            files: file_results,
            counts,
            duration_ms,
        };
        emit_done(&app, &result);
        return Ok(result);
    }

    let chunks: Vec<Chunk> = match chunk_store.lock() {
        Ok(store) => request
            .files
            .iter()
            .filter_map(|input| store.get(&input.id).cloned())
            .flatten()
            .collect(),
        Err(_) => Vec::new(),
    };
    let corpus_files: Vec<CorpusFile> = request
        .files
        .iter()
        .zip(file_results.iter())
        .map(|(input, result)| CorpusFile {
            input_path: input.path.clone(),
            name: input.name.clone(),
            relative: mirror_relative(&input.relative),
            format: input.format.clone(),
            hash: result.hash.clone(),
            status: result.status.clone(),
            error_code: result.error_code.clone(),
            chars: result.chars,
            headings: result.headings,
            table_rows: result.table_rows,
            chunks: result.chunks,
            warnings: result.warnings.clone(),
            engine: result.engine.clone(),
            fallback_reason: result.fallback_reason.clone(),
            ocr_pages: result.ocr_pages.clone(),
            ocr_engine: result.ocr_engine.clone(),
        })
        .collect();

    let meta = CorpusMeta {
        batch_id: request.batch_id.clone(),
        batch_name: batch_name.clone(),
        preset: config.preset.clone(),
        engine: config.engine.clone(),
        params: config.params(),
        created_at: chrono::Utc::now().to_rfc3339(),
        duration_ms,
        include_chunks: config.corpus && config.chunk,
        include_metadata: config.metadata,
    };
    corpus::write_package(&temp, &meta, &corpus_files, &chunks).map_err(|error| {
        let _ = fs::remove_dir_all(&temp);
        error
    })?;
    fs::rename(&temp, &package_dir).map_err(|error| {
        let _ = fs::remove_dir_all(&temp);
        AppError::new(
            "E_OUTPUT_PERMISSION",
            format!("提交语料包失败: {error}"),
            "检查输出目录权限或同名目录",
        )
    })?;

    for result in &mut file_results {
        if result.status == "completed" {
            let relative = mirror_relative(&result.relative);
            result.output_path = Some(
                package_dir
                    .join("mirror")
                    .join(relative)
                    .to_string_lossy()
                    .into_owned(),
            );
        }
    }
    let counts = counts(&file_results);
    let status = corpus::status(&corpus_summary(&counts)).to_string();
    let result = BatchResult {
        batch_id: request.batch_id.clone(),
        name: batch_name.clone(),
        status,
        package_dir: package_dir.to_string_lossy().into_owned(),
        files: file_results,
        counts,
        duration_ms,
    };

    record_history(&result, &config, &request.files);
    if let Some(state) = app.try_state::<crate::commands::AppState>() {
        if let Ok(mut sessions) = state.batches.sessions.lock() {
            sessions.insert(
                result.batch_id.clone(),
                StoredBatch {
                    result: result.clone(),
                    config: config.clone(),
                    inputs: request.files.clone(),
                    package_dir: package_dir.clone(),
                },
            );
        }
    }
    emit_done(&app, &result);
    Ok(result)
}

pub fn retry_file(app: AppHandle, batch_id: &str, file_id: &str) -> Result<BatchResult, AppError> {
    let stored = {
        let state = app
            .try_state::<crate::commands::AppState>()
            .ok_or_else(|| AppError::new("E_BATCH_NOT_FOUND", "找不到批次状态", "重新开始批量"))?;
        let sessions = state
            .batches
            .sessions
            .lock()
            .map_err(|_| AppError::new("E_BATCH_NOT_FOUND", "批次状态损坏", "重新开始批量"))?;
        sessions
            .get(batch_id)
            .cloned()
            .ok_or_else(|| AppError::new("E_BATCH_NOT_FOUND", "批次会话已丢失", "重新开始批量"))?
    };

    let index = stored
        .inputs
        .iter()
        .position(|input| input.id == file_id)
        .ok_or_else(|| AppError::new("E_BATCH_NOT_FOUND", "找不到该文件", "移除后重新添加"))?;
    let input = stored.inputs[index].clone();
    let cancel = Arc::new(AtomicBool::new(false));
    let (result, _) = process_file(
        &input,
        &stored.config,
        &stored.package_dir.join("mirror"),
        &cancel,
    );

    let mut session = stored.clone();
    let mut result = result;
    if result.status == "completed" {
        result.output_path = Some(
            session
                .package_dir
                .join("mirror")
                .join(mirror_relative(&input.relative))
                .to_string_lossy()
                .into_owned(),
        );
    }
    let mut outputs = session.result.files.clone();
    if index < outputs.len() {
        outputs[index] = result.clone();
    }
    let mut all_chunks = Vec::new();
    for (input, file) in session.inputs.iter().zip(outputs.iter()) {
        if file.status != "completed" {
            continue;
        }
        let relative = mirror_relative(&input.relative);
        let path = session.package_dir.join("mirror").join(&relative);
        if let Ok(markdown) = fs::read_to_string(&path) {
            let (mut chunks, _) = chunk::chunk_markdown(
                &markdown,
                &ChunkContext {
                    source: &relative,
                    source_path: &input.path,
                    file_name: &input.name,
                    format: &input.format,
                    hash: &file.hash,
                    engine: &file.engine,
                    params: session.config.params(),
                    ocr_engine: file.ocr_engine.as_deref(),
                    ocr_pages: &file.ocr_pages,
                },
                session.config.chunk_size,
                session.config.overlap,
            );
            all_chunks.append(&mut chunks);
        }
    }
    let corpus_files: Vec<CorpusFile> = session
        .inputs
        .iter()
        .zip(outputs.iter())
        .map(|(input, result)| CorpusFile {
            input_path: input.path.clone(),
            name: input.name.clone(),
            relative: mirror_relative(&input.relative),
            format: input.format.clone(),
            hash: result.hash.clone(),
            status: result.status.clone(),
            error_code: result.error_code.clone(),
            chars: result.chars,
            headings: result.headings,
            table_rows: result.table_rows,
            chunks: result.chunks,
            warnings: result.warnings.clone(),
            engine: result.engine.clone(),
            fallback_reason: result.fallback_reason.clone(),
            ocr_pages: result.ocr_pages.clone(),
            ocr_engine: result.ocr_engine.clone(),
        })
        .collect();
    let meta = CorpusMeta {
        batch_id: session.result.batch_id.clone(),
        batch_name: session.result.name.clone(),
        preset: session.config.preset.clone(),
        engine: session.config.engine.clone(),
        params: session.config.params(),
        created_at: chrono::Utc::now().to_rfc3339(),
        duration_ms: session.result.duration_ms,
        include_chunks: session.config.corpus && session.config.chunk,
        include_metadata: session.config.metadata,
    };
    corpus::write_package(&session.package_dir, &meta, &corpus_files, &all_chunks)?;

    let counts = counts(&outputs);
    let status = corpus::status(&corpus_summary(&counts)).to_string();
    session.result.files = outputs;
    session.result.counts = counts;
    session.result.status = status;
    record_history(&session.result, &session.config, &session.inputs);
    if let Some(state) = app.try_state::<crate::commands::AppState>() {
        if let Ok(mut sessions) = state.batches.sessions.lock() {
            sessions.insert(batch_id.to_string(), session.clone());
        }
    }
    emit_done(&app, &session.result);
    Ok(session.result)
}

pub fn cancel(app: &AppHandle, batch_id: &str) {
    if let Some(state) = app.try_state::<crate::commands::AppState>() {
        if let Ok(map) = state.batch_cancels.lock() {
            if let Some(flag) = map.get(batch_id) {
                flag.store(true, Ordering::Relaxed);
            }
        }
    }
}

fn process_file(
    input: &BatchInput,
    config: &BatchConfig,
    mirror_root: &Path,
    cancel: &AtomicBool,
) -> (BatchFileResult, Vec<Chunk>) {
    let started = Instant::now();
    if cancel.load(Ordering::Relaxed) {
        return (cancelled_input(input), Vec::new());
    }
    let path = Path::new(&input.path);
    let hash = fs::read(path)
        .map(|bytes| chunk::sha256_hex(&bytes))
        .unwrap_or_default();
    let choice = crate::engine::EngineChoice::parse(&config.engine);
    let extracted = match engine::extract(path, choice, |_| {}) {
        Ok(extracted) => extracted,
        Err(error) => {
            return (
                failed_result(input, &hash, &error, started.elapsed().as_millis() as u64),
                Vec::new(),
            )
        }
    };
    if cancel.load(Ordering::Relaxed) {
        return (cancelled_input(input), Vec::new());
    }
    let cleaned = clean::clean(&extracted.markdown, &config.clean_options());
    let relative = mirror_relative(&input.relative);
    let target = mirror_root.join(&relative);
    if let Some(parent) = target.parent() {
        if let Err(error) = fs::create_dir_all(parent) {
            let app_error = AppError::new(
                "E_OUTPUT_PERMISSION",
                format!("无法创建镜像目录: {error}"),
                "检查输出目录权限",
            );
            return (
                failed_result(
                    input,
                    &hash,
                    &app_error,
                    started.elapsed().as_millis() as u64,
                ),
                Vec::new(),
            );
        }
    }
    if let Err(error) = fs::write(&target, cleaned.markdown.as_bytes()) {
        let app_error = AppError::new(
            "E_OUTPUT_PERMISSION",
            format!("写入镜像失败: {error}"),
            "检查输出目录权限",
        );
        return (
            failed_result(
                input,
                &hash,
                &app_error,
                started.elapsed().as_millis() as u64,
            ),
            Vec::new(),
        );
    }

    let stats = engine::stats(&cleaned.markdown);
    let mut chunks = Vec::new();
    let mut warnings = cleaned.warnings.clone();
    if config.corpus && config.chunk {
        let (mut produced, mut chunk_warnings) = chunk::chunk_markdown(
            &cleaned.markdown,
            &ChunkContext {
                source: &relative,
                source_path: &input.path,
                file_name: &input.name,
                format: &input.format,
                hash: &hash,
                engine: &extracted.engine,
                params: config.params(),
                ocr_engine: extracted.ocr_engine.as_deref(),
                ocr_pages: &extracted.ocr_pages,
            },
            config.chunk_size,
            config.overlap,
        );
        warnings.append(&mut chunk_warnings);
        chunks.append(&mut produced);
    }
    warnings.extend(extracted.warnings.iter().cloned());

    (
        BatchFileResult {
            id: input.id.clone(),
            path: input.path.clone(),
            name: input.name.clone(),
            relative: input.relative.clone(),
            format: input.format.clone(),
            hash,
            status: "completed".into(),
            engine: extracted.engine.clone(),
            engine_choice: extracted.engine_choice.clone(),
            fallback_reason: extracted.fallback_reason.clone(),
            ocr_pages: extracted.ocr_pages.clone(),
            ocr_engine: extracted.ocr_engine.clone(),
            table_grade: Some(cleaned.table_grade),
            warnings,
            error_code: None,
            error_message: None,
            next_step: None,
            chars: stats.chars,
            headings: stats.headings,
            table_rows: stats.table_rows,
            chunks: chunks.len(),
            duration_ms: started.elapsed().as_millis() as u64,
            output_path: None,
        },
        chunks,
    )
}

fn failed_result(
    input: &BatchInput,
    hash: &str,
    error: &AppError,
    duration_ms: u64,
) -> BatchFileResult {
    BatchFileResult {
        id: input.id.clone(),
        path: input.path.clone(),
        name: input.name.clone(),
        relative: input.relative.clone(),
        format: input.format.clone(),
        hash: hash.to_string(),
        status: "failed".into(),
        engine: "anydoc".into(),
        engine_choice: "auto".into(),
        fallback_reason: None,
        ocr_pages: Vec::new(),
        ocr_engine: None,
        table_grade: None,
        warnings: vec![error.message.clone()],
        error_code: Some(error.code.clone()),
        error_message: Some(error.message.clone()),
        next_step: Some(error.next_step.clone()),
        chars: 0,
        headings: 0,
        table_rows: 0,
        chunks: 0,
        duration_ms,
        output_path: None,
    }
}

fn cancelled_input(input: &BatchInput) -> BatchFileResult {
    BatchFileResult {
        id: input.id.clone(),
        path: input.path.clone(),
        name: input.name.clone(),
        relative: input.relative.clone(),
        format: input.format.clone(),
        hash: String::new(),
        status: "skipped".into(),
        engine: "anydoc".into(),
        engine_choice: "auto".into(),
        fallback_reason: None,
        ocr_pages: Vec::new(),
        ocr_engine: None,
        table_grade: None,
        warnings: vec!["批次已取消".into()],
        error_code: Some("E_CANCELLED".into()),
        error_message: Some("批次已取消".into()),
        next_step: Some("可以重新开始批次".into()),
        chars: 0,
        headings: 0,
        table_rows: 0,
        chunks: 0,
        duration_ms: 0,
        output_path: None,
    }
}

fn counts(files: &[BatchFileResult]) -> BatchCounts {
    BatchCounts {
        success: files
            .iter()
            .filter(|file| file.status == "completed")
            .count(),
        skipped: files.iter().filter(|file| file.status == "skipped").count(),
        failed: files.iter().filter(|file| file.status == "failed").count(),
        warnings: files.iter().map(|file| file.warnings.len()).sum(),
        chunks: files.iter().map(|file| file.chunks).sum(),
    }
}

fn corpus_summary(counts: &BatchCounts) -> corpus::CorpusSummary {
    corpus::CorpusSummary {
        success: counts.success,
        skipped: counts.skipped,
        failed: counts.failed,
        warnings: counts.warnings,
        chunks: counts.chunks,
    }
}

fn record_history(result: &BatchResult, config: &BatchConfig, inputs: &[BatchInput]) {
    let task = HistoryTask {
        id: result.batch_id.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
        input_path: inputs
            .first()
            .map(|input| input.path.clone())
            .unwrap_or_default(),
        input_name: result.name.clone(),
        direction: "批量 → Markdown".into(),
        // 批量记录保留批次引擎选择；逐文件实际引擎见 manifest 与详情。
        engine: config.engine.clone(),
        status: result.status.clone(),
        output_path: Some(result.package_dir.clone()),
        error_code: if result.counts.failed > 0 {
            Some("E_PARTIAL_BATCH".into())
        } else {
            None
        },
        stats: TaskStats {
            chars: result.files.iter().map(|file| file.chars).sum(),
            headings: result.files.iter().map(|file| file.headings).sum(),
            table_rows: result.files.iter().map(|file| file.table_rows).sum(),
        },
        duration_ms: result.duration_ms as u128,
        kind: "batch".into(),
        input_count: inputs.len(),
        input_paths: inputs.iter().map(|input| input.path.clone()).collect(),
        output_dir: Some(result.package_dir.clone()),
        counts: Some(result.counts.clone()),
        params: Some(config.params()),
    };
    let _ = storage::upsert_task(task);
}

fn emit_progress(
    app: &AppHandle,
    batch_id: &str,
    file_id: Option<&str>,
    status: &str,
    completed: usize,
    total: usize,
) {
    let percent = if total == 0 {
        0
    } else {
        ((completed * 100) / total) as u8
    };
    let _ = app.emit(
        "batch-progress",
        BatchProgress {
            batch_id: batch_id.to_string(),
            file_id: file_id.map(str::to_string),
            status: status.to_string(),
            completed,
            total,
            percent,
        },
    );
}

fn emit_done(app: &AppHandle, result: &BatchResult) {
    let _ = app.emit(
        "batch-progress",
        BatchProgress {
            batch_id: result.batch_id.clone(),
            file_id: None,
            status: result.status.clone(),
            completed: result.files.len(),
            total: result.files.len(),
            percent: 100,
        },
    );
}

fn mirror_relative(relative: &str) -> String {
    let path = Path::new(relative);
    let stem = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".into());
    let parent = path
        .parent()
        .map(|parent| parent.to_string_lossy().into_owned())
        .unwrap_or_default();
    let name = format!("{stem}.md");
    if parent.is_empty() {
        name
    } else {
        format!("{parent}/{name}")
    }
}

fn sanitize_name(name: String) -> String {
    let cleaned: String = name
        .chars()
        .map(|ch| {
            if ch.is_alphanumeric() || ch == '-' || ch == '_' || ch == ' ' {
                ch
            } else {
                '-'
            }
        })
        .collect();
    let cleaned = cleaned.trim().replace(' ', "-");
    if cleaned.is_empty() {
        "corpus".into()
    } else {
        cleaned
    }
}

fn unique_package(root: &Path, name: &str) -> PathBuf {
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let base = root.join(format!("{name}-{stamp}"));
    if !base.exists() {
        return base;
    }
    for index in 1..1000 {
        let candidate = root.join(format!("{name}-{stamp}-{index}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    root.join(format!("{name}-{}-{}", stamp, uuid::Uuid::new_v4()))
}

fn short_hash(value: &str) -> String {
    chunk::sha256_hex(value.as_bytes())[..16].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mirror_paths_keep_relative_directories() {
        assert_eq!(mirror_relative("a/b/c.docx"), "a/b/c.md");
        assert_eq!(mirror_relative("plain"), "plain.md");
    }

    #[test]
    fn sanitizes_package_names() {
        assert_eq!(sanitize_name("等保 资料/2026".into()), "等保-资料-2026");
        assert_eq!(sanitize_name("  ".into()), "corpus");
    }

    #[test]
    fn default_concurrency_is_bounded() {
        let value = default_concurrency();
        assert!((1..=8).contains(&value));
    }

    #[test]
    #[ignore = "依赖本地 example 样本"]
    fn scans_and_processes_example_folder() {
        let example = Path::new(env!("CARGO_MANIFEST_DIR")).join("../example");
        let scan = scan_inputs(
            &[example.to_string_lossy().into_owned()],
            "/tmp/dulldoc-batch-output",
        )
        .expect("扫描应成功");
        assert!(scan.files.len() >= 3, "files={}", scan.files.len());

        let temp = std::env::temp_dir().join(format!("dulldoc-batch-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(temp.join("mirror")).unwrap();
        let cancel = AtomicBool::new(false);
        let config = BatchConfig::default();
        let mut completed = 0;
        let mut chunk_count = 0;
        let mut ocr_seen = false;
        for input in &scan.files {
            let (result, chunks) = process_file(input, &config, &temp.join("mirror"), &cancel);
            if result.status == "completed" {
                completed += 1;
                chunk_count += chunks.len();
                if result.engine == "xberg" {
                    ocr_seen = true;
                    assert!(!result.ocr_pages.is_empty(), "xberg 结果必须记录 OCR 页");
                    assert_eq!(result.ocr_engine.as_deref(), Some("paddleocr"));
                    assert!(result.fallback_reason.is_some());
                }
            } else {
                assert!(result.error_code.is_some(), "失败行必须有错误码");
            }
        }
        assert!(completed >= 2, "completed={completed}");
        assert!(chunk_count > 0);
        if crate::ocr::status().ready {
            assert!(ocr_seen, "模型就绪时扫描 PDF 应走 xberg OCR");
        }
        let _ = fs::remove_dir_all(&temp);
    }
}

#[cfg(test)]
mod scan_tests {
    use super::*;

    #[test]
    fn scan_filters_hidden_and_output_directory() {
        let root = std::env::temp_dir().join(format!("dulldoc-scan-{}", uuid::Uuid::new_v4()));
        let output = root.join("out");
        fs::create_dir_all(root.join("sub")).unwrap();
        fs::create_dir_all(&output).unwrap();
        fs::write(root.join("a.docx"), b"x").unwrap();
        fs::write(root.join(".hidden.docx"), b"x").unwrap();
        fs::write(root.join(".DS_Store"), b"x").unwrap();
        fs::write(root.join("sub/b.xlsx"), b"x").unwrap();
        fs::write(root.join("note.md"), b"x").unwrap();
        let scan = scan_inputs(
            &[root.to_string_lossy().into_owned()],
            &output.to_string_lossy(),
        )
        .expect("扫描应成功");
        let names: Vec<&str> = scan.files.iter().map(|file| file.name.as_str()).collect();
        assert!(names.contains(&"a.docx"));
        assert!(names.contains(&"b.xlsx"));
        assert!(!names.contains(&".hidden.docx"));
        assert!(!names.contains(&".DS_Store"));
        assert!(!names.contains(&"note.md"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn cancelled_file_keeps_identity() {
        let input = BatchInput {
            id: "f1".into(),
            path: "/tmp/never-read.docx".into(),
            name: "never-read.docx".into(),
            relative: "never-read.docx".into(),
            format: "DOCX".into(),
            size_bytes: 1,
        };
        let cancel = AtomicBool::new(true);
        let (result, chunks) = process_file(
            &input,
            &BatchConfig::default(),
            &std::env::temp_dir(),
            &cancel,
        );
        assert_eq!(result.status, "skipped");
        assert_eq!(result.id, "f1");
        assert_eq!(result.error_code.as_deref(), Some("E_CANCELLED"));
        assert!(chunks.is_empty());
    }
}
