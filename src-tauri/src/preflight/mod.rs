//! PDF 页级预检：统计每页非空白字符，输出无文本页、占比与建议策略。
//!
//! 预检只读文本层，不触发 OCR；扫描件会得到空页列表，再由 UI 让用户选择
//! 自动 OCR、跳过或失败。

use std::path::Path;

use serde::Serialize;

use crate::engine::xberg::page_texts;
use crate::models::AppError;
use crate::ocr;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageCheck {
    pub page: u32,
    pub chars: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreflightResult {
    pub page_count: u32,
    pub pages_without_text: Vec<u32>,
    pub pages: Vec<PageCheck>,
    pub ratio: f64,
    pub ocr_available: bool,
    pub over_threshold: bool,
    pub suggested: String,
}

/// 只读页数，用于文件信息展示；失败时返回 None，不阻塞加载。
pub fn page_count(path: &Path) -> Option<u32> {
    let document = lopdf::Document::load(path).ok()?;
    u32::try_from(document.get_pages().len()).ok()
}

/// 把无文本页从副本中删除，供「跳过并继续」策略使用。
pub fn strip_pages(path: &Path, skip: &[u32], target: &Path) -> Result<(), AppError> {
    let mut document = lopdf::Document::load(path).map_err(|error| {
        AppError::new(
            "E_PREFLIGHT",
            format!("无法读取 PDF: {error}"),
            "尝试重新导出 PDF 后重试",
        )
    })?;
    let existing = document
        .get_pages()
        .keys()
        .copied()
        .filter(|page| skip.contains(page))
        .collect::<Vec<u32>>();
    if existing.len() >= document.get_pages().len() {
        return Err(AppError::new(
            "E_PREFLIGHT",
            "所有页面都没有文本层，无法跳过",
            "安装 OCR 模型后重试",
        ));
    }
    document.delete_pages(&existing);
    document.save(target).map(|_| ()).map_err(|error| {
        AppError::new(
            "E_PREFLIGHT",
            format!("写入跳过页后的 PDF 失败: {error}"),
            "检查临时目录权限",
        )
    })
}

pub fn inspect(path: &Path) -> Result<PreflightResult, AppError> {
    let pages = page_texts(path)?;
    let page_count = pages.len() as u32;
    let checks = pages
        .into_iter()
        .map(|(page, text)| PageCheck {
            page,
            chars: text.chars().filter(|c| !c.is_whitespace()).count(),
        })
        .collect::<Vec<_>>();
    let without = checks
        .iter()
        .filter(|check| check.chars < 3)
        .map(|check| check.page)
        .collect::<Vec<_>>();
    let ratio = if page_count == 0 {
        0.0
    } else {
        without.len() as f64 / page_count as f64
    };
    let ocr_available = ocr::status().ready;
    let over_threshold = without.len() > 3 || ratio > 0.10;
    let suggested = if without.is_empty() {
        "continue"
    } else if ocr_available {
        "ocr"
    } else if over_threshold {
        "fail"
    } else {
        "skip"
    }
    .to_string();
    Ok(PreflightResult {
        page_count,
        pages_without_text: without,
        pages: checks,
        ratio,
        ocr_available,
        over_threshold,
        suggested,
    })
}
