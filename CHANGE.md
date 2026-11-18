# CHANGE.md — DullDoc Change 总索引

> 本文件是 change 状态的唯一总索引。每个 change 在 Open / Build / Verify / Archive 阶段都必须同步本表；未同步的 change 不得归档或提交。
> 需求编号引用 `docs/superpowers/需求文档.md`；Comet 正式产物位于 `docs/comet/changes/<change>/`。

## 当前 change

| ID | Change | 范围 | 状态 | 阶段 | 正式产物 |
| --- | --- | --- | --- | --- | --- |
| CHG-001 | `mvp-implementation` | Tauri 2 应用壳；四页导航；设置/主题/中英文；Cmd+K；anydoc 单文件 Office/文本型 PDF → Markdown；真实历史 | Archive | 已归档（A1–A8 通过） | `docs/comet/archive/2026-11-18-mvp-implementation/` |

## 后续 change（待启动）

| ID | Change | 范围 | 状态 | 依赖 |
| --- | --- | --- | --- | --- |
| CHG-002 | `batch-corpus` | 批量队列、清洗、切块、`chunks.jsonl` 与 `manifest.json`、语料包配置 | Open | CHG-001 |
| CHG-003 | `xberg-ocr` | xberg 第二内核、扫描件 OCR、复杂表格、anydoc 回退；PaddleOCR P1 运行时 | Open | CHG-001 |
| CHG-004 | `pandoc-typst` | pandoc sidecar、Markdown ↔ DOCX/HTML/EPUB、Markdown → PDF（Typst）、PDF → Word 质量提示 | Open | CHG-001 |
| CHG-005 | `packaging-signing` | `.app`/`.dmg` 打包、sidecar 预签名、Developer ID 公证与 staple | Open | CHG-003、CHG-004 |

## 状态定义

| 状态 | 含义 |
| --- | --- |
| Open | 已登记、未开始实现 |
| Build | 正在实现或修复 |
| Verify | 候选实现已提交，等待验收 |
| Archive | 验收通过，等待归档或已归档 |

## 同步记录

| 日期 | Change | 动作 | 说明 |
| --- | --- | --- | --- |
| 2026-11-19 | CHG-001 | Open → Build | Comet Native Shape 已确认，进入纵向切片实现 |
| 2026-11-19 | CHG-001 | Build → Verify | 候选实现提交；Runtime 检查与只读验收覆盖 A1–A8 |
| 2026-11-19 | CHG-001 | Verify → Archive | 用户接受结果，A1–A8 全部通过；归档至 `docs/comet/archive/2026-11-18-mvp-implementation/` |
