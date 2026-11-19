mod anydoc;
pub mod export;
mod pandoc;
pub mod routing;
mod sidecar;
mod typst;
pub mod xberg;

pub use anydoc::stats;
pub use export::TargetFormat;
pub use routing::{extract, format_label as input_format_label, EngineChoice, InputKind};
pub use sidecar::status as sidecar_status;

/// 兼容旧调用：内容识别优先于扩展名。
pub fn detect_input(bytes: &[u8], path: &std::path::Path) -> Option<InputKind> {
    routing::detect(bytes, path)
}
