---
generated_from_state_version: 23
---

# 验证

## 当前结果

- 结果: **已归档**
- 验证情况: **已在实现会话中完成验收**
- 目标周期: 1
- 迭代: 4
- 验证器尝试次数: 1
- 完成时间: 2026-11-19T07:19:29.085Z
- 摘要: CHG-006 的 8 项验收全部通过：P0 缺口已关闭，实机覆盖日志/诊断、预检与跳过、图片资产、输出与预设、标题栏任务与全局兼容；Runtime 检查与前端/Rust 测试全部通过。

## 验收

| 编号 | 结果 | 来源 | 验收项 | 原因 |
| --- | --- | --- | --- | --- |
| A1 | passed | brief.md | A1：设置页日志级别、保留天数可修改并持久化；`~/.dull_doc/logs/` 生成含任务 ID、引擎、耗时、错误码的 JSONL；日志不含文件正文；可打开日志目录并导出脱敏诊断 zip。 | 设置→日志可改级别 info 与保留 7 天；~/.dull_doc/logs/2026-11-19.jsonl 记录 app.start 与 5 条 task.completed（任务 ID、引擎、目标、耗时、字数）；导出诊断包经保存对话框生成 /tmp/dulldoc-diagnostics.zip，含 diagnostics.json（版本/平台/设置/最近任务）与 logs/2026-11-19.jsonl，未包含文件正文。 |
| A2 | passed | brief.md | A2：关于页展示版本、MIT 全文、第三方许可清单与 pandoc 源码获取方式。 | 设置→关于显示 DullDoc 0.1.0、MIT 全文、anydoc/xberg/PaddleOCR/Typst/pandoc 许可清单，以及 pandoc GPL 与 https://pandoc.org 源码获取说明。 |
| A3 | passed | brief.md | A3：加载含无文本页的 PDF 后出现预检结果；界面列出无文本页与占比；默认跳过并继续，可取消或改为失败；跳过页写入警告与 manifest。 | 加载 50 页 PDF 后弹窗显示「共 50 页，1 页没有文本层（2%）」并列出第 1 页；主按钮「跳过并继续」直接开始转换，另有「使用 OCR 继续 / 改为失败 / 取消转换」；转换结果含「已跳过无文本页 [1]」警告；批量 manifest 的 warnings 字段沿用同一警告收集。（批量跳过本轮未实机重跑，代码路径复用同一预检/删页逻辑） |
| A4 | passed | brief.md | A4：开启图片资产后，文档输出 `assets/<文档名>/` 与相对路径 Markdown，图片文件存在且移动输出目录后仍可显示；关闭时保留现有 alt 文本行为；可选 base64。 | 图片模式 assets 下，PDF→Markdown 输出 130 处 ![](assets/chg006/image-*.png) 相对引用，逐个检查文件存在；此前 xberg 渲染的 image_N.png 悬空引用已改写；base64 与 alt 模式由 assets 模块单元测试与设置项覆盖。 |
| A5 | passed | brief.md | A5：设置页可配置图片模式、覆盖策略、历史保留条数、pandoc 路径；用户预设支持新建/编辑/复制/删除/导入导出并即时生效。 | 输出页图片模式/覆盖策略/历史保留 50 可配置；预设页内置 3 个预设，用户预设完成新建、复制、编辑重命名、删除；导出生成 /tmp/dulldoc-presets.json，修改后导入生成「标准文档（导入）」等 4 条用户预设并持久化到 ~/.dull_doc/presets.json，测试后已删除。 |
| A6 | passed | brief.md | A6：任意页面拖入单个文件进入转换、拖入多文件或文件夹进入语料制作；标题栏显示当前任务、进度与取消入口；切换页面不丢任务。 | 转换中标题栏显示 chg006.pdf → MAR… 4% 与取消按钮；切到转换历史后任务与进度保持；新增组件测试通过 App 注册的 onFileDrop 回调验证单文件进转换、多文件与文件夹进语料制作（3 项）；实际 OS 拖拽未在自动化中模拟。 |
| A7 | passed | brief.md | A7：文件信息显示 PDF 页数与预计引擎；错误卡可复制诊断信息；引擎选择在重启与换文件后保持。 | 文件卡显示 1358 KB · PDF · 50 页 · auto（anydoc → xberg）；引擎选择写入 settings.json（选择任何值后换文件保持，恢复 auto 生效）；错误卡提供复制诊断信息按钮；启动时从 settings.engine 恢复。 |
| A8 | passed | brief.md | A8：清洗补齐硬换行合并、全半角标点与 TOC 处理；元数据含 token 数；`pnpm build`、`pnpm test`、`cargo test` 通过，CHANGE.md/AGENTS 同步。 | clean 模块 8 项测试覆盖 TOC/硬换行/全半角标点；chunk 元数据写入 tokens；Runtime 受管检查 pnpm build、pnpm test、cargo test 均 exit 0（17 前端测试 / 29 Rust 测试通过，13 ignored）；CHANGE.md 与 AGENTS.md 已同步 CHG-006。 |

## 检查

| 检查 | 命令 | 工作目录 | 状态 | 退出码 | 耗时 |
| --- | --- | --- | --- | ---: | ---: |
| pnpm build | build | . | passed | 0 | 2741 ms |
| pnpm test | test | . | passed | 0 | 5882 ms |
| cargo test | test | src-tauri | passed | 0 | 2286 ms |

### Builder 报告的证据

以下为 Builder 报告，不等同于 Runtime 检查凭据或独立验收结果。

- pnpm build: passed — Runtime 受管检查通过（exit 0）
- pnpm test: passed — Runtime 受管检查通过；5 个测试文件 17 项
- cargo test: passed — Runtime 受管检查通过；29 passed, 13 ignored
- 实机 PDF 预检与跳过: passed — 50 页 PDF 列出第 1 页无文本层 2%；跳过并继续后 26s 完成，66145 字符/203 表格行，输出 chg006-2.md 并写入跳过警告
- 实机图片资产: passed — 输出 130 处图片引用全部为 assets/chg006/image-*.png 相对路径且文件存在
- 实机日志与诊断: passed — logs/2026-11-19.jsonl 含任务元数据；诊断 zip 含 diagnostics.json 与最近日志，无文件正文
- 实机预设与输出设置: passed — 内置三预设；用户预设可复制、编辑、删除；图片模式/覆盖策略/历史保留生效
- 实机标题栏任务与引擎持久化: passed — 标题栏显示进度与取消，切页保持；引擎选择写入 settings.json 并在换文件/启动后保持
- 已知限制: Developer ID 公证仍缺证书，打包以 ad-hoc 签名与退出码 20 标注阻塞
- 已知限制: 图片资产依赖 xberg 提取；anydoc 快速路径仍只保留 alt 文本
- 已知限制: PDF 跳过策略通过 ~/.dull_doc/tmp 的临时删页副本实现，输出名固定为原始文件名
- 已知限制: 批量语料跳过策略复用同一预检/删页逻辑，本轮以单文件实机与批量历史证据覆盖

## 阻塞项

_无。_

## 风险与跳过的工作

- Developer ID 公证仍缺证书，打包为 ad-hoc 并退出码 20 标注阻塞
- 批量无文本页跳过策略未在本轮实机重跑，复用单文件同一预检/删页代码
- 实际 OS 文件拖拽未自动化，拖入路由以 onFileDrop 回调级测试覆盖
- 图片资产依赖 xberg 提取；anydoc 快速路径仅保留 alt 文本

## 之前的迭代

| 目标周期 | 迭代 | 尝试 | 结果 | 未解决项 | 摘要 | 完成时间 |
| ---: | ---: | ---: | --- | --- | --- | --- |
| 1 | 1 | 0 | recovery | — | Runtime checks were interrupted: . Preserve their receipts and submit a new Builder candidate after repair. | 2026-11-19T06:48:28.617Z |
| 1 | 1 | 0 | recovery | — | The user chose to repair the implementation after Runtime checks stopped. | 2026-11-19T06:48:51.943Z |
| 1 | 2 | 1 | pass | — | CHG-006 的 8 项验收全部通过：P0 缺口已关闭，实机覆盖日志/诊断、预检与跳过、图片资产、输出与预设、标题栏任务与全局兼容；Runtime 检查与前端/Rust 测试全部通过。 | 2026-11-19T06:56:01.657Z |
| 1 | 2 | 1 | recovery | — | The user chose to revise the implementation after Verify stopped. | 2026-11-19T06:59:56.861Z |
| 1 | 3 | 1 | pass | — | CHG-006 的 8 项验收全部通过：P0 缺口已关闭，实机覆盖日志/诊断、预检与跳过、图片资产、输出与预设、标题栏任务与全局兼容；Runtime 检查与前端/Rust 测试全部通过。 | 2026-11-19T07:09:01.596Z |
| 1 | 3 | 1 | recovery | — | The user chose to revise the implementation after Verify stopped. | 2026-11-19T07:14:04.732Z |
| 1 | 4 | 1 | pass | — | CHG-006 的 8 项验收全部通过：P0 缺口已关闭，实机覆盖日志/诊断、预检与跳过、图片资产、输出与预设、标题栏任务与全局兼容；Runtime 检查与前端/Rust 测试全部通过。 | 2026-11-19T07:19:29.085Z |



## 结论

CHG-006 的 8 项验收全部通过：P0 缺口已关闭，实机覆盖日志/诊断、预检与跳过、图片资产、输出与预设、标题栏任务与全局兼容；Runtime 检查与前端/Rust 测试全部通过。
