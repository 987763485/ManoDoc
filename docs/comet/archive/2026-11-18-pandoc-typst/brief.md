# 目标

交付 CHG-004「pandoc/Typst」：把项目内的 pandoc 3.14 与 Typst 0.15.1 作为 sidecar 接入，打通 Markdown ↔ Word/HTML/EPUB、Markdown → PDF 与 PDF → Word 管线，并在文档转换页提供可用的目标格式、引擎与质量提示。

本 change 复用 CHG-001/003 的 anydoc、xberg、历史、设置与 Proofing Desk 界面；批量语料仍固定 Markdown。

# 范围

## 目录结构

### 新建

- `src-tauri/src/engine/pandoc.rs` — pandoc sidecar 解析、参数数组调用、超时、临时文件与错误映射。
- `src-tauri/src/engine/typst.rs` — Typst sidecar 解析与 `.typ` → PDF 编译。
- `src-tauri/src/engine/export.rs` — 目标格式能力矩阵与管线：文档→Markdown、Markdown→DOCX/HTML/EPUB、Markdown→PDF、PDF→DOCX。
- `src/components/TargetSelect.tsx` — 转换页目标格式与兼容引擎选择（含不可用原因）。
- `src-tauri/src/engine/export_tests.rs`、`src/pages/ConvertPage.export.test.tsx` — 管线与界面测试。

### 修改

- `src-tauri/src/engine/mod.rs` — 暴露统一 export 入口与 sidecar 状态。
- `src-tauri/src/commands/mod.rs`、`src-tauri/src/lib.rs` — `convert_file` 接收目标格式；新增 sidecar 状态命令。
- `src-tauri/src/models.rs` — `FileInfo` 目标格式列表、`ConversionResult` 目标/引擎字段。
- `src-tauri/src/security/mod.rs` — 输出扩展名与临时目录复用。
- `src/lib/ipc.ts` — 目标格式、sidecar 状态与转换参数。
- `src/pages/ConvertPage.tsx` — 目标格式、引擎、质量提示、进度阶段与结果展示。
- `src/pages/HistoryPage.tsx` — 展示目标格式与实际引擎。
- `src/i18n/index.tsx` — 中英文目标格式、质量提示与错误文案。
- `docs/superpowers/技术选型.md`、`docs/superpowers/需求文档.md`、`docs/superpowers/软件功能设计说明.md`、`AGENTS.md`、`CHANGE.md` — 同步实现状态。

### 明确不建

- LaTeX/TeX 安装与 PDF 引擎 — 继续使用 Typst。
- PDF→Word 的 pdf2docx — 保持 PDF→Markdown→DOCX，不分发 AGPL 依赖。
- 图片资产提取与 base64 — 与本 change 无关；Markdown 中的相对图片按 `--resource-path` 尽力解析。
- 输出目录模板、批量导出多格式 — 单文件先行，批量保持 Markdown。

# 非目标

- CHG-005 的签名、公证、DMG 与 sidecar 打包资源路径。
- 图片型 PDF 的 OCR 路由细节（已在 CHG-003 完成）。
- 高保真 PDF→Word（目录、复杂表格与图片保真仍有限，需在界面说明）。
- Markdown 编辑/预览与语料多格式导出。

# 约束与不变量

- 仅调用项目内 `tools/macos/pandoc/bin/pandoc` 与 `tools/macos/typst/typst`；开发期可用 `DULLDOC_PANDOC` / `DULLDOC_TYPST` 覆盖，打包期由 CHG-005 提供资源路径。
- 所有外部命令使用参数数组，不拼接 shell；超时 120 秒，超时/非零退出返回可读错误码。
- 临时 `.typ`、中间 Markdown 与 PDF 先写入 `~/.dull_doc/tmp/<uuid>/`，成功或失败都清理。
- 输出文件使用临时文件 + rename，失败不留半写文件；同名自动 `-1`、`-2`。
- PDF→Word 必须显示「目录/图片保真有限」提示；Markdown→PDF 必须显示 Typst 渲染说明。
- 不支持的源/目标组合禁用并说明原因，不静默降级。
- 界面颜色只来自 token；1280×800 与 1440×900 无横向溢出。

# Decisions

- 目标格式矩阵：Markdown → DOCX/PDF/HTML/EPUB；PDF → Markdown/DOCX；docx/xlsx/pptx/odt/rtf/epub/csv 等 → Markdown；PNG/JPEG → Markdown；HTML/MHTML → Markdown。
- 引擎选择：auto/anydoc/xberg/pandoc；auto 按来源与目标路由；显式引擎不兼容时禁用。
- Markdown→PDF：pandoc `-t typst` 生成 `.typ`，再 `typst compile` 输出 PDF；系统字体优先，不 bundle 字体。
- PDF→Word：anydoc/xberg → Markdown → pandoc DOCX；实际引擎显示 `pandoc`，质量提示常驻。
- HTML→Markdown 由 pandoc 负责；Markdown 输入直通导出，不重新解析。
- 批量语料页不新增目标格式，继续只产出 Markdown。

# 验收示例

- A1：`tools/macos/pandoc/bin/pandoc` 可执行（3.14）；设置/转换页显示 pandoc 就绪；Markdown 输入转换为 DOCX 成功，输出文件可打开，历史显示方向 `Markdown → Word`、引擎 `pandoc`。
- A2：同一 Markdown 输入可导出 HTML 与 EPUB；HTML 含文档标题与正文，EPUB 为非空 zip 且包含 mimetype/content，输出扩展名正确。
- A3：Markdown → PDF 走 pandoc → Typst；中文文档生成多页 PDF，页面无缺字方块，输出存在且大小合理，历史显示 `pandoc + typst`。
- A4：example 50 页文本型 PDF → Word 走 PDF→Markdown→DOCX，输出 DOCX 非空、可打开；界面与历史显示质量提示「目录/图片保真有限」，引擎为 `pandoc`。
- A5：转换页按来源显示可用目标格式；不支持组合禁用并说明；Markdown→PDF 与 PDF→Word 显示各自质量/渲染提示；手动选择不兼容引擎时被禁用。
- A6：目标格式与引擎写入历史参数；输出同名文件自动生成 `-1`；打开输出与重跑使用相同目标格式。
- A7：pandoc 或 Typst 缺失/不可执行时返回 `E_PANDOC_MISSING` / `E_TYPST_MISSING` 与安装/路径下一步；Typst 编译失败返回 `E_PDF_COMPILE_FAILED`；失败不产生半写输出。
- A8：`pnpm build`、`pnpm test`、`cargo check`、`cargo test` 通过；转换全程离线；外部命令参数数组无 shell 拼接；英文/深色 1280×800 与 1440×900 无横向溢出。
