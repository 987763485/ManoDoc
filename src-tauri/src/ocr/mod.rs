//! OCR 模型管理。
//!
//! 模型不随仓库提交，也不在转换时静默下载：状态检测只读本地目录，下载必须由
//! 用户显式触发并校验 SHA-256；转换路径会设置离线模式，缺失模型直接失败。

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use hf_hub::progress::{DownloadEvent, Progress, ProgressEvent, ProgressHandler};
use hf_hub::{HFClient, HFClientSync};
use serde::Serialize;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter};
use xberg::{ModelManager, ModelManifestEntry};

use crate::models::AppError;
use crate::storage;

/// 仅启用 PP-OCRv6 small 的最小集合；与 xberg 默认 OCR 参数保持一致。
const REQUIRED_FILES: &[&str] = &[
    "v6/det/small/model.onnx",
    "v6/rec/small/model.onnx",
    "v6/rec/small/dict.txt",
];

const REPO_OWNER: &str = "xberg-io";
const REPO_NAME: &str = "paddleocr-onnx-models";
const REPO_DIR: &str = "models--xberg-io--paddleocr-onnx-models";
const HF_ENDPOINT_OFFICIAL: &str = "https://huggingface.co";
const DOWNLOAD_SOURCE: &str = "https://huggingface.co/xberg-io/paddleocr-onnx-models";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrModelFile {
    pub relative_path: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub source_url: String,
    pub present: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrStatus {
    pub engine: String,
    pub engine_version: String,
    pub ready: bool,
    pub partial: bool,
    pub installed_bytes: u64,
    pub required_bytes: u64,
    pub model_dir: String,
    pub source: String,
    pub files: Vec<OcrModelFile>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrProgress {
    pub file: String,
    pub downloaded: u64,
    pub total: u64,
    pub percent: u8,
    pub file_index: usize,
    pub file_count: usize,
}

/// 测试与诊断可用 `MANODOC_MODEL_DIR` 指向独立目录，生产固定落在应用数据目录的 `models`。
pub fn model_dir() -> PathBuf {
    if let Ok(value) = std::env::var("MANODOC_MODEL_DIR") {
        if !value.trim().is_empty() {
            return PathBuf::from(value);
        }
    }
    match storage::data_dir() {
        Ok(root) => {
            let models = root.join("models");
            migrate_legacy_dir(&models);
            models
        }
        Err(_) => PathBuf::from(".manodoc-models"),
    }
}

/// 早期版本把 HF 缓存放在 `models/paddleocr`；整体上移一层即可，blob 相对链接保持有效。
fn migrate_legacy_dir(models: &Path) {
    let legacy_repo = models.join("paddleocr").join(REPO_DIR);
    let target = models.join(REPO_DIR);
    if legacy_repo.is_dir() && !target.exists() {
        if fs::rename(&legacy_repo, &target).is_ok() {
            let _ = fs::remove_dir(models.join("paddleocr"));
        }
    }
}

/// manifest 的 source_url 形如 `.../resolve/<revision>/<file>`，缓存目录按同一 revision 存放。
fn revision(entry: &ModelManifestEntry) -> Option<&str> {
    entry
        .source_url
        .split("/resolve/")
        .nth(1)?
        .split('/')
        .next()
}

fn required_entries() -> Vec<ModelManifestEntry> {
    ModelManager::manifest()
        .into_iter()
        .filter(|entry| REQUIRED_FILES.contains(&entry.relative_path.as_str()))
        .collect()
}

/// xberg 把模型放在 Hugging Face 的 snapshot 布局中，快照目录就是固定 revision。
fn snapshot_dir(root: &Path, entry: &ModelManifestEntry) -> Option<PathBuf> {
    let marker = "/resolve/";
    let rest = entry.source_url.split(marker).nth(1)?;
    let revision = rest.split('/').next()?;
    Some(root.join(REPO_DIR).join("snapshots").join(revision))
}

fn file_path(root: &Path, entry: &ModelManifestEntry) -> Option<PathBuf> {
    Some(snapshot_dir(root, entry)?.join(&entry.relative_path))
}

fn present(path: &Path, expected_size: u64) -> bool {
    fs::metadata(path)
        .map(|meta| meta.is_file() && meta.len() == expected_size)
        .unwrap_or(false)
}

pub fn status() -> OcrStatus {
    status_at(&model_dir())
}

pub fn status_at(root: &Path) -> OcrStatus {
    let entries = required_entries();
    let mut files = Vec::with_capacity(entries.len());
    let mut installed_bytes = 0;
    let mut present_count = 0;

    for entry in &entries {
        let path = file_path(&root, entry);
        let exists = path
            .as_deref()
            .map(|path| present(path, entry.size_bytes))
            .unwrap_or(false);
        if exists {
            installed_bytes += entry.size_bytes;
            present_count += 1;
        }
        files.push(OcrModelFile {
            relative_path: entry.relative_path.clone(),
            size_bytes: entry.size_bytes,
            sha256: entry.sha256.clone(),
            source_url: entry.source_url.clone(),
            present: exists,
        });
    }

    OcrStatus {
        engine: "xberg".into(),
        engine_version: "1.4.0".into(),
        ready: !entries.is_empty() && present_count == entries.len(),
        partial: present_count > 0 && present_count < entries.len(),
        installed_bytes,
        required_bytes: entries.iter().map(|entry| entry.size_bytes).sum(),
        model_dir: root.to_string_lossy().into_owned(),
        source: DOWNLOAD_SOURCE.into(),
        files,
    }
}

fn sha256_file(path: &Path) -> Result<String, AppError> {
    let mut file = fs::File::open(path).map_err(|error| {
        AppError::new(
            "E_OCR_MODEL_CHECKSUM",
            format!("无法读取模型文件: {error}"),
            "删除该文件后重新安装 OCR 模型",
        )
    })?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 256 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|error| {
            AppError::new(
                "E_OCR_MODEL_CHECKSUM",
                format!("读取模型文件失败: {error}"),
                "删除该文件后重新安装 OCR 模型",
            )
        })?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// 本地镜像与离线测试通过 `MANODOC_MODEL_BASE_URL` 覆盖 HF endpoint。
fn hf_client(root: &Path) -> Result<HFClientSync, AppError> {
    // 固定官方 endpoint：避免用户环境的 HF_ENDPOINT 镜像缺少 ETag 等必需响应头。
    let endpoint = match std::env::var("MANODOC_MODEL_BASE_URL") {
        Ok(base) if !base.trim().is_empty() => base.trim().trim_end_matches('/').to_string(),
        _ => HF_ENDPOINT_OFFICIAL.to_string(),
    };
    let builder = HFClient::builder().cache_dir(root).endpoint(endpoint);
    builder.build_sync().map_err(|error| {
        AppError::new(
            "E_OCR_MODEL_DOWNLOAD",
            format!("无法初始化 Hugging Face 客户端: {error}"),
            "检查网络或代理设置后重试",
        )
    })
}

/// hf-hub 的进度回调要求 `Send + Sync`；用互斥锁把回调转成可共享的 sink。
struct ProgressBridge<F> {
    sink: Arc<Mutex<F>>,
    file: String,
    index: usize,
    count: usize,
}

impl<F: FnMut(&OcrProgress) + Send + 'static> ProgressHandler for ProgressBridge<F> {
    fn on_progress(&self, event: &ProgressEvent) {
        let ProgressEvent::Download(DownloadEvent::Progress { files }) = event else {
            return;
        };
        let Some(file) = files.iter().find(|file| file.filename == self.file) else {
            return;
        };
        if let Ok(mut sink) = self.sink.lock() {
            sink(&OcrProgress {
                file: self.file.clone(),
                downloaded: file.bytes_completed,
                total: file.total_bytes,
                percent: percent(file.bytes_completed, file.total_bytes),
                file_index: self.index,
                file_count: self.count,
            });
        }
    }
}

fn percent(downloaded: u64, total: u64) -> u8 {
    if total == 0 {
        0
    } else {
        (downloaded.saturating_mul(100) / total).min(100) as u8
    }
}

fn download_error(entry: &ModelManifestEntry, error: impl std::fmt::Display) -> AppError {
    AppError::new(
        "E_OCR_MODEL_DOWNLOAD",
        format!(
            "无法下载 {}（离线或网络不可达）: {error}",
            entry.relative_path
        ),
        "联网后重试；也可手动把模型放入应用数据目录的 models",
    )
}

fn download<F: FnMut(&OcrProgress) + Send + 'static>(
    client: &HFClientSync,
    sink: Arc<Mutex<F>>,
    entry: &ModelManifestEntry,
    index: usize,
    count: usize,
) -> Result<PathBuf, AppError> {
    let revision = revision(entry).ok_or_else(|| {
        AppError::new(
            "E_OCR_MODEL_DOWNLOAD",
            format!("模型地址缺少 revision: {}", entry.source_url),
            "更新应用后重试",
        )
    })?;
    let repo = client.model(REPO_OWNER, REPO_NAME);
    let progress = Progress::new(ProgressBridge {
        sink,
        file: entry.relative_path.clone(),
        index,
        count,
    });
    let path = repo
        .download_file()
        .filename(entry.relative_path.clone())
        .revision(revision.to_string())
        .progress(progress)
        .send()
        .map_err(|error| download_error(entry, error))?;

    // hf-hub 负责缓存与并发安全；这里仍校验 manifest 的固定大小与 SHA-256。
    let size = fs::metadata(&path)
        .map(|meta| meta.len())
        .unwrap_or_default();
    if size != entry.size_bytes {
        let _ = fs::remove_file(&path);
        return Err(AppError::new(
            "E_OCR_MODEL_DOWNLOAD",
            format!(
                "模型大小不匹配: {} 期望 {} 字节，实际 {} 字节",
                entry.relative_path, entry.size_bytes, size
            ),
            "重新下载 OCR 模型",
        ));
    }
    let actual = sha256_file(&path)?;
    if !actual.eq_ignore_ascii_case(&entry.sha256) {
        let _ = fs::remove_file(&path);
        return Err(AppError::new(
            "E_OCR_MODEL_CHECKSUM",
            format!("模型校验失败: {}", entry.relative_path),
            "删除该文件后重新安装 OCR 模型",
        ));
    }
    Ok(path)
}

/// 只下载缺失或校验失败的文件；已通过校验的文件不重复下载。
pub fn install(app: &AppHandle) -> Result<OcrStatus, AppError> {
    let handle = app.clone();
    install_at(&model_dir(), move |progress| {
        let _ = handle.emit("ocr-model-progress", progress);
    })
}

pub fn install_at<F: FnMut(&OcrProgress) + Send + 'static>(
    root: &Path,
    on_progress: F,
) -> Result<OcrStatus, AppError> {
    let entries = required_entries();
    let count = entries.len();
    let sink = Arc::new(Mutex::new(on_progress));
    let client = hf_client(root)?;

    for (index, entry) in entries.iter().enumerate() {
        let target = file_path(root, entry).ok_or_else(|| {
            AppError::new(
                "E_OCR_MODEL_DOWNLOAD",
                "无法解析模型路径",
                "重新安装应用后重试",
            )
        })?;
        let valid = present(&target, entry.size_bytes)
            && sha256_file(&target)
                .map(|hash| hash.eq_ignore_ascii_case(&entry.sha256))
                .unwrap_or(false);
        if valid {
            continue;
        }
        // 大小正确但校验失败的文件必须先删掉，避免下载中途被旧文件干扰。
        if target.exists() {
            let _ = fs::remove_file(&target);
        }
        download(&client, sink.clone(), entry, index + 1, count)?;
    }

    Ok(status_at(root))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_contains_required_v6_small_models() {
        let entries = required_entries();
        assert_eq!(entries.len(), REQUIRED_FILES.len());
        let total: u64 = entries.iter().map(|entry| entry.size_bytes).sum();
        assert!(total > 30_000_000 && total < 32_000_000, "total={total}");
        for entry in entries {
            assert!(entry.source_url.contains("/resolve/"));
            assert_eq!(entry.sha256.len(), 64);
        }
    }

    #[test]
    fn missing_model_dir_reports_not_ready() {
        let dir = std::env::temp_dir().join(format!("manodoc-ocr-{}", uuid::Uuid::new_v4()));
        let status = status_at(&dir);
        assert!(!status.ready);
        assert!(!status.partial);
        assert_eq!(status.installed_bytes, 0);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    #[ignore = "使用本地 HTTP 镜像验证 hf-hub 下载、进度与 SHA-256；需要已安装模型作为源"]
    fn downloads_models_from_local_mirror() {
        use std::io::{Read as _, Write as _};
        use std::net::TcpListener;
        use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
        use std::thread;
        use std::time::Duration;

        let source_root = model_dir();
        let entries = required_entries();
        let mut payloads = Vec::new();
        for entry in &entries {
            let path = file_path(&source_root, entry).expect("源模型路径");
            payloads.push((
                entry.relative_path.clone(),
                entry.sha256.clone(),
                revision(entry).expect("revision").to_string(),
                fs::read(path).expect("读取源模型"),
            ));
        }

        let listener = TcpListener::bind("127.0.0.1:0").expect("绑定本地端口");
        listener.set_nonblocking(true).expect("非阻塞监听");
        let port = listener.local_addr().unwrap().port();
        let stop = Arc::new(AtomicBool::new(false));
        let requests = Arc::new(AtomicUsize::new(0));
        let server_payloads = payloads.clone();
        let server_stop = stop.clone();
        let server_requests = requests.clone();
        let server = thread::spawn(move || {
            while !server_stop.load(Ordering::Relaxed) {
                let Ok((mut stream, _)) = listener.accept() else {
                    thread::sleep(Duration::from_millis(10));
                    continue;
                };
                // 监听 socket 非阻塞会让 accept 出的 stream 也不阻塞，写大文件会截断。
                stream.set_nonblocking(false).ok();
                server_requests.fetch_add(1, Ordering::Relaxed);
                let mut buffer = [0u8; 8192];
                let read = stream.read(&mut buffer).unwrap_or(0);
                let request = String::from_utf8_lossy(&buffer[..read]);
                let mut lines = request.lines();
                let method = lines
                    .next()
                    .and_then(|line| line.split_whitespace().next())
                    .unwrap_or("");
                let target = request
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .unwrap_or("");
                let hit = server_payloads
                    .iter()
                    .find(|(relative, _, _, _)| target.ends_with(relative.as_str()));
                match hit {
                    Some((_, sha, rev, data)) => {
                        let header = format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nETag: \"{}\"\r\nX-Repo-Commit: {}\r\nConnection: close\r\n\r\n",
                            data.len(),
                            sha,
                            rev
                        );
                        let _ = stream.write_all(header.as_bytes());
                        if method != "HEAD" {
                            let _ = stream.write_all(data);
                        }
                    }
                    None => {
                        let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n");
                    }
                }
            }
        });

        let dir = std::env::temp_dir().join(format!("manodoc-ocr-mirror-{}", uuid::Uuid::new_v4()));
        std::env::set_var("MANODOC_MODEL_BASE_URL", format!("http://127.0.0.1:{port}"));
        let progress_events = Arc::new(AtomicUsize::new(0));
        let counter = progress_events.clone();
        let result = install_at(&dir, move |_| {
            counter.fetch_add(1, Ordering::Relaxed);
        });
        std::env::remove_var("MANODOC_MODEL_BASE_URL");
        stop.store(true, Ordering::Relaxed);
        server.join().ok();

        let status = result.expect("本地镜像安装应成功");
        assert!(status.ready);
        assert!(status.installed_bytes > 30_000_000);
        assert!(progress_events.load(Ordering::Relaxed) > 0, "应产生进度事件");
        assert!(requests.load(Ordering::Relaxed) >= entries.len(), "每个文件至少一次 HEAD");
        // 第二次安装应复用已校验文件，不再发起下载。
        let before = requests.load(Ordering::Relaxed);
        let again = install_at(&dir, |_| {}).expect("重复安装应成功");
        assert!(again.ready);
        assert_eq!(requests.load(Ordering::Relaxed), before, "已安装文件不应重复下载");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    #[ignore = "需要访问 huggingface.co；验证 hf-hub 真实下载与 SHA-256"]
    fn installs_models_from_huggingface() {
        let dir = std::env::temp_dir().join(format!("manodoc-ocr-hf-{}", uuid::Uuid::new_v4()));
        let status = install_at(&dir, |_| {}).expect("应从 Hugging Face 下载模型");
        assert!(status.ready);
        assert!(status.installed_bytes > 30_000_000);
        assert!(dir.join(REPO_DIR).is_dir());
        let _ = fs::remove_dir_all(dir);
    }
}
