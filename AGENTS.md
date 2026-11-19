# AGENTS.md — ManoDoc 项目运营手册

> 本文件由 S0【生成 AGENTS.md】产出，是全项目的运营手册与索引，需随阶段持续更新。
> 项目流程规范见根目录 `SIMPLE.md`，两者冲突时以用户最新确认为准。

## 一、Agent 角色与协作方式

- **角色**：全栈工程师 + 产品/交互协作者。负责需求梳理、竞品分析、PRD、功能拆解、UI 规范与高保真原型、技术选型、编码、测试与验收。
- **工作语言**：中文。代码注释使用中文。
- **交互原则**：
  - 每个阶段的产出必须先经用户确认；未确认不得进入下一阶段。
  - 需求、架构、功能变更时，主动声明并同步更新 `docs/` 下受影响文档。
  - 不替用户做不可逆决策；涉及删除、覆盖、发布、远端推送时先征得同意。
  - 遇到流程中不存在的 skill（如 `comet`、`grill-me`）时，向用户说明并请求手动触发或确认替代方案。

## 二、当前状态

| 项目 | 内容 |
| --- | --- |
| 产品名 | **ManoDoc**（2026-11-19 由 DullDoc 更名） |
| 当前阶段 | **S7 CHG-001~006 全部归档，等待用户人工验收** |
| 下一阶段 | 用户人工验收；取得 Developer ID 后执行公证与 staple |
| 当前事项 | CHG-006 已关闭 71 条 P0 缺口：新增 `logging`（JSONL 分级日志、保留天数、脱敏诊断 zip）、`preflight`（页级预检与删页）、`presets`（内置/自定义预设 CRUD 与导入导出）、`assets`（图片落盘与相对路径/base64/alt）；设置页补齐输出（图片模式/覆盖/历史条数）、预设、日志与关于（MIT 协议名称、第三方许可、pandoc 源码）；转换页支持 PDF 预检弹窗、页数与预计引擎、错误诊断复制、引擎选择持久化；标题栏显示任务、进度与取消，任意页拖入按单/多文件与文件夹路由。实机：50 页 PDF 跳过封面后 26s 完成（66145 字符/203 表格行），Markdown 130 处图片引用全部写入 `assets/chg006/` 相对路径；诊断 zip 含 diagnostics.json 与最近日志；预设复制/编辑/删除与日志设置生效。修复 async 命令内 `block_on` 死锁（预检放到 spawn_blocking）与跳过页临时文件名泄漏到输出名。`pnpm build`、`pnpm test`（17 项）、`cargo test` 通过；详见 `docs/superpowers/P0自检.md`。CHG-005 已接入 `scripts/package-macos.sh`：暂存并预签名 pandoc/Typst/uv 后由 Tauri 打包 `.app` 与 `.dmg`；产物 380MB/128MB，`codesign --verify --deep --strict` 通过（ad-hoc），DMG 可挂载且复制出的应用可启动；隐藏 `tools/macos` 后包内 sidecar 仍完成 Markdown→Word（25071 字符/212 标题/121 表格行）与 Markdown→PDF（21 页/672KB）；移除包内 pandoc 返回 `E_PANDOC_MISSING`；无 Developer ID 时脚本以退出码 20 标注公证阻塞。CHG-004 已接入 pandoc 3.14 + Typst 0.15.1：Markdown→DOCX/HTML/EPUB/PDF、PDF→DOCX、HTML→Markdown；转换页支持目标格式、引擎与质量提示，历史记录目标格式并支持重跑恢复。实机：Markdown→Word（25071 字符、212 标题、121 表格行）、Markdown→PDF（21 页、672KB，中文表格正常）、PDF→Word（64154 字符、203 表格行，OCR 2 页 + 保真提示）；重跑生成 `-1.docx`。`pnpm build`、`pnpm test`（13 项）、`cargo test`（25 项 + 12 ignored）通过。CHG-003 已接入 xberg 1.4.0 与 PaddleOCR ONNX（PP-OCRv6 small）：`engine/routing.rs` 实现 auto/anydoc/xberg 路由与可见回退；`ocr/mod.rs` 管理 `~/.manodoc/models` 的模型状态与显式安装（hf-hub 1.0 下载、约 31MB、SHA-256、离线不下载）；单文件、PNG/JPEG 与批量语料均写入实际引擎、OCR 页与回退原因。实机验证：封面无文本 50 页 PDF 以 xberg OCR 完成（64154 字符、9 标题、203 表格行，OCR 2 页，回退 anydoc:needs-ocr）；docx 保持 anydoc；无模型时图片返回 `E_NEEDS_OCR` 并给出安装入口；批量 4 文件（docx/xlsx/jpg/pdf）全部成功，356 chunks，manifest 记录 `engineUsed/ocrPages/ocrEngine/fallbackReason`。`pnpm build`、`pnpm test`（12 项）、`cargo test`（23 项 + 9 个 ignored）通过。模型下载已改用 hf-hub 1.0 并实测从 huggingface.co 完成（31MB，14.2s，SHA-256 通过）；本地镜像测试覆盖进度、校验与重复安装跳过。 |
| 已确认 | ① `maker` = datalab-to/marker；② 首要场景同时覆盖「通用办公格式互转」与「批量知识库构建」；③ 首发 macOS Apple Silicon；④ MVP 接 anydoc + pandoc；⑤ 2026-11-18 追加 xberg 内核（P0）；⑥ 2026-11-18 追加 PaddleOCR（P1 OCR 组件）；⑦ 2026-11-18 追加中英文切换、主题切换、命令面板（P0） |

### 已确认的产品决策

- **引擎分工**：anydoc 负责 Office/文本型 PDF 的快速路径；xberg（原 Kreuzberg，Rust/MIT）作为第二内核，负责扫描件/图片 PDF 的 OCR、复杂表格与 anydoc 失败回退；pandoc 负责 Markdown ↔ DOCX/HTML/EPUB 与部分格式兜底；Markdown→PDF 由 pandoc 生成 Typst 再编译 PDF；PaddleOCR（P1，Apache-2.0）作为中文 OCR 增强组件；PDF→Word 走 PDF→Markdown→DOCX，**不分发 pdf2docx**；Marker 保留为 P1 的备选高质量引擎。
- **引擎路由**：按文件内容判定格式；文本型 PDF 先由 anydoc 处理，失败或质量不足时回退 xberg；扫描件优先 xberg OCR，中文低置信度或用户选择时可用 PaddleOCR（P1）；xberg/PaddleOCR 均不可用时才进入「跳过无文本页 / 提示需要 OCR」的降级路径，不得静默失败。
- **批量知识库产物**：厂商中立的「镜像 `.md` + `chunks.jsonl` + `manifest.json`」语料包；清洗、切块、元数据均为可勾选项；不绑定 Dify / RAGFlow 等具体平台。
- **测试样本**：`example/` 下为测试文件——`.docx`（GB/T 28448-2019 等保测评要求，表格密集）、50 页 PDF（有完整文本层，约 6.1 万字）、`.xlsx`（等保指标清单）、`.md`（1074 行，用于验证 Markdown 再导出）。
- **名称与协议**：产品名保留 ManoDoc；开源协议 MIT；pandoc 以独立 sidecar 按 GPL 合规分发。
- **图片策略**：默认输出到同目录 `assets/` 并使用相对路径；单文件导出可选内嵌 base64。
- **历史记录**：保留最近 50 条任务元数据（可配置），只存路径、参数、方向、引擎、哈希与结果摘要，不复制文件。
- **多方向转换（2026-11-18 已确认）**：保留 Word ↔ Markdown、PDF → Markdown/Word、Markdown → Word/PDF/HTML/EPUB、PNG/JPEG → Markdown、Excel/CSV 与 PowerPoint → Markdown 等基础一对一方向；同一方向多引擎时提供「自动 + 显式引擎」选择；不支持的组合禁用并说明原因。
- **Python 运行时（2026-11-18，P1）**：内置 uv 0.13.9 sidecar；在 `~/.manodoc/runtimes/<engine>/.venv` 为每个 Python 引擎创建隔离环境；Python 由 uv 按需下载，下载前显示版本、体积、来源与哈希，用户确认后才联网；不依赖系统 Python，不静默下载；安装时使用 `--no-bin`，不向 `~/.local/bin` 写可执行文件；按需安装 PaddleOCR / Marker 等包并记录版本。pdf2docx 不默认安装、不随包分发。
- **文件图标（2026-11-18，P0）**：转换页、批量、历史与方向矩阵统一使用 `public/icons/` 下的文件类型图标；缺失时回退综合文档；本次补齐 Markdown、HTML、EPUB、RTF、ODT、ODS、ODP、TXT。
- **取消预览（2026-11-18）**：按用户决定，文档转换页不提供文档预览（部分格式无法可靠预览），语料制作完成行也不提供 Markdown 预览；完成行只保留「打开」。进度、成功、错误与无权限改为配置卡内联显示；PRD 中 REQ-020~022、REQ-048、REQ-111 标记为已取消，编号保留不复用。
- **单文件与语料配置分工（2026-11-18）**：文档转换页只保留方向、引擎与预设，PDF 无文本页策略在预检弹窗选择；页眉页脚、中文标题、表格分级、图片资产、切块、元数据与语料包开关集中在「语料制作」的每任务配置。系统设置 → 输出保留语料包全局默认，批次内可覆盖。语料制作的输出格式固定 Markdown，其他格式禁用；其他格式走单文件转换。
- **体验偏好（2026-11-18 追加，P0）**：中英文切换（中文默认、英文完整、无需重启）；主题切换（跟随系统/浅色/深色）；Cmd+K 命令面板（页面/操作/最近文件，危险操作二次确认）。所有 UI 组件与颜色必须走 token，禁止硬编码。
- **S6 技术决策（2026-11-18，待确认）**：前端 React 19 + TypeScript + Vite + Tailwind CSS 4；Markdown→PDF 立即接入 Typst；PDF→Word 走 PDF→MD→DOCX，不分发 pdf2docx；Python 运行时由 uv 按需下载 Python；Rust 固定 1.99.1，xberg 1.4.0、anydoc 0.2.4、pandoc 3.14（项目内 arm64）、Typst 0.15.1。

## 三、技术原理（初步）

- **应用框架**：Tauri 2。Rust 负责文件系统、进程管理、转换编排与安全边界；前端 React 19 + TypeScript + Vite + Tailwind CSS 4 渲染界面。
- **转换内核**（四个解析/导出引擎 + 可选 OCR 组件）：
  - **anydoc**（Firecrawl，纯 Rust，MIT）：Word / PowerPoint / Excel / OpenDocument / RTF / EPUB / CSV / 文本型 PDF → GitHub 风格 Markdown；速度快、无网络、无 OCR。Rust 可直接以 crate 形式嵌入 Tauri 后端。
  - **xberg**（原 Kreuzberg，Rust，MIT）：107 种格式 → 文本 / Markdown / HTML / JSON / DocTags；内置 OCR（Tesseract / PaddleOCR / Candle / VLM）、版面与表格识别、语法切块、CLI/API/MCP。作为第二内核，负责扫描件、复杂 PDF 表格与 anydoc 回退。
  - **PaddleOCR**（P1，Python，Apache-2.0）：中文 OCR / PP-Structure；作为 xberg 之外的中文 OCR 增强组件，检测环境或引导安装，不随包分发模型。
  - **pandoc**（3.14，项目内 `tools/macos/pandoc/bin/pandoc`，arm64）：通用标记语言转换器，负责 Markdown → HTML / DOCX / ODT / EPUB / Typst 等导出，以及部分格式兜底。以 sidecar 方式随应用分发。
  - **Typst**（0.15.1，Apache-2.0）：pandoc 先输出 `.typ`，再由 Typst sidecar 编译 PDF；中文字体优先使用 macOS 系统字体，必要时 bundle Noto CJK。
  - **Marker**（`marker-pdf`，Datalab，Python + ML 模型）：扫描 PDF / 图片 / 复杂版式 → Markdown / HTML / JSON，含版面分析、OCR、表格与公式识别。代码 Apache-2.0，模型权重为 OpenRAIL-M（含营收/竞品限制）；资源占用高，仅 P1 用户显式安装。
- **数据处理原则**：本地优先、默认不上传；应用数据统一存放 `~/.manodoc/`（settings/presets/history/logs/tmp/runtimes），用户输出目录独立；转换任务异步化、可取消、可查看进度与日志；失败可重试并保留中间产物。
- **日志与错误**：全局日志、全局错误处理、任务级错误归因；敏感信息（API Key、token）不得落盘或写日志，本地敏感信息存系统钥匙串。

## 四、文档索引

| 阶段 | 文档 | 路径 | 状态 |
| --- | --- | --- | --- |
| S0 | 项目运营手册 | `AGENTS.md` | 已生成 |
| S1 | 产品简报 | `docs/superpowers/产品简报.html` | 已确认（v0.7） |
| S2 | 竞品分析 + 功能面矩阵 | `docs/superpowers/竞品分析.html` | 已确认（v0.7） |
| S3 | 需求文档（PRD） | `docs/superpowers/需求文档.md` | 已确认（v1.6） |
| S4 | 软件功能设计说明 | `docs/superpowers/软件功能设计说明.md` | 已确认（v1.6） |
| S5 | UI 设计规范 | `docs/superpowers/UI设计规范.html` | 已确认（v1.4） |
| S5 | UI 高保真原型 | `docs/superpowers/UI.html` | 已确认 |
| S6 | 技术选型 | `docs/superpowers/技术选型.md` | 已确认（v1.1）；S6-5 待发布前补公证 |
| S7 | Change 总索引 | `CHANGE.md` | CHG-001~006 已归档 |
| S7 | Change 归档 | `docs/comet/archive/2026-11-18-mvp-implementation/`、`docs/comet/archive/2026-11-18-batch-corpus-v2/`、`docs/comet/archive/2026-11-18-xberg-ocr/`、`docs/comet/archive/2026-11-18-pandoc-typst/`、`docs/comet/archive/2026-11-18-packaging-signing/`、`docs/comet/archive/2026-11-19-p0-completion/` | A1–A8 通过 |
| S7 | P0 自检 | `docs/superpowers/P0自检.md` | 71 条 P0 收口记录（CHG-006） |
| S0 | 产品 Logo | `docs/superpowers/logo/logo-manodoc.svg` | MD 共笔 + 双向转换箭头（1024 PNG 同目录） |
| — | 项目说明（正式发布口径） | `README.md` | 已生成 |
| — | 产品文档站 | `docs/index.html` | 已生成（截图在 `docs/assets/screenshots/`） |
| — | 开源协议 | `LICENSE` | MIT |

> 文档归类规则：哪个 skill 生成，就归入 `docs/.{skill-name}/` 对应目录。

## 五、用户的严格性约束

### 5.1 Git

1. 采用 `main` / `dev` 二层分支模型；所有开发在 `dev` 进行，严禁直接在 `main` 开发。
2. `dev` → `main` 只允许 squash merge：`git merge --squash dev`，检查暂存内容后创建单个提交；禁止普通 merge / merge commit / rebase 快进。
3. 发现 `main` 存在非 squash 合并提交时停止并告知用户，不自行改写历史。
4. 提交信息使用中文 + 约定式前缀（`feat`/`fix`/`docs`/`style`/`refactor`/`perf`/`test`/`build`/`ci`/`chore`/`revert`）。
5. 禁止任何远程推送（含 `--force`、`--tags`、指定远端/分支或间接推送）；仅当用户明确要求并确认目标远端与分支后才可执行。
6. 提交信息禁止包含 `Co-Authored-By: Claude` 或其他 AI 协作者尾注。
7. 原型与 UI 迭代期间只改文件，不执行 `git add` / `git commit`；用户确认 UI 阶段结束后统一提交。

### 5.2 文档与需求

1. 文档按来源 skill 归入 `docs/.{skill-name}/`。
2. `AGENTS.md` 记录角色、当前事项、阶段、技术原理、文档索引、约束、注意事项与工作日志。
3. `README.md` 只写软件介绍、安装、使用手册、开源协议，不写需求/阶段/进度。
4. 需求使用稳定编号 `REQ-001`；功能设计、change、测试、验收必须引用编号。
5. 代码与文档不得脱节；需求/架构/功能变化必须同步更新文档。
6. 阶段文档经用户确认后，将“状态”改为“已确认”；未完成状态更新不得进入下一阶段。

### 5.3 设计与编码

1. S3 必须使用 `grill-me` 做方案/需求可行性辩驳（该 skill 禁止模型自动调用，需用户执行 `/grill-me`）。
2. S5 先写 UI 设计规范/设计系统，再做 HTML 高保真原型；UI 需覆盖核心流程、页面跳转、空/加载/失败/无权限状态、关键交互与表单校验、目标屏幕尺寸。
3. 界面必须 100% 还原原型（颜色、背景、间距等严格遵守）。
4. 必须设计全局日志、全局错误处理。
5. 组件化开发：抽离 button、input、switch、card 等公用组件；核心功能/算法抽离为 SDK。
6. 只注释「为什么」和非显而易见之处，不逐行复述。
7. 有数据库时编写数据库设计说明。
8. 密钥/token 不入日志与仓库；本地敏感信息使用系统钥匙串。

### 5.4 Change 管理

- `CHANGE.md` 是 change 状态的唯一总索引，必须在 Open / Build / Verify / Archive 各阶段同步。
- 未更新 `CHANGE.md` 的 change 不得归档或提交；Verify 报告必须包含“`CHANGE.md` 已同步”证据。

## 六、注意事项与风险

1. **anydoc 无 OCR**：图片型 PDF 会返回 `NeedsOcr`，必须优先路由 xberg；xberg 不可用但 PaddleOCR 可用时路由 PaddleOCR；两者均不可用时才提示用户跳过或等待 OCR。
2. **xberg 依赖与模型**：xberg 1.4.0 要求 Rust 1.99；OCR / 版面模型可能增加安装体积与平台差异；PP-OCRv6 small 模型缓存约 36MB，需随包预置或经确认下载；最小 feature 集、显式模型目录、禁止静默下载；需实机验证速度、体积、离线与许可。
3. **PaddleOCR 依赖**：Python + Paddle 模型，macOS 上以 CPU 为主；本项目实测 `.venv` 约 795MB；作为 P1 可选组件，必须引导安装、不可静默下载；中文低置信度时才启用，避免默认变慢。
4. **Marker 体量与许可**：代码 Apache-2.0，权重 OpenRAIL-M，含营收/竞品限制；仅 P1 用户显式安装，不随包分发模型。
5. **pandoc 为 GPL 程序**：以独立 sidecar 进程调用，不静态链接；分发时附带许可证与源码获取说明。
6. **PDF 输出**：MVP 采用 pandoc → Typst → PDF，不使用 LaTeX；需验证中文字体、分页、表格与图片，Typst sidecar 需独立签名。
7. **Tauri 跨平台 WebView 差异**：macOS WebKit、Windows WebView2、Linux WebKitGTK 的字体与渲染需分别验证。
8. **macOS 签名与公证**：Tauri 不会可靠地给 sidecar 加 hardened runtime/timestamp；需在构建前预签名所有 sidecar，分别公证 `.app` 与 `.dmg` 并 staple。项目 pandoc 3.14 已是 arm64；当前无 Developer ID，公证阻塞。
9. **pdf2docx 许可**：代码 MIT，但依赖 PyMuPDF AGPL-3.0；不随包分发、不默认安装，PDF→Word 走解析 + pandoc 管线。
10. **安全**：所有外部命令以参数数组调用，禁止拼接 shell 字符串；限制文件路径与符号链接，防止命令注入。
11. **隐私**：默认完全本地转换；仅当用户显式开启 OCR 云服务时才上传，并明确提示。

## 七、工作日志

| 日期 | 阶段 | 内容 |
| --- | --- | --- |
| 2026-11-18 | S0 | 阅读 `SIMPLE.md`；确认环境：pandoc 3.8.3、Node 22.23.3、pnpm 12.14.0、cargo 1.99.1；`marker`/`anydoc` CLI 未安装；创建本文件。 |
| 2026-11-18 | S0 → S1 | 用户确认：`maker` = Marker；场景覆盖通用互转 + 批量知识库；macOS Apple Silicon 首发；MVP 仅 anydoc + pandoc。清点测试样本（`example/` 四份文档）。进入 S1 头脑风暴。 |
| 2026-11-18 | S1 | 确认核心场景为「常见可读文档 → 排版清晰的 Markdown」，痛点为工具离散与强制联网；语料包定为厂商中立 Markdown + JSONL。生成 `docs/superpowers/产品简报.html`（含 MVP 范围、成功标准、风险与 4 项待确认）。等待用户确认。 |
| 2026-11-18 | S1 → S2 | 用户确认 S1 简报及四项决策（ManoDoc / assets / 50 条历史 / MIT）。S1 状态改为「已确认」；开始 S2 竞品分析。 |
| 2026-11-18 | Git | 初始化仓库：`main` 仅保留基线空提交，开发在 `dev`；提交 `.gitignore`、`AGENTS.md` 与 S1 产品简报；`example/` 因第三方资料仅本地保留，不纳入版本库；未配置远程、未推送。 |
| 2026-11-18 | Git | 按用户要求：`SIMPLE.md` 属于个人知识产权，从全部提交历史中移除并加入 `.gitignore`；已重写本地 `dev` 历史、清理 reflog 与不可达对象，文件仅保留在本地工作区，未推送远程。 |
| 2026-11-18 | S2 | 完成竞品分析与功能面矩阵：商业/在线 15 个、开源核心 8 个、组件 13 个、相邻平台 8 个；61 项功能面。真实样本验证：docx anydoc 49 标题 vs pandoc 0；xlsx 1048 表格行；PDF 因封面页无文本层 anydoc 失败，去封面后 12 标题/204 行；pandoc md→docx/html/epub 成功、md→pdf 缺 pdflatex。生成 `docs/superpowers/竞品分析.html`。 |
| 2026-11-18 | S2 → S3 | 用户确认 S2。状态改为「已确认」；开始编写 S3 需求文档，完成后由用户执行 `/grill-me` 辩驳。 |
| 2026-11-18 | S3 | 生成 `docs/superpowers/需求文档.md` 初稿：菜单结构、REQ-001~096、NFR-001~012、文件/语料 schema、PDF 预检策略、错误码、验收标准与 12 个待辩驳问题。MVP 采用本地 JSON 存储，暂不引入数据库。等待 `/grill-me`。 |
| 2026-11-18 | S3 变更 | 用户指示增加 xberg 内核。已同步 S1（v0.2）、S2（v0.2 决策说明）与 PRD（v0.2，REQ-097~100、REQ-015/016、路由/验收/风险）；xberg 按 P0 第二内核设计，S6 验证不达标可降级 P1。 |
| 2026-11-18 | S3 变更 | 用户指示 P1 增加 PaddleOCR。已同步 S1/S2/PRD 至 v0.3：新增 P1 中文 OCR 组件、REQ-101/102、OCR 路由/回退、验收与风险；xberg 仍为 P0 主 OCR 路径。 |
| 2026-11-18 | S3 → S4 | 用户确认 S3；本次 `/grill-me` 由用户明确跳过并在 PRD 中记录。PRD 版本升至 v1.0，状态「已确认」；开始 S4 功能拆解与软件功能设计。 |
| 2026-11-18 | S4 | 生成 `docs/superpowers/软件功能设计说明.md`：模块/菜单拆解（每行含 REQ）、6 个功能流程、5 个页面流程、5 个业务流程、数据流转、组件/SDK、状态/错误/日志、82 条 REQ + 12 条 NFR 追溯、里程碑。等待用户确认。 |
| 2026-11-18 | S4 → S5 | 用户确认 S4，状态改为「已确认」；进入 S5，先写 UI 设计规范，再做高保真原型。UI 迭代期间只改文件，不提交 Git，待用户确认 UI 阶段后统一提交。 |
| 2026-11-18 | S5 变更 | 用户指定应用数据统一存放在 `~/.manodoc/`；已同步 PRODUCT.md、PRD 6.4、S4 6.2/6.4 与 AGENTS。用户输出目录仍独立，不放入该目录。 |
| 2026-11-18 | S5 变更 | 用户增补中英文切换、主题切换、Cmd+K 命令面板（P0）。已同步 PRODUCT.md、S1（v0.4）、S2（v0.4）、PRD（v1.1，REQ-103~105）、S4（v1.1）与 AGENTS；S5 设计规范与原型必须覆盖。 |
| 2026-11-18 | S5 | 视觉方向经决策页选定 **Proofing Desk**（校样台）。生成 `UI设计规范.html`（Proofing Desk token、浅/深色、双语、命令面板、组件与验收）与 `UI.html` 高保真原型（转换/批量/历史/设置、空/加载/失败、PDF 预检、Cmd+K、主题/语言切换）。finish review 判定 fix，8 项 material fixes 已全部修复：空白校样纸空状态与单一拖放目标、开关/历史键盘可操作、英文补全、OCR 跳过恢复、深色纸张层级、SVG 校对记号、选择态改蓝、1280 适配；新增转换中状态条与 toast。Playwright 交互/视觉回归 60+ 项全部通过，无控制台错误；detector 0 反模式（仅 em-dash 建议项）。截图在 `.impeccable/review/`。等待用户确认 S5；UI 迭代期间未提交 Git。 |
| 2026-11-18 | S5 → Git | 用户指示提交 Git；S5 状态改为「已确认」；提交 UI 设计规范、高保真原型与 AGENTS 更新；未推送远程。 |
| 2026-11-18 | S5 变更 | 用户要求历史详情以弹窗呈现。历史页改为全宽任务列表；点击行打开原生 `<dialog>` 详情，支持 Esc/遮罩/关闭按钮退出、焦点返回触发行、行内打开/重试动作与弹窗内复制/打开反馈；同步 UI 设计规范与验收清单。新增弹窗回归测试 25 项；UI 迭代期间未提交。 |
| 2026-11-18 | S5 变更 / 范围扩展 | 用户优化请求：① 保留基础一对一互转（Word/PDF/Markdown/PNG/JPEG 等）并在多引擎方向提供引擎选择；② Python 引擎使用内置 uv 在应用数据目录创建 `.venv`；③ 补齐文件类型图标。已更新 UI 原型（方向/引擎选择、支持的方向矩阵、批量输出/引擎、历史图标与输出格式、设置页 Python 运行时）、UI 设计规范、PRD v1.2、S4 v1.2、S1/S2 v0.5 与 PRODUCT.md；新增 8 个图标；全套回归 118 项通过。UI 迭代期间未提交。 |
| 2026-11-18 | S5 变更 | 按用户要求将一级菜单改为「文档转换 / 语料制作 / 转换历史 / 系统设置」；单文件页移除清洗/切块/语料开关，全部并入语料制作配置；同步 UI、UI 规范、PRD、S4 与 AGENTS。UI 迭代期间未提交。 |
| 2026-11-18 | S5 变更 | 「批量配置」标题改为「配置」；语料制作输出格式锁定 Markdown，Word/PDF 选项禁用并给出提示；同步 UI 规范、PRD、S4 与 AGENTS。 |
| 2026-11-18 | S5 变更 | 语料制作列表完成行增加 Markdown 预览：预览渲染 GFM，仅保留预览；新增 REQ-111，同步 PRD v1.3、S4 v1.3、UI 规范与 AGENTS。 |
| 2026-11-18 | S5 变更 | 语料制作完成行新增 Markdown 预览；仅保留预览（按用户要求移除编辑）；新增 REQ-111，PRD/S4 升至 v1.3；全套回归 145 项通过、detector 0 反模式；未提交 Git。 |
| 2026-11-18 | S5 变更 | 按用户决定取消预览：移除转换页文档预览块与语料制作 Markdown 预览/编辑，进度与 OCR 错误改为配置卡内联；同步 UI、UI 规范、PRD v1.4、S4 v1.4、S1/S2 v0.6、PRODUCT 与 AGENTS；REQ-020~022、REQ-048、REQ-111 标记已取消（编号保留）；新增内联状态样式与空态拖放区；回归 119 项通过、detector 0 反模式；已提交 Git，未推送远程。 |
| 2026-11-18 | S5 → Git | 用户指示提交 Git；S1/S2 v0.6、PRD v1.4、S4 v1.4、UI 规范 v1.1 与原型状态改为「已确认」；提交 UI、设计文档、需求文档与 `icons/`；未推送远程。 |
| 2026-11-18 | S5 自检 | 对照原型与 S1/S2/PRD/S4/UI 规范逐项检查：补齐成功与无权限转换状态、批量清洗/跳过状态、并发与切块重叠、设置「预设」管理；统一一级菜单正式名称；澄清 PDF 策略在预检弹窗、语料包全局默认与批次覆盖关系；同步 PRD v1.5、S4 v1.5、UI 规范 v1.2；回归 137 项通过、detector 0 反模式；已提交 Git，未推送远程。 |
| 2026-11-18 | S6 | 完成技术选型讨论并生成 `docs/superpowers/技术选型.md` v0.1：确认 Typst PDF、PDF→MD→DOCX（不分发 pdf2docx）、uv 按需下载 Python、React + TS + Vite + Tailwind；记录版本矩阵、许可、打包与 spike 门槛；待用户确认。 |
| 2026-11-18 | S6 spike | 技术选型冻结为 v1.0（系统字体优先）；S6-1 通过：anydoc docx 49 标题/181ms、xlsx 1048 表格行/26ms、PDF 封面 `NeedsOcr`、去封面 14 标题/1049ms、Markdown 直通不交 anydoc；S6-2 部分：xberg formats 47MB/3–5ms，OCR 构建缺 cmake；S6-3 部分：pandoc→Typst 生成 39,550 字节 `.typ`，Typst 编译器未安装；S6-6 部分：去封面 PDF→MD→DOCX 55,709 字节/18 表，可打开；未提交 Git。 |
| 2026-11-18 | S6 spike | 工具就绪：Typst 0.15.1 项目内安装、cmake 4.4.5 brew。S6-2：xberg `formats-no-heic` 47MB/1m48s，加 `paddle-ocr-tract` 101MB/2m40s；PaddleOCR 扫描页自动下载模型失败（hf-mirror 缺 ETag），需预置模型。S6-3：pandoc `-s` → Typst 编译 24 页中文 PDF（691KB），系统字体正常；非 standalone 因 `#horizontalrule` 失败。S6-6：PDF→MD→DOCX 55,709 字节/18 表可打开。未提交 Git。 |
| 2026-11-18 | S6 spike | 补充 S6-2 模型结论：PP-OCRv6 small det≈9.9MB、rec≈21.2MB，总计约 31MB；建议随包预置或经确认下载并校验哈希。S6-3 已通过（pandoc `-s` + Typst 0.15.1，24 页中文 PDF，691KB，系统字体正常）。 |
| 2026-11-18 | S6 spike | S6-2 通过：`paddle-ocr-ort` 104.4MB/2m12s，中文扫描页 1.58s、文本正确；tract 同页 30.4s 弃用；官方 HF 模型缓存 36MB，hf-mirror 缺 ETag 失败，正式版需预置模型。 |
| 2026-11-19 | S6 → Git | 用户指示提交 Git；提交技术选型 v1.0、S6-1/2/3 spike 结果、PRD/S4/S1/S2/UI 同步与 `tools/` 忽略规则；未推送远程。 |
| 2026-11-19 | S6 spike | S6-4 通过（项目 uv 0.13.9）：CPython 3.12.16 下载 24.1MB/4.86s、安装 67MB；`.venv` 指向项目内 Python；paddleocr 3.7.0 + paddlepaddle 3.3.1 安装 30s，venv 795MB，CPU 导入正常；离线无缓存下载失败、已有 Python 可离线建 venv、缓存包可离线安装。 |
| 2026-11-19 | S6-4 → Git | 用户指示提交 Git；提交 S6-4 uv 0.13.9 验证结果、PaddleOCR venv 795MB/离线行为、`--no-bin` 约束与文档同步；未推送远程。 |
| 2026-11-19 | S6 spike | S6-5 结构验证：临时 `.app` 中 typst/uv/pandoc-arm64 按 inside-out ad-hoc 签名，`codesign --verify --deep --strict` 通过，`spctl` 按预期拒绝；本机 0 个 codesigning identity，无法公证；系统 pandoc x86_64，官方 arm64 zip 验证可重签。 |
| 2026-11-19 | S6 spike | 改用项目 pandoc 3.14（arm64，188MB）：Typst writer 输出 `#divider()`，非 standalone 编译 21 页中文 PDF（688KB），不再需要 `-s`；S6-5 临时 `.app` 用项目 typst/uv/pandoc 重签，ad-hoc 结构验证通过，公证仍缺 Developer ID。 |
| 2026-11-19 | S6-6 | PDF→Word 质量复核：源 PDF 49 页/61,172 字符/65 图；anydoc MD 59,781 字符/14 标题/0 图；pandoc DOCX 55,709 字节/18 表/0 媒体；Quick Look 目录压平、页码粘连。结论：功能可用、质量 C，需 UI 标注目录/图片保真限制。 |
| 2026-11-19 | S6-6 | 用户接受 PDF→Word 质量 C：保留 PDF→MD→DOCX 管线，UI 方向矩阵标注目录/图片保真有限；S6-5 公证延后到发布前；技术选型升至 v1.1。 |
| 2026-11-19 | S6-6 → Git | 用户指示提交 Git；提交 PDF→Word 质量 C 接受结论、方向矩阵提示、技术选型 v1.1 与 AGENTS 同步；未推送远程。 |
| 2026-11-19 | S7 | 建立 `CHANGE.md` 总索引：CHG-001 `mvp-implementation`（Build）与后续 CHG-002 批量语料、CHG-003 xberg OCR、CHG-004 pandoc/Typst、CHG-005 打包公证。 |
| 2026-11-19 | S7 Build | 完成 CHG-001 纵向切片：React 19 + Vite + Tailwind 前端，Tauri 2 + anydoc 0.2.4 后端；四页导航、设置/主题/中英文、Cmd+K、单文件转换、历史与详情弹窗。真实 docx → Markdown 315,999 字符/49 标题/15 表格行；封面无文本 PDF 返回 `E_NEEDS_OCR`；不支持格式返回 `E_UNSUPPORTED_FORMAT`；语言/主题写入 `~/.manodoc/settings.json`，历史写入 `history.json`。`pnpm build`、`pnpm test`（7 项）、`cargo check`、`cargo test`（5 项 + 3 个样例测试）通过；未提交 Git。 |
| 2026-11-19 | S7 Verify → Archive | CHG-001 通过 Comet Verify：A1–A8 全部通过；用户接受结果并归档至 `docs/comet/archive/2026-11-18-mvp-implementation/`。补充实机验证：1440×900 与 1280×800 无横向溢出；文本型 PDF → Markdown 不触发 OCR；CSV/RTF/EPUB 由 anydoc 0.2.4 独立 harness 验证；不可写输出目录显示 `E_OUTPUT_PERMISSION`。代码仍未提交 Git。 |
| 2026-11-19 | S7 → Git | 用户指示提交 Git；提交 CHG-001 实现、`CHANGE.md`、AGENTS 更新与 Comet 归档材料到 `dev`（`ba220d2`）；未推送远程。 |
| 2026-11-19 | S7 Build | CHG-002 完成批量语料候选实现：新增 `batch`/`clean`/`chunk`/`corpus` Rust 模块、批量 Tauri commands、历史批量字段；前端语料制作页接入扫描/队列/配置/取消/重试/报告，历史支持批量详情与重跑。实机 example 批次 3 文件（2 成功、1 个封面 PDF `E_NEEDS_OCR`）、293 chunks、2 warnings；输出包含 mirror、chunks.jsonl、manifest.json、report.json/csv，JSONL 与 manifest 校验通过。`pnpm build`、`pnpm test`（10 项）、`cargo test`（18 项 + 4 个样例测试）通过；未提交 Git。 |
| 2026-11-19 | S7 Verify → Archive | CHG-002 通过 Comet Verify（A1–A8）；用户接受并归档至 `docs/comet/archive/2026-11-18-batch-corpus-v2/`。原 `batch-corpus` change 的编辑复核状态在还原已批准正式材料后仍阻塞 Verify，已取消并重建为 `batch-corpus-v2`；代码、需求与验收不变。风险：实机取消按钮未取得直接中断证据，取消逻辑由单元测试覆盖。 |
| 2026-11-19 | S7 → Git | 用户指示提交 Git；提交 CHG-002 实现、测试、`CHANGE.md`、AGENTS 与 Comet 归档材料到 `dev`（`f5cafb6`）；未推送远程。 |
| 2026-11-19 | S7 Build | CHG-003 完成 xberg OCR 候选实现：新增 `engine/routing.rs`、`engine/xberg.rs`、`ocr/mod.rs` 与 `OcrModelCard`；单文件/图片/批量接入 auto/anydoc/xberg 路由、OCR 元数据与模型显式安装。实机 PDF OCR 64154 字符/2 页、图片 OCR、批量 4 文件/356 chunks；`pnpm build`、`pnpm test`（12 项）、`cargo test`（23 项 + 9 ignored）通过。本地 HTTP 镜像验证安装下载/SHA-256/幂等。 |
| 2026-11-19 | S7 Verify → Archive | CHG-003 通过 Comet Verify（A1–A8）；用户按既有授权接受并归档至 `docs/comet/archive/2026-11-18-xberg-ocr/`。后继候选仅同步 `CHANGE.md`，检查证据复用后通过。 |
| 2026-11-19 | S7 Build | CHG-004 完成 pandoc/Typst 候选实现：新增 `engine/export.rs`、`pandoc.rs`、`typst.rs`、`sidecar.rs`；转换页目标格式、质量提示与历史重跑；Markdown→Word/PDF、PDF→Word 实机通过。未提交 Git。 |
| 2026-11-19 | S7 Verify → Archive | CHG-004 通过 Comet Verify（A1–A8）；用户按既有授权接受并归档至 `docs/comet/archive/2026-11-18-pandoc-typst/`。Runtime 检查 `pnpm build`、`pnpm test`、`cargo check`、`cargo test` 均通过。 |
| 2026-11-19 | S7 Build | CHG-005 完成打包与签名候选实现：新增 `scripts/package-macos.sh` 与 `docs/superpowers/打包与签名.md`；配置 `externalBin` 与 dmg、生成 macOS 图标集；修复 Tauri 无签名身份时不生成 `_CodeSignature` 的问题，并用 `TESSERACT_RS_CACHE_DIR` 复用离线缓存。 |
| 2026-11-19 | S7 Verify → Archive | CHG-005 通过 Comet Verify（A1–A8）；用户按既有授权接受并归档至 `docs/comet/archive/2026-11-18-packaging-signing/`。公证因缺少 Developer ID 记录为发布前阻塞。 |
| 2026-11-19 | 维护 | OCR 模型下载改用 hf-hub 1.0：模型根目录调整为 `~/.manodoc/models`，固定官方 endpoint，保留 SHA-256 与进度；实测从 huggingface.co 下载 31MB/14.2s 成功，本地镜像测试通过；`pnpm build`、`pnpm test`（13 项）、`cargo test`（25 项 + 13 ignored）通过。 |
| 2026-11-19 | 维护 | 明确前端资源策略：保持 Tauri 内嵌，打包脚本新增 `Resources` 不含前端产物与 >10MB 未声明文件的断言；`打包与签名.md` 与 `CHANGE.md` 同步验收口径。 |
| 2026-11-19 | 维护 | 顶部改为 macOS 风格整宽标题栏：系统红绿灯 + 侧栏折叠 + 命令面板搜索 + 居中 ManoDoc + 语言/主题；同步 `UI.html` 与 `UI设计规范` v1.5；实机验证中英文、深浅色、1280×800 与侧栏折叠；`pnpm build`、`pnpm test`（13 项）通过，UI 迭代期间未提交。 |
| 2026-11-19 | 维护 | 语料制作配置将并发、清洗与语料、切块上限/重叠折叠为「高级设置」，折叠时显示并发与切块摘要；实机展开/收起验证，`pnpm build`、`pnpm test`（14 项）通过；同步 UI 原型与设计规范。 |
| 2026-11-19 | 维护 | 全部原生 select 替换为 shadcn/ui Select（Radix Select 2.4.1）：新增 `components/ui/select.tsx`、`lib/utils.ts`、`@` 别名与 `components.json`；覆盖转换、语料、设置共 13 处；补充 Radix 测试替身，`pnpm build`、`pnpm test`（14 项）通过；实机校验菜单、禁用态与方向行内选择器。 |
| 2026-11-19 | 维护 | 弹窗迁移到 shadcn/ui Dialog（Radix Dialog 1.3.1）：统一 Modal 封装改为固定屏幕居中，遮罩、Esc、焦点返回由 Radix 处理；方向矩阵、历史详情、批量详情与清空确认四处已覆盖；测试改为 role=dialog，`pnpm build`、`pnpm test`（14 项）通过。 |
| 2026-11-19 | S7 Build | CHG-006 P0 收口候选实现：新增日志/预检/预设/图片资产模块与设置页输出、预设、日志、关于；标题栏任务状态、全局拖入路由、预检弹窗与跳过策略、诊断复制、引擎持久化；修复 async 命令内 `block_on` 死锁与跳过页临时名泄漏。实机 PDF 26s 完成、130 处资产引用有效、诊断 zip 与预设编辑通过；`pnpm build`、`pnpm test`（17 项）、`cargo test` 通过；生成 `docs/superpowers/P0自检.md`。 |
| 2026-11-19 | S7 Verify → Archive | CHG-006 通过 Comet Verify（A1–A8）；用户接受结果并归档至 `docs/comet/archive/2026-11-19-p0-completion/`；71 条 P0 关闭 |

| 2026-11-19 | 维护 | 项目与产品更名为 ManoDoc：应用标识 `dev.manodoc.app`、crate/包名、数据目录 `~/.manodoc`、环境变量 `MANODOC_*`、界面文案与文档同步；按新项目处理，不保留 `.dull_doc` 目录兼容。`pnpm build`、`pnpm test`（17 项）、`cargo test`（29 项）通过。 |

| 2026-11-19 | 维护 | 重新设计产品 Logo：MD 共享竖笔构成，D 字怀内双向箭头表达 Markdown ↔ Document 转换；暖纸 + 深绿，输出 `docs/superpowers/logo/logo-manodoc.svg` 与 1024 PNG，并替换 Tauri 全套图标（32/64/128/256/512 与 `.icns`）。 |
| 2026-11-19 | 发布 | 完成正式发布材料：`README.md`（介绍/安装/使用/注意事项/协议）、`LICENSE`（MIT）与产品文档站 `docs/index.html`（产品介绍、截图、下载安装、使用、注意事项、版权、协议、版本日志）；截图使用中性演示文件，未包含 `example/` 内容。`pnpm package:mac` 产出 `ManoDoc_0.1.0_aarch64.dmg`（119.7MB），ad-hoc 签名通过，公证因缺 Developer ID 仍以退出码 20 阻塞。 |
| 2026-11-19 | 维护 | 补充 Windows / Linux 自行编译入口：README 与产品文档站新增平台分发表、Windows（MSVC + WebView2 + PowerShell 暂存 sidecar）与 Linux（WebKitGTK 4.1 + sidecar）构建步骤、target triple 命名与产物路径；Tauri 配置移除 macOS 专用 `bundle.targets` 并加入 `icon.ico`，sidecar 解析按平台补 `.exe` 后缀、错误提示改为平台中立；`cargo test`（29 项）、`pnpm build`、`pnpm test`（17 项）通过。Windows / Linux 构建尚未实机验证。 |
| 2026-11-19 | 维护 | 修复 macOS 图标：原 PNG/ICNS 圆角外为不透明白色且铺满画布；重制为 824×824 居中、四周 100px 透明的 macOS 图标网格，重新导出 PNG / `icon.icns` / `icon.ico`，品牌 Logo 两份 PNG 同步去白底；重新打包 `.app` 与 `.dmg` 验证嵌入 icns 角像素透明、卷图标一致，ad-hoc 签名通过（公证仍缺 Developer ID）。 |
| 2026-11-19 | 维护 | PDF 转换新增「全文强制 OCR」与乱码自动回退：`force_ocr` 贯通 xberg 适配器、导出、路由与 `convert_file`；路由按控制字符/私用区/替换符占比（≥1%）检测文字层乱码，OCR 可用时自动改用全文 OCR 并记录 `garbled-text` 回退原因，不可用时返回 `E_NEEDS_OCR`；转换页新增开关，预检弹窗新增「全文 OCR」。测试：3 项单元测试（乱码判定、正常中文、短文本不误报）与 2 项 ignored 集成测试（example 乱码 PDF 的全文 OCR 与自动检测）通过；`pnpm build`、`pnpm test`（19 项）、`cargo test`（32 项 + 15 ignored）通过。 |

<comet-ambient-resume>
<!-- Managed by Comet. Edits inside this block may be replaced by comet init/update. -->
<!-- Contract: comet.resume_probe.v2 -->

## Comet Ambient Resume

在这个仓库中，开始处理需要改动或调查的任务前，如果可能存在活跃 Comet workflow，把当前用户请求传入只读探针：`comet resume-probe . --stdin --json`。

- 如果用户通过宿主明确调用任意 Comet Skill（例如 `@comet`、`/comet`、`@comet-native` 或 `/comet-hotfix`），显式调用优先于本恢复协议；不要运行 resume probe，直接进入被调用的 Skill。
- 如果用户通过宿主明确调用的是非 Comet 的 Skill 或斜杠命令，任务意图已由该调用明确：不要运行 resume probe，直接执行该 Skill。
- 如果你正在 Comet 流程内（包括正在等待用户回复你在流程中提出的问题），不要运行 resume probe；把这类回复（例如方案/选项选择）当作当前 change 的继续，直接按用户的选择推进。
- 只信任返回的 `workflow`、`skill` 和 `entrySource`；它们只由项目配置或无配置兼容回退决定。不得扫描或切换另一套 workflow。
- 如果 probe 返回 `auto_resume`，简短说明选中的 active change，并进入 `nextCommand` 指向的永久入口。不要把状态命令当作恢复入口直接推进。
- 如果 probe 返回 `ask_user`，只问一个简短问题并等待用户回复。
- 如果当前请求未明确调用 Comet Skill，且 probe 返回 `out_of_scope` 或 `none`，不要进入 Comet workflow。
- `out_of_scope` 或 `none` 只表示不要因为这个新请求进入 Comet workflow；它绝不表示要暂停或退出一个已在进行的 Comet 流程。
- 如果配置或状态无效且没有 `nextCommand`，停止并报告原因；不要猜测另一个 workflow。
- 不能只因为存在 active change 就把无关任务挂到该 change。Native 的未提交改动由 Native 入口检查，不由探针自动归因。
</comet-ambient-resume>
