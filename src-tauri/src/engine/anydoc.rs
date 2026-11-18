use std::fs;
use std::path::Path;

use anydoc::{ConvertError, Format};

use crate::models::{AppError, TaskStats};

#[derive(Debug)]
pub struct Extracted {
    pub markdown: String,
    /// 仅用于诊断与样例测试；路由的展示标签来自输入类型。
    #[allow(dead_code)]
    pub format: String,
}

pub fn detect_format(bytes: &[u8], path: &Path) -> Option<Format> {
    Format::from_bytes(bytes).or_else(|| Format::from_path(path))
}

pub fn format_label(format: Format) -> &'static str {
    match format {
        Format::Doc => "DOC",
        Format::Docx => "DOCX",
        Format::Odt => "ODT",
        Format::Pdf => "PDF",
        Format::Ppt => "PPT",
        Format::Pptx => "PPTX",
        Format::Rtf => "RTF",
        Format::Epub => "EPUB",
        Format::Excel => "XLSX",
        Format::Ods => "ODS",
        Format::Odp => "ODP",
        Format::Csv => "CSV",
    }
}

/// anydoc 的 Markdown 只保留图片 alt 文本；本切片不写资产，
/// 在 UI 中明确说明，避免把“有图片”当成隐式承诺。
pub fn extract(input: &Path) -> Result<Extracted, AppError> {
    let bytes = fs::read(input).map_err(|e| {
        AppError::new(
            "E_IO",
            format!("读取文件失败: {e}"),
            "确认文件仍存在并具有读取权限",
        )
    })?;
    let format = detect_format(&bytes, input).ok_or_else(|| {
        AppError::new(
            "E_UNSUPPORTED_FORMAT",
            "无法识别该文件的格式",
            "请选择 docx、xlsx、pptx、rtf、epub、csv 或文本型 PDF；Markdown 输入将在后续版本支持",
        )
    })?;

    let markdown = anydoc::to_markdown_bytes(&bytes, format).map_err(map_error)?;
    Ok(Extracted {
        markdown,
        format: format_label(format).to_string(),
    })
}

pub fn stats(markdown: &str) -> TaskStats {
    TaskStats {
        chars: markdown.chars().count(),
        headings: markdown
            .lines()
            .filter(|line| line.trim_start().starts_with('#'))
            .count(),
        table_rows: markdown
            .lines()
            .filter(|line| line.trim_start().starts_with('|'))
            .count(),
    }
}

fn map_error(error: ConvertError) -> AppError {
    match error {
        ConvertError::NeedsOcr { pages, page_count } => AppError::new(
            "E_NEEDS_OCR",
            format!("该 PDF 有 {} 页没有文本层", pages.len()),
            "OCR 组件将在后续版本接入；请先转 Markdown 或提供文本型 PDF",
        )
        .with_detail(serde_json::json!({ "pages": pages, "pageCount": page_count })),
        ConvertError::Encrypted => AppError::new(
            "E_ENCRYPTED",
            "文档已加密或受密码保护",
            "请先解密文件后重试",
        ),
        ConvertError::Unsupported(detail) => AppError::new(
            "E_UNSUPPORTED_FORMAT",
            format!("该格式暂不支持: {detail}"),
            "选择受支持的文件，或等待后续版本接入更多格式",
        ),
        ConvertError::Malformed { detail, .. } => AppError::new(
            "E_PARSE_FAILED",
            format!("文档结构无法解析: {detail}"),
            "尝试用原应用重新导出，或选择其他文件",
        ),
        ConvertError::MissingPart { part } => AppError::new(
            "E_PARSE_FAILED",
            format!("文档缺少必要内容: {part}"),
            "尝试重新导出文件",
        ),
        ConvertError::ResourceLimit { limit, detail } => AppError::new(
            "E_RESOURCE_LIMIT",
            format!("文件超过安全限制（{limit}）: {detail}"),
            "拆分文件后重试",
        ),
        ConvertError::Io(error) => AppError::new(
            "E_IO",
            format!("读取文件失败: {error}"),
            "确认文件权限后重试",
        ),
        _ => AppError::new(
            "E_PARSE_FAILED",
            "转换引擎返回了未处理的错误",
            "重试或复制诊断信息",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE_DIR: &str = "../example";
    const DOCX: &str = "GB:T 28448-2019 信息安全技术 网络安全等级保护测评要求.docx";
    const XLSX: &str = "等级保护测评2.0要求指标清单v1.xlsx";
    const PDF: &str = "关于印发《网络安全等级保护测评高风险判定实施指引(试行 )》的通知).pdf";

    fn sample(name: &str) -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(EXAMPLE_DIR)
            .join(name)
    }

    #[test]
    fn stats_count_headings_and_table_rows() {
        let markdown = "# 标题\n\n正文\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\n## 二级\n";
        let stats = stats(markdown);
        assert_eq!(stats.headings, 2);
        assert_eq!(stats.table_rows, 3);
        assert!(stats.chars > 0);
    }

    #[test]
    fn unknown_extension_is_not_detected() {
        assert!(detect_format(b"", Path::new("unknown.bin")).is_none());
    }

    #[test]
    #[ignore = "依赖本地 example 样本，CI 无样例时跳过"]
    fn real_docx_produces_headings_and_tables() {
        let output = extract(&sample(DOCX)).expect("docx 应能转换");
        let stats = stats(&output.markdown);
        assert_eq!(output.format, "DOCX");
        assert!(stats.headings >= 40, "headings={}", stats.headings);
        assert!(stats.table_rows > 0, "tableRows={}", stats.table_rows);
    }

    #[test]
    #[ignore = "依赖本地 example 样本，CI 无样例时跳过"]
    fn real_xlsx_produces_tables() {
        let output = extract(&sample(XLSX)).expect("xlsx 应能转换");
        let stats = stats(&output.markdown);
        assert_eq!(output.format, "XLSX");
        assert!(stats.table_rows > 100, "tableRows={}", stats.table_rows);
    }

    #[test]
    #[ignore = "依赖本地 example 样本，CI 无样例时跳过"]
    fn pdf_without_text_layer_needs_ocr() {
        let error = extract(&sample(PDF)).expect_err("无文本封面应要求 OCR");
        assert_eq!(error.code, "E_NEEDS_OCR");
    }
}
