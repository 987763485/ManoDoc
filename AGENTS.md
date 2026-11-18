# AGENTS.md — DullDoc 项目运营手册

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
| 产品名 | **DullDoc**（暂定，S1 确认） |
| 当前阶段 | **S5 UI 设计规范 + 高保真原型（进行中）** |
| 下一阶段 | S6 技术选型探讨 |
| 当前事项 | 先编写 `docs/superpowers/UI设计规范.html`，再编写 `docs/superpowers/UI.html`；覆盖中英文切换、主题切换、命令面板；UI 迭代期间不提交 Git |
| 已确认 | ① `maker` = datalab-to/marker；② 首要场景同时覆盖「通用办公格式互转」与「批量知识库构建」；③ 首发 macOS Apple Silicon；④ MVP 接 anydoc + pandoc；⑤ 2026-11-18 追加 xberg 内核（P0）；⑥ 2026-11-18 追加 PaddleOCR（P1 OCR 组件）；⑦ 2026-11-18 追加中英文切换、主题切换、命令面板（P0） |

### 已确认的产品决策

- **引擎分工**：anydoc 负责 Office/文本型 PDF 的快速路径；xberg（原 Kreuzberg，Rust/MIT）作为第二内核，负责扫描件/图片 PDF 的 OCR、复杂表格与 anydoc 失败回退；PaddleOCR（P1，Apache-2.0）作为中文 OCR 增强组件；pandoc 负责 Markdown → HTML/DOCX/EPUB 导出；Marker 保留为 P1 的备选高质量引擎。
- **引擎路由**：按文件内容判定格式；文本型 PDF 先由 anydoc 处理，失败或质量不足时回退 xberg；扫描件优先 xberg OCR，中文低置信度或用户选择时可用 PaddleOCR（P1）；xberg/PaddleOCR 均不可用时才进入「跳过无文本页 / 提示需要 OCR」的降级路径，不得静默失败。
- **批量知识库产物**：厂商中立的「镜像 `.md` + `chunks.jsonl` + `manifest.json`」语料包；清洗、切块、元数据均为可勾选项；不绑定 Dify / RAGFlow 等具体平台。
- **测试样本**：`example/` 下为测试文件——`.docx`（GB/T 28448-2019 等保测评要求，表格密集）、50 页 PDF（有完整文本层，约 6.1 万字）、`.xlsx`（等保指标清单）、`.md`（1074 行，用于验证 Markdown 再导出）。
- **名称与协议**：产品名保留 DullDoc；开源协议 MIT；pandoc 以独立 sidecar 按 GPL 合规分发。
- **图片策略**：默认输出到同目录 `assets/` 并使用相对路径；单文件导出可选内嵌 base64。
- **历史记录**：保留最近 50 条任务元数据（可配置），只存路径、参数、哈希与结果摘要，不复制文件。
- **体验偏好（2026-11-18 追加，P0）**：中英文切换（中文默认、英文完整、无需重启）；主题切换（跟随系统/浅色/深色）；Cmd+K 命令面板（页面/操作/最近文件，危险操作二次确认）。所有 UI 组件与颜色必须走 token，禁止硬编码。

## 三、技术原理（初步）

- **应用框架**：Tauri 2。Rust 负责文件系统、进程管理、转换编排与安全边界；前端使用 WebView 渲染界面。
- **转换内核**（四个解析/导出引擎 + 可选 OCR 组件）：
  - **anydoc**（Firecrawl，纯 Rust，MIT）：Word / PowerPoint / Excel / OpenDocument / RTF / EPUB / CSV / 文本型 PDF → GitHub 风格 Markdown；速度快、无网络、无 OCR。Rust 可直接以 crate 形式嵌入 Tauri 后端。
  - **xberg**（原 Kreuzberg，Rust，MIT）：107 种格式 → 文本 / Markdown / HTML / JSON / DocTags；内置 OCR（Tesseract / PaddleOCR / Candle / VLM）、版面与表格识别、语法切块、CLI/API/MCP。作为第二内核，负责扫描件、复杂 PDF 表格与 anydoc 回退。
  - **PaddleOCR**（P1，Python，Apache-2.0）：中文 OCR / PP-Structure；作为 xberg 之外的中文 OCR 增强组件，检测环境或引导安装，不随包分发模型。
  - **pandoc**（3.8.3，已安装）：通用标记语言转换器，负责 Markdown → HTML / DOCX / ODT / EPUB / LaTeX 等导出，以及部分格式兜底。以 sidecar 方式随应用分发。
  - **Marker**（`marker-pdf`，Datalab，Python + ML 模型）：扫描 PDF / 图片 / 复杂版式 → Markdown / HTML / JSON，含版面分析、OCR、表格与公式识别。资源占用高，保留为 P1 备选。
- **数据处理原则**：本地优先、默认不上传；应用数据统一存放 `~/.dull_doc/`（settings/presets/history/logs/tmp），用户输出目录独立；转换任务异步化、可取消、可查看进度与日志；失败可重试并保留中间产物。
- **日志与错误**：全局日志、全局错误处理、任务级错误归因；敏感信息（API Key、token）不得落盘或写日志，本地敏感信息存系统钥匙串。

## 四、文档索引

| 阶段 | 文档 | 路径 | 状态 |
| --- | --- | --- | --- |
| S0 | 项目运营手册 | `AGENTS.md` | 已生成 |
| S1 | 产品简报 | `docs/superpowers/产品简报.html` | 已确认（v0.4） |
| S2 | 竞品分析 + 功能面矩阵 | `docs/superpowers/竞品分析.html` | 已确认（v0.4） |
| S3 | 需求文档（PRD） | `docs/superpowers/需求文档.md` | 已确认（v1.1） |
| S4 | 软件功能设计说明 | `docs/superpowers/软件功能设计说明.md` | 已确认（v1.1） |
| S5 | UI 设计规范 | `docs/superpowers/UI设计规范.html` | 进行中 |
| S5 | UI 高保真原型 | `docs/superpowers/UI.html` | 未开始 |
| S7 | Change 总索引 | `CHANGE.md` | 未开始 |
| — | 项目说明（正式发布口径） | `README.md` | 未开始 |

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
2. **xberg 依赖与模型**：OCR / 版面模型可能增加安装体积与平台差异；优先检测环境或使用最小模型，禁止静默下载；S6 用真实样本验证速度、体积与许可。
3. **PaddleOCR 依赖**：Python + Paddle 模型，macOS 上以 CPU 为主；作为 P1 可选组件，必须引导安装、不可静默下载；中文低置信度时才启用，避免默认变慢。
4. **Marker 体量大**：保留为 P1 备选；Python 环境 + ML 模型可能达到数 GB，优先“检测系统环境 / 引导安装”，避免打包模型。
5. **pandoc 为 GPL 程序**：以独立 sidecar 进程调用，不静态链接；分发时附带许可证与源码获取说明。
6. **pandoc 输出 PDF** 需要 LaTeX 引擎或替代方案（如 WebView 打印、Typst），需在 S6 选型时单独评估。
7. **Tauri 跨平台 WebView 差异**：macOS WebKit、Windows WebView2、Linux WebKitGTK 的字体与渲染需分别验证。
8. **macOS 签名与公证**：包含外部二进制、xberg 模型与可选 Python 环境时，分发前需处理 Hardened Runtime、entitlements 与 notarization。
9. **安全**：所有外部命令以参数数组调用，禁止拼接 shell 字符串；限制文件路径与符号链接，防止命令注入。
10. **隐私**：默认完全本地转换；仅当用户显式开启 OCR 云服务时才上传，并明确提示。

## 七、工作日志

| 日期 | 阶段 | 内容 |
| --- | --- | --- |
| 2026-11-18 | S0 | 阅读 `SIMPLE.md`；确认环境：pandoc 3.8.3、Node 22.23.3、pnpm 12.14.0、cargo 1.99.1；`marker`/`anydoc` CLI 未安装；创建本文件。 |
| 2026-11-18 | S0 → S1 | 用户确认：`maker` = Marker；场景覆盖通用互转 + 批量知识库；macOS Apple Silicon 首发；MVP 仅 anydoc + pandoc。清点测试样本（`example/` 四份文档）。进入 S1 头脑风暴。 |
| 2026-11-18 | S1 | 确认核心场景为「常见可读文档 → 排版清晰的 Markdown」，痛点为工具离散与强制联网；语料包定为厂商中立 Markdown + JSONL。生成 `docs/superpowers/产品简报.html`（含 MVP 范围、成功标准、风险与 4 项待确认）。等待用户确认。 |
| 2026-11-18 | S1 → S2 | 用户确认 S1 简报及四项决策（DullDoc / assets / 50 条历史 / MIT）。S1 状态改为「已确认」；开始 S2 竞品分析。 |
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
| 2026-11-18 | S5 变更 | 用户指定应用数据统一存放在 `~/.dull_doc/`；已同步 PRODUCT.md、PRD 6.4、S4 6.2/6.4 与 AGENTS。用户输出目录仍独立，不放入该目录。 |
| 2026-11-18 | S5 变更 | 用户增补中英文切换、主题切换、Cmd+K 命令面板（P0）。已同步 PRODUCT.md、S1（v0.4）、S2（v0.4）、PRD（v1.1，REQ-103~105）、S4（v1.1）与 AGENTS；S5 设计规范与原型必须覆盖。 |
