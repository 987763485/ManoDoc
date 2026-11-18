//! OCR 模型管理。
//!
//! 模型不随仓库提交，也不在转换时静默下载：状态检测只读本地目录，下载必须由
//! 用户显式触发并校验 SHA-256；转换路径会设置离线模式，缺失模型直接失败。

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

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

const REPO_DIR: &str = "models--xberg-io--paddleocr-onnx-models";
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

/// 测试与诊断可用 `DULLDOC_MODEL_DIR` 指向独立目录，生产固定落在应用数据目录。
pub fn model_dir() -> PathBuf {
    if let Ok(value) = std::env::var("DULLDOC_MODEL_DIR") {
        if !value.trim().is_empty() {
            return PathBuf::from(value);
        }
    }
    match storage::data_dir() {
        Ok(root) => root.join("models").join("paddleocr"),
        Err(_) => PathBuf::from(".dull_doc-models"),
    }
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

/// 测试或诊断可用 `DULLDOC_MODEL_BASE_URL` 把 HF 源替换为本地镜像；
/// 生产默认始终使用 xberg manifest 中的官方地址。
fn download_url(entry: &ModelManifestEntry) -> String {
    match std::env::var("DULLDOC_MODEL_BASE_URL") {
        Ok(base) if !base.trim().is_empty() => {
            let path = entry
                .source_url
                .strip_prefix("https://huggingface.co")
                .unwrap_or(&entry.relative_path);
            format!("{}{}", base.trim_end_matches('/'), path)
        }
        _ => entry.source_url.clone(),
    }
}

fn download<F: FnMut(&OcrProgress)>(
    on_progress: &mut F,
    entry: &ModelManifestEntry,
    target: &Path,
    index: usize,
    count: usize,
) -> Result<(), AppError> {
    let parent = target.parent().ok_or_else(|| {
        AppError::new(
            "E_OCR_MODEL_DOWNLOAD",
            "模型目标路径无效",
            "检查应用数据目录权限后重试",
        )
    })?;
    fs::create_dir_all(parent).map_err(|error| {
        AppError::new(
            "E_OCR_MODEL_DOWNLOAD",
            format!("无法创建模型目录: {error}"),
            "检查 ~/.dull_doc 是否可写",
        )
    })?;

    let temp = target.with_extension(format!("part-{}", uuid::Uuid::new_v4()));
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(300)))
        .build()
        .into();

    // 下载失败必须清理半写文件，避免下次状态检测把不完整文件当成已安装。
    let result = (|| -> Result<(), AppError> {
        let mut response = agent.get(&download_url(entry)).call().map_err(|error| {
            AppError::new(
                "E_OCR_MODEL_DOWNLOAD",
                format!("无法下载 OCR 模型（离线或网络不可达）: {error}"),
                "联网后重试；也可手动把模型放入应用数据目录的 models/paddleocr",
            )
        })?;
        let mut reader = response.body_mut().as_reader();
        let mut file = fs::File::create(&temp).map_err(|error| {
            AppError::new(
                "E_OCR_MODEL_DOWNLOAD",
                format!("无法创建临时模型文件: {error}"),
                "检查磁盘空间与目录权限",
            )
        })?;
        let mut hasher = Sha256::new();
        let mut downloaded = 0u64;
        let mut buffer = vec![0u8; 256 * 1024];
        let mut last_emit = Instant::now();
        loop {
            let read = reader.read(&mut buffer).map_err(|error| {
                AppError::new(
                    "E_OCR_MODEL_DOWNLOAD",
                    format!("下载中断: {error}"),
                    "联网后重试",
                )
            })?;
            if read == 0 {
                break;
            }
            file.write_all(&buffer[..read]).map_err(|error| {
                AppError::new(
                    "E_OCR_MODEL_DOWNLOAD",
                    format!("写入模型失败: {error}"),
                    "检查磁盘空间与目录权限",
                )
            })?;
            hasher.update(&buffer[..read]);
            downloaded += read as u64;
            if last_emit.elapsed() >= Duration::from_millis(200) {
                on_progress(&progress_payload(entry, downloaded, index, count));
                last_emit = Instant::now();
            }
        }
        file.sync_all().ok();
        drop(file);
        on_progress(&progress_payload(entry, downloaded, index, count));

        if downloaded != entry.size_bytes {
            return Err(AppError::new(
                "E_OCR_MODEL_DOWNLOAD",
                format!(
                    "模型大小不匹配: {} 期望 {} 字节，实际 {} 字节",
                    entry.relative_path, entry.size_bytes, downloaded
                ),
                "重新下载 OCR 模型",
            ));
        }
        let actual = format!("{:x}", hasher.finalize());
        if !actual.eq_ignore_ascii_case(&entry.sha256) {
            return Err(AppError::new(
                "E_OCR_MODEL_CHECKSUM",
                format!("模型校验失败: {}", entry.relative_path),
                "删除该文件后重新安装 OCR 模型",
            ));
        }
        fs::rename(&temp, target).map_err(|error| {
            AppError::new(
                "E_OCR_MODEL_DOWNLOAD",
                format!("提交模型文件失败: {error}"),
                "检查目录权限后重试",
            )
        })?;
        Ok(())
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

fn progress_payload(
    entry: &ModelManifestEntry,
    downloaded: u64,
    index: usize,
    count: usize,
) -> OcrProgress {
    let percent = if entry.size_bytes == 0 {
        0
    } else {
        ((downloaded.saturating_mul(100) / entry.size_bytes).min(100)) as u8
    };
    OcrProgress {
        file: entry.relative_path.clone(),
        downloaded,
        total: entry.size_bytes,
        percent,
        file_index: index,
        file_count: count,
    }
}

/// 只下载缺失或校验失败的文件；已通过校验的文件不重复下载。
pub fn install(app: &AppHandle) -> Result<OcrStatus, AppError> {
    install_at(&model_dir(), |progress| {
        let _ = app.emit("ocr-model-progress", progress);
    })
}

pub fn install_at<F: FnMut(&OcrProgress)>(root: &Path, mut on_progress: F) -> Result<OcrStatus, AppError> {
    let entries = required_entries();
    let count = entries.len();

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
        download(&mut on_progress, entry, &target, index + 1, count)?;
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
        let dir = std::env::temp_dir().join(format!("dulldoc-ocr-{}", uuid::Uuid::new_v4()));
        let status = status_at(&dir);
        assert!(!status.ready);
        assert!(!status.partial);
        assert_eq!(status.installed_bytes, 0);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    #[ignore = "使用本地 HTTP 镜像验证下载、进度、SHA-256 与原子写入；需要已安装模型作为源"]
    fn downloads_models_from_local_mirror() {
        use std::io::{Read as _, Write as _};
        use std::net::TcpListener;
        use std::thread;

        let source_root = model_dir();
        let entries = required_entries();
        let mut payloads = Vec::new();
        for entry in &entries {
            let path = file_path(&source_root, entry).expect("源模型路径");
            payloads.push((entry.relative_path.clone(), fs::read(path).expect("读取源模型")));
        }

        let listener = TcpListener::bind("127.0.0.1:0").expect("绑定本地端口");
        let port = listener.local_addr().unwrap().port();
        let server_payloads = payloads.clone();
        let server = thread::spawn(move || {
            for _ in 0..server_payloads.len() {
                let Ok((mut stream, _)) = listener.accept() else {
                    break;
                };
                let mut buffer = [0u8; 4096];
                let read = stream.read(&mut buffer).unwrap_or(0);
                let request = String::from_utf8_lossy(&buffer[..read]);
                let target = request
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .unwrap_or("");
                let hit = server_payloads
                    .iter()
                    .find(|(relative, _)| target.ends_with(relative.as_str()));
                match hit {
                    Some((_, data)) => {
                        let header = format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            data.len()
                        );
                        let _ = stream.write_all(header.as_bytes());
                        let _ = stream.write_all(data);
                    }
                    None => {
                        let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n");
                    }
                }
            }
        });

        let dir = std::env::temp_dir().join(format!("dulldoc-ocr-mirror-{}", uuid::Uuid::new_v4()));
        std::env::set_var("DULLDOC_MODEL_BASE_URL", format!("http://127.0.0.1:{port}"));
        let mut progress_events = 0usize;
        let result = install_at(&dir, |_| progress_events += 1);
        std::env::remove_var("DULLDOC_MODEL_BASE_URL");
        let status = result.expect("本地镜像安装应成功");
        server.join().ok();

        assert!(status.ready);
        assert!(status.installed_bytes > 30_000_000);
        assert!(progress_events > 0, "应产生进度事件");
        // 第二次安装应复用已校验文件，不再发起下载。
        let again = install_at(&dir, |_| {}).expect("重复安装应成功");
        assert!(again.ready);
        let _ = fs::remove_dir_all(dir);
    }
}
