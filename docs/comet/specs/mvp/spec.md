# DullDoc MVP 纵向切片规格

> Capability: `mvp`  
> 本规格描述归档后 DullDoc 第一条可运行切片的行为。批量语料、OCR、pandoc/Typst 与发布签名不在本规格内。

## 1. 应用外壳与导航

DullDoc 是 Tauri 2 macOS 应用，前端使用 React 19、TypeScript、Vite 与 Tailwind CSS 4。应用启动后默认进入「文档转换」页。

四个一级页面固定为：

| 页面 | 本切片行为 |
| --- | --- |
| 文档转换 | 完整实现：选择/拖入文件、方向与引擎展示、转换、内联进度/成功/错误 |
| 语料制作 | 视觉占位：说明批量与语料将在后续 change 接入，不伪造队列或转换能力 |
| 转换历史 | 完整实现：真实历史列表、详情、重跑、打开、清空 |
| 系统设置 | 完整实现通用设置；引擎/输出/预设/日志/关于分组按原型展示，未实现的控件标注后续 |

界面必须复用 Proofing Desk token：纸色、墨色、校样红、修正蓝、成功绿、8pt 间距、原型字号与圆角。禁止在组件中写 hex 颜色。1280×800 与 1440×900 下不得出现横向滚动或主操作被截断。

### 场景 A1｜应用启动与四页导航

- Given 应用已启动
- When 用户点击四个一级导航
- Then 默认页为文档转换，四页均可到达，页面标题与当前导航一致
- And 1280×800 与 1440×900 无横向溢出
- And 文档转换、历史、设置页的布局、间距与颜色符合 `docs/superpowers/UI.html`

验收：A1

## 2. 语言、主题与命令面板

- 默认语言中文；English 覆盖导航、按钮、表单、空/加载/成功/错误、历史与设置。
- 语言切换即时生效，无需重启，并写入 `settings.json`。
- 主题支持跟随系统、浅色、深色；根元素 `data-theme` 切换；选择持久化。
- `Cmd+K` 打开命令面板，可搜索页面、最近转换、打开输出目录、切换主题/语言；方向键选择，Enter 执行，Esc 关闭并返回焦点。
- 所有对话框使用原生 `<dialog>`，Esc/关闭按钮/遮罩可退出；焦点返回触发元素。

### 场景 A2｜语言、主题与命令面板

- Given 应用处于中文浅色主题
- When 用户切换到 English 和深色主题
- Then 导航、按钮、错误文案与设置项即时变为英文和深色
- And 重启后仍保持 English 与深色
- When 用户按 Cmd+K 并搜索「设置」
- Then 面板列出系统设置并可 Enter 进入
- And Esc 关闭后焦点回到命令面板触发按钮

验收：A2

## 3. 单文件转换

### 3.1 输入

支持通过文件选择器或拖放选择单个文件。格式由内容识别，扩展名只作回退。本切片支持 anydoc 可解析的：

`doc`、`docx`、`odt`、`rtf`、`epub`、文本型 `pdf`、`ppt`、`pptx`、`xls`、`xlsx`、`csv`。

Markdown 输入本切片不做直通，返回 `E_UNSUPPORTED_FORMAT` 并提示后续支持。加密文件返回 `E_ENCRYPTED`。

### 3.2 方向与引擎

本切片目标格式固定为 Markdown。方向控件显示「源格式 → Markdown」；引擎为「自动 / anydoc」，其中自动实际使用 anydoc。xberg、pandoc 选项可见但禁用，并说明将在后续 change 接入。

### 3.3 转换与输出

- 转换在 Tauri 后台命令中执行，前端显示内联进度（读取、转换、写入）与取消按钮。
- 输出目录来自设置；默认 `~/Documents/DullDoc Output`。
- 输出文件为 `<stem>.md`；同名文件存在时自动生成 `<stem>-1.md`、`<stem>-2.md`，不覆盖已有文件。
- 输出写入失败返回 `E_OUTPUT_PERMISSION`，不留下半写文件；使用临时文件 + rename。
- anydoc 返回 Markdown 后统计字符数、标题数、表格行数，写入历史。
- 本切片不提取内嵌图片资产；图片只在 Markdown 中保留可读 alt 文本，后续 change 补齐 assets/base64。该限制在转换页的帮助提示中说明。

### 3.4 错误

| 错误码 | 触发 | 用户可见行为 |
| --- | --- | --- |
| `E_UNSUPPORTED_FORMAT` | 无法识别格式或 Markdown 输入 | 错误卡：支持格式列表与「选择其他文件」 |
| `E_NEEDS_OCR` | PDF 含无文本页 | 错误卡：`E_NEEDS_OCR`、页数与「OCR 将在后续版本接入」 |
| `E_ENCRYPTED` | 加密文档 | 错误卡：请先解密后重试 |
| `E_PARSE_FAILED` | 引擎解析失败 | 错误卡：原因、复制诊断、重试 |
| `E_OUTPUT_PERMISSION` | 输出目录不可写 | 错误卡：打开设置 / 更换目录 |
| `E_ENGINE_UNAVAILABLE` | anydoc 初始化失败 | 错误卡：引擎状态与重启建议 |

错误不得导致应用崩溃；不得产生不完整输出；不得静默跳过。

### 场景 A3｜Word 转 Markdown

- Given 设置中的输出目录可写
- When 用户拖入 `example/GB:T 28448-2019 信息安全技术 网络安全等级保护测评要求.docx`
- Then 方向显示「Word → Markdown」，引擎显示自动/anydoc
- When 用户点击开始转换
- Then 界面显示内联进度，完成后显示输出文件摘要与「打开输出」
- And 输出目录出现 `.md` 文件，包含 40 个以上标题与至少一个表格
- And 历史新增一条完成记录

验收：A3

### 场景 A4｜其他 anydoc 格式

- Given 用户选择 `example/等级保护测评2.0要求指标清单v1.xlsx`
- When 转换完成
- Then 输出 Markdown 包含表格行，文件名与输入 stem 一致
- And 选择 CSV/RTF/EPUB/文本型 PDF 时同样得到 Markdown，文本型 PDF 不触发 OCR
- And 不支持的组合或格式返回错误卡而不是静默失败

验收：A4

### 场景 A5｜需要 OCR 与不可转换错误

- Given 用户选择封面页无文本层的 PDF
- When 点击开始转换
- Then 界面显示 `E_NEEDS_OCR`、页数说明与后续 OCR 提示
- And 不产生不完整输出文件
- And 选择不支持格式时显示 `E_UNSUPPORTED_FORMAT` 与可执行下一步
- And 应用保持运行

验收：A5

## 4. 转换历史

- 存储 `~/.dull_doc/history.json`，最多 50 条，可配置。
- 每条记录：`id`、`created_at`、`input_path`、`input_name`、`direction`、`engine`、`status`、`output_path`、`error_code`、`stats`、`duration_ms`。
- 只存路径与元数据，不复制文件。
- 历史页按时间倒序显示文件、状态、引擎、统计与操作；点击行打开详情弹窗，展示方向、预设、输出格式、引擎、输出路径、统计与警告。
- 操作：重跑（使用相同输入与参数，回到转换页）、打开输出、复制参数。
- 清空历史必须二次确认；只删除历史，不删除输出文件。
- 重启应用后历史仍在。

### 场景 A6｜历史持久化与清理

- Given 已完成至少一次转换
- When 用户进入转换历史并点击该行
- Then 弹窗显示文件、方向、引擎、输出路径与统计
- And Esc/关闭按钮可退出，焦点回到触发行
- When 用户点击清空历史并确认
- Then 历史列表为空，输出文件仍存在
- And 重启应用后历史仍为空

验收：A6

## 5. 设置与存储

### 5.1 设置

`~/.dull_doc/settings.json`，schema 版本 1：

```json
{
  "version": 1,
  "language": "zh",
  "theme": "system",
  "output_dir": "~/Documents/DullDoc Output"
}
```

- 读取失败或格式无效时使用默认值，不阻塞启动。
- 写入使用临时文件 + rename；并发写入以最后一次成功写入为准。
- 语言、主题、输出目录修改后立即生效。
- 引擎分组显示 anydoc 版本与就绪状态；PaddleOCR/Python 运行时显示「后续接入」。
- 预设、日志、关于分组按原型展示；本切片不实际管理预设。

### 5.2 路径与权限

- 输入文件必须是用户选择的普通文件；拒绝目录、符号链接逃逸与设备文件。
- 输出目录必须是绝对路径且可创建；写入前规范化路径。
- 应用数据目录固定 `~/.dull_doc/`，包含 `settings.json`、`history.json`、`logs/`、`tmp/`。

### 场景 A7｜设置持久化与输出权限

- Given 用户把语言改为 English、主题改为深色、输出目录改为 `~/Documents/DullDoc Test`
- When 重启应用
- Then 三项设置保持
- When 输出目录被设为不可写位置并开始转换
- Then 界面显示 `E_OUTPUT_PERMISSION` 与打开系统设置/更换目录操作
- And 不产生半写文件

验收：A7

## 6. 构建、离线与安全

- `pnpm build` 成功；`cargo check --manifest-path src-tauri/Cargo.toml` 成功；前端单元测试与 Rust 测试通过。
- 应用启动与转换不发起网络请求；anydoc 为本地 crate。
- 不引入 pandoc、Typst、xberg、uv sidecar；不读取网络模型。
- 外部命令（本切片只有必要的系统打开目录）使用参数数组；不拼接 shell。
- 日志写入 `~/.dull_doc/logs/`，不记录文件正文、密钥或完整路径中的敏感片段；诊断可复制。
- 不提交 `example/`、`target/`、`node_modules/`、`tools/` 与任何密钥。

### 场景 A8｜构建与离线

- Given 干净的前端与 Rust 依赖环境
- When 执行 `pnpm build`、`cargo check --manifest-path src-tauri/Cargo.toml` 与测试命令
- Then 全部成功
- When 应用在断网环境启动并完成一次 docx→md 转换
- Then 转换成功且没有网络请求
- And `~/.dull_doc` 只新增 settings/history/logs/tmp 数据

验收：A8

## 7. 非目标

- 批量队列、语料清洗、切块、chunks.jsonl、manifest.json。
- xberg OCR、PaddleOCR、Marker、扫描件与图片型 PDF。
- Markdown → Word/PDF/HTML/EPUB、PDF → Word、Typst PDF。
- 图片资产保留、base64、清洗/切块开关的真实执行。
- Developer ID 签名、公证、DMG、自动更新、远程推送。
- 云服务、账号、团队协作、向量库 UI。

## 8. 数据与错误契约

### 8.1 历史记录

```json
{
  "version": 1,
  "tasks": [
    {
      "id": "uuid",
      "created_at": "2026-11-19T00:00:00Z",
      "input_path": "/path/file.docx",
      "input_name": "file.docx",
      "direction": "Word → Markdown",
      "engine": "anydoc",
      "status": "completed",
      "output_path": "/path/file.md",
      "error_code": null,
      "stats": { "chars": 1234, "headings": 12, "table_rows": 8 },
      "duration_ms": 180
    }
  ]
}
```

### 8.2 错误响应

Tauri command 错误统一返回：

```json
{
  "code": "E_NEEDS_OCR",
  "message": "该 PDF 第 1 页没有文本层",
  "detail": { "pages": [1], "page_count": 50 },
  "next_step": "安装 OCR 组件或先转 Markdown"
}
```

用户可见错误必须包含 code、原因与下一步；日志可包含技术 detail，但不得包含文件正文或密钥。

## Requirement: batch-pipeline 批量输入、队列与调度

> 本节新增的批量语料能力替代第 1 节表格中「语料制作」的占位说明与第 7 节中「批量队列、语料清洗、切块、chunks.jsonl、manifest.json」的非目标；归档后这些行为属于 MVP capability。

- 输入：
  - 文件选择器支持多选文件；文件夹选择递归扫描。
  - 跳过隐藏文件/目录、`.DS_Store`、输出目录自身、符号链接与设备文件；不重复添加同一规范化路径。
  - 只接受 anydoc 可解析格式；不可识别文件在添加时跳过并在界面给出原因，不进入队列。
- 队列字段：文件、状态（等待/转换/清洗/切块/完成/失败/跳过）、引擎、表格等级、耗时、警告数、操作（取消/移除/重试/打开）。
- 调度：
  - 并发默认 `min(4, CPU/2)`，限制 1–8；配置只作用于本次批次，不写全局设置。
  - 等待任务按添加顺序启动；每次批次使用独立输出包，不存在跨批次覆盖。
  - 单文件失败或 `E_NEEDS_OCR` 只标记该行，继续处理其他文件；汇总状态为 `partial` 或 `failed`。
- 取消：停止调度；对运行中任务设置取消标记；未完成临时包目录删除，不保留半写文件。
- 重试：仅重跑失败行；成功/跳过行不变。
- 错误复用 CHG-001 错误码；每行显示 code 与下一步，并写入报告与 manifest。
- 扫描与调度在 Tauri 后台执行，通过事件推送单文件与总进度。

#### Scenario: 批量导入与队列统计

- Given 用户选择包含嵌套目录的文件夹
- When 扫描完成
- Then 队列只包含受支持文件，隐藏文件、`.DS_Store` 与输出目录被跳过
- And 统计中的文件数等于队列行数

Acceptance: A1

#### Scenario: 失败隔离、取消与重试

- Given 队列包含一个无文本 PDF 和多个 docx
- When 开始批量
- Then PDF 行显示 `E_NEEDS_OCR`，其余文件继续完成
- When 用户取消批次
- Then 不再启动新文件，当前批次不留下半写包
- When 用户重试失败行
- Then 只重新处理该文件，成功行不重复

Acceptance: A2

## Requirement: batch-cleaning 清洗与规范化

- 固定保守规范化（不提供开关）：合并连续 3 个以上空行为 1 个空行；标题编号 `5 . 1` → `5.1` 并去除标题内多余空格；不调整标题层级。
- 开关：
  - 页眉页脚/页码：默认开启。只删除匹配页码模式（纯数字行、`第 N 页`、`Page N of M`）与在文档首尾重复出现的短行；不删除正文数字。
  - 中文排版：默认开启。删除 CJK 字符之间空格；保留 CJK 与 Latin/数字之间空格；跳过 fenced code、行内 code、URL、公式与表格行。
  - 表格质量分级：默认开启。检查 GFM 表格列数一致性；一致为 A，存在空列/列数不一致为 B/C，只写警告，不修改表格。
- TOC 默认保留；断行合并默认关闭，本 change 不提供断行开关。
- 任一开关关闭时，该规则不执行，输出与未启用时一致；其他规则不受影响。
- 清洗后 Markdown 直接写入镜像文件与后续切块，不提供预览。

#### Scenario: 清洗开关与保护

- Given 原始 Markdown 含重复页眉、页码、CJK 间空格、标题 `5 . 1` 与代码块
- When 全部清洗开关关闭
- Then 输出与原始 Markdown 完全一致
- When 开启清洗
- Then 页眉页码删除、空行压缩、标题变为 `5.1`、CJK 间空格移除
- And 代码块、URL、表格与正文数字保持不变

Acceptance: A3

## Requirement: batch-chunking 切块与元数据

- 默认按 H1–H3 标题切块；标题路径保存为数组；首个标题之前的内容作为独立块。
- 切块上限默认 1200 字符（可选 500–4000）；超长块先按空行/段落切分，仍超长时按字符硬切，并按 0–300 字符重叠（默认 50）。
- 表格整块不拆；超长表格保留为单块并写警告。fenced code 整块不拆；超长代码块保留并写警告。
- 每块元数据：`source`（镜像相对路径）、`source_path`（输入绝对路径）、`file_name`、`format`、`title`（文档标题或文件名）、`heading_path`、`page`（anydoc 路径为 null）、`hash`（输入 SHA-256）、`engine`、`params`（chunkSize/overlap/cleaning）、`chars`。
- chunk id：`sha256(source + heading_path.join("/") + ordinal + sha256(text))` 取前 16 个十六进制字符；同一输入与参数重复运行得到相同 id。
- 空文本块不写入；表格块与代码块可独立成块。

#### Scenario: 切块与稳定 id

- Given 一份包含多级标题、表格与超长段落的 Markdown
- When 使用默认 1200 字符/50 重叠切块两次
- Then 两次 chunk 数量与 id 完全一致
- And 表格与代码块未被拆分
- And 每个 chunk 含来源、章节路径、字符数与参数

Acceptance: A4

## Requirement: batch-corpus 语料包与校验

- 输出目录：`<outputDir>/<batchStem>-<YYYYMMDD-HHmmss>/`；`batchStem` 取扫描根目录名，无法确定时为 `corpus`，并做文件名安全化。
- 布局：
  - `mirror/` — 保留扫描根内相对路径的 Markdown。
  - `chunks.jsonl` — 每行一个 `{id,text,metadata}`，UTF-8 无 BOM，末行换行。
  - `manifest.json` — schema、工具名/版本、引擎名/版本、预设、创建时间、参数、输入文件列表（路径、格式、哈希、状态、错误码、统计、警告）、总计。
  - `report.json`、`report.csv` — 成功/跳过/失败/警告计数与逐文件错误码。
- 写入先在同级临时目录完成，校验通过后原子 rename 为批次目录；失败或取消删除临时目录，不留下半写包。
- 校验：JSONL 每行可解析且含 `id/text/metadata`；id 唯一；manifest 文件列表与 mirror 一致；chunk 的 `source` 均存在；空块、块数不一致或 JSON 解析失败返回 `E_CORPUS_VALIDATION`，附文件与行号。
- 语料包不绑定平台，可用于后续向量库导入。

#### Scenario: 语料包产物与校验

- Given 批量完成
- When 打开批次目录
- Then 存在 `mirror/`、`chunks.jsonl`、`manifest.json`、`report.json`、`report.csv`
- And `chunks.jsonl` 每行可被 `jq` 解析且 id 唯一
- And manifest 的文件列表与 mirror、报告计数一致
- When 人为破坏一个 chunk 行后重跑校验
- Then 返回 `E_CORPUS_VALIDATION` 并指出文件与行号

Acceptance: A5

## Requirement: batch-history 批量历史

- 每次批次写入一条 `history.json` 记录：`kind: "batch"`、`created_at`、`input_paths`、`input_count`、`preset`、`engine`、`status`（completed/partial/failed）、`output_dir`、`counts`（success/skipped/failed/warnings/chunks）、`duration_ms`、`params`。
- 旧单文件记录保持可读；`kind` 缺省视为 `single`，新增字段使用默认值。
- 历史列表：批量行显示「批量 · N 文件」、状态、块数或警告数；点击打开详情，展示计数、输出目录、参数与失败明细。
- 操作：打开语料包、复制参数、重跑（把原输入与参数送回语料制作页，不自动开始）。
- 清空历史与 50 条上限规则不变；清空不删除语料包。

#### Scenario: 批量历史与重跑

- Given 完成一次 3 文件批次，其中 1 个失败
- When 进入转换历史
- Then 出现一条 batch 记录，显示 3 文件、部分完成与失败数
- And 详情显示输出目录、参数与失败明细
- When 重启应用
- Then 该记录仍在
- When 点击重跑
- Then 语料制作页恢复输入与参数，等待用户点击开始

Acceptance: A6

## Requirement: batch-ui 语料制作页

- 页面按 `docs/superpowers/UI.html` 的语料制作页还原：标题与四个操作按钮、三项统计、队列面板（筛选/搜索/表格）与右侧配置面板。
- 队列列：文件、状态、引擎、表格等级、耗时、操作；状态 badge 使用 Proofing Desk token；行内支持取消/移除/重试/打开。
- 配置项：预设（标准/表格/知识库）、输出格式固定 Markdown（Word/PDF 禁用并说明）、引擎自动/anydoc（xberg/pandoc 禁用）、并发 1/2/4/8、清洗开关（页眉页脚、中文标题、表格分级）、图片资产（禁用并说明 CHG-003）、切块、元数据、生成语料包、切块上限与重叠。
- 空状态、加载、部分失败与取消状态均有内联说明；不提供 Markdown 预览。
- 中英文文案完整；1280×800 与 1440×900 无横向溢出；颜色只来自 token。

#### Scenario: 语料制作页交互与还原

- Given 语料制作页为空
- When 用户添加文件并调整配置
- Then 统计、队列与配置状态即时更新
- And 输出格式只可选 Markdown，图片开关禁用并给出原因
- When 切换 English 与深色主题
- Then 页面文案与颜色即时更新
- And 1280×800 与 1440×900 无横向滚动

Acceptance: A7

## Requirement: batch-runtime 构建、离线与安全

- `pnpm build`、前端测试、`cargo check`、Rust 测试全部通过。
- 批量转换、清洗、切块与语料包生成不发起网络请求；不引入 xberg/pandoc/Typst/OCR sidecar。
- 扫描与写入限制在用户选择的输入根与输出目录内；拒绝符号链接逃逸与设备文件；所有外部命令使用参数数组。
- 语料包与报告只写入输出目录；`~/.dull_doc` 只写 settings/history/logs/tmp。
- 日志不记录文件正文、密钥或完整敏感路径；错误报告可定位但可安全分享。

#### Scenario: 构建与离线批量

- Given 已安装前端与 Rust 依赖并断网
- When 执行构建与测试命令
- Then 全部通过
- When 完成一次包含 docx 与 xlsx 的批量运行
- Then 语料包生成成功且无网络请求
- And `~/.dull_doc` 未新增除 settings/history/logs/tmp 之外的数据

Acceptance: A8

