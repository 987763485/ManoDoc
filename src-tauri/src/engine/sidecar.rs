//! 外部 sidecar 解析与受控执行。
//!
//! 所有调用都使用参数数组并带超时；sidecar 路径可以来自开发目录、应用资源或
//! 环境变量，缺失时给出明确错误，不尝试联网或安装。

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::models::AppError;

pub const TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolStatus {
    pub ready: bool,
    pub version: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SidecarStatus {
    pub pandoc: ToolStatus,
    pub typst: ToolStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Pandoc,
    Typst,
}

impl Tool {
    fn env_name(self) -> &'static str {
        match self {
            Self::Pandoc => "DULLDOC_PANDOC",
            Self::Typst => "DULLDOC_TYPST",
        }
    }

    fn binary(self) -> &'static str {
        match self {
            Self::Pandoc => "pandoc",
            Self::Typst => "typst",
        }
    }

    fn missing_code(self) -> &'static str {
        match self {
            Self::Pandoc => "E_PANDOC_MISSING",
            Self::Typst => "E_TYPST_MISSING",
        }
    }
}

fn candidates(tool: Tool) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(value) = std::env::var(tool.env_name()) {
        if !value.trim().is_empty() {
            paths.push(PathBuf::from(value));
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            // Tauri 打包后的 sidecar 通常与主程序同目录。
            paths.push(dir.join(tool.binary()));
            paths.push(dir.join(format!("{}-aarch64-apple-darwin", tool.binary())));
        }
    }
    // 开发期使用项目内工具目录；S6 已固定版本与路径。
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let project = manifest.parent().unwrap_or(manifest);
    match tool {
        Tool::Pandoc => paths.push(project.join("tools/macos/pandoc/bin/pandoc")),
        Tool::Typst => paths.push(project.join("tools/macos/typst/typst")),
    }
    paths
}

pub fn resolve(tool: Tool) -> Result<PathBuf, AppError> {
    let candidates = candidates(tool);
    for candidate in &candidates {
        if candidate.is_file() {
            // 只检查存在与可执行；版本检查由 status/调用错误负责。
            if is_executable(candidate) {
                return Ok(candidate.clone());
            }
        }
    }
    Err(AppError::new(
        tool.missing_code(),
        format!("未找到可执行的 {} sidecar", tool.binary()),
        format!(
            "把 {} 放到应用包 Contents/MacOS/（或开发期 tools/macos），或设置 {}；已检查: {}",
            tool.binary(),
            tool.env_name(),
            candidates
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    ))
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .map(|meta| meta.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

pub struct RunOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
}

/// 以参数数组运行 sidecar；超时会终止进程，不留下悬挂子进程。
pub fn run(tool: Tool, args: &[String]) -> Result<RunOutput, AppError> {
    let binary = resolve(tool)?;
    let mut child = Command::new(&binary)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            AppError::new(
                tool.missing_code(),
                format!("无法启动 {}: {error}", tool.binary()),
                format!("检查 {} 是否可执行", binary.display()),
            )
        })?;

    let deadline = Instant::now() + TIMEOUT;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Ok(RunOutput {
                    success: false,
                    stdout: String::new(),
                    stderr: format!("{} 执行超过 {} 秒", tool.binary(), TIMEOUT.as_secs()),
                    timed_out: true,
                });
            }
            Ok(None) => thread::sleep(Duration::from_millis(40)),
            Err(error) => {
                return Err(AppError::new(
                    tool.missing_code(),
                    format!("等待 {} 失败: {error}", tool.binary()),
                    "重启应用后重试",
                ))
            }
        }
    };

    let mut stdout = String::new();
    let mut stderr = String::new();
    if let Some(mut pipe) = child.stdout.take() {
        let _ = pipe.read_to_string(&mut stdout);
    }
    if let Some(mut pipe) = child.stderr.take() {
        let _ = pipe.read_to_string(&mut stderr);
    }

    Ok(RunOutput {
        success: status.success(),
        stdout,
        stderr,
        timed_out: false,
    })
}

fn version_of(tool: Tool) -> Option<String> {
    let args = vec!["--version".to_string()];
    let output = run(tool, &args).ok()?;
    if !output.success {
        return None;
    }
    output
        .stdout
        .lines()
        .next()
        .map(|line| line.trim().to_string())
}

pub fn status() -> SidecarStatus {
    let pandoc_path = resolve(Tool::Pandoc);
    let typst_path = resolve(Tool::Typst);
    SidecarStatus {
        pandoc: ToolStatus {
            ready: pandoc_path.is_ok(),
            version: version_of(Tool::Pandoc).unwrap_or_default(),
            path: pandoc_path
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or_default(),
        },
        typst: ToolStatus {
            ready: typst_path.is_ok(),
            version: version_of(Tool::Typst).unwrap_or_default(),
            path: typst_path
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or_default(),
        },
    }
}

/// 把 sidecar 的 stderr 压缩成可复制的单行摘要，避免整段日志进入界面。
pub fn summarize(stderr: &str) -> String {
    stderr
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(3)
        .collect::<Vec<_>>()
        .join(" / ")
}
