---
generated_from_state_version: 8
---

# 验证

## 当前结果

- 结果: **已归档**
- 验证情况: **已完成检查，验证结果已确认**
- 目标周期: 1
- 迭代: 1
- 验证器尝试次数: 1
- 完成时间: 2026-11-18T17:38:11.613Z
- 摘要: 8/8 通过：批量队列、清洗、切块、语料包、批量历史与 UI 在真实 example 批次上可运行；构建与离线约束满足。唯一未取得的直接证据是实机取消按钮中断，已在风险中注明。

## 验收

| 编号 | 结果 | 来源 | 验收项 | 原因 |
| --- | --- | --- | --- | --- |
| A1 | passed | brief.md | A1：语料制作页可添加多个文件或文件夹；递归扫描跳过隐藏文件、`.DS_Store` 与输出目录自身，队列显示文件、状态、引擎、耗时与警告，统计数量与队列一致。 | 实机选择 example 文件夹扫描出 3 个受支持文件（docx/xlsx/pdf），Markdown 被跳过并提示；统计与队列行数一致。单元测试覆盖隐藏文件、.DS_Store、输出目录与不支持格式过滤。 |
| A2 | passed | brief.md | A2：批量开始后按并发上限调度；单文件失败不阻塞其他文件，失败行可重试，取消后不再新增任务；不产生半写文件，输出包完整或整体不存在。 | 实机批次中封面 PDF 返回 E_NEEDS_OCR，docx/xlsx 继续完成；失败行重试只重跑该文件；取消标志在文件级单元测试返回 E_CANCELLED，批次临时目录在取消路径删除。风险：实机点击取消按钮未成功中断运行中批次（点击落在完成后的筛选按钮），取消 UI 交互未取得直接截图证据。 |
| A3 | passed | brief.md | A3：清洗开关可逐项关闭；关闭时输出等于原始 anydoc Markdown；开启时删除匹配的页眉页脚/页码、压缩空行、规范标题编号与 CJK 间距，代码块、URL 与表格内容不被破坏。 | 单元测试覆盖全部开关关闭时输出与原始 Markdown 完全一致、页眉页码/空行/标题编号/CJK 处理、代码块/URL/表格保护；实机批次输出使用同一 clean() 路径。 |
| A4 | passed | brief.md | A4：切块默认按 H1–H3，超长块按字符上限二次切分并带重叠；表格整块保留；每个 chunk 含稳定 id、来源、章节路径、字符数与引擎参数。 | 单元测试验证同一输入两次切块 id 完全一致、表格与代码块整块保留、超长块按上限切分；实机 docx 产出 291 块、xlsx 2 块。 |
| A5 | passed | brief.md | A5：语料包包含镜像 Markdown、`chunks.jsonl`、`manifest.json` 与 JSON/CSV 报告；`chunks.jsonl` 每行可解析，`manifest.json` 含工具/引擎版本、输入哈希、参数、文件列表、警告与总计；输出后校验失败时给出可定位错误。 | 实机语料包含 mirror/、chunks.jsonl（293 行）、manifest.json、report.json/csv；manifest 与 JSONL 校验通过；单元测试验证人为写入坏行时返回 E_CORPUS_VALIDATION。 |
| A6 | passed | brief.md | A6：一次批次写入一条批量历史记录（类型、输入数、状态、输出目录、成功/跳过/失败与块数）；重启后仍在；历史详情可打开语料包、复制参数与重跑。 | 实机历史出现 batch 行（3 文件、Partial、293 chunks），详情展示计数/输出/参数并可重跑；重启应用后历史仍加载；重跑回填 3 个输入并等待开始。 |
| A7 | passed | brief.md | A7：语料制作页按 `docs/superpowers/UI.html` 还原队列、统计、配置面板与状态；中英文完整，1280×800 与 1440×900 无横向溢出，所有颜色来自 token。 | 实机检查 1440×900 与 1280×800 无横向溢出；输出格式固定 Markdown，图片开关禁用并说明 CHG-003；英文界面下批量配置/队列表头完整；颜色来自 token。 |
| A8 | passed | brief.md | A8：`pnpm build`、前端测试、`cargo check`、Rust 测试通过；批量转换与语料包生成全程离线；输出目录只写入语料包与报告，`~/.dull_doc` 仍只写 settings/history/logs/tmp。 | pnpm build、pnpm test(10)、cargo check、cargo test(18+4 样例)通过；代码无外部 URL；实机批次全程离线；输出目录仅写入批次包，~/.dull_doc 仍只有 settings/history/logs/tmp。 |

## 检查

| 检查 | 命令 | 工作目录 | 状态 | 退出码 | 耗时 |
| --- | --- | --- | --- | ---: | ---: |
| pnpm build | build | . | passed | 0 | 2157 ms |
| pnpm test | test | . | passed | 0 | 4968 ms |
| cargo check | check --manifest-path src-tauri/Cargo.toml | . | passed | 0 | 550 ms |
| cargo test | test --manifest-path src-tauri/Cargo.toml | . | passed | 0 | 818 ms |
| cargo sample tests | test --manifest-path src-tauri/Cargo.toml -- --ignored | . | passed | 0 | 14787 ms |

### Builder 报告的证据

以下为 Builder 报告，不等同于 Runtime 检查凭据或独立验收结果。

- pnpm build: passed — tsc -b + vite build 成功
- pnpm test: passed — 2 个测试文件、10 项通过（含批量队列/历史重跑）
- cargo check: passed — src-tauri 编译通过
- cargo test: passed — 18 项通过，4 个样例测试默认忽略
- cargo test -- --ignored: passed — 4 项通过：docx/xlsx/PDF 样例、example 批量扫描处理、语料包写入校验
- manual-app-smoke: passed — 实机：example 文件夹扫描 3 文件；批次 2 成功/1 E_NEEDS_OCR、293 chunks、2 warnings；包内 mirror/chunks/manifest/report 完整；重试与历史重跑可用；1280×800 无横向溢出
- 已知限制: 图片资产开关禁用并注明由 CHG-003 接入；Markdown 只保留 alt 文本。
- 已知限制: 取消批次会等待已开始的文件完成当前解析，再返回并删除临时包；不保留半写文件。
- 已知限制: 扫描会跳过 Markdown 等 anydoc 不支持格式，并在界面提示。
- 已知限制: anydoc 无页映射，chunk 元数据 page 为 null。
- 已知限制: 关闭元数据开关时只保留 source 字段，以满足可追溯与校验。

## 阻塞项

_无。_

## 风险与跳过的工作

- 实机取消按钮点击未成功触发（点击落在完成后的筛选按钮）；取消逻辑由单元测试与代码路径覆盖，建议人工复核取消交互。
- 图片资产与 OCR 按已确认非目标禁用/后置。
- anydoc 无页映射，chunk 元数据 page 为 null。

## 之前的迭代

| 目标周期 | 迭代 | 尝试 | 结果 | 未解决项 | 摘要 | 完成时间 |
| ---: | ---: | ---: | --- | --- | --- | --- |
| 1 | 1 | 1 | pass | — | 8/8 通过：批量队列、清洗、切块、语料包、批量历史与 UI 在真实 example 批次上可运行；构建与离线约束满足。唯一未取得的直接证据是实机取消按钮中断，已在风险中注明。 | 2026-11-18T17:38:11.613Z |



## 结论

8/8 通过：批量队列、清洗、切块、语料包、批量历史与 UI 在真实 example 批次上可运行；构建与离线约束满足。唯一未取得的直接证据是实机取消按钮中断，已在风险中注明。
