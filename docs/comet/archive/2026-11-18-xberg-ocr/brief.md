# 目标

交付 CHG-003「xberg OCR」：把 xberg 1.4.0 作为第二内核嵌入 Tauri 后端，为扫描 PDF、图片与 anydoc 失败场景提供 OCR、复杂表格与回退转换；在文档转换与语料制作中实现可见的引擎路由，并提供 OCR 模型的本地状态检测与用户显式安装。

本 change 复用 CHG-001 的转换/历史与 CHG-002 的批量/语料包管线；OCR 使用 xberg 内置的 PaddleOCR ONNX 后端，Python 运行时与 Marker 仍留待 P1。

# 范围

## 目录结构

### 新建

- `src-tauri/src/engine/xberg.rs` — xberg 抽取适配器：图片/文档输入、ExtractionConfig、OCR 配置、错误映射与引擎元数据。
- `src-tauri/src/engine/routing.rs` — auto/anydoc/xberg 路由决策、回退原因与去重规则。
- `src-tauri/src/ocr/mod.rs` — OCR 模型目录、状态检测、必需文件清单、显式下载、SHA-256 校验与离线错误。
- `src/components/OcrModelCard.tsx` — 设置页 OCR 模型卡片：状态、大小、来源、安装进度与错误。
- `src/pages/ConvertPage.test.tsx`、`src/pages/SettingsPage.test.tsx` — 前端引擎选择与模型安装交互测试。

### 修改

- `src-tauri/Cargo.toml` — 增加 `xberg = 1.4.0`（`formats-no-heic`、`tokio-runtime`、`paddle-ocr-ort`）与显式模型下载所需依赖。
- `src-tauri/src/engine/mod.rs`、`src-tauri/src/engine/anydoc.rs` — 图片检测、统一抽取入口、引擎与回退元数据。
- `src-tauri/src/commands/mod.rs`、`src-tauri/src/lib.rs` — `convert_file` 接收引擎选择；新增 OCR 状态/安装命令；`inspect_file` 支持图片。
- `src-tauri/src/models.rs` — `FileInfo`、`ConversionResult`、`EngineInfo` 与历史参数扩展。
- `src-tauri/src/batch/mod.rs`、`src-tauri/src/corpus/mod.rs` — 批量引擎选择、OCR 路由与 manifest 逐文件元数据。
- `src/lib/ipc.ts` — 新增 OCR 状态/安装/进度事件与引擎选择类型。
- `src/pages/ConvertPage.tsx` — 引擎选择、OCR 进度阶段、回退原因与 `E_NEEDS_OCR` 下一步。
- `src/pages/SettingsPage.tsx` — 引擎页接入真实 xberg/模型状态与安装入口。
- `src/pages/BatchPage.tsx` — 启用 auto/xberg 引擎选项与 OCR 状态提示。
- `src/i18n/index.tsx` — 中英文 OCR、引擎、模型与回退文案。
- `docs/superpowers/技术选型.md`、`docs/superpowers/需求文档.md`、`docs/superpowers/软件功能设计说明.md`、`AGENTS.md`、`CHANGE.md` — 同步实现范围与阶段状态。

### 明确不建

- Python uv 运行时与 PaddleOCR/Marker Python 引擎 — 本 change 以 xberg 的 `paddle-ocr-ort` ONNX 后端交付 OCR；Python `.venv`（约 795MB）仍为 P1，避免静默下载与体积膨胀。
- 独立 `xberg-cli` sidecar — crate 嵌入已满足 P0，减少打包与签名复杂度。
- 图片资产落盘与 base64 — 与本 change 的 OCR 能力无关，仍按后续约定处理。

# 非目标

- Markdown → Word/PDF/HTML/EPUB、PDF → Word、Typst 渲染（CHG-004）。
- Developer ID 签名、公证、DMG 与自动更新（CHG-005）。
- PaddleOCR Python 环境、Marker 安装与引擎质量对比报告（REQ-100/101）。
- 改变清洗、切块与语料包既有 schema 的语义；只追加 OCR 相关字段。
- 任何把文件上传到云端的转换方式。

# 约束与不变量

- 模型目录固定 `~/.dull_doc/models/paddleocr/`（Hugging Face cache 布局）；`DULLDOC_MODEL_DIR` 可覆盖该目录，仅用于测试与诊断。
- 必需模型为 PP-OCRv6 small：检测 9.9MB、识别 21.2MB 与字典 74KB，总计约 31MB；下载源与 SHA-256 取自 xberg 固定 manifest。
- 不得静默下载：提取路径设置 `HF_HUB_OFFLINE=1`；模型缺失时直接返回可操作的错误，不发起网络请求。
- 显式选择 xberg 但模型缺失且文件需要 OCR 时返回 `E_NEEDS_OCR`；不得静默丢页或返回空文本。
- `auto` 路由：anydoc 优先；仅 `NeedsOcr`、解析失败或表格质量 C 级才回退 xberg；每次回退记录原因。
- 文本型文档不因启用 OCR 而走 OCR；xberg 的 `ocr_strategy=auto` 只 OCR 低质量页。
- 界面颜色只来自 Proofing Desk token；1280×800 与 1440×900 无横向溢出。
- 代码注释使用中文，只解释「为什么」和非显而易见之处。

# Decisions

- xberg 以 crate 嵌入，feature 固定 `formats-no-heic`、`tokio-runtime`、`paddle-ocr-ort`；OCR 后端使用 xberg 内置 PaddleOCR ONNX，不使用 tract。
- 引擎选择为 `auto`（默认）/`anydoc`/`xberg`；选择随任务写入历史参数，不写全局设置。
- 模型安装是用户显式动作：设置页展示状态、大小、来源与哈希；点击后下载并校验；离线报错且不自动重试。
- OCR 参数固定中文优先（`ch` 覆盖 CJK+Latin）、`use_angle_cls=false`、`enable_table_detection=true`、`pp-ocrv6/small`。
- PNG/JPEG 输入只支持 → Markdown，自动走 xberg OCR；其他方向禁用并说明。
- 批量语料页启用 auto/anydoc/xberg；OCR 文件在 manifest 中记录 `engine`、`ocrPages`、`ocrEngine` 与回退原因。
- 回退与失败都必须可见：界面显示实际引擎与原因，历史参数保留；同一文件不重复转换。

# 验收示例

- A1：模型目录为空时，系统设置 → 引擎显示 xberg 1.4.0 与 OCR 模型「未安装」、约 31MB 与下载来源；安装前不发起网络请求，点击安装后显示进度并在完成后变为「已安装」。
- A2：OCR 模型已安装时，转换封面页无文本层的 `example/` PDF 自动路由到 xberg OCR，输出非空且包含可读中文的 Markdown，界面与历史显示实际引擎 `xberg`、OCR 页与回退原因。
- A3：docx、xlsx、文本型 PDF 与 CSV 在默认 auto 引擎下由 anydoc 完成转换，不加载 OCR 模型、不产生 OCR 网络请求；显式选择 anydoc 时行为一致。
- A4：模型目录为空或模型文件缺失时，转换扫描 PDF 或图片返回 `E_NEEDS_OCR`，包含页数/文件信息与「安装 OCR 模型」下一步，不产生不完整输出，也不自动下载模型。
- A5：OCR 模型已安装时，选择包含中文正文的 PNG/JPEG 可得到包含可读中文的 Markdown，界面与历史显示 `xberg` 与 OCR 来源。
- A6：批量队列包含 docx 与扫描 PDF 且 OCR 模型已安装时，扫描 PDF 以 xberg 完成、docx 以 anydoc 完成；`chunks.jsonl` 的 OCR 块元数据含 `engine=xberg`，manifest 记录实际引擎、OCR 页与回退原因，语料包校验通过。
- A7：文档转换与语料制作页的引擎选择可选 auto/anydoc/xberg；模型未安装时 xberg 禁用并说明原因；发生回退或 OCR 时界面显示实际引擎与原因，英文与深色主题文案完整，1280×800 与 1440×900 无横向溢出。
- A8：`pnpm build`、`pnpm test`、`cargo check` 与 `cargo test` 全部通过；断网后已安装模型仍可 OCR 且不发起网络请求；模型缺失时安装返回清晰的离线错误，不静默重试。
