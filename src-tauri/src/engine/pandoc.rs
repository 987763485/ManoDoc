//! pandoc 导出适配。
//!
//! 只负责构造固定参数并调用 sidecar；目标格式校验与临时文件管理由 export 管线负责。

use std::fs;
use std::path::Path;

use crate::models::AppError;

use super::sidecar::{self, Tool};

fn failure(stderr: &str) -> AppError {
    AppError::new(
        "E_PANDOC_FAILED",
        format!("pandoc 转换失败: {}", sidecar::summarize(stderr)),
        "检查输入 Markdown 是否完整，或复制诊断信息",
    )
}

fn invoke(args: Vec<String>) -> Result<(), AppError> {
    let output = sidecar::run(Tool::Pandoc, &args)?;
    if output.timed_out {
        return Err(AppError::new(
            "E_ENGINE_TIMEOUT",
            "pandoc 执行超时",
            "拆分文档或减少图片后重试",
        ));
    }
    if !output.success {
        return Err(failure(&output.stderr));
    }
    Ok(())
}

fn base_args(input: &Path, output: &Path, title: &str, target: &str) -> Vec<String> {
    let mut args = vec![
        "-f".into(),
        "markdown".into(),
        "-t".into(),
        target.into(),
        "-o".into(),
        output.to_string_lossy().into_owned(),
        "--metadata".into(),
        format!("title={title}"),
    ];
    if let Some(parent) = input.parent() {
        // 相对图片按输入目录解析；本 change 不复制资产。
        args.push(format!("--resource-path={}", parent.to_string_lossy()));
    }
    args.push(input.to_string_lossy().into_owned());
    args
}

pub fn markdown_to_docx(input: &Path, output: &Path, title: &str) -> Result<(), AppError> {
    invoke(base_args(input, output, title, "docx"))
}

pub fn markdown_to_html(input: &Path, output: &Path, title: &str) -> Result<(), AppError> {
    let mut args = base_args(input, output, title, "html5");
    // HTML 需要完整骨架，否则只有正文片段。
    args.insert(0, "--standalone".into());
    invoke(args)
}

pub fn markdown_to_epub(input: &Path, output: &Path, title: &str) -> Result<(), AppError> {
    invoke(base_args(input, output, title, "epub3"))
}

/// 生成 Typst 源文件；随后由 typst sidecar 编译 PDF。
pub fn markdown_to_typst(input: &Path, output: &Path, title: &str) -> Result<(), AppError> {
    invoke(base_args(input, output, title, "typst"))
}

pub fn html_to_markdown(input: &Path, output: &Path) -> Result<(), AppError> {
    let args = vec![
        "-f".into(),
        "html".into(),
        "-t".into(),
        "gfm".into(),
        "-o".into(),
        output.to_string_lossy().into_owned(),
        input.to_string_lossy().into_owned(),
    ];
    invoke(args)
}

/// 读取 Markdown 输入；直通导出不重新解析。
pub fn read_markdown(path: &Path) -> Result<String, AppError> {
    fs::read_to_string(path).map_err(|error| {
        AppError::new(
            "E_IO",
            format!("读取 Markdown 失败: {error}"),
            "确认文件存在且为 UTF-8 文本",
        )
    })
}
