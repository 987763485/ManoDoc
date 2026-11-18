# 目标

交付 CHG-002「批量语料」：把「语料制作」页从占位变成可用的本地批量管线，让用户一次选择多个文件或文件夹，完成 Markdown 转换、保守清洗、按标题切块、元数据补充，并产出厂商中立的语料包（镜像 Markdown + `chunks.jsonl` + `manifest.json` + JSON/CSV 报告），同时写入批量历史。

本 change 复用 CHG-001 的 anydoc 转换、设置、历史与 Proofing Desk 界面；只处理 anydoc 可解析的文本型文档。扫描件 OCR、图片资产提取、其他导出方向与发布签名仍由后续 change 负责。

# 范围

## 目录结构

### 新建

- `src-tauri/src/batch/mod.rs` — 批量任务模型、递归扫描、过滤、并发调度、取消、重试与汇总。
- `src-tauri/src/clean/mod.rs` — 清洗规则：页眉页脚/页码、空行、标题编号、中文排版、表格保护与质量分级。
- `src-tauri/src/chunk/mod.rs` — 按标题切块、超长二次切分、重叠、表格整块与稳定 chunk id。
- `src-tauri/src/corpus/mod.rs` — 语料包布局、元数据、`chunks.jsonl`、`manifest.json`、JSON/CSV 报告与输出校验。
- `src/pages/batch/` — 语料制作页的队列、统计、配置面板与状态行组件。
- `src-tauri/src/batch/tests.rs`、`src-tauri/src/clean/tests.rs`、`src-tauri/src/chunk/tests.rs`、`src-tauri/src/corpus/tests.rs` — Rust 单元测试。
- `src/pages/batch/BatchPage.test.tsx` — 前端队列与配置交互测试。

### 修改

- `src/pages/BatchPage.tsx` — 替换占位内容，接入真实队列、配置、开始/取消/重试与打开语料包。
- `src/lib/ipc.ts` — 新增批量扫描、开始、取消、重试、报告与打开语料包的类型化命令。
- `src/i18n/index.tsx` — 补齐中英文批量、清洗、切块、语料包与报告文案。
- `src/App.tsx`、`src/pages/HistoryPage.tsx` — 历史支持批量记录类型与详情。
- `src-tauri/src/models.rs` — 扩展 `HistoryTask`（兼容旧单文件记录）与批量任务/结果/报告结构。
- `src-tauri/src/commands/mod.rs`、`src-tauri/src/lib.rs` — 注册批量相关 Tauri command。
- `src-tauri/src/storage/mod.rs` — 历史读写兼容新字段，仍限制 50 条。
- `src-tauri/Cargo.toml` — 增加语料所需的本地哈希依赖（SHA-256），不引入网络或数据库。
- `CHANGE.md`、`AGENTS.md` — 同步 change 状态与工作日志。

### 明确不建

- 不建 xberg/PaddleOCR/Marker 适配与 OCR 模型下载 — 属 CHG-003。
- 不建 pandoc/Typst 导出与 PDF→Word — 属 CHG-004。
- 不建 zip 打包、文件夹监听、CLI、Finder 分享与平台适配器 — P1/P2，本 change 只输出目录形态的语料包。
- 不建数据库或远程服务 — 沿用 JSON 与本地文件。

# 非目标

- 图片资产提取与 base64 内嵌；「图片落 assets/」开关禁用并说明由 CHG-003 接入。
- 扫描件 OCR、图片型 PDF、复杂表格结构还原。
- Markdown → Word/PDF/HTML/EPUB、PDF → Word、Typst PDF。
- zip 打包、文件夹监听、CLI、右键分享、Dify/RAGFlow 适配器。
- 签名、公证、DMG、自动更新、远程推送与云服务。

# 验收示例

- A1：语料制作页可添加多个文件或文件夹；递归扫描跳过隐藏文件、`.DS_Store` 与输出目录自身，队列显示文件、状态、引擎、耗时与警告，统计数量与队列一致。
- A2：批量开始后按并发上限调度；单文件失败不阻塞其他文件，失败行可重试，取消后不再新增任务；不产生半写文件，输出包完整或整体不存在。
- A3：清洗开关可逐项关闭；关闭时输出等于原始 anydoc Markdown；开启时删除匹配的页眉页脚/页码、压缩空行、规范标题编号与 CJK 间距，代码块、URL 与表格内容不被破坏。
- A4：切块默认按 H1–H3，超长块按字符上限二次切分并带重叠；表格整块保留；每个 chunk 含稳定 id、来源、章节路径、字符数与引擎参数。
- A5：语料包包含镜像 Markdown、`chunks.jsonl`、`manifest.json` 与 JSON/CSV 报告；`chunks.jsonl` 每行可解析，`manifest.json` 含工具/引擎版本、输入哈希、参数、文件列表、警告与总计；输出后校验失败时给出可定位错误。
- A6：一次批次写入一条批量历史记录（类型、输入数、状态、输出目录、成功/跳过/失败与块数）；重启后仍在；历史详情可打开语料包、复制参数与重跑。
- A7：语料制作页按 `docs/superpowers/UI.html` 还原队列、统计、配置面板与状态；中英文完整，1280×800 与 1440×900 无横向溢出，所有颜色来自 token。
- A8：`pnpm build`、前端测试、`cargo check`、Rust 测试通过；批量转换与语料包生成全程离线；输出目录只写入语料包与报告，`~/.dull_doc` 仍只写 settings/history/logs/tmp。

# Constraints and invariants

- 应用数据继续位于 `~/.dull_doc/`；语料包写入用户输出目录，不复制到应用数据目录。
- 所有外部路径先规范化；扫描拒绝符号链接逃逸、设备文件与输出目录自身；不拼接 shell。
- 批量输出格式固定 Markdown；引擎只接 anydoc，其他引擎选项禁用并说明后续 change。
- 清洗默认保守，任何规则关闭后不得影响其他规则；代码块、URL、公式、表格整块不参与中文空格处理。
- 语料包不绑定 Dify/RAGFlow；文件名与字段保持稳定、可被 `jq` 逐行解析。
- 失败不得静默：每个失败/跳过文件都有错误码或原因，写入队列、报告与 manifest。
- 代码注释为中文，只解释「为什么」和非显而易见之处；界面颜色只来自 Proofing Desk token。

# Decisions

- 交付范围：完整 P0 闭环（批量队列 + 清洗 + 切块 + 语料包 + JSON/CSV 报告 + 批量历史）；zip、图片提取与 OCR 后置。
- 语料包布局：每次批次使用独立目录 `<输出目录>/<批次名>-<时间戳>/`，内含 `mirror/`、`chunks.jsonl`、`manifest.json`、`report.json`、`report.csv`；批次之间互不覆盖。
- 清洗默认：开启页眉页脚/页码、空行压缩、标题编号与 CJK 间距；表格只做质量分级警告；TOC 保留；断行合并不默认开启。
- 图片资产：本 change 禁用「图片落 assets/」并注明由 CHG-003 接入；Markdown 只保留 alt 文本。
- 批量历史：每次批次写入一条记录，类型 `batch`，保存输入路径、输入数、输出目录、计数与参数；不逐文件写历史。
- 复用 CHG-001 的 anydoc 转换与历史存储；批量记录与单文件记录共用 `history.json`，旧记录可继续读取。
- 清洗只处理 Markdown 文本层，不重新解析原文档；页码元数据在 anydoc 路径下为 `null`。

# Open questions

无。
