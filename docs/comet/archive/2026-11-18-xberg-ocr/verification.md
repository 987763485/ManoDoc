---
generated_from_state_version: 14
---

# 验证

## 当前结果

- 结果: **已归档**
- 验证情况: **已在实现会话中完成验收**
- 目标周期: 1
- 迭代: 2
- 验证器尝试次数: 1
- 完成时间: 2026-11-18T18:27:12.976Z
- 摘要: CHG-003 xberg OCR 实现覆盖 A1–A8：xberg 1.4.0 + PaddleOCR ONNX 接入、auto/anydoc/xberg 路由与可见回退、模型显式安装与离线约束、图片与批量语料元数据、历史展示与双语深色界面。构建与测试通过，实机扫描 PDF、图片与 4 文件批量均得到预期结果。

## 验收

| 编号 | 结果 | 来源 | 验收项 | 原因 |
| --- | --- | --- | --- | --- |
| A1 | passed | brief.md | A1：模型目录为空时，系统设置 → 引擎显示 xberg 1.4.0 与 OCR 模型「未安装」、约 31MB 与下载来源；安装前不发起网络请求，点击安装后显示进度并在完成后变为「已安装」。 | 设置→引擎展示 xberg 1.4.0 就绪；OCR 模型卡显示 29.7 MB / 29.7 MB、来源 huggingface.co/xberg-io 与本地目录。model_status 单元测试确认空目录为未安装；本地 HTTP 镜像测试验证了显式安装的进度事件、SHA-256 校验、原子写入与重复安装复用。安装前仅做本地文件检查，未发起网络请求。 |
| A2 | passed | brief.md | A2：OCR 模型已安装时，转换封面页无文本层的 `example/` PDF 自动路由到 xberg OCR，输出非空且包含可读中文的 Markdown，界面与历史显示实际引擎 `xberg`、OCR 页与回退原因。 | 实机转换 example 封面无文本 50 页 PDF：auto 路由回退 xberg，输出 64154 字符、9 标题、203 表格行；结果卡与历史均显示引擎 xberg、OCR 页 1,2、paddleocr、回退原因 anydoc:needs-ocr（42.6s）。 |
| A3 | passed | brief.md | A3：docx、xlsx、文本型 PDF 与 CSV 在默认 auto 引擎下由 anydoc 完成转换，不加载 OCR 模型、不产生 OCR 网络请求；显式选择 anydoc 时行为一致。 | 实机 docx 转换实际引擎 anydoc（315999 字符、49 标题、15 表格行），未出现 OCR 信息；routing 测试 text_document_stays_on_anydoc 通过，确认不加载 OCR 模型。 |
| A4 | passed | brief.md | A4：模型目录为空或模型文件缺失时，转换扫描 PDF 或图片返回 `E_NEEDS_OCR`，包含页数/文件信息与「安装 OCR 模型」下一步，不产生不完整输出，也不自动下载模型。 | 临时移除模型目录后实机转换 JPG 返回 E_NEEDS_OCR，错误卡显示该图片需要 OCR、安装下一步与「安装 OCR 模型」按钮；routing 测试 missing_models_return_needs_ocr 确认扫描 PDF 同样返回 E_NEEDS_OCR 且不产生输出。无模型时转换路径不会联网下载。 |
| A5 | passed | brief.md | A5：OCR 模型已安装时，选择包含中文正文的 PNG/JPEG 可得到包含可读中文的 Markdown，界面与历史显示 `xberg` 与 OCR 来源。 | 实机转换 example JPG 成功：369 字符、17 表格行，界面显示实际引擎 xberg · OCR 1 页；routing 测试 real_image_routes_to_ocr 确认 ocr_engine=paddleocr，批量 jpg 的 chunk 元数据同样记录 xberg/paddleocr。 |
| A6 | passed | brief.md | A6：批量队列包含 docx 与扫描 PDF 且 OCR 模型已安装时，扫描 PDF 以 xberg 完成、docx 以 anydoc 完成；`chunks.jsonl` 的 OCR 块元数据含 `engine=xberg`，manifest 记录实际引擎、OCR 页与回退原因，语料包校验通过。 | 实机批量 example 目录 4 文件全部成功（docx/xlsx 走 anydoc，jpg/pdf 走 xberg），生成 356 chunks；manifest 逐文件含 engineUsed、ocrPages、ocrEngine、fallbackReason；抽样 chunk 元数据为 engine=xberg、ocrEngine=paddleocr、ocrPages=[1]；语料包校验通过。 |
| A7 | passed | brief.md | A7：文档转换与语料制作页的引擎选择可选 auto/anydoc/xberg；模型未安装时 xberg 禁用并说明原因；发生回退或 OCR 时界面显示实际引擎与原因，英文与深色主题文案完整，1280×800 与 1440×900 无横向溢出。 | 转换与语料制作页引擎选择包含 auto/anydoc/xberg；模型缺失时显示「OCR 模型未安装，xberg 暂不可用」并禁用 xberg，转换失败详情会即时刷新该状态。英文与深色主题下引擎页、OCR 卡、结果卡文案完整；1280×800 与 1440×900 下页面无横向溢出。 |
| A8 | passed | brief.md | A8：`pnpm build`、`pnpm test`、`cargo check` 与 `cargo test` 全部通过；断网后已安装模型仍可 OCR 且不发起网络请求；模型缺失时安装返回清晰的离线错误，不静默重试。 | pnpm build、pnpm test（12 项）、cargo check、cargo test（23 passed / 9 ignored）全部通过。本机无法访问 huggingface.co，但已安装模型仍完成 OCR，证明转换不依赖网络；模型安装失败返回 E_OCR_MODEL_DOWNLOAD（离线或网络不可达）与下一步，不静默重试。本地镜像测试覆盖下载成功路径。 |

## 检查

| 检查 | 命令 | 工作目录 | 状态 | 退出码 | 耗时 |
| --- | --- | --- | --- | ---: | ---: |
| pnpm build | build | . | passed | 0 | 2284 ms |
| pnpm test | test | . | passed | 0 | 5329 ms |
| cargo test | test --manifest-path src-tauri/Cargo.toml | . | passed | 0 | 2081 ms |

### Builder 报告的证据

以下为 Builder 报告，不等同于 Runtime 检查凭据或独立验收结果。

- pnpm build: passed — tsc -b && vite build 成功，302.64 kB JS
- pnpm test: passed — 4 个测试文件、12 项全部通过
- cargo check: passed — src-tauri 编译通过，仅链接器 eh_frame 警告
- cargo test: passed — 23 passed / 0 failed / 9 ignored
- real scanned PDF OCR: passed — example 封面无文本 50 页 PDF：auto → xberg，64154 字符、9 标题、203 表格行，OCR 页 [1,2]，回退 anydoc:needs-ocr
- real image OCR: passed — PNG 扫描页：xberg + paddleocr，6.08s
- text document stays anydoc: passed — docx auto 路由实际引擎 anydoc，未加载 OCR
- missing models fallback: passed — 模型目录为空时扫描 PDF 返回 E_NEEDS_OCR 与安装下一步
- batch OCR corpus: passed — example 批次 4 文件全部成功、356 chunks；manifest 记录 engineUsed/ocrPages/ocrEngine/fallbackReason，校验通过
- model install via local mirror: passed — 本地 HTTP 镜像验证下载进度、SHA-256、原子写入与重复安装复用
- 已知限制: 本机无法访问 huggingface.co，真实公网模型下载未实测；已用本地 HTTP 镜像覆盖下载、校验与进度路径，离线安装错误路径也已验证。
- 已知限制: PaddleOCR Python 运行时、Marker 与引擎质量对比（REQ-100/101）仍为后续 P1。

## 阻塞项

_无。_

## 风险与跳过的工作

- 真实 huggingface.co 模型下载因本机网络不可达未实测；下载/进度/SHA-256/原子写入由本地 HTTP 镜像测试覆盖。
- PaddleOCR Python 运行时与 Marker 仍为 P1，未包含在本 change；xberg 使用内置 PaddleOCR ONNX。
- 批量 OCR 在 4 并发下 PDF 与图片同时处理时耗时约 2 分钟，模型加载与 OCR 串行可能成为大批量瓶颈，后续可优化引擎实例复用。

## 之前的迭代

| 目标周期 | 迭代 | 尝试 | 结果 | 未解决项 | 摘要 | 完成时间 |
| ---: | ---: | ---: | --- | --- | --- | --- |
| 1 | 1 | 1 | pass | — | CHG-003 xberg OCR 实现覆盖 A1–A8：xberg 1.4.0 + PaddleOCR ONNX 接入、auto/anydoc/xberg 路由与可见回退、模型显式安装与离线约束、图片与批量语料元数据、历史展示与双语深色界面。构建与测试通过，实机扫描 PDF、图片与 4 文件批量均得到预期结果。 | 2026-11-18T18:21:15.868Z |
| 1 | 1 | 1 | recovery | — | The user chose to revise the implementation after Verify stopped. | 2026-11-18T18:23:01.359Z |
| 1 | 2 | 1 | pass | — | CHG-003 xberg OCR 实现覆盖 A1–A8：xberg 1.4.0 + PaddleOCR ONNX 接入、auto/anydoc/xberg 路由与可见回退、模型显式安装与离线约束、图片与批量语料元数据、历史展示与双语深色界面。构建与测试通过，实机扫描 PDF、图片与 4 文件批量均得到预期结果。 | 2026-11-18T18:27:12.976Z |



## 结论

CHG-003 xberg OCR 实现覆盖 A1–A8：xberg 1.4.0 + PaddleOCR ONNX 接入、auto/anydoc/xberg 路由与可见回退、模型显式安装与离线约束、图片与批量语料元数据、历史展示与双语深色界面。构建与测试通过，实机扫描 PDF、图片与 4 文件批量均得到预期结果。
