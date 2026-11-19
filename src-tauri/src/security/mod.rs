use std::fs;
use std::path::{Path, PathBuf};

use crate::models::AppError;
use crate::storage;

/// 输入必须先规范化并确认是普通文件，防止目录、设备文件与符号链接越界。
pub fn canonical_input(path: &str) -> Result<PathBuf, AppError> {
    let input = Path::new(path)
        .canonicalize()
        .map_err(|e| AppError::new("E_IO", format!("无法访问文件: {e}"), "确认文件仍存在"))?;
    if !input.is_file() {
        return Err(AppError::new(
            "E_UNSUPPORTED_FORMAT",
            "请选择一个普通文件",
            "目录与设备文件不在本切片范围内",
        ));
    }
    Ok(input)
}

/// 输出目录只用于拼接文件名；先创建、再规范化，确保后续写入落在稳定路径。
pub fn prepare_output_dir(raw: &str) -> Result<PathBuf, AppError> {
    let dir = storage::expand_tilde(raw);
    fs::create_dir_all(&dir).map_err(|e| {
        AppError::new(
            "E_OUTPUT_PERMISSION",
            format!("无法创建输出目录: {e}"),
            "在系统设置中更换可写目录，或授予文件夹访问权限",
        )
    })?;
    let canonical = dir.canonicalize().map_err(|e| {
        AppError::new(
            "E_OUTPUT_PERMISSION",
            format!("无法访问输出目录: {e}"),
            "更换输出目录后重试",
        )
    })?;
    if !canonical.is_dir() {
        return Err(AppError::new(
            "E_OUTPUT_PERMISSION",
            "输出路径不是目录",
            "更换输出目录后重试",
        ));
    }
    Ok(canonical)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepares_writable_output_dir() {
        let dir = std::env::temp_dir().join(format!("manodoc-test-{}", uuid::Uuid::new_v4()));
        let prepared = prepare_output_dir(&dir.to_string_lossy()).expect("临时目录应可写");
        assert!(prepared.is_dir());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_directory_as_input() {
        let error = canonical_input("/tmp").expect_err("目录不能作为输入");
        assert_eq!(error.code, "E_UNSUPPORTED_FORMAT");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn reports_permission_error_for_read_only_system_path() {
        let target = format!("/System/ManoDocTest-{}", std::process::id());
        let error = prepare_output_dir(&target).expect_err("/System 应不可写");
        assert_eq!(error.code, "E_OUTPUT_PERMISSION");
    }
}
