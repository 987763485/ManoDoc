mod anydoc;
pub mod routing;
mod xberg;

pub use anydoc::stats;
pub use routing::{extract, format_label as input_format_label, EngineChoice, InputKind};

/// 兼容旧调用：内容识别优先于扩展名。
pub fn detect_input(bytes: &[u8], path: &std::path::Path) -> Option<InputKind> {
    routing::detect(bytes, path)
}
