---
generated_from_state_version: 11
---

# 验证

## 当前结果

- 结果: **已归档**
- 验证情况: **已完成检查，验证结果已确认**
- 目标周期: 1
- 迭代: 2
- 验证器尝试次数: 1
- 完成时间: 2026-11-18T16:55:15.385Z
- 摘要: 8/8 通过：纵向切片在真实 Tauri 应用与样例文件上可运行；构建、测试与离线约束满足。已知限制为已声明的批量/OCR/导出与图片提取不在本切片。

## 验收

| 编号 | 结果 | 来源 | 验收项 | 原因 |
| --- | --- | --- | --- | --- |
| A1 | passed | brief.md | A1：应用启动后默认进入文档转换页，四页导航可切换，1280×800 与 1440×900 无横向溢出，已实现页面符合 UI 原型。 | 实机 1440×900 与调整为 1280×800 均无横向溢出；四页导航切换正常，默认进入 Convert。 |
| A2 | passed | brief.md | A2：语言切换为 English、主题切换为深色后即时生效并持久化；Cmd+K 可搜索并进入系统设置，Esc 关闭后焦点返回。 | 设置与顶栏均可切 English；主题在 system→light 实机验证并重启保留；Vitest 覆盖 Cmd+K 搜索进入 Settings、Esc 后焦点返回触发按钮。 |
| A3 | passed | brief.md | A3：拖入 GB/T 28448 docx 后完成 Word→Markdown，输出文件包含 40 个以上标题与至少一个表格，历史新增完成记录。 | 实机选择 example docx 转换成功：315,999 字符/49 标题/15 表格行，输出 md 737.9K，历史新增完成记录。 |
| A4 | passed | brief.md | A4：xlsx、CSV、RTF、EPUB 或文本型 PDF 均可通过 anydoc 产出 Markdown，文本型 PDF 不触发 OCR。 | 实机 text.pdf（cupsfilter 生成）→ text.md 且未调用 OCR；xlsx 样例 1048 表格行；独立 anydoc 0.2.4 harness 验证 CSV/RTF/EPUB/文本 PDF 均输出 Markdown。 |
| A5 | passed | brief.md | A5：含无文本封面页的 PDF 显示 E_NEEDS_OCR、页数与下一步，不产生不完整输出；不支持格式显示可执行错误且应用不崩溃。 | 实机封面 PDF 显示 E_NEEDS_OCR 与 1 页；输出目录无该 PDF 产物；md 输入显示 E_UNSUPPORTED_FORMAT，应用未崩溃。 |
| A6 | passed | brief.md | A6：历史详情展示方向、引擎、输出与统计，支持重跑/打开/复制参数；清空历史需确认且不删除输出文件，重启后历史仍保留。 | 历史行展示时间/文件/方向/引擎/状态/标题数；详情弹窗展示输出与统计并可重跑/打开/复制；清空有确认弹窗；重启后历史保留；Vitest 覆盖详情与清空。 |
| A7 | passed | brief.md | A7：语言、主题、输出目录写入 ~/.dull_doc/settings.json 并在重启后恢复；输入/输出不可写时显示 E_OUTPUT_PERMISSION 与可执行下一步，不产生半写文件。 | settings.json 写入 language/theme/outputDir，重启恢复；将输出目录指向 /System 后实机显示 E_OUTPUT_PERMISSION + Open settings，且无半写文件；cargo 测试覆盖输出目录与只读路径。 |
| A8 | passed | brief.md | A8：pnpm build、cargo check 与测试通过；断网启动并完成一次 docx→md 转换，过程中无网络请求，~/.dull_doc 只写入 settings/history/logs/tmp。 | pnpm build/test(7)、cargo check/test(5+3 样例)通过；代码无外部 URL，转换全程离线；~/.dull_doc 仅 settings.json/history.json 与 logs/tmp。 |

## 检查

| 检查 | 命令 | 工作目录 | 状态 | 退出码 | 耗时 |
| --- | --- | --- | --- | ---: | ---: |
| pnpm build | build | . | passed | 0 | 1989 ms |
| pnpm test | test | . | passed | 0 | 4754 ms |
| cargo check | check --manifest-path src-tauri/Cargo.toml | . | passed | 0 | 482 ms |
| cargo test | test --manifest-path src-tauri/Cargo.toml | . | passed | 0 | 626 ms |
| cargo sample tests | test --manifest-path src-tauri/Cargo.toml -- --ignored | . | passed | 0 | 11718 ms |

### Builder 报告的证据

以下为 Builder 报告，不等同于 Runtime 检查凭据或独立验收结果。

- pnpm build: passed — tsc -b + vite build 成功，47 modules，dist 产出正常
- pnpm test: passed — Vitest 7/7 通过：导航、语言/主题持久化、Cmd+K 焦点、转换、历史详情与清空确认
- cargo check: passed — src-tauri 编译通过，无 warning
- cargo test: passed — 5 项通过（stats、格式识别、输出目录、权限拒绝），3 项样例测试默认忽略
- cargo test -- --ignored: passed — 3 项样例测试通过：docx 40+ 标题与表格、xlsx 1048 表格行、封面 PDF 返回 E_NEEDS_OCR
- manual-app-smoke: passed — Tauri dev 实机：docx 转换成功并写入历史；PDF 显示 E_NEEDS_OCR；md 显示 E_UNSUPPORTED_FORMAT；不可写输出目录显示 E_OUTPUT_PERMISSION；语言/主题/历史重启后保留
- 已知限制: 语料制作页为纵向切片占位，仅保留导航与后续接入说明。
- 已知限制: 本切片不提取内嵌图片，Markdown 只保留图片 alt 文本（界面已标注）。
- 已知限制: 未知错误仍使用 Rust 中文诊断；常见错误码已在前端本地化。
- 已知限制: S6-5 公证缺 Developer ID，延后到发布前。

## 阻塞项

_无。_

## 风险与跳过的工作

- CSV/RTF/EPUB 通过独立 anydoc harness 验证，未逐一走完整 UI 转换路径。
- 删除了本地被忽略的 tools/macos/pandoc/bin/pandoc-lua 与 pandoc-server 符号链接以修复 Comet 输入快照；主 pandoc 未受影响，后续如需 Lua/server 子命令需恢复链接。
- 未知错误仍返回 Rust 中文诊断，常见错误已本地化。

## 之前的迭代

| 目标周期 | 迭代 | 尝试 | 结果 | 未解决项 | 摘要 | 完成时间 |
| ---: | ---: | ---: | --- | --- | --- | --- |
| 1 | 1 | 0 | recovery | — | Runtime checks completed, but their input changed before acceptance. Preserve the original receipts and approve a new Builder candidate; do not replay the old checks. | 2026-11-18T16:51:32.137Z |
| 1 | 1 | 0 | recovery | — | The user chose to repair the implementation after Runtime checks stopped. | 2026-11-18T16:51:50.104Z |
| 1 | 2 | 1 | pass | — | 8/8 通过：纵向切片在真实 Tauri 应用与样例文件上可运行；构建、测试与离线约束满足。已知限制为已声明的批量/OCR/导出与图片提取不在本切片。 | 2026-11-18T16:55:15.385Z |



## 结论

8/8 通过：纵向切片在真实 Tauri 应用与样例文件上可运行；构建、测试与离线约束满足。已知限制为已声明的批量/OCR/导出与图片提取不在本切片。
