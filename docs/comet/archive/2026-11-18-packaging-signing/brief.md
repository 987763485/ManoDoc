# 目标

交付 CHG-005「packaging-signing」：把 DullDoc 打成可分发的 macOS Apple Silicon 应用，pandoc 3.14、Typst 0.15.1 与 uv 0.13.9 sidecar 随包内置于 `.app`，在构建前逐个预签名；在缺少 Developer ID 证书的当前环境下完成 ad-hoc 结构签名、`.app` 与 `.dmg` 打包及实机启动验证，并把公证（notarization）与 staple 的阻塞和发布前剩余步骤明确记录。

本 change 复用 CHG-001/003/004 已完成的转换、OCR 与导出能力；不新增业务功能，只补齐分发形态与运行时 sidecar 解析。

# 范围

## 目录结构

### 新建

- `scripts/package-macos.sh` — 打包入口：校验主机架构、暂存并预签名 sidecar、调用 `pnpm tauri build`、后置校验 `.app`/`.dmg`、输出公证阻塞说明。
- `src-tauri/binaries/.gitignore` — 暂存目录不入库，sidecar 二进制从 `tools/` 复制。
- `docs/superpowers/打包与签名.md` — 打包命令、签名链、验证证据、公证阻塞与发布前 checklist。

### 修改

- `src-tauri/tauri.conf.json` — `bundle.targets` 增加 `dmg`；声明 `externalBin`（pandoc/typst/uv）；补 macOS 图标、分类与最低系统版本。
- `src-tauri/src/engine/sidecar.rs` — 明确打包后解析顺序（环境变量 → 应用同目录 → 开发目录），并在错误信息中列出应用内路径。
- `package.json` — 增加 `package:mac` 脚本。
- `docs/superpowers/技术选型.md`、`docs/superpowers/需求文档.md`、`docs/superpowers/软件功能设计说明.md`、`AGENTS.md`、`CHANGE.md` — 同步打包实现与公证状态。
- `src-tauri/icons/` — 由 `tauri icon` 生成 macOS 所需图标集。

### 明确不建

- Developer ID 证书申请、Apple Developer 账号配置与真实公证上传 — 本机无证书，无法执行。
- 自动更新、Sparkle、公证后的 `staple` — 依赖公证成功。
- Intel/x86_64 与 Windows 打包 — 首发仅 Apple Silicon。
- 代码签名后的 Gatekeeper 首次启动引导页 — 属发布运营文档。

# 非目标

- 新增转换能力或修改目标格式矩阵（CHG-004 已完成）。
- P1 Python 引擎（PaddleOCR/Marker）的实际运行时安装流程；本 change 只把 uv sidecar 打进包，运行时逻辑保持现状。
- 应用商店分发与沙盒化（App Sandbox）。
- 远程推送、Release 上传与自动发布流水线。

# 约束与不变量

- 仓库不提交 sidecar 二进制：`tools/` 与 `src-tauri/binaries/` 均在本地产出，打包脚本从 `tools/macos` 复制并加 `aarch64-apple-darwin` 后缀。
- 签名顺序自内向外：先 sidecar，再 `.app`；无 Developer ID 时使用 ad-hoc（`codesign --sign -`），不伪造证书。
- 打包与验证全程离线，不访问 Apple 公证服务（无证书时）。
- 现有 sidecar 调用约定不变：参数数组、120 秒超时、错误码 `E_PANDOC_MISSING`/`E_TYPST_MISSING`。
- 打包不得破坏开发流程：`pnpm tauri dev` 仍从 `tools/macos` 解析 sidecar。
- 界面与文案不需改动；不新增 i18n key。
- 未获得用户明确授权前不执行远程推送。

# Decisions

- 打包工具链：使用仓库已安装的 `@tauri-apps/cli`（Tauri 2），目标 `aarch64-apple-darwin`，产物 `.app` + `.dmg`。
- sidecar 分发方式：Tauri `externalBin`。暂存文件命名 `src-tauri/binaries/<name>-aarch64-apple-darwin`，打包后位于 `Contents/MacOS/<name>`，由现有解析器按“应用同目录”命中。
- 随包二进制：pandoc、typst、uv 三个。uv 当前无调用方，按已确认的 P1 决策预留，不启用任何安装逻辑。
- 签名策略：本机 0 个 codesigning identity，使用 ad-hoc 签名；脚本预留 `DULLDOC_SIGN_IDENTITY` 环境变量，填入 Developer ID 后自动切换到正式签名与 notarytool 流程。
- 公证策略：无证书时脚本不尝试 notarize，明确输出所需条件（Developer ID Application 证书、Apple ID/Team ID、`xcrun notarytool`、`.app` 与 `.dmg` 分别公证并 staple），并以可区分的“打包成功、公证待证书”结果退出。
- 图标：用现有 `icons/icon.png` 通过 `tauri icon` 生成完整图标集，避免手写 icns。
- 版本与标识：保持 `0.1.0`、`dev.dulldoc.app`、最低系统 12.0。

# 验收示例

- A1：执行打包入口后产出 `src-tauri/target/release/bundle/macos/DullDoc.app`，`Contents/MacOS/` 内含 DullDoc 与 pandoc、typst、uv 三个 sidecar，`Contents/Resources` 含前端产物。
- A2：打包前每个 sidecar 已单独签名；`codesign --verify --deep --strict` 对 `.app` 通过，`codesign -dv` 显示 `Signature=adhoc`，且 sidecar 的签名不晚于 `.app` 外层签名。
- A3：从打包后的 `.app` 启动（清空 `DULLDOC_PANDOC`/`DULLDOC_TYPST`，不依赖 `tools/` 路径）完成 Markdown → PDF 与 Markdown → Word，输出有效文件，证明使用内置 sidecar。
- A4：应用内 sidecar 解析顺序为环境变量 → 应用同目录 → 开发目录；把内置 pandoc 移走后转换返回 `E_PANDOC_MISSING`，恢复后可用，不静默改用系统二进制。
- A5：生成 `.dmg`，可挂载，卷内含 `DullDoc.app`，从挂载卷复制出的应用可启动。
- A6：`Info.plist` 中版本 `0.1.0`、标识 `dev.dulldoc.app`、最低系统 `12.0` 与图标正确；DMG 文件名含版本与架构。
- A7：无 Developer ID 时打包脚本明确报告公证/staple 阻塞与补齐步骤，`spctl --assess` 拒绝为预期结果；脚本退出码区分“打包失败”与“打包成功但待公证”。
- A8：`pnpm build`、`pnpm test`、`cargo check`、`cargo test` 通过；打包脚本可重复执行且不污染仓库；未执行远程推送；`CHANGE.md`、`AGENTS.md` 与文档已同步。
