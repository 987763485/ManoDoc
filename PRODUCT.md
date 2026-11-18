# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

> DullDoc 是 macOS 桌面应用，使用 Tauri 2 的 WebView 渲染界面；设计语言按桌面 Web 应用处理，不套用 iOS/Android 原生组件。

## Stack

Tauri 2（Rust 后端 + WebView 前端）；UI 原型使用纯 HTML / CSS / 原生 JavaScript；最终前端框架在 S6 技术选型中确认。

## Users

- **主要用户**：安全合规、研究、法务、咨询、技术写作等知识工作者。长期处理标准、测评报告、制度、手册，需要把 docx / PDF / xlsx 转成可直接使用的 Markdown，放入 Obsidian、语料库或喂给 LLM。
- **次要用户**：不写代码的同事。需要拖拽、进度、失败原因和可交付结果，而不是命令行。
- **高级用户**：需要可复现、可脚本化的自动化使用者；关注预设、CLI、manifest 与批量效率。

## Product Purpose

DullDoc 是一个 **macOS 本地优先的文档转换工作台**：以 Markdown 为核心，同时保留 Word、PDF、HTML、EPUB、图片等基础一对一转换；需要时，再把一批文档清洗、切块、补充元数据，产出厂商中立的语料包。

成功意味着：日常 80% 的转换不再需要打开终端或浏览器；断网可用；失败可定位；同输入 + 同预设可复现。

## Positioning

不是在线转换站，也不是需要写脚本的 CLI 集合。DullDoc 的机制是：

1. **本地离线**：默认零网络请求，不上传、不登录、不埋点。
2. **引擎分层与可选**：anydoc 快路径 + xberg OCR/复杂表格/回退 + PaddleOCR（P1）中文 OCR 增强 + pandoc 互转；同一方向有多个引擎时由用户选择或自动路由。
3. **最后一公里**：把「转 → 洗 → 切 → 打包」串成一个桌面工作台，而不是散落的工具。
4. **可解释**：每个文件显示引擎、参数、警告与错误；质量分级不承诺做不到的事。

## Operating Context

- 用户通常在 macOS 上处理内部标准、测评报告、合同与制度文件；这些文件不适合上传到第三方服务。
- **应用数据统一存放在 `~/.dull_doc/`**：`settings.json`、`presets.json`、`history.json`、`logs/`、`tmp/`、`runtimes/`；用户输出目录独立，可配置。
- 典型任务：单个文件快速转换；一个资料文件夹批量转换；把一批文档变成可入库的 Markdown + JSONL 语料包。
- 产物要能放进 Obsidian、静态站点、RAG / 向量库或直接交给同事。
- 用户可能不熟悉命令行，但愿意为「准确、省事、可复现」付出学习成本。
- 文件可能包含中文标题编号、密集表格、页眉页脚、目录点线、扫描页与图片封面。

## Capabilities and Constraints

**已确认能力**：Word / PDF / Markdown / PNG / JPEG 等基础一对一转换（含 Word→Markdown、PDF→Markdown、PDF→Word、Markdown→Word、Markdown→PDF、图片→Markdown）；多引擎方向可显式选择引擎；docx / pdf / xlsx / pptx / odt / rtf / epub / csv / md 输入；pandoc 导出 HTML / DOCX / EPUB；PDF 页级预检；xberg OCR 与复杂表格回退；PaddleOCR（P1）中文 OCR；批量队列；清洗；切块；元数据；`chunks.jsonl` + `manifest.json`；历史；预设；全局日志与错误处理；中英文切换；跟随系统/浅色/深色主题；Cmd+K 命令面板；文件类型图标。

**约束**：
- 首发 macOS Apple Silicon；Windows / Linux 为 P2。
- Marker / PaddleOCR 等 Python 引擎为 P1 可选组件；由内置 `uv` 在 `~/.dull_doc/runtimes/<engine>/.venv` 按需安装，不依赖系统 Python，不静默下载。
- PDF→Word 走 `pdf2docx` 或 PDF→Markdown→Word 管线；Markdown→PDF 需 PDF 渲染器（Typst / WebKit 打印 / LaTeX）；复杂版式不承诺像素级保真。
- MVP 使用本地 JSON 存储，统一放在 `~/.dull_doc/`，不引入数据库；用户输出目录独立。
- 默认离线；任何云服务必须显式开启并二次确认。
- 语料制作的输出格式固定 Markdown；其他格式走单文件转换；清洗/切块/语料开关集中在语料制作配置。
- 应用 MIT；pandoc 以独立 sidecar 按 GPL 合规分发；PaddleOCR Apache-2.0。

## Brand Commitments

- 名称：**DullDoc**；中英文双语，默认中文。
- 语气：克制、专业、直接；错误信息给原因与下一步，不卖萌、不堆感叹号。
- 不虚构客户、案例、价格或性能数据。

## Evidence on Hand

- `example/`：GB/T 28448-2019 docx、50 页等保 PDF、等保 2.0 指标 xlsx、等保测评 skill md。仅本地测试，不纳入版本库。
- `docs/superpowers/`：产品简报、竞品分析、需求文档、软件功能设计说明。
- 真实转换结果：anydoc docx 49 标题 vs pandoc 0；xlsx 1048 行表格；PDF 封面页需 OCR。

## Product Principles

1. **本地优先**：默认离线、不上传、不埋点。
2. **保真优先**：标题、表格、列表、公式、图片位置尽量保留；宁可慢，不可错。
3. **一站式**：格式识别、转换方向、引擎选择、清洗、切块、打包都在界面内完成。
4. **可解释**：每个文件都能看到引擎、参数、警告与失败原因。
5. **可复现**：输入哈希、引擎版本、预设与参数写入 manifest。

## Accessibility & Inclusion

- 目标 WCAG AA 对比度；键盘可完成主要流程（含 Cmd+K 命令面板）；焦点可见；支持减少动效。
- 中英文完整，切换不重启；浅色/深色主题均满足对比度；错误信息不使用仅靠颜色传达的状态。
