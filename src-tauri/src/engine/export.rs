//! 目标格式能力矩阵与导出管线。
//!
//! Markdown 是中间格式：文档先由 anydoc/xberg 提取，再由 pandoc/Typst 导出；
//! Markdown 输入直通，不重新解析。

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::models::AppError;
use crate::storage;

use super::pandoc;
use super::routing::{self, EngineChoice, InputKind};
use super::typst;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetFormat {
    Markdown,
    Docx,
    Pdf,
    Html,
    Epub,
}

impl TargetFormat {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "md" | "markdown" => Some(Self::Markdown),
            "docx" | "word" => Some(Self::Docx),
            "pdf" => Some(Self::Pdf),
            "html" => Some(Self::Html),
            "epub" => Some(Self::Epub),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Markdown => "Markdown",
            Self::Docx => "Word",
            Self::Pdf => "PDF",
            Self::Html => "HTML",
            Self::Epub => "EPUB",
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::Markdown => "md",
            Self::Docx => "docx",
            Self::Pdf => "pdf",
            Self::Html => "html",
            Self::Epub => "epub",
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::Markdown => "markdown",
            Self::Docx => "docx",
            Self::Pdf => "pdf",
            Self::Html => "html",
            Self::Epub => "epub",
        }
    }
}

/// 按来源返回可用目标；顺序即界面默认顺序。
pub fn targets_for(kind: &InputKind) -> Vec<TargetFormat> {
    match kind {
        InputKind::Image => vec![TargetFormat::Markdown],
        InputKind::Markdown => vec![
            TargetFormat::Docx,
            TargetFormat::Pdf,
            TargetFormat::Html,
            TargetFormat::Epub,
        ],
        InputKind::Html => vec![TargetFormat::Markdown],
        InputKind::Document(format) => match format {
            anydoc::Format::Pdf => vec![TargetFormat::Markdown, TargetFormat::Docx],
            _ => vec![TargetFormat::Markdown],
        },
    }
}

#[derive(Debug)]
pub struct ExportResult {
    pub output_path: PathBuf,
    /// 用于统计的 Markdown 文本（导出二进制时取中间 Markdown）。
    pub stats_markdown: String,
    pub target: TargetFormat,
    pub engine: String,
    pub source_engine: Option<String>,
    pub fallback_reason: Option<String>,
    pub ocr_pages: Vec<u32>,
    pub ocr_engine: Option<String>,
    pub warnings: Vec<String>,
}

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Result<Self, AppError> {
        let root = storage::data_dir()
            .map_err(|error| AppError::new("E_IO", error, "检查用户目录权限"))?
            .join("tmp")
            .join(format!("export-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).map_err(|error| {
            AppError::new(
                "E_OUTPUT_PERMISSION",
                format!("无法创建临时目录: {error}"),
                "检查 ~/.dull_doc 是否可写",
            )
        })?;
        Ok(Self(root))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// 与文本输出共用同名规则：`stem.ext`、`stem-1.ext`……
fn unique_output(root: &Path, stem: &str, extension: &str) -> PathBuf {
    for index in 0..1000 {
        let name = if index == 0 {
            format!("{stem}.{extension}")
        } else {
            format!("{stem}-{index}.{extension}")
        };
        let candidate = root.join(name);
        if !candidate.exists() {
            return candidate;
        }
    }
    root.join(format!("{stem}-{}.{}", uuid::Uuid::new_v4(), extension))
}

fn write_text(path: &Path, text: &str) -> Result<(), AppError> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            AppError::new(
                "E_OUTPUT_PERMISSION",
                format!("无法创建输出文件: {error}"),
                "更换可写目录后重试",
            )
        })?;
    file.write_all(text.as_bytes()).map_err(|error| {
        let _ = fs::remove_file(path);
        AppError::new(
            "E_OUTPUT_PERMISSION",
            format!("写入输出失败: {error}"),
            "检查磁盘空间与目录权限",
        )
    })
}

/// 把 sidecar 在临时目录生成的二进制文件移动到最终唯一路径。
fn commit_generated(temp: &Path, final_path: &Path) -> Result<(), AppError> {
    fs::rename(temp, final_path).map_err(|error| {
        AppError::new(
            "E_OUTPUT_PERMISSION",
            format!("提交输出文件失败: {error}"),
            "检查输出目录权限与磁盘空间",
        )
    })
}

fn title_for(input: &Path) -> String {
    input
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| "DullDoc".into())
}

pub fn convert<F: FnMut(&str)>(
    input: &Path,
    kind: &InputKind,
    target: TargetFormat,
    choice: EngineChoice,
    output_root: &Path,
    mut on_stage: F,
) -> Result<ExportResult, AppError> {
    if !targets_for(kind).contains(&target) {
        return Err(AppError::new(
            "E_UNSUPPORTED_DIRECTION",
            format!("不支持 {} → {}", routing::format_label(kind), target.label()),
            "在转换页选择可用目标格式",
        ));
    }

    let temp = TempDir::new()?;
    let title = title_for(input);
    let mut warnings = Vec::new();
    let mut source_engine = None;
    let mut fallback_reason = None;
    let mut ocr_pages = Vec::new();
    let mut ocr_engine = None;
    let (stats_markdown, engine) = match target {
        TargetFormat::Markdown => match kind {
            InputKind::Markdown => (pandoc::read_markdown(input)?, "direct".to_string()),
            InputKind::Html => {
                on_stage("rendering");
                let converted = temp.path().join("html.md");
                pandoc::html_to_markdown(input, &converted)?;
                (fs::read_to_string(&converted).unwrap_or_default(), "pandoc".to_string())
            }
            _ => {
                on_stage("converting");
                let extracted = routing::extract(input, choice, &mut on_stage)?;
                warnings.extend(extracted.warnings.clone());
                source_engine = Some(extracted.engine.clone());
                fallback_reason = extracted.fallback_reason.clone();
                ocr_pages = extracted.ocr_pages.clone();
                ocr_engine = extracted.ocr_engine.clone();
                (extracted.markdown, extracted.engine)
            }
        },
        TargetFormat::Docx => match kind {
            InputKind::Markdown => {
                on_stage("rendering");
                let markdown = pandoc::read_markdown(input)?;
                let generated = temp.path().join("output.docx");
                pandoc::markdown_to_docx(input, &generated, &title)?;
                (markdown, "pandoc".to_string())
            }
            InputKind::Document(anydoc::Format::Pdf) => {
                on_stage("converting");
                let extracted = routing::extract(input, choice, &mut on_stage)?;
                source_engine = Some(extracted.engine.clone());
                fallback_reason = extracted.fallback_reason.clone();
                ocr_pages = extracted.ocr_pages.clone();
                ocr_engine = extracted.ocr_engine.clone();
                warnings.extend(extracted.warnings.clone());
                warnings.push("PDF → Word 由 PDF → Markdown → DOCX 生成，目录与图片保真有限".into());
                on_stage("rendering");
                let markdown_path = temp.path().join("source.md");
                fs::write(&markdown_path, &extracted.markdown).map_err(|error| {
                    AppError::new(
                        "E_OUTPUT_PERMISSION",
                        format!("写入临时 Markdown 失败: {error}"),
                        "检查 ~/.dull_doc 是否可写",
                    )
                })?;
                let generated = temp.path().join("output.docx");
                pandoc::markdown_to_docx(&markdown_path, &generated, &title)?;
                (extracted.markdown, "pandoc".to_string())
            }
            _ => {
                return Err(AppError::new(
                    "E_UNSUPPORTED_DIRECTION",
                    "该来源暂不支持导出 Word",
                    "先转换为 Markdown，再导出 Word",
                ))
            }
        },
        TargetFormat::Pdf => match kind {
            InputKind::Markdown => {
                let markdown = pandoc::read_markdown(input)?;
                on_stage("rendering");
                let typ = temp.path().join("output.typ");
                pandoc::markdown_to_typst(input, &typ, &title)?;
                let generated = temp.path().join("output.pdf");
                // Typst 要求源文件位于 --root 内；临时目录同时容纳 .typ 与 PDF。
                typst::compile(&typ, &generated, temp.path())?;
                (markdown, "pandoc + typst".to_string())
            }
            _ => {
                return Err(AppError::new(
                    "E_UNSUPPORTED_DIRECTION",
                    "Markdown → PDF 仅支持 Markdown 输入",
                    "先转换为 Markdown，再导出 PDF",
                ))
            }
        },
        TargetFormat::Html => match kind {
            InputKind::Markdown => {
                on_stage("rendering");
                let markdown = pandoc::read_markdown(input)?;
                let generated = temp.path().join("output.html");
                pandoc::markdown_to_html(input, &generated, &title)?;
                (markdown, "pandoc".to_string())
            }
            _ => {
                return Err(AppError::new(
                    "E_UNSUPPORTED_DIRECTION",
                    "该来源暂不支持导出 HTML",
                    "先转换为 Markdown，再导出 HTML",
                ))
            }
        },
        TargetFormat::Epub => match kind {
            InputKind::Markdown => {
                on_stage("rendering");
                let markdown = pandoc::read_markdown(input)?;
                let generated = temp.path().join("output.epub");
                pandoc::markdown_to_epub(input, &generated, &title)?;
                (markdown, "pandoc".to_string())
            }
            _ => {
                return Err(AppError::new(
                    "E_UNSUPPORTED_DIRECTION",
                    "该来源暂不支持导出 EPUB",
                    "先转换为 Markdown，再导出 EPUB",
                ))
            }
        },
    };

    on_stage("writing");
    let final_path = unique_output(output_root, &title, target.extension());
    if target == TargetFormat::Markdown {
        write_text(&final_path, &stats_markdown)?;
    } else {
        let temp_name = match target {
            TargetFormat::Docx => "output.docx",
            TargetFormat::Pdf => "output.pdf",
            TargetFormat::Html => "output.html",
            TargetFormat::Epub => "output.epub",
            TargetFormat::Markdown => unreachable!(),
        };
        let generated = temp.path().join(temp_name);
        if !generated.is_file() {
            return Err(AppError::new(
                "E_ENGINE_UNAVAILABLE",
                "导出引擎没有生成输出文件",
                "复制诊断信息后重试",
            ));
        }
        commit_generated(&generated, &final_path)?;
    }

    Ok(ExportResult {
        output_path: final_path,
        stats_markdown,
        target,
        engine,
        source_engine,
        fallback_reason,
        ocr_pages,
        ocr_engine,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_matrix_matches_prd() {
        assert_eq!(
            targets_for(&InputKind::Markdown),
            vec![
                TargetFormat::Docx,
                TargetFormat::Pdf,
                TargetFormat::Html,
                TargetFormat::Epub
            ]
        );
        assert_eq!(
            targets_for(&InputKind::Document(anydoc::Format::Pdf)),
            vec![TargetFormat::Markdown, TargetFormat::Docx]
        );
        assert_eq!(
            targets_for(&InputKind::Document(anydoc::Format::Docx)),
            vec![TargetFormat::Markdown]
        );
        assert_eq!(targets_for(&InputKind::Image), vec![TargetFormat::Markdown]);
    }

    #[test]
    fn parses_targets_and_extensions() {
        assert_eq!(TargetFormat::parse("docx"), Some(TargetFormat::Docx));
        assert_eq!(TargetFormat::parse("pdf"), Some(TargetFormat::Pdf));
        assert_eq!(TargetFormat::parse("epub"), Some(TargetFormat::Epub));
        assert_eq!(TargetFormat::parse("unknown"), None);
        assert_eq!(TargetFormat::Html.extension(), "html");
        assert_eq!(TargetFormat::Epub.label(), "EPUB");
    }

    #[test]
    #[ignore = "依赖项目内 pandoc/Typst 与 example Markdown"]
    fn real_markdown_exports_all_targets() {
        let input = Path::new(env!("CARGO_MANIFEST_DIR")).join("../example/等保测评skill.md");
        let out = std::env::temp_dir().join(format!("dulldoc-export-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&out).unwrap();
        let kind = InputKind::Markdown;
        for target in [TargetFormat::Docx, TargetFormat::Html, TargetFormat::Epub, TargetFormat::Pdf] {
            let result = convert(&input, &kind, target, EngineChoice::Auto, &out, |_| {})
                .unwrap_or_else(|error| panic!("{target:?}: {}", error.message));
            let bytes = fs::read(&result.output_path).unwrap();
            assert!(!bytes.is_empty(), "{target:?} 输出为空");
            match target {
                TargetFormat::Pdf => assert!(bytes.starts_with(b"%PDF")),
                TargetFormat::Epub => assert!(bytes.starts_with(b"PK")),
                _ => {}
            }
            assert!(result.output_path.extension().map(|ext| ext == target.extension()).unwrap_or(false));
        }
        let _ = fs::remove_dir_all(out);
    }

    #[test]
    #[ignore = "依赖项目内 pandoc；用 /usr/bin/false 模拟 Typst 编译失败"]
    fn typst_failure_returns_compile_error() {
        let input = Path::new(env!("CARGO_MANIFEST_DIR")).join("../example/等保测评skill.md");
        let out = std::env::temp_dir().join(format!("dulldoc-typstfail-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&out).unwrap();
        std::env::set_var("DULLDOC_TYPST", "/usr/bin/false");
        let error = convert(
            &input,
            &InputKind::Markdown,
            TargetFormat::Pdf,
            EngineChoice::Auto,
            &out,
            |_| {},
        )
        .expect_err("Typst 失败应返回错误");
        std::env::remove_var("DULLDOC_TYPST");
        assert_eq!(error.code, "E_PDF_COMPILE_FAILED");
        assert_eq!(fs::read_dir(&out).unwrap().count(), 0, "失败不应产生输出");
        let _ = fs::remove_dir_all(out);
    }

    #[test]
    #[ignore = "依赖项目内 pandoc、Typst 与已安装 OCR 模型的 example PDF"]
    fn real_pdf_to_docx_reports_quality_limit() {
        let input = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../example/关于印发《网络安全等级保护测评高风险判定实施指引(试行 )》的通知).pdf");
        let out = std::env::temp_dir().join(format!("dulldoc-pdfdocx-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&out).unwrap();
        let result = convert(
            &input,
            &InputKind::Document(anydoc::Format::Pdf),
            TargetFormat::Docx,
            EngineChoice::Auto,
            &out,
            |_| {},
        )
        .expect("PDF → Word 应完成");
        assert!(fs::metadata(&result.output_path).map(|meta| meta.len() > 10_000).unwrap_or(false));
        assert!(result.warnings.iter().any(|warning| warning.contains("保真")));
        assert_eq!(result.engine, "pandoc");
        let _ = fs::remove_dir_all(out);
    }
}
