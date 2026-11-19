//! Typst PDF 编译适配。

use std::path::Path;

use crate::models::AppError;

use super::sidecar::{self, Tool};

pub fn compile(typ: &Path, pdf: &Path, root: &Path) -> Result<(), AppError> {
    let args = vec![
        "compile".into(),
        typ.to_string_lossy().into_owned(),
        pdf.to_string_lossy().into_owned(),
        "--root".into(),
        root.to_string_lossy().into_owned(),
    ];
    let output = sidecar::run(Tool::Typst, &args)?;
    if output.timed_out {
        return Err(AppError::new(
            "E_ENGINE_TIMEOUT",
            "Typst 编译超时",
            "减少图片或分页内容后重试",
        ));
    }
    if !output.success {
        return Err(AppError::new(
            "E_PDF_COMPILE_FAILED",
            format!("PDF 编译失败: {}", sidecar::summarize(&output.stderr)),
            "检查 Markdown 中的图片路径与字体后重试",
        ));
    }
    Ok(())
}
