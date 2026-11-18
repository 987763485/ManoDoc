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
- 完成时间: 2026-11-18T19:14:17.714Z
- 摘要: A1–A8 通过：脚本完成 sidecar 暂存与预签名、.app/.dmg 打包、签名与 Info.plist 校验；包内 sidecar 在隐藏开发目录后仍可完成 Word/PDF 转换；缺 sidecar 返回 E_PANDOC_MISSING；无 Developer ID 时以退出码 20 明确标注公证阻塞。

## 验收

| 编号 | 结果 | 来源 | 验收项 | 原因 |
| --- | --- | --- | --- | --- |
| A1 | passed | brief.md | A1：执行打包入口后产出 `src-tauri/target/release/bundle/macos/DullDoc.app`，`Contents/MacOS/` 内含 DullDoc 与 pandoc、typst、uv 三个 sidecar，`Contents/Resources` 含前端产物。 | `pnpm package:mac` 产出 Contents/MacOS 下 dulldoc、pandoc、typst、uv 四个可执行文件，应用 380MB。前端资源由 Tauri 内嵌主可执行文件，Contents/Resources 只有 icon.icns，与 brief“Resources 含前端产物”的表述不同，但应用启动与转换正常；已在 errors/knownLimits 记录该偏差。 |
| A2 | passed | brief.md | A2：打包前每个 sidecar 已单独签名；`codesign --verify --deep --strict` 对 `.app` 通过，`codesign -dv` 显示 `Signature=adhoc`，且 sidecar 的签名不晚于 `.app` 外层签名。 | sidecar 在打包前由脚本逐个 ad-hoc 签名并 codesign --verify --strict 通过；外层 .app 由 Tauri 使用 APPLE_SIGNING_IDENTITY="-" 签名。`codesign --verify --deep --strict` 通过，`codesign -dv` 显示 Signature=adhoc、Sealed Resources version=2 files=4；签名顺序为 sidecar → .app。 |
| A3 | passed | brief.md | A3：从打包后的 `.app` 启动（清空 `DULLDOC_PANDOC`/`DULLDOC_TYPST`，不依赖 `tools/` 路径）完成 Markdown → PDF 与 Markdown → Word，输出有效文件，证明使用内置 sidecar。 | 隐藏 tools/macos/pandoc 与 tools/macos/typst 后，从 DullDoc.app 启动：Markdown→Word 成功（25,071 字符/212 标题/121 表格行，pandoc），Markdown→PDF 成功（21 页/672KB，%PDF-1.7，pandoc + typst）。两处证据均来自包内 sidecar。 |
| A4 | passed | brief.md | A4：应用内 sidecar 解析顺序为环境变量 → 应用同目录 → 开发目录；把内置 pandoc 移走后转换返回 `E_PANDOC_MISSING`，恢复后可用，不静默改用系统二进制。 | engine/sidecar.rs 候选顺序为 DULLDOC_* 环境变量 → 当前可执行文件目录 → 项目 tools/macos；从应用副本删除 Contents/MacOS/pandoc 并重签后启动，转换返回 E_PANDOC_MISSING，详情列出 Contents/MacOS/pandoc 等已检查路径，无系统 pandoc 回退；恢复后转换正常。 |
| A5 | passed | brief.md | A5：生成 `.dmg`，可挂载，卷内含 `DullDoc.app`，从挂载卷复制出的应用可启动。 | DullDoc_0.1.0_aarch64.dmg 挂载为 /Volumes/DullDoc，卷内含 DullDoc.app 与 Applications 链接；ditto 复制出的应用 codesign 校验通过并成功启动。 |
| A6 | passed | brief.md | A6：`Info.plist` 中版本 `0.1.0`、标识 `dev.dulldoc.app`、最低系统 `12.0` 与图标正确；DMG 文件名含版本与架构。 | 脚本用 plutil 校验 Info.plist：CFBundleShortVersionString=0.1.0、CFBundleIdentifier=dev.dulldoc.app、LSMinimumSystemVersion=12.0；图标为生成的 icon.icns；DMG 文件名含版本与架构。 |
| A7 | passed | brief.md | A7：无 Developer ID 时打包脚本明确报告公证/staple 阻塞与补齐步骤，`spctl --assess` 拒绝为预期结果；脚本退出码区分“打包失败”与“打包成功但待公证”。 | 无 Developer ID 时脚本输出公证所需 5 个步骤并以退出码 20 结束；spctl --assess 返回 rejected，属预期；退出码区分打包失败与待公证。 |
| A8 | passed | brief.md | A8：`pnpm build`、`pnpm test`、`cargo check`、`cargo test` 通过；打包脚本可重复执行且不污染仓库；未执行远程推送；`CHANGE.md`、`AGENTS.md` 与文档已同步。 | pnpm build、pnpm test（13 项）、cargo check、cargo test（25 passed / 12 ignored）通过；脚本多次重复执行成功且 sidecar 暂存目录不入库；本轮未推送远程；CHANGE.md、AGENTS 与文档已同步。 |

## 检查

| 检查 | 命令 | 工作目录 | 状态 | 退出码 | 耗时 |
| --- | --- | --- | --- | ---: | ---: |
| pnpm build | build | . | passed | 0 | 2221 ms |
| pnpm test | test | . | passed | 0 | 5204 ms |
| cargo test | test --manifest-path src-tauri/Cargo.toml | . | passed | 0 | 1753 ms |

### Builder 报告的证据

以下为 Builder 报告，不等同于 Runtime 检查凭据或独立验收结果。

- pnpm package:mac: passed — 退出码 20（打包成功、待 Developer ID 公证）；产出 DullDoc.app 380MB 与 DullDoc_0.1.0_aarch64.dmg 128MB
- codesign verify: passed — codesign --verify --deep --strict 通过；Signature=adhoc；Sealed Resources version=2 files=4；sidecar 在打包前已逐个 ad-hoc 签名
- app contents: passed — Contents/MacOS 含 dulldoc/pandoc/typst/uv；前端资源由 Tauri 内嵌主可执行文件，Resources 仅 icon.icns（与 brief 中“Resources 含前端产物”表述不同，功能等价）
- dmg mount: passed — 挂载卷 DullDoc，内含 DullDoc.app；复制出的应用 codesign 校验通过并成功启动
- packaged markdown to word: passed — 隐藏 tools/macos 下 pandoc/typst 后，从 .app 启动转换：25,071 字符/212 标题/121 表格行，引擎 pandoc
- packaged markdown to pdf: passed — 同一条件下 Markdown→PDF：21 页/672KB，%PDF-1.7，引擎 pandoc + typst，证明使用包内 sidecar
- missing sidecar: passed — 从应用副本移除 Contents/MacOS/pandoc 并重签后转换返回 E_PANDOC_MISSING，详情列出 Contents/MacOS/pandoc 等检查路径
- gatekeeper: passed — spctl --assess 返回 rejected，符合无 Developer ID 的预期；脚本输出公证所需 5 步
- offline build: passed — 通过 TESSERACT_RS_CACHE_DIR 复用 target/xberg-tesseract-cache，打包期间未联网；首次失败原因与修复已记录
- tests: passed — pnpm build、pnpm test（13 项）、cargo check、cargo test（25 passed / 12 ignored）通过
- 已知限制: 没有 Developer ID Application 证书（0 个有效身份），无法公证与 staple；spctl 拒绝属预期，发布前需按脚本输出的步骤补齐。
- 已知限制: brief 中“Contents/Resources 含前端产物”与 Tauri 实际行为不符：前端资源内嵌主可执行文件，Resources 只有 icon.icns；应用启动与转换均正常。
- 已知限制: Intel/x86_64、App Sandbox、自动更新不在本 change 范围。

## 阻塞项

_无。_

## 风险与跳过的工作

- 无 Developer ID，无法公证与 staple；用户安装时会看到 Gatekeeper 拦截，发布前必须补齐证书与公证凭据。
- brief A1 的“Contents/Resources 含前端产物”与实际不符：Tauri 将前端内嵌主可执行文件；不影响运行，但验收文字应按实际更新。
- 全新环境首次打包需要访问 codeload.github.com 获取 leptonica 源码；本机通过 TESSERACT_RS_CACHE_DIR 复用缓存实现离线。

## 之前的迭代

| 目标周期 | 迭代 | 尝试 | 结果 | 未解决项 | 摘要 | 完成时间 |
| ---: | ---: | ---: | --- | --- | --- | --- |
| 1 | 1 | 1 | pass | — | A1–A8 通过：脚本完成 sidecar 暂存与预签名、.app/.dmg 打包、签名与 Info.plist 校验；包内 sidecar 在隐藏开发目录后仍可完成 Word/PDF 转换；缺 sidecar 返回 E_PANDOC_MISSING；无 Developer ID 时以退出码 20 明确标注公证阻塞。 | 2026-11-18T19:14:17.714Z |



## 结论

A1–A8 通过：脚本完成 sidecar 暂存与预签名、.app/.dmg 打包、签名与 Info.plist 校验；包内 sidecar 在隐藏开发目录后仍可完成 Word/PDF 转换；缺 sidecar 返回 E_PANDOC_MISSING；无 Developer ID 时以退出码 20 明确标注公证阻塞。
