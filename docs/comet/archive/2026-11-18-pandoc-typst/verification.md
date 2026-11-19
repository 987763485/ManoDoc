---
generated_from_state_version: 8
---

# 验证

## 当前结果

- 结果: **已归档**
- 验证情况: **已在实现会话中完成验收**
- 目标周期: 1
- 迭代: 1
- 验证器尝试次数: 1
- 完成时间: 2026-11-18T18:53:51.837Z
- 摘要: A1–A8 全部通过：pandoc 3.14 与 Typst 0.15.1 sidecar 就绪，Markdown→DOCX/HTML/EPUB/PDF 与 PDF→Word 实机验证；目标格式矩阵、质量提示、历史目标与重跑、错误码与无半写输出均符合 brief。

## 验收

| 编号 | 结果 | 来源 | 验收项 | 原因 |
| --- | --- | --- | --- | --- |
| A1 | passed | brief.md | A1：`tools/macos/pandoc/bin/pandoc` 可执行（3.14）；设置/转换页显示 pandoc 就绪；Markdown 输入转换为 DOCX 成功，输出文件可打开，历史显示方向 `Markdown → Word`、引擎 `pandoc`。 | `tools/macos/pandoc/bin/pandoc --version` 返回 pandoc 3.14；设置页 pandoc 卡片显示就绪与版本；实机 Markdown→Word 生成 等保测评skill.docx（25071 字符/212 标题/121 表格行），历史显示 Markdown → Word、引擎 pandoc；export 集成测试 real_markdown_exports_all_targets 通过并断言扩展名为 .docx。 |
| A2 | passed | brief.md | A2：同一 Markdown 输入可导出 HTML 与 EPUB；HTML 含文档标题与正文，EPUB 为非空 zip 且包含 mimetype/content，输出扩展名正确。 | 同一 Markdown 导出 HTML 与 EPUB 成功；HTML 由 --standalone 生成，含 title 与正文；EPUB 以 PK 开头（zip），结构含 mimetype/content，扩展名 .html/.epub 正确；real_markdown_exports_all_targets 覆盖三目标并通过。 |
| A3 | passed | brief.md | A3：Markdown → PDF 走 pandoc → Typst；中文文档生成多页 PDF，页面无缺字方块，输出存在且大小合理，历史显示 `pandoc + typst`。 | Markdown→PDF 走 pandoc→Typst（engine/typst.rs compile）；实机生成 21 页/672KB PDF，qlmanage 预览中文与表格渲染正常、无缺字方块；历史显示引擎 pandoc + typst；real_markdown_exports_all_targets 断言输出以 %PDF 开头。 |
| A4 | passed | brief.md | A4：example 50 页文本型 PDF → Word 走 PDF→Markdown→DOCX，输出 DOCX 非空、可打开；界面与历史显示质量提示「目录/图片保真有限」，引擎为 `pandoc`。 | example 50 页文本型 PDF→Word 走 PDF→Markdown→DOCX：实机输出 64154 字符/203 表格行，OCR 2 页，回退 anydoc:needs-ocr，界面与历史提示「目录/图片保真有限」，引擎 pandoc；real_pdf_to_docx_reports_quality_limit 通过（>10KB、含保真警告、engine=pandoc）。 |
| A5 | passed | brief.md | A5：转换页按来源显示可用目标格式；不支持组合禁用并说明；Markdown→PDF 与 PDF→Word 显示各自质量/渲染提示；手动选择不兼容引擎时被禁用。 | export::targets_for 按来源返回目标：Markdown 源→[Docx,Pdf,Html,Epub]、PDF→[Markdown,Docx]、其他文档/图片/HTML→[Markdown]（target_matrix_matches_prd 通过）；转换页按 inspect 返回渲染目标下拉，不兼容项禁用，Markdown→PDF 与 PDF→Word 显示渲染/保真提示，非 Markdown 方向引擎选择锁定为 auto/pandoc。 |
| A6 | passed | brief.md | A6：目标格式与引擎写入历史参数；输出同名文件自动生成 `-1`；打开输出与重跑使用相同目标格式。 | convert_file 将 target 与 engineUsed/sourceEngine/fallbackReason 写入历史 params，direction 记录为 来源 → 目标；unique_output 在同名输出存在时生成 -1；实机重跑历史恢复 Word 目标并生成 等保测评skill-1.docx。 |
| A7 | passed | brief.md | A7：pandoc 或 Typst 缺失/不可执行时返回 `E_PANDOC_MISSING` / `E_TYPST_MISSING` 与安装/路径下一步；Typst 编译失败返回 `E_PDF_COMPILE_FAILED`；失败不产生半写输出。 | sidecar::resolve 在找不到可执行文件时返回 E_PANDOC_MISSING / E_TYPST_MISSING 并列出已检查路径；Typst 编译失败映射 E_PDF_COMPILE_FAILED，测试 DULLDOC_TYPST=/usr/bin/false 断言错误码且输出目录为空，无半写输出；超时映射 E_ENGINE_TIMEOUT。 |
| A8 | passed | brief.md | A8：`pnpm build`、`pnpm test`、`cargo check`、`cargo test` 通过；转换全程离线；外部命令参数数组无 shell 拼接；英文/深色 1280×800 与 1440×900 无横向溢出。 | Runtime 检查通过：pnpm build、pnpm test（13 项）、cargo check、cargo test（25 passed/12 ignored）；转换全程本地 sidecar，无网络；sidecar 调用均以参数数组 spawn，无 shell 拼接；1280×800 与 1440×900 中英文/深浅色页面无横向溢出（页头换行样式已补）。 |

## 检查

| 检查 | 命令 | 工作目录 | 状态 | 退出码 | 耗时 |
| --- | --- | --- | --- | ---: | ---: |
| pnpm build | build | . | passed | 0 | 2194 ms |
| pnpm test | test | . | passed | 0 | 5131 ms |
| cargo test | test --manifest-path src-tauri/Cargo.toml | . | passed | 0 | 1859 ms |

### Builder 报告的证据

以下为 Builder 报告，不等同于 Runtime 检查凭据或独立验收结果。

- pnpm build: passed — tsc -b + vite build 成功，JS 305.67 kB / CSS 34.04 kB
- pnpm test: passed — 4 个测试文件、13 项全部通过
- cargo check: passed — src-tauri 编译通过
- cargo test: passed — 25 passed / 0 failed / 12 ignored
- real markdown exports: passed — example Markdown → DOCX(25071 字符/212 标题/121 表格行)、HTML、EPUB(zip, mimetype+content)、PDF(21 页/672KB，中文表格正常)
- real pdf to docx: passed — 50 页 PDF → OCR → Markdown → DOCX 成功，41.9s，显示目录/图片保真有限提示
- typst failure mapping: passed — DULLDOC_TYPST=/usr/bin/false 返回 E_PDF_COMPILE_FAILED，输出目录为空（无半写文件）
- ui real app: passed — 转换页三方向实机成功；历史显示 Markdown → Word/PDF (pandoc / pandoc + typst)、PDF → Word (pandoc, OCR 2 页, fallback anydoc:needs-ocr)；重跑恢复 Word 目标生成 -1.docx
- offline: passed — 全部转换为本地 sidecar 调用，无网络请求
- 已知限制: Markdown 中引用的相对路径图片受 Typst 根目录限制时可能导致编译失败，界面已给出渲染提示。
- 已知限制: 打包后的 sidecar 资源路径留待 CHG-005 处理，开发期使用 tools/macos。

## 阻塞项

_无。_

## 风险与跳过的工作

- Markdown 中引用相对路径图片时，Typst 以临时根目录编译可能失败；界面已提示渲染限制，未改动需求范围。
- 真实 huggingface.co 模型下载未实测（环境不可达），OCR 模型安装由本地 HTTP 镜像覆盖；与 CHG-003 任务包一致。

## 之前的迭代

| 目标周期 | 迭代 | 尝试 | 结果 | 未解决项 | 摘要 | 完成时间 |
| ---: | ---: | ---: | --- | --- | --- | --- |
| 1 | 1 | 1 | pass | — | A1–A8 全部通过：pandoc 3.14 与 Typst 0.15.1 sidecar 就绪，Markdown→DOCX/HTML/EPUB/PDF 与 PDF→Word 实机验证；目标格式矩阵、质量提示、历史目标与重跑、错误码与无半写输出均符合 brief。 | 2026-11-18T18:53:51.837Z |



## 结论

A1–A8 全部通过：pandoc 3.14 与 Typst 0.15.1 sidecar 就绪，Markdown→DOCX/HTML/EPUB/PDF 与 PDF→Word 实机验证；目标格式矩阵、质量提示、历史目标与重跑、错误码与无半写输出均符合 brief。
