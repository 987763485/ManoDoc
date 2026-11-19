# Outcome

交付 DullDoc 的第一条可运行编码切片：macOS 上可启动的 Tauri 2 桌面应用，包含四页导航、按 UI 原型还原的界面、设置/主题/中英文切换、Cmd+K 命令面板、单文件 Office/文本型 PDF → Markdown（anydoc）与转换历史。

本 change 的目标是让真实文件可以在本机完成一次可复现的转换，并把结果、错误和历史呈现给用户；批量语料、xberg OCR、pandoc/Typst 导出与签名公证留给后续 change，并在 `CHANGE.md` 中登记。

# Scope

## 目录结构

### 新建

- `package.json`、`pnpm-lock.yaml`、`vite.config.ts`、`tsconfig*.json`、`index.html` — React 19 + TypeScript + Vite 前端工程。
- `src/main.tsx`、`src/App.tsx` — 应用入口、四页导航与全局状态。
- `src/pages/ConvertPage.tsx` — 文档转换页：文件选择/拖放、方向与引擎展示、转换状态、内联成功/错误。
- `src/pages/BatchPage.tsx` — 语料制作页的纵向切片占位：保留导航与原型视觉，明确标注后续 change 接入。
- `src/pages/HistoryPage.tsx` — 转换历史列表、详情弹窗、重跑、打开、清空确认。
- `src/pages/SettingsPage.tsx` — 通用、引擎、输出、预设、日志、关于分组的设置页；本切片只持久化语言、主题、输出目录。
- `src/components/` — Button、Dialog、FileIcon、Progress、StatusBadge、EmptyState、ErrorCard、SuccessCard 等公用组件。
- `src/i18n/` — 中英文文案与 `react-i18next` 初始化。
- `src/lib/ipc.ts` — Tauri command 调用与类型化错误封装。
- `src/styles/` — Proofing Desk token、Tailwind 主题与全局样式。
- `src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json`、`src-tauri/src/main.rs`、`src-tauri/src/lib.rs` — Tauri 2 应用骨架。
- `src-tauri/src/commands/` — 转换、取消、历史、设置、打开输出目录等 command。
- `src-tauri/src/engine/anydoc.rs` — anydoc 适配、格式识别、资产写出与错误映射。
- `src-tauri/src/storage/` — `~/.dull_doc` 下 settings/history 的 JSON 原子读写与 schema 版本。
- `src-tauri/src/security/` — 路径规范化、输出目录边界与命令参数约束。
- `CHANGE.md` — 登记本 change 与后续 change。

### 修改

- `AGENTS.md` — 编码阶段状态、当前 change、测试与后续 change 索引。
- `docs/comet/changes/mvp-implementation/brief.md`、`docs/comet/changes/mvp-implementation/specs/mvp/spec.md` — 本 change 的正式需求与规格。
- `PRODUCT.md` — 仅在本切片改变已确认能力边界时同步；当前不改产品定位。

### 明确不建

- 不建批量队列后端与语料切块/打包实现；由后续 change 负责。
- 不建 xberg OCR 适配与模型下载；本切片只把 `NeedsOcr` 呈现为可理解的错误。
- 不建 pandoc/Typst sidecar 与 PDF 导出；由后续 change 负责。
- 不建签名、公证、DMG 与发布流水线；S6-5 在发布前补。
- 不建数据库；本切片使用 JSON 持久化。

# Non-goals

- 批量转换、语料清洗、切块、chunks.jsonl、manifest.json。
- xberg OCR、PaddleOCR、Marker、扫描件处理。
- Markdown → Word/PDF/HTML/EPUB、PDF → Word、Typst PDF。
- 签名、公证、自动更新、远程推送与发布。
- 云服务、账号、团队协作、向量库 UI。

# Acceptance examples

- A1：应用启动后默认进入文档转换页，四页导航可切换，1280×800 与 1440×900 无横向溢出，已实现页面符合 UI 原型。
- A2：语言切换为 English、主题切换为深色后即时生效并持久化；Cmd+K 可搜索并进入系统设置，Esc 关闭后焦点返回。
- A3：拖入 GB/T 28448 docx 后完成 Word→Markdown，输出文件包含 40 个以上标题与至少一个表格，历史新增完成记录。
- A4：xlsx、CSV、RTF、EPUB 或文本型 PDF 均可通过 anydoc 产出 Markdown，文本型 PDF 不触发 OCR。
- A5：含无文本封面页的 PDF 显示 E_NEEDS_OCR、页数与下一步，不产生不完整输出；不支持格式显示可执行错误且应用不崩溃。
- A6：历史详情展示方向、引擎、输出与统计，支持重跑/打开/复制参数；清空历史需确认且不删除输出文件，重启后历史仍保留。
- A7：语言、主题、输出目录写入 ~/.dull_doc/settings.json 并在重启后恢复；输入/输出不可写时显示 E_OUTPUT_PERMISSION 与可执行下一步，不产生半写文件。
- A8：pnpm build、cargo check 与测试通过；断网启动并完成一次 docx→md 转换，过程中无网络请求，~/.dull_doc 只写入 settings/history/logs/tmp。
### A1 — 应用启动与四页导航

打开应用后默认进入文档转换页；四页导航可切换，1280×800 与 1440×900 无横向溢出；已实现页面按 `docs/superpowers/UI.html` 的 Proofing Desk 视觉还原。

### A2 — 语言、主题与命令面板

在设置页切换中文/English、浅色/深色/跟随系统后界面即时更新并持久化；Cmd+K 可搜索并切换页面、执行打开输出目录等操作，Esc 关闭。

### A3 — Word 转 Markdown

拖入或选择 `example/GB:T 28448-2019 信息安全技术 网络安全等级保护测评要求.docx`，开始转换后得到 Markdown 文件；文件中包含标题与表格，输出路径可见，历史新增一条完成记录。

### A4 — 其他 anydoc 格式

选择 `example/等级保护测评2.0要求指标清单v1.xlsx` 或 CSV/RTF/EPUB/文本型 PDF，均通过 anydoc 产出 Markdown；文本型 PDF 不调用 OCR；结果与输入格式匹配。

### A5 — 需要 OCR 与不可转换错误

打开含无文本封面页的 PDF 时，界面显示 `E_NEEDS_OCR`、页数与下一步，不产生不完整输出；不支持的格式或加密文件显示错误码与可执行建议，应用不崩溃。

### A6 — 历史

历史页展示最近任务的时间、文件、状态、引擎与输出摘要；点击行打开详情弹窗；支持重跑、打开输出、复制参数；清空历史需二次确认且不删除输出文件；重启后历史仍在，最多保留 50 条。

### A7 — 设置持久化

语言、主题、输出目录写入 `~/.dull_doc/settings.json`；重启后恢复；输出目录不可写时转换前给出 `E_PERMISSION_DENIED` 与系统设置指引。

### A8 — 构建与离线

`pnpm build`、`cargo check` 与前端测试通过；应用启动和转换过程不发起网络请求；`~/.dull_doc` 下只写入 settings/history/logs/tmp。

# Constraints and invariants

- 应用数据统一放在 `~/.dull_doc/`；用户输出目录独立。
- 所有外部命令使用参数数组，不拼接 shell；路径必须规范化并限制在输入/输出根内。
- 界面颜色、间距、字号只能来自 Proofing Desk token；禁止硬编码颜色。
- 代码注释使用中文，只解释为什么和非显而易见之处。
- 不引入数据库，不使用远程服务，不提交密钥或第三方样本。
- 本切片只依赖 anydoc crate；不引入 pandoc/Typst/xberg sidecar。

# Decisions

- 纵向切片范围：应用壳 + 设置/主题/i18n/命令面板 + 单文件 anydoc → Markdown + 历史。
- 语料制作页在本切片只保留导航与明确的后续接入提示，不伪造真实队列。
- PDF 无文本页不做跳过/OCR 降级，只返回可理解的 `E_NEEDS_OCR`。
- 批量、xberg、pandoc/Typst、签名公证进入 `CHANGE.md` 的后续 change。

# Open questions

无。
