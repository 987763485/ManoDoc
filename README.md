<p align="center">
  <img src="docs/assets/logo.png" width="128" alt="ManoDoc">
</p>

<h1 align="center">ManoDoc</h1>

<p align="center">本地优先的文档转换与语料制作工作台</p>

<p align="center">
  <a href="https://github.com/987763485/ManoDoc/releases">下载</a> ·
  <a href="docs/index.html">产品文档</a> ·
  <a href="#使用">使用</a> ·
  <a href="#开源协议">开源协议</a>
</p>

---

ManoDoc 把 Word、PDF、Excel、PowerPoint、Markdown 与图片整理成干净、可检索的 Markdown，并在同一个应用里完成清洗、切块与语料包导出。默认完全离线，文件不离开你的电脑。

## 功能

- **多格式转换**：DOCX、XLSX、PPTX、RTF、EPUB、CSV、文本型 PDF、Markdown、HTML、PNG / JPEG。
- **双内核 + OCR**：anydoc 负责 Office 与文本型 PDF 的快速路径；xberg + PaddleOCR 处理扫描件、图片与复杂表格，失败时给出可解释的回退原因。
- **文档导出**：Markdown → Word / PDF / HTML / EPUB；PDF → Word 走 PDF → Markdown → DOCX。
- **语料制作**：批量队列、清洗、按标题切块，输出厂商中立的镜像 `.md` + `chunks.jsonl` + `manifest.json`。
- **结果可追溯**：历史记录保存方向、实际引擎、OCR 页、回退原因、耗时与输出摘要。
- **本地日志与诊断**：分级 JSONL 日志、保留天数、脱敏诊断包；不记录文件正文。
- **中英文与主题**：中文 / English 即时切换，浅色 / 深色 / 跟随系统。

## 下载安装

ManoDoc 目前只为 macOS Apple Silicon 提供预编译安装包；Windows 与 Linux 需要从源码自行编译。

| 平台 | 分发方式 | 说明 |
| --- | --- | --- |
| macOS 12+ Apple Silicon | [GitHub Releases](https://github.com/987763485/ManoDoc/releases) 下载 `.dmg` | ad-hoc 签名，首次启动需右键「打开」 |
| Windows 10/11 x64 | 自行编译 | 无预编译包、未签名 |
| Linux x64（WebKitGTK 4.1） | 自行编译 | 无预编译包、未签名 |

macOS 安装步骤：

1. 打开 DMG，把 **ManoDoc** 拖入「应用程序」。
2. 首次启动若被 Gatekeeper 拦截，右键应用选择「打开」。
3. 需要 macOS 12 或更高版本，首发支持 Apple Silicon。

> 当前 macOS 构建为 ad-hoc 签名，尚未完成 Developer ID 公证；正式发布前会补齐。

### 从源码构建

Windows 与 Linux 没有预编译包，需要用同一份源码在本机编译。项目目前只在 macOS Apple Silicon 上做过实机验证，Windows / Linux 构建尚未实机验证。

#### 通用环境

- Node.js 22+、pnpm
- Rust 1.99+（用 [rustup](https://rustup.rs) 安装对应平台的 toolchain）
- CMake（xberg 的 OCR 依赖需要）
- 三个 sidecar：pandoc、Typst、uv。构建前按 `<名称>-<target-triple>` 命名放进 `src-tauri/binaries/`，Windows 额外保留 `.exe` 后缀。

建议与 macOS 预编译包保持相同版本，减少行为差异：

| Sidecar | 版本 | 下载 |
| --- | --- | --- |
| pandoc | 3.14 | <https://github.com/jgm/pandoc/releases> |
| Typst | 0.15.1 | <https://github.com/typst/typst/releases> |
| uv | 0.13.9 | <https://github.com/astral-sh/uv/releases> |

#### macOS

环境：Xcode Command Line Tools。

```bash
pnpm install
pnpm package:mac    # 暂存并签名 sidecar，生成 .app 与 .dmg
```

打包脚本会把 pandoc、Typst 与 uv 暂存并签名到 `src-tauri/binaries/`，再调用 Tauri 打包。

#### Windows

环境：Visual Studio「使用 C++ 的桌面开发」工作负载（MSVC 与 Windows SDK）、WebView2（Windows 10 1803+ 与 Windows 11 通常已内置）、Rust 的 `x86_64-pc-windows-msvc` toolchain。

在 PowerShell 中，把下载的 sidecar 按 target triple 命名后暂存：

```powershell
Copy-Item .\pandoc.exe .\src-tauri\binaries\pandoc-x86_64-pc-windows-msvc.exe
Copy-Item .\typst.exe  .\src-tauri\binaries\typst-x86_64-pc-windows-msvc.exe
Copy-Item .\uv.exe     .\src-tauri\binaries\uv-x86_64-pc-windows-msvc.exe

pnpm install
pnpm tauri build
```

产物在 `src-tauri\target\release\bundle\`：`nsis\ManoDoc_0.1.0_x64-setup.exe` 与 `msi\ManoDoc_0.1.0_x64_en-US.msi`。安装包未签名，Windows SmartScreen 可能提示。

开发模式（`pnpm tauri dev`）不会自动找到这些 sidecar，需要先指向本机二进制：

```powershell
$env:MANODOC_PANDOC = "C:\path\to\pandoc.exe"
$env:MANODOC_TYPST  = "C:\path\to\typst.exe"
pnpm tauri dev
```

#### Linux

以 Debian / Ubuntu 为例，先安装 Tauri 2 与 WebKitGTK 4.1 依赖：

```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

暂存 sidecar（x86_64；ARM64 改用 `aarch64-unknown-linux-gnu` 并把下载换为对应架构）：

```bash
cp pandoc src-tauri/binaries/pandoc-x86_64-unknown-linux-gnu
cp typst  src-tauri/binaries/typst-x86_64-unknown-linux-gnu
cp uv     src-tauri/binaries/uv-x86_64-unknown-linux-gnu
chmod +x src-tauri/binaries/*

pnpm install
pnpm tauri build
```

产物在 `src-tauri/target/release/bundle/`：`deb/ManoDoc_0.1.0_amd64.deb` 与 `appimage/ManoDoc_0.1.0_amd64.AppImage`。安装包未签名。

开发模式同样需要先指定 sidecar：

```bash
export MANODOC_PANDOC="/path/to/pandoc"
export MANODOC_TYPST="/path/to/typst"
pnpm tauri dev
```

> 窗口按 1280×800 以上设计；Windows / Linux 上建议使用 1440×900 及以上的屏幕。

## 使用

### 文档转换

1. 在「文档转换」页拖入或选择文件。
2. 选择目标格式与引擎（默认「自动」）。
3. PDF 含无文本页时，在预检弹窗选择「跳过并继续 / 使用 OCR 继续 / 改为失败」。
4. 开始转换；标题栏显示进度，可随时取消。

输出默认写入 `~/Documents/ManoDoc Output/`，可在设置中修改；图片按设置写入 `assets/<文档名>/`、内嵌 base64 或仅保留 alt 文本。

### 语料制作

1. 在「语料制作」页添加文件或文件夹。
2. 在右侧配置清洗、切块、元数据与语料包开关；并发、切块上限与重叠在「高级设置」。
3. 开始批量，完成后打开输出目录查看镜像 Markdown、`chunks.jsonl` 与 `manifest.json`。

### 转换历史

历史页保留最近 50 条任务（可配置），支持查看详情、打开输出、复制参数与重跑。重跑会恢复当时的预设、引擎与目标格式。

### 快捷键

| 快捷键 | 作用 |
| --- | --- |
| `⌘K` | 命令面板：页面、操作与最近文件 |
| `⌘1` – `⌘4` | 切换文档转换 / 语料制作 / 转换历史 / 系统设置 |
| `Esc` | 关闭弹窗或命令面板 |

### 应用数据

设置、预设、历史、日志与 OCR 模型保存在 `~/.manodoc/`；用户输出目录独立。可在「系统设置 → 通用」查看并打开。

## 注意事项

- **离线优先**：只有你在「引擎」页显式安装 OCR 模型时才会联网；转换期间强制离线。
- **扫描件与图片**：需要 OCR 模型；模型约 31 MB，安装前会显示来源与哈希。
- **PDF → Word**：由 PDF → Markdown → DOCX 生成，目录与图片保真有限，界面会给出质量提示；不内置 pdf2docx。
- **PDF 无文本页**：预检结果会列出页码；跳过页不会写入输出，并记录在警告与批量报告中。
- **文字层乱码**：部分排版软件（如 DocGen / 方正书版）生成的 PDF 文字层编码错位，直接抽取会出现乱码。ManoDoc 会自动检测并改用全文 OCR；也可在转换页开启「全文强制 OCR」，或在预检弹窗选择「全文 OCR」。
- **pandoc 许可**：pandoc 为 GPL-2.0 程序，以独立 sidecar 进程调用；源码获取方式见「关于」页与 [pandoc.org](https://pandoc.org)。
- **隐私**：默认不做任何上传；诊断包只含版本、设置摘要、任务元数据与最近日志，不含文件正文。

## 开源协议

ManoDoc 使用 [MIT License](LICENSE)。

第三方组件：

| 组件 | 协议 | 用途 |
| --- | --- | --- |
| anydoc | MIT | Office / 文本型 PDF 解析 |
| xberg | MIT | 第二内核、OCR、表格识别 |
| PaddleOCR | Apache-2.0 | 中文 OCR 模型 |
| Typst | Apache-2.0 | PDF 排版编译 |
| pandoc | GPL-2.0 | Markdown 与文档格式导出（独立进程） |

## 参与贡献

问题与建议请提交 [Issues](https://github.com/987763485/ManoDoc/issues)。提交代码前请先阅读 `AGENTS.md` 中的开发约束。
