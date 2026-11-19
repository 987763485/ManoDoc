//! xberg 第二内核适配器。
//!
//! 只负责把 xberg 的抽取结果翻译成 ManoDoc 的统一结构；引擎选择与回退由
//! `engine::routing` 决定。OCR 模型路径显式传入，避免 xberg 使用默认 HF 缓存。

use std::path::Path;
use std::sync::OnceLock;

use xberg::{
    ExtractionConfig, ExtractionMethod, ExtractInput, ImageExtractionConfig, OcrConfig, OcrStrategy,
    OutputFormat, PageConfig, PaddleOcrConfig,
};

use crate::models::AppError;
use crate::ocr;

#[derive(Debug, Clone)]
pub struct XbergExtracted {
    pub markdown: String,
    pub method: String,
    pub ocr_pages: Vec<u32>,
    pub warnings: Vec<String>,
    pub images: Vec<XbergImage>,
}

#[derive(Debug, Clone)]
pub struct XbergImage {
    /// xberg 注入 Markdown 的占位键，例如 `embedded:p1_i0`。
    pub placeholder: String,
    pub format: String,
    pub data: Vec<u8>,
    pub page: Option<u32>,
    pub index: u32,
}

/// xberg 内部仍可能触达 HF 下载逻辑；转换期间强制离线，缺失模型直接报错。
fn enforce_offline() {
    static OFFLINE: OnceLock<()> = OnceLock::new();
    OFFLINE.get_or_init(|| {
        std::env::set_var("HF_HUB_OFFLINE", "1");
        std::env::set_var("HUGGINGFACE_HUB_OFFLINE", "1");
    });
}

fn ocr_config() -> OcrConfig {
    let mut paddle = PaddleOcrConfig::new("ch");
    paddle.cache_dir = Some(ocr::model_dir());
    paddle.use_angle_cls = false;
    // 表格识别只做词框聚类，不额外推理；扫描表格需要它才有 GFM 输出。
    paddle.enable_table_detection = true;
    paddle.model_version = "pp-ocrv6".into();
    paddle.model_tier = "small".into();

    OcrConfig {
        enabled: true,
        backend: "paddleocr".into(),
        language: vec!["ch".into()],
        paddle_ocr_settings: Some(paddle),
        ..Default::default()
    }
}

/// `ocr_ready=false` 时仍允许文本型文档走 xberg 原生抽取；需要 OCR 的文件由
/// routing 在调用前拦截。
pub fn extract(input: &Path, ocr_ready: bool) -> Result<XbergExtracted, AppError> {
    extract_with_options(input, ocr_ready, false, false)
}

/// `want_images=true` 时要求 xberg 提取图片并注入 `embedded:` 占位；导出层负责
/// 把占位改写为实际 assets 相对路径。
/// `force_ocr=true` 时忽略源文字层，逐页重新 OCR；用于文字层编码异常的 PDF。
pub fn extract_with_options(
    input: &Path,
    ocr_ready: bool,
    want_images: bool,
    force_ocr: bool,
) -> Result<XbergExtracted, AppError> {
    enforce_offline();

    let mut config = ExtractionConfig::default();
    config.output_format = OutputFormat::Markdown;
    // 不启用 xberg 自己的结果缓存，避免在 ~/.manodoc 之外产生额外文件。
    config.use_cache = false;
    config.ocr = if ocr_ready { Some(ocr_config()) } else { None };
    config.ocr_strategy = OcrStrategy::Auto;
    config.force_ocr = force_ocr && ocr_ready;
    config.pages = Some(PageConfig {
        extract_pages: true,
        ..Default::default()
    });
    if want_images {
        config.images = Some(ImageExtractionConfig {
            extract_images: true,
            inject_placeholders: true,
            ..Default::default()
        });
    }

    let path = input.to_path_buf();
    let result = tauri::async_runtime::block_on(xberg::extract(
        ExtractInput::from_uri(path.to_string_lossy().into_owned()),
        &config,
    ))
    .map_err(map_error)?;

    let document = result.results.into_iter().next().ok_or_else(|| {
        AppError::new(
            "E_PARSE_FAILED",
            "转换引擎没有返回结果",
            "重试或复制诊断信息",
        )
    })?;

    let images = document
        .images
        .unwrap_or_default()
        .into_iter()
        .map(|image| XbergImage {
            placeholder: format!(
                "embedded:p{}_i{}",
                image.page_number.unwrap_or(0),
                image.image_index
            ),
            format: image.format.to_string(),
            data: image.data.to_vec(),
            page: image.page_number,
            index: image.image_index,
        })
        .collect::<Vec<_>>();

    let mut ocr_pages = Vec::new();
    if let Some(pages) = document.pages.as_ref() {
        for page in pages {
            // ocr_confidence 只在页面真正经过 OCR 时出现，比猜测文本层更可靠。
            if page.ocr_confidence.is_some() {
                ocr_pages.push(page.page_number);
            }
        }
    }
    ocr_pages.sort_unstable();
    ocr_pages.dedup();

    let method = document
        .extraction_method
        .unwrap_or(ExtractionMethod::Native)
        .as_str()
        .to_string();
    let warnings = document
        .processing_warnings
        .iter()
        .filter_map(|warning| {
            let message = warning.message.to_string();
            if message.contains("No page of this document holds enough prose") {
                // 语言检测无法判断时的解释性告警，对用户没有可执行价值。
                None
            } else if message.contains("PaddleOCR rejected") {
                Some("PaddleOCR 跳过了部分表格候选区域，请人工复核表格".to_string())
            } else if message.contains("suspected OCR recognition noise") {
                Some("OCR 置信度偏低，建议人工复核".to_string())
            } else {
                Some(format!("{}: {}", warning.source, message))
            }
        })
        .collect::<Vec<_>>();
    let pages = if document.counts.pages > 0 {
        document.counts.pages
    } else {
        document.pages.as_ref().map(|pages| pages.len()).unwrap_or(0)
    };

    if document.content.trim().is_empty() {
        return Err(AppError::new(
            "E_NEEDS_OCR",
            if ocr_ready {
                "OCR 未识别出有效文本".to_string()
            } else {
                "该文件需要 OCR，但 OCR 模型未安装".to_string()
            },
            if ocr_ready {
                "检查文件清晰度，或改用其他文件后重试"
            } else {
                "在系统设置的引擎页安装 OCR 模型后重试"
            },
        )
        .with_detail(serde_json::json!({
            "pages": ocr_pages,
            "pageCount": pages,
            "ocrAvailable": ocr_ready,
            "modelDir": ocr::model_dir().to_string_lossy()
        })));
    }

    Ok(XbergExtracted {
        markdown: document.content,
        method,
        ocr_pages,
        warnings,
        images,
    })
}

/// 只读文本层做 PDF 预检：不启用 OCR、不提取图片，返回每页文本。
pub fn page_texts(input: &Path) -> Result<Vec<(u32, String)>, AppError> {
    enforce_offline();
    let mut config = ExtractionConfig::default();
    config.output_format = OutputFormat::Markdown;
    config.use_cache = false;
    config.ocr = None;
    config.pages = Some(PageConfig {
        extract_pages: true,
        ..Default::default()
    });

    let path = input.to_path_buf();
    let result = tauri::async_runtime::block_on(xberg::extract(
        ExtractInput::from_uri(path.to_string_lossy().into_owned()),
        &config,
    ))
    .map_err(map_error)?;

    let document = result.results.into_iter().next().ok_or_else(|| {
        AppError::new(
            "E_PARSE_FAILED",
            "预检没有返回结果",
            "重试或复制诊断信息",
        )
    })?;

    if let Some(pages) = document.pages {
        return Ok(pages
            .into_iter()
            .map(|page| (page.page_number, page.content))
            .collect());
    }
    Ok(vec![(1, document.content)])
}

fn map_error(error: xberg::XbergError) -> AppError {
    use xberg::XbergError;
    match error {
        XbergError::Io(error) => AppError::new(
            "E_IO",
            format!("读取文件失败: {error}"),
            "确认文件仍存在并具有读取权限",
        ),
        XbergError::Parsing { message, .. } => AppError::new(
            "E_PARSE_FAILED",
            format!("xberg 无法解析文档: {message}"),
            "尝试重新导出文件，或选择其他文件",
        ),
        XbergError::Ocr { message, .. } => AppError::new(
            "E_OCR_FAILED",
            format!("OCR 识别失败: {message}"),
            "确认 OCR 模型已安装且文件清晰后重试",
        ),
        XbergError::Validation { message, .. } => AppError::new(
            "E_PARSE_FAILED",
            format!("xberg 配置或输入无效: {message}"),
            "复制诊断信息后重试",
        ),
        XbergError::MissingDependency(detail) => AppError::new(
            "E_ENGINE_UNAVAILABLE",
            format!("xberg 缺少运行依赖: {detail}"),
            "重启应用；若仍失败，请复制诊断信息",
        ),
        other => AppError::new(
            "E_PARSE_FAILED",
            format!("xberg 转换失败: {other}"),
            "重试或复制诊断信息",
        ),
    }
}
