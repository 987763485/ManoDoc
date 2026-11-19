# 目标

交付 CHG-006「P0 收口」：把 PRD 中尚未完成或仅部分完成的 P0 需求补齐，使 71 条 P0 在功能、界面与验收上达到可交付状态。本次以 2026-11-19 的 P0 自检结果为起点，覆盖缺口清单中的日志、诊断、PDF 预检、无文本页策略、图片资产、输出/预设/日志设置、全局拖入与任务状态等，不新增 PRD 之外的格式或云能力。

自检基线：41 条已完成、18 条部分完成、12 条未完成；本 change 的目标是把后两类全部关闭，或在用户明确同意的前提下调整范围与原因。

# 范围

## 目录结构

### 新建

- `src-tauri/src/logging/mod.rs` — 本地 JSONL 日志、级别过滤、保留天数与诊断包导出。
- `src-tauri/src/preflight/mod.rs` — PDF 页级预检、无文本页列表与阈值判断，供单文件与批量复用。
- `src-tauri/src/presets/mod.rs` — 用户预设的读写、复制与导入导出。
- `src-tauri/src/assets/mod.rs` — 图片资产落盘、命名、去重与 Markdown 链接改写。
- `src/components/PreflightDialog.tsx` — 无文本页列表与跳过/失败策略选择。
- `src/pages/settings/` 下按需拆出的设置子组件（输出、日志、预设管理）— 复用现有设置页结构，避免单文件继续膨胀。
- `docs/superpowers/P0自检.md`（或同等名称）— 记录自检结论与关闭状态，作为发布前审计材料。

### 修改

- `src-tauri/src/commands/mod.rs` — 接入预检、导出前检查、诊断导出、预设与设置命令。
- `src-tauri/src/engine/export.rs`、`engine/xberg.rs`、`engine/anydoc.rs` — 图片资产提取与相对路径写入；导出前能力检查。
- `src-tauri/src/engine/routing.rs` — 预检结果、无文本页阈值与 skip/fail 策略、图片请求时的引擎选择。
- `src-tauri/src/batch/mod.rs`、`corpus/mod.rs`、`chunk/mod.rs` — 批量策略、图片资产、警告与 manifest 记录；元数据补 token 数。
- `src-tauri/src/clean/mod.rs` — 硬换行中文句子合并、全半角标点规范、TOC 处理。
- `src-tauri/src/storage/mod.rs`、`models.rs` — 新增设置字段、保留条数、预设文件与历史裁剪。
- `src/App.tsx`、`src/components/CommandPalette.tsx`、`src/pages/ConvertPage.tsx`、`BatchPage.tsx`、`HistoryPage.tsx`、`SettingsPage.tsx` — 标题栏任务状态、全局拖入路由、预检弹窗、诊断复制、设置项与预设管理。
- `src/i18n/index.tsx`、`src/styles/app.css` — 新增文案与状态样式。
- `docs/superpowers/需求文档.md`、`软件功能设计说明.md`、`技术选型.md`、`AGENTS.md`、`CHANGE.md` — 同步 P0 关闭状态与实现说明。

### 明确不建

- Marker、Python 运行时与云 OCR — 属 P1/远期，不在 P0 收口内。
- App Sandbox / Mac App Store 分发 — 与当前 Developer ID 发布路径无关。
- 新的文件格式与新的转换方向 — 只在 PRD 已列方向内补齐。
- 撤销已确认的取消项（文档预览、语料完成行预览）— 保持取消状态。

# 非目标

- 不改变已确认的转换方向矩阵与引擎分工。
- 不引入遥测、账号或云端处理。
- 不重写设置页信息架构，只补齐缺失项与状态。
- 不承诺 Intel 或 Windows 平台。

# 约束与不变量

- 所有外部命令继续使用参数数组与超时，不拼接 shell。
- 日志与诊断不得包含文件正文、密钥或用户内容；诊断包只含配置摘要、版本、错误码与任务元数据。
- 图片资产默认写入输出目录下 `assets/<文档名>/`，Markdown 使用相对路径；移动输出目录后仍可显示。
- 破坏性操作（清空历史、覆盖输出）保持二次确认；清空历史不删除用户输出。
- 界面颜色只来自 token；新增控件沿用 shadcn/ui Select / Dialog 与现有 Proofing Desk 样式。
- 离线仍为默认：只有用户显式安装 OCR 模型时才联网。

# Decisions

- P0 收口以一个 Native Change 推进，不拆 Supervisor：各批次共享设置、转换页与路由代码，拆分协调成本高于并行收益。
- 日志采用应用数据目录下的 JSONL 文件，按天滚动并按设置保留；诊断包为 zip，导入前脱敏。
- PDF 预检在加载 PDF 后异步执行；无文本页策略单文件默认「跳过并继续」，批量默认 `skip`，可改为失败。
- 图片资产通过 xberg 的图像提取能力实现；不提取图片的快速路径仍走 anydoc，界面注明差异。
- 预设与设置继续存于 `~/.dull_doc` 的 JSON 文件，不引入数据库。
- 引擎选择在用户显式选择后写入设置，下次启动与换文件时保持；自动编码为 `auto`。
- 用户确认：P0 范围取**全部 71 条**（12 缺失 + 18 部分），不在本次缩小范围。
- 用户确认：图片资产取 **xberg 提取**，保留 anydoc 快速路径的 alt 文本行为。
- 用户确认：日志与诊断取 **完整实现**，包含级别、保留、打开目录与脱敏诊断 zip。
- 用户确认：全局体验取 **三项全部**，包含标题栏任务状态、任意页拖入路由与引擎选择持久化。

# 验收示例

- A1：设置页日志级别、保留天数可修改并持久化；`~/.dull_doc/logs/` 生成含任务 ID、引擎、耗时、错误码的 JSONL；日志不含文件正文；可打开日志目录并导出脱敏诊断 zip。
- A2：关于页展示版本、MIT 全文、第三方许可清单与 pandoc 源码获取方式。
- A3：加载含无文本页的 PDF 后出现预检结果；界面列出无文本页与占比；默认跳过并继续，可取消或改为失败；跳过页写入警告与 manifest。
- A4：开启图片资产后，文档输出 `assets/<文档名>/` 与相对路径 Markdown，图片文件存在且移动输出目录后仍可显示；关闭时保留现有 alt 文本行为；可选 base64。
- A5：设置页可配置图片模式、覆盖策略、历史保留条数、pandoc 路径；用户预设支持新建/编辑/复制/删除/导入导出并即时生效。
- A6：任意页面拖入单个文件进入转换、拖入多文件或文件夹进入语料制作；标题栏显示当前任务、进度与取消入口；切换页面不丢任务。
- A7：文件信息显示 PDF 页数与预计引擎；错误卡可复制诊断信息；引擎选择在重启与换文件后保持。
- A8：清洗补齐硬换行合并、全半角标点与 TOC 处理；元数据含 token 数；`pnpm build`、`pnpm test`、`cargo test` 通过，CHANGE.md/AGENTS 同步。
