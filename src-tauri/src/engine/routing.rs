//! 引擎路由与回退。
//!
//! `auto` 优先 anydoc；只有需要 OCR、解析失败或表格质量为 C 时才切换 xberg。
//! 路由只决定引擎，不改变输出方向；每次回退都带可解释的原因。

use std::fs;
use std::path::Path;

use anydoc::Format;

use crate::clean;
use crate::engine::anydoc as anydoc_engine;
use crate::engine::xberg as xberg_engine;
use crate::models::AppError;
use crate::ocr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineChoice {
    Auto,
    Anydoc,
    Xberg,
}

impl EngineChoice {
    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "anydoc" => Self::Anydoc,
            "xberg" => Self::Xberg,
            _ => Self::Auto,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Anydoc => "anydoc",
            Self::Xberg => "xberg",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Extracted {
    pub markdown: String,
    pub format: String,
    pub engine: String,
    pub engine_choice: String,
    pub fallback_reason: Option<String>,
    pub ocr_pages: Vec<u32>,
    pub ocr_engine: Option<String>,
    pub warnings: Vec<String>,
    pub pages: Option<usize>,
    pub method: Option<String>,
}

#[derive(Debug, Clone)]
pub enum InputKind {
    Document(Format),
    Image,
}

/// 内容优先识别；PNG/JPEG 没有 anydoc Format，需要单独走 OCR。
pub fn detect(bytes: &[u8], path: &Path) -> Option<InputKind> {
    if let Some(format) = anydoc_engine::detect_format(bytes, path) {
        return Some(InputKind::Document(format));
    }
    if is_png(bytes) || is_jpeg(bytes) {
        return Some(InputKind::Image);
    }
    match path
        .extension()
        .map(|ext| ext.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
        .as_str()
    {
        "png" => Some(InputKind::Image),
        "jpg" | "jpeg" => Some(InputKind::Image),
        _ => None,
    }
}

pub fn detect_path(path: &Path) -> Option<InputKind> {
    let bytes = fs::read(path).ok()?;
    detect(&bytes, path)
}

pub fn format_label(kind: &InputKind) -> &'static str {
    match kind {
        InputKind::Document(format) => anydoc_engine::format_label(*format),
        InputKind::Image => "PNG/JPEG",
    }
}

fn is_png(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A])
}

fn is_jpeg(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0xFF, 0xD8, 0xFF])
}

fn needs_ocr_error(message: &str, next_step: &str, pages: Vec<u32>, page_count: usize) -> AppError {
    AppError::new("E_NEEDS_OCR", message, next_step).with_detail(serde_json::json!({
        "pages": pages,
        "pageCount": page_count,
        "ocrAvailable": ocr::status().ready,
        "modelDir": ocr::model_dir().to_string_lossy()
    }))
}

fn from_anydoc(
    extracted: anydoc_engine::Extracted,
    choice: EngineChoice,
    fallback_reason: Option<String>,
    warnings: Vec<String>,
) -> Extracted {
    Extracted {
        markdown: extracted.markdown,
        format: extracted.format,
        engine: "anydoc".into(),
        engine_choice: choice.as_str().into(),
        fallback_reason,
        ocr_pages: Vec::new(),
        ocr_engine: None,
        warnings,
        pages: None,
        method: Some("native".into()),
    }
}

fn from_xberg(
    extracted: xberg_engine::XbergExtracted,
    format: String,
    choice: EngineChoice,
    fallback_reason: Option<String>,
) -> Extracted {
    let ocr_engine = if extracted.method == "native" {
        None
    } else {
        Some("paddleocr".to_string())
    };
    Extracted {
        markdown: extracted.markdown,
        format,
        engine: "xberg".into(),
        engine_choice: choice.as_str().into(),
        fallback_reason,
        ocr_pages: extracted.ocr_pages,
        ocr_engine,
        warnings: extracted.warnings,
        pages: Some(extracted.pages),
        method: Some(extracted.method),
    }
}

/// 从 xberg 的失败中提取页信息，保证错误卡能说明“哪几页需要 OCR”。
fn ocr_pages_from(error: &AppError) -> (Vec<u32>, usize) {
    let detail = error.detail.as_ref();
    let pages = detail
        .and_then(|value| value.get("pages"))
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_u64().map(|value| value as u32))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let count = detail
        .and_then(|value| value.get("pageCount"))
        .and_then(|value| value.as_u64())
        .unwrap_or(0) as usize;
    (pages, count)
}

/// `on_stage` 用于把 OCR 阶段反馈给前端；路由本身不依赖 UI。
pub fn extract<F: FnMut(&str)>(
    input: &Path,
    choice: EngineChoice,
    mut on_stage: F,
) -> Result<Extracted, AppError> {
    let bytes = fs::read(input).map_err(|error| {
        AppError::new(
            "E_IO",
            format!("读取文件失败: {error}"),
            "确认文件仍存在并具有读取权限",
        )
    })?;
    let kind = detect(&bytes, input).ok_or_else(|| {
        AppError::new(
            "E_UNSUPPORTED_FORMAT",
            "无法识别该文件的格式",
            "请选择 docx、xlsx、pptx、rtf、epub、csv、文本型 PDF、PNG 或 JPEG",
        )
    })?;
    let label = format_label(&kind).to_string();
    let ocr_ready = ocr::status().ready;

    match (&kind, choice) {
        (InputKind::Image, EngineChoice::Anydoc) => Err(AppError::new(
            "E_UNSUPPORTED_FORMAT",
            "图片输入需要使用 OCR 引擎",
            "把引擎切换为「自动」或 xberg 后重试",
        )),
        (InputKind::Image, _) => {
            if !ocr_ready {
                return Err(needs_ocr_error(
                    "该图片需要 OCR，但 OCR 模型未安装",
                    "在系统设置的引擎页安装 OCR 模型后重试",
                    Vec::new(),
                    1,
                ));
            }
            on_stage("ocr");
            let extracted = xberg_engine::extract(input, true)?;
            Ok(from_xberg(extracted, label, choice, None))
        }
        (InputKind::Document(_), EngineChoice::Anydoc) => {
            let extracted = anydoc_engine::extract(input).map_err(|error| {
                if error.code == "E_NEEDS_OCR" {
                    let (pages, count) = ocr_pages_from(&error);
                    needs_ocr_error(
                        &format!("该 PDF 有 {} 页没有文本层", pages.len().max(1)),
                        if ocr_ready {
                            "切换到「自动」或 xberg 引擎后重试"
                        } else {
                            "在系统设置的引擎页安装 OCR 模型后重试"
                        },
                        pages,
                        count,
                    )
                } else {
                    error
                }
            })?;
            Ok(from_anydoc(extracted, choice, None, Vec::new()))
        }
        (InputKind::Document(_), EngineChoice::Xberg) => {
            on_stage(if ocr_ready { "ocr" } else { "converting" });
            match xberg_engine::extract(input, ocr_ready) {
                Ok(extracted) => Ok(from_xberg(extracted, label, choice, None)),
                Err(error) if error.code == "E_NEEDS_OCR" => Err(needs_ocr_error(
                    &error.message,
                    &error.next_step,
                    ocr_pages_from(&error).0,
                    ocr_pages_from(&error).1,
                )),
                Err(error) => Err(error),
            }
        }
        (InputKind::Document(_), EngineChoice::Auto) => {
            let anydoc_result = anydoc_engine::extract(input);
            match anydoc_result {
                Ok(extracted) => {
                    // 表格 C 级说明 anydoc 的结构还原不可靠，交给 xberg 再试一次。
                    let grade = clean::table_grade(&extracted.markdown);
                    if grade == "C" && ocr_ready {
                        on_stage("ocr");
                        if let Ok(retried) = xberg_engine::extract(input, true) {
                            let reason = Some("anydoc:table-grade-c".to_string());
                            return Ok(from_xberg(retried, label, choice, reason));
                        }
                    }
                    let warnings = if grade == "C" {
                        vec!["表格质量为 C 级，建议人工复核".to_string()]
                    } else {
                        Vec::new()
                    };
                    Ok(from_anydoc(extracted, choice, None, warnings))
                }
                Err(error) if error.code == "E_NEEDS_OCR" => {
                    let (pages, count) = ocr_pages_from(&error);
                    if !ocr_ready {
                        return Err(needs_ocr_error(
                            &format!("该 PDF 有 {} 页没有文本层", pages.len().max(1)),
                            "在系统设置的引擎页安装 OCR 模型后重试",
                            pages,
                            count,
                        ));
                    }
                    on_stage("ocr");
                    let retried = xberg_engine::extract(input, true).map_err(|error| {
                        if error.code == "E_NEEDS_OCR" {
                            needs_ocr_error(
                                &error.message,
                                &error.next_step,
                                pages.clone(),
                                count,
                            )
                        } else {
                            error
                        }
                    })?;
                    Ok(from_xberg(
                        retried,
                        label,
                        choice,
                        Some("anydoc:needs-ocr".into()),
                    ))
                }
                Err(error) => {
                    // 解析失败时 xberg 原生抽取仍可能成功；OCR 模型只在需要时加载。
                    on_stage(if ocr_ready { "ocr" } else { "converting" });
                    match xberg_engine::extract(input, ocr_ready) {
                        Ok(retried) => Ok(from_xberg(
                            retried,
                            label,
                            choice,
                            Some(format!("anydoc:{}", error.code)),
                        )),
                        Err(xberg_error) if xberg_error.code == "E_NEEDS_OCR" => {
                            Err(needs_ocr_error(
                                &xberg_error.message,
                                &xberg_error.next_step,
                                pages_from_detail(&xberg_error),
                                0,
                            ))
                        }
                        Err(_) => Err(error),
                    }
                }
            }
        }
    }
}

fn pages_from_detail(error: &AppError) -> Vec<u32> {
    error
        .detail
        .as_ref()
        .and_then(|value| value.get("pages"))
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_u64().map(|value| value as u32))
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_engine_choice() {
        assert_eq!(EngineChoice::parse("auto"), EngineChoice::Auto);
        assert_eq!(EngineChoice::parse("anydoc"), EngineChoice::Anydoc);
        assert_eq!(EngineChoice::parse("xberg"), EngineChoice::Xberg);
        assert_eq!(EngineChoice::parse("unknown"), EngineChoice::Auto);
    }

    #[test]
    fn detects_png_by_signature() {
        let bytes = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0];
        assert!(matches!(
            detect(&bytes, Path::new("sample.png")),
            Some(InputKind::Image)
        ));
    }

    #[test]
    fn detects_jpeg_by_signature() {
        let bytes = [0xFF, 0xD8, 0xFF, 0xE0, 0, 0];
        assert!(matches!(
            detect(&bytes, Path::new("sample.jpg")),
            Some(InputKind::Image)
        ));
    }

    #[test]
    #[ignore = "依赖本地 example 样本与已安装的 OCR 模型"]
    fn real_scanned_pdf_routes_to_xberg_ocr() {
        // 封面页无文本层的 50 页样本，用来验证 auto 路由会回退 xberg。
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../example/关于印发《网络安全等级保护测评高风险判定实施指引(试行 )》的通知).pdf");
        let output = extract(&path, EngineChoice::Auto, |_| {}).expect("扫描 PDF 应完成 OCR");
        assert_eq!(output.engine, "xberg");
        assert_eq!(output.fallback_reason.as_deref(), Some("anydoc:needs-ocr"));
        assert!(!output.ocr_pages.is_empty());
        assert!(output.markdown.chars().count() > 1000);
    }

    #[test]
    #[ignore = "依赖 DULLDOC_TEST_IMAGE 指向的本地图片与已安装的 OCR 模型"]
    fn real_image_routes_to_ocr() {
        let path = std::env::var("DULLDOC_TEST_IMAGE").expect("需要 DULLDOC_TEST_IMAGE");
        let output = extract(Path::new(&path), EngineChoice::Auto, |_| {}).expect("图片应完成 OCR");
        assert_eq!(output.engine, "xberg");
        assert_eq!(output.ocr_engine.as_deref(), Some("paddleocr"));
        assert!(output.markdown.chars().count() > 10);
    }

    #[test]
    #[ignore = "依赖本地 example 样本"]
    fn text_document_stays_on_anydoc() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../example/GB:T 28448-2019 信息安全技术 网络安全等级保护测评要求.docx");
        let output = extract(&path, EngineChoice::Auto, |_| {}).expect("docx 应转换");
        assert_eq!(output.engine, "anydoc");
        assert!(output.ocr_pages.is_empty());
        assert!(output.ocr_engine.is_none());
    }

    #[test]
    #[ignore = "临时把模型目录指向空目录，验证无模型时的降级"]
    fn missing_models_return_needs_ocr() {
        let empty = std::env::temp_dir().join(format!("dulldoc-empty-models-{}", uuid::Uuid::new_v4()));
        std::env::set_var("DULLDOC_MODEL_DIR", &empty);
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../example/关于印发《网络安全等级保护测评高风险判定实施指引(试行 )》的通知).pdf");
        let error = extract(&path, EngineChoice::Auto, |_| {}).expect_err("无模型时扫描 PDF 应失败");
        std::env::remove_var("DULLDOC_MODEL_DIR");
        assert_eq!(error.code, "E_NEEDS_OCR");
        assert!(error.next_step.contains("OCR"));
        let _ = fs::remove_dir_all(empty);
    }
}
