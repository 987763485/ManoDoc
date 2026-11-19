use std::collections::HashSet;
use std::fs;
use std::path::Path;

use serde_json::{json, Value};

use crate::chunk::Chunk;
use crate::models::AppError;

#[derive(Debug, Clone)]
pub struct CorpusFile {
    pub input_path: String,
    pub name: String,
    pub relative: String,
    pub format: String,
    pub hash: String,
    pub status: String,
    pub error_code: Option<String>,
    pub chars: usize,
    pub headings: usize,
    pub table_rows: usize,
    pub chunks: usize,
    pub warnings: Vec<String>,
    pub engine: String,
    pub fallback_reason: Option<String>,
    pub ocr_pages: Vec<u32>,
    pub ocr_engine: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CorpusMeta {
    pub batch_id: String,
    pub batch_name: String,
    pub preset: String,
    pub engine: String,
    pub params: Value,
    pub created_at: String,
    pub duration_ms: u64,
    pub include_chunks: bool,
    pub include_metadata: bool,
}

pub struct CorpusSummary {
    pub success: usize,
    pub skipped: usize,
    pub failed: usize,
    pub warnings: usize,
    pub chunks: usize,
}

pub fn write_package(
    root: &Path,
    meta: &CorpusMeta,
    files: &[CorpusFile],
    chunks: &[Chunk],
) -> Result<CorpusSummary, AppError> {
    fs::create_dir_all(root.join("mirror")).map_err(|error| {
        AppError::new(
            "E_OUTPUT_PERMISSION",
            format!("无法创建语料包目录: {error}"),
            "检查输出目录权限后重试",
        )
    })?;

    let summary = summarize(files, chunks.len());
    let manifest = manifest(meta, files, &summary, chunks.len());
    atomic_write(
        &root.join("manifest.json"),
        &serde_json::to_vec_pretty(&manifest).unwrap(),
    )?;
    let report = json!({
        "schema": "dulldoc.corpus-report.v1",
        "batchId": meta.batch_id,
        "createdAt": meta.created_at,
        "status": status(&summary),
        "totals": {
            "files": files.len(),
            "success": summary.success,
            "skipped": summary.skipped,
            "failed": summary.failed,
            "warnings": summary.warnings,
            "chunks": if meta.include_chunks { summary.chunks } else { 0 }
        },
        "files": files.iter().map(|file| json!({
            "path": file.relative,
            "status": file.status,
            "errorCode": file.error_code,
            "chars": file.chars,
            "headings": file.headings,
            "tableRows": file.table_rows,
            "chunks": file.chunks,
            "warnings": file.warnings,
            "engineUsed": file.engine,
            "fallbackReason": file.fallback_reason,
            "ocrPages": file.ocr_pages,
            "ocrEngine": file.ocr_engine
        })).collect::<Vec<_>>()
    });
    atomic_write(
        &root.join("report.json"),
        &serde_json::to_vec_pretty(&report).unwrap(),
    )?;

    let mut csv = String::from("file,status,errorCode,chars,headings,tableRows,chunks,warnings\n");
    for file in files {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{}\n",
            csv_escape(&file.relative),
            csv_escape(&file.status),
            csv_escape(file.error_code.as_deref().unwrap_or("")),
            file.chars,
            file.headings,
            file.table_rows,
            file.chunks,
            csv_escape(&file.warnings.join("; "))
        ));
    }
    atomic_write(&root.join("report.csv"), csv.as_bytes())?;

    if meta.include_chunks {
        let mut jsonl = String::new();
        for chunk in chunks {
            let value = if meta.include_metadata {
                serde_json::to_string(chunk).map_err(|error| {
                    AppError::new(
                        "E_CORPUS_VALIDATION",
                        format!("序列化 chunk 失败: {error}"),
                        "检查文件内容后重试",
                    )
                })?
            } else {
                // 关闭元数据时仍保留 source，保证 chunk 可追溯且能通过校验。
                serde_json::to_string(&json!({
                    "id": chunk.id,
                    "text": chunk.text,
                    "metadata": { "source": chunk.metadata.source }
                }))
                .map_err(|error| {
                    AppError::new(
                        "E_CORPUS_VALIDATION",
                        format!("序列化 chunk 失败: {error}"),
                        "检查文件内容后重试",
                    )
                })?
            };
            jsonl.push_str(&value);
            jsonl.push('\n');
        }
        atomic_write(&root.join("chunks.jsonl"), jsonl.as_bytes())?;
    } else {
        // 关闭切块时清掉旧文件，避免重试后残留不一致的语料。
        let _ = fs::remove_file(root.join("chunks.jsonl"));
    }

    validate_package(root, meta.include_chunks)?;
    Ok(summary)
}

pub fn validate_package(root: &Path, expect_chunks: bool) -> Result<(), AppError> {
    let manifest_bytes = fs::read(root.join("manifest.json")).map_err(|error| {
        AppError::new(
            "E_CORPUS_VALIDATION",
            format!("无法读取 manifest.json: {error}"),
            "重新运行批次或检查输出目录",
        )
    })?;
    let manifest: Value = serde_json::from_slice(&manifest_bytes).map_err(|error| {
        AppError::new(
            "E_CORPUS_VALIDATION",
            format!("manifest.json 不是合法 JSON: {error}"),
            "重新运行批次",
        )
    })?;
    let mirror = root.join("mirror");
    if let Some(files) = manifest.get("files").and_then(Value::as_array) {
        for file in files {
            let status = file.get("status").and_then(Value::as_str).unwrap_or("");
            if status != "completed" {
                continue;
            }
            let relative = file.get("path").and_then(Value::as_str).unwrap_or("");
            if !mirror.join(relative).is_file() {
                return Err(AppError::new(
                    "E_CORPUS_VALIDATION",
                    format!("manifest 引用的镜像文件不存在: {relative}"),
                    "重新运行批次",
                ));
            }
        }
    }

    let chunks_path = root.join("chunks.jsonl");
    if !expect_chunks {
        let _ = fs::remove_file(chunks_path);
        return Ok(());
    }
    let text = fs::read_to_string(&chunks_path).map_err(|error| {
        AppError::new(
            "E_CORPUS_VALIDATION",
            format!("无法读取 chunks.jsonl: {error}"),
            "重新运行批次",
        )
    })?;
    let mut ids = HashSet::new();
    let mut count = 0usize;
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(line).map_err(|error| {
            AppError::new(
                "E_CORPUS_VALIDATION",
                format!("chunks.jsonl 第 {} 行无法解析: {error}", index + 1),
                "重新运行批次",
            )
        })?;
        let id = value.get("id").and_then(Value::as_str).unwrap_or("");
        let chunk_text = value.get("text").and_then(Value::as_str).unwrap_or("");
        let metadata = value.get("metadata");
        if id.is_empty() || chunk_text.trim().is_empty() || metadata.is_none() {
            return Err(AppError::new(
                "E_CORPUS_VALIDATION",
                format!("chunks.jsonl 第 {} 行缺少 id/text/metadata", index + 1),
                "重新运行批次",
            ));
        }
        let metadata = metadata.expect("metadata checked above");
        if !ids.insert(id.to_string()) {
            return Err(AppError::new(
                "E_CORPUS_VALIDATION",
                format!("chunks.jsonl 第 {} 行 id 重复: {id}", index + 1),
                "重新运行批次",
            ));
        }
        if let Some(source) = metadata
            .get("source")
            .and_then(Value::as_str)
            .filter(|source| !source.is_empty())
        {
            if !mirror.join(source).is_file() {
                return Err(AppError::new(
                    "E_CORPUS_VALIDATION",
                    format!("chunks.jsonl 第 {} 行引用的镜像不存在: {source}", index + 1),
                    "重新运行批次",
                ));
            }
        }
        count += 1;
    }
    let expected = manifest
        .get("totals")
        .and_then(|totals| totals.get("chunks"))
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    if expected != count {
        return Err(AppError::new(
            "E_CORPUS_VALIDATION",
            format!("manifest 记录 {expected} 个 chunk，chunks.jsonl 实际 {count} 个"),
            "重新运行批次",
        ));
    }
    Ok(())
}

fn manifest(
    meta: &CorpusMeta,
    files: &[CorpusFile],
    summary: &CorpusSummary,
    chunk_count: usize,
) -> Value {
    json!({
        "schema": "dulldoc.corpus.v1",
        "tool": { "name": "DullDoc", "version": env!("CARGO_PKG_VERSION") },
        "engine": { "name": meta.engine.clone(), "version": "anydoc 0.2.4; xberg 1.4.0" },
        "batchId": meta.batch_id,
        "batchName": meta.batch_name,
        "createdAt": meta.created_at,
        "preset": meta.preset,
        "engineName": meta.engine,
        "durationMs": meta.duration_ms,
        "params": meta.params,
        "metadataEnabled": meta.include_metadata,
        "files": files.iter().map(|file| json!({
            "path": file.relative,
            "inputPath": file.input_path,
            "name": file.name,
            "format": file.format,
            "hash": file.hash,
            "status": file.status,
            "errorCode": file.error_code,
            "chars": file.chars,
            "headings": file.headings,
            "tableRows": file.table_rows,
            "chunks": file.chunks,
            "warnings": file.warnings,
            "engineUsed": file.engine,
            "fallbackReason": file.fallback_reason,
            "ocrPages": file.ocr_pages,
            "ocrEngine": file.ocr_engine
        })).collect::<Vec<_>>(),
        "totals": {
            "files": files.len(),
            "success": summary.success,
            "skipped": summary.skipped,
            "failed": summary.failed,
            "warnings": summary.warnings,
            "chunks": if meta.include_chunks { chunk_count } else { 0 }
        },
        "warnings": files.iter().flat_map(|file| file.warnings.clone()).collect::<Vec<_>>()
    })
}

fn summarize(files: &[CorpusFile], chunk_count: usize) -> CorpusSummary {
    CorpusSummary {
        success: files
            .iter()
            .filter(|file| file.status == "completed")
            .count(),
        skipped: files.iter().filter(|file| file.status == "skipped").count(),
        failed: files.iter().filter(|file| file.status == "failed").count(),
        warnings: files.iter().map(|file| file.warnings.len()).sum(),
        chunks: chunk_count,
    }
}

pub fn status(summary: &CorpusSummary) -> &'static str {
    if summary.failed > 0 && summary.success > 0 {
        "partial"
    } else if summary.failed > 0 {
        "failed"
    } else {
        "completed"
    }
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), AppError> {
    let tmp = path.with_extension(format!("tmp-{}", uuid::Uuid::new_v4()));
    fs::write(&tmp, bytes).map_err(|error| {
        AppError::new(
            "E_OUTPUT_PERMISSION",
            format!("写入 {} 失败: {error}", path.display()),
            "检查输出目录权限与磁盘空间",
        )
    })?;
    fs::rename(&tmp, path).map_err(|error| {
        let _ = fs::remove_file(&tmp);
        AppError::new(
            "E_OUTPUT_PERMISSION",
            format!("提交 {} 失败: {error}", path.display()),
            "检查输出目录权限与磁盘空间",
        )
    })
}

fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::{Chunk, ChunkMetadata};
    use std::path::PathBuf;

    fn sample() -> (PathBuf, CorpusMeta, Vec<CorpusFile>, Vec<Chunk>) {
        let root = std::env::temp_dir().join(format!("dulldoc-corpus-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(root.join("mirror")).unwrap();
        fs::write(root.join("mirror/a.md"), "# A\n\n正文\n").unwrap();
        let meta = CorpusMeta {
            batch_id: "batch-1".into(),
            batch_name: "corpus".into(),
            preset: "standard".into(),
            engine: "anydoc".into(),
            params: json!({"chunkSize": 1200}),
            created_at: "2026-11-19T00:00:00Z".into(),
            duration_ms: 100,
            include_chunks: true,
            include_metadata: true,
        };
        let files = vec![CorpusFile {
            input_path: "/tmp/a.docx".into(),
            name: "a.docx".into(),
            relative: "a.md".into(),
            format: "DOCX".into(),
            hash: "hash".into(),
            status: "completed".into(),
            error_code: None,
            chars: 5,
            headings: 1,
            table_rows: 0,
            chunks: 1,
            warnings: vec![],
            engine: "xberg".into(),
            fallback_reason: Some("anydoc:needs-ocr".into()),
            ocr_pages: vec![1],
            ocr_engine: Some("paddleocr".into()),
        }];
        let chunks = vec![Chunk {
            id: "chunk-1".into(),
            text: "正文".into(),
            metadata: ChunkMetadata {
                source: "a.md".into(),
                source_path: "/tmp/a.docx".into(),
                file_name: "a.docx".into(),
                format: "DOCX".into(),
                title: "A".into(),
                heading_path: vec!["A".into()],
                page: None,
                hash: "hash".into(),
                engine: "anydoc".into(),
                params: json!({"chunkSize": 1200}),
                chars: 2,
                tokens: 1,
                ocr_engine: None,
                ocr_pages: Vec::new(),
            },
        }];
        (root, meta, files, chunks)
    }

    #[test]
    fn writes_and_validates_package() {
        let (root, meta, files, chunks) = sample();
        write_package(&root, &meta, &files, &chunks).expect("写入应成功");
        assert!(root.join("chunks.jsonl").is_file());
        assert!(root.join("manifest.json").is_file());
        assert!(root.join("report.csv").is_file());
        validate_package(&root, true).expect("校验应成功");

        let manifest: Value =
            serde_json::from_slice(&fs::read(root.join("manifest.json")).unwrap()).unwrap();
        let file = &manifest["files"][0];
        assert_eq!(file["engineUsed"], "xberg");
        assert_eq!(file["ocrPages"][0], 1);
        assert_eq!(file["ocrEngine"], "paddleocr");
        assert_eq!(file["fallbackReason"], "anydoc:needs-ocr");

        let path = root.join("chunks.jsonl");
        let mut text = fs::read_to_string(&path).unwrap();
        text.push_str("not-json\n");
        fs::write(&path, text).unwrap();
        let error = validate_package(&root, true).expect_err("坏行应失败");
        assert_eq!(error.code, "E_CORPUS_VALIDATION");
        let _ = fs::remove_dir_all(&root);
    }
}
