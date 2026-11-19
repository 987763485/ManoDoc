//! 图片资产落盘与 Markdown 链接改写。
//!
//! xberg 在 Markdown 中注入 `![..](embedded:pN_iM)` 占位；这里按内容去重写入
//! `assets/<文档名>/`，并把占位替换为相对路径或 base64 data URI。

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::engine::xberg::XbergImage;
use crate::models::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageMode {
    Assets,
    Base64,
    Alt,
}

impl ImageMode {
    pub fn parse(value: &str) -> Self {
        match value {
            "base64" => Self::Base64,
            "alt" => Self::Alt,
            _ => Self::Assets,
        }
    }
}

fn extension(format: &str) -> &str {
    match format.to_ascii_lowercase().as_str() {
        "jpeg" | "jpg" => "jpg",
        "png" => "png",
        "webp" => "webp",
        "gif" => "gif",
        "bmp" => "bmp",
        "tiff" | "tif" => "tiff",
        _ => "bin",
    }
}

pub struct Materialized {
    pub markdown: String,
    pub files: Vec<String>,
    pub warnings: Vec<String>,
}

/// `link_prefix` 是写入 Markdown 的相对路径前缀（例如 `assets/报告`）。
pub fn materialize(
    markdown: &str,
    images: &[XbergImage],
    assets_dir: &Path,
    link_prefix: &str,
    mode: ImageMode,
) -> Result<Materialized, AppError> {
    if images.is_empty() || mode == ImageMode::Alt {
        return Ok(Materialized {
            markdown: markdown.to_string(),
            files: Vec::new(),
            warnings: Vec::new(),
        });
    }
    fs::create_dir_all(assets_dir).map_err(|error| {
        AppError::new(
            "E_OUTPUT_PERMISSION",
            format!("无法创建图片目录: {error}"),
            "检查输出目录权限",
        )
    })?;

    let mut by_hash: HashMap<String, String> = HashMap::new();
    let mut files = Vec::new();
    let mut warnings = Vec::new();
    let mut result = markdown.to_string();

    for (position, image) in images.iter().enumerate() {
        let hash = format!("{:x}", Sha256::digest(&image.data));
        let name = if let Some(existing) = by_hash.get(&hash) {
            existing.clone()
        } else {
            let name = format!("image-{}.{}", position + 1, extension(&image.format));
            let path = assets_dir.join(&name);
            if let Err(error) = fs::write(&path, &image.data) {
                warnings.push(format!("写入图片 {} 失败: {error}", image.placeholder));
                continue;
            }
            by_hash.insert(hash, name.clone());
            files.push(name.clone());
            name
        };

        let replacement = if mode == ImageMode::Base64 {
            format!(
                "data:image/{};base64,{}",
                extension(&image.format),
                base64_encode(&image.data)
            )
        } else {
            format!("{link_prefix}/{name}")
        };
        result = result.replace(&format!("embedded:{}", image.placeholder.trim_start_matches("embedded:")), &replacement);
        result = result.replace(&image.placeholder, &replacement);
        // xberg 的 Markdown 渲染器实际写的是 `image_<index>.<format>`；与占位符
        // 一起替换，避免输出保留指向不存在文件的相对链接。
        result = result.replace(&format!("image_{}.{}", image.index, image.format), &replacement);
    }

    if result.contains("embedded:") {
        warnings.push("部分图片占位未能匹配，已保留原始引用".into());
    }

    Ok(Materialized { markdown: result, files, warnings })
}

/// 最小 base64 实现，避免为单个功能引入额外依赖。
fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let value = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(TABLE[(value >> 18) as usize & 63] as char);
        out.push(TABLE[(value >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            TABLE[(value >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[value as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_base64_padding() {
        assert_eq!(base64_encode(b"a"), "YQ==");
        assert_eq!(base64_encode(b"ab"), "YWI=");
        assert_eq!(base64_encode(b"abc"), "YWJj");
    }

    #[test]
    fn rewrites_placeholders_without_duplicates() {
        let dir = std::env::temp_dir().join(format!("dull-assets-{}", uuid::Uuid::new_v4()));
        let images = vec![
            XbergImage {
                placeholder: "embedded:p1_i0".into(),
                format: "png".into(),
                data: vec![1, 2, 3],
                page: Some(1),
                index: 0,
            },
            XbergImage {
                placeholder: "embedded:p1_i1".into(),
                format: "png".into(),
                data: vec![1, 2, 3],
                page: Some(1),
                index: 1,
            },
        ];
        let markdown = "![a](embedded:p1_i0)\n![b](embedded:p1_i1)";
        let result = materialize(markdown, &images, &dir, "assets/doc", ImageMode::Assets).unwrap();
        assert_eq!(result.files.len(), 1, "相同内容应去重");
        assert!(result.markdown.contains("(assets/doc/image-1.png)"));
        assert!(!result.markdown.contains("embedded:"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn rewrites_renderer_image_names() {
        let dir = std::env::temp_dir().join(format!("dull-assets-{}", uuid::Uuid::new_v4()));
        let images = vec![XbergImage {
            placeholder: "embedded:p2_i3".into(),
            format: "png".into(),
            data: vec![9, 9, 9],
            page: Some(2),
            index: 3,
        }];
        let result = materialize("![](image_3.png)", &images, &dir, "assets/doc", ImageMode::Assets).unwrap();
        assert_eq!(result.markdown, "![](assets/doc/image-1.png)");
        assert!(dir.join("image-1.png").is_file());
        let _ = fs::remove_dir_all(dir);
    }
}
