#!/usr/bin/env bash
# DullDoc macOS Apple Silicon 打包入口。
#
# 顺序自内向外：先暂存并签名 sidecar，再由 Tauri 打包 .app/.dmg。
# 本机没有 Developer ID 时使用 ad-hoc 签名，公证阶段只输出剩余步骤，
# 并以退出码 20 区分「打包成功、待证书公证」与真正的打包失败。

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TRIPLE="aarch64-apple-darwin"
STAGE="$ROOT/src-tauri/binaries"
BUNDLE="$ROOT/src-tauri/target/$TRIPLE/release/bundle"
APP="$BUNDLE/macos/DullDoc.app"
DMG_DIR="$BUNDLE/dmg"
SIDECARS=(pandoc typst uv)
SIGN_IDENTITY="${DULLDOC_SIGN_IDENTITY:--}"

log() { printf '\n==> %s\n' "$*"; }
fail() { printf '\n错误: %s\n' "$*" >&2; exit 1; }

[ "$(uname -m)" = "arm64" ] || fail "首发仅支持 Apple Silicon，当前架构 $(uname -m)"

stage_sidecar() {
  local name="$1" source="$2"
  [ -x "$source" ] || fail "缺少可执行 sidecar: $source"
  cp -f "$source" "$STAGE/$name-$TRIPLE"
  chmod +x "$STAGE/$name-$TRIPLE"
}

# 预签名必须在 Tauri 封装 .app 之前完成：外层签名会密封已签名的内层二进制。
sign_sidecar() {
  local binary="$1"
  if [ "$SIGN_IDENTITY" = "-" ]; then
    codesign --force --sign - --timestamp=none "$binary"
  else
    codesign --force --sign "$SIGN_IDENTITY" --options runtime --timestamp "$binary"
  fi
  codesign --verify --strict "$binary" || fail "sidecar 签名校验失败: $binary"
}

log "暂存 sidecar 到 src-tauri/binaries"
mkdir -p "$STAGE"
stage_sidecar pandoc "$ROOT/tools/macos/pandoc/bin/pandoc"
stage_sidecar typst "$ROOT/tools/macos/typst/typst"
stage_sidecar uv "$ROOT/tools/macos/uv/uv"

log "签名 sidecar（身份: ${SIGN_IDENTITY}）"
for name in "${SIDECARS[@]}"; do
  sign_sidecar "$STAGE/$name-$TRIPLE"
done

log "构建 .app 与 .dmg"
# xberg-tesseract 的构建脚本会按 target 目录另建缓存并联网拉取 leptonica；
# 复用 debug 构建已缓存的源码包，保证打包全程离线。
export TESSERACT_RS_CACHE_DIR="$ROOT/src-tauri/target/xberg-tesseract-cache"
# 没有 APPLE_SIGNING_IDENTITY 时 Tauri 会跳过 .app 签名，只留下 linker 签名，
# 导致 Codesign 资源封条缺失；显式传入 "-" 才会生成 _CodeSignature。
export APPLE_SIGNING_IDENTITY="$SIGN_IDENTITY"
(cd "$ROOT" && pnpm tauri build --target "$TRIPLE" --bundles app,dmg)

log "校验 .app"
[ -d "$APP" ] || fail "未生成 $APP"
for name in "${SIDECARS[@]}"; do
  [ -x "$APP/Contents/MacOS/$name" ] || fail "应用内缺少 sidecar: Contents/MacOS/$name"
done
# 前端由 Tauri 内嵌主可执行文件；Resources 只应放图标等资源，避免重复打包导致真源分裂。
for asset in index.html assets; do
  [ ! -e "$APP/Contents/Resources/$asset" ] || fail "Resources 不应包含前端产物: $asset"
done
BIG_RESOURCE="$(find "$APP/Contents/Resources" -type f -size +10M -print -quit)"
[ -z "$BIG_RESOURCE" ] || fail "Resources 出现意外大文件: $BIG_RESOURCE"
codesign --verify --deep --strict --verbose=2 "$APP" || fail ".app 签名校验失败"
SIGNATURE="$(codesign -dv --verbose=2 "$APP" 2>&1 | awk -F= '/^Signature=/{print $2}')"
[ -n "$SIGNATURE" ] || fail "无法读取 .app 签名信息"

DMG="$(ls "$DMG_DIR"/*.dmg 2>/dev/null | head -1 || true)"
[ -n "$DMG" ] || fail "未生成 .dmg"

log "校验 .dmg"
MOUNT="$(hdiutil attach -nobrowse -readonly "$DMG" | awk '/\/Volumes\//{sub(/^.*\/Volumes\//,"/Volumes/"); print}' | tail -1)"
[ -n "$MOUNT" ] || fail "无法挂载 $DMG"
trap 'hdiutil detach "$MOUNT" -quiet >/dev/null 2>&1 || true' EXIT
[ -d "$MOUNT/DullDoc.app" ] || fail "DMG 内缺少 DullDoc.app"
hdiutil detach "$MOUNT" -quiet
trap - EXIT

log "校验 Info.plist"
plutil -extract CFBundleShortVersionString raw "$APP/Contents/Info.plist" | grep -qx "0.1.0" || fail "版本号不符"
plutil -extract CFBundleIdentifier raw "$APP/Contents/Info.plist" | grep -qx "dev.dulldoc.app" || fail "Bundle ID 不符"
plutil -extract LSMinimumSystemVersion raw "$APP/Contents/Info.plist" | grep -qx "12.0" || fail "最低系统版本不符"

log "Gatekeeper 评估（无 Developer ID 时期望被拒绝）"
if spctl --assess --type execute --verbose "$APP" 2>&1; then
  GATEKEEPER="accepted"
else
  GATEKEEPER="rejected"
fi
printf 'app=%s\nsignature=%s\ndmg=%s\ngatekeeper=%s\n' "$APP" "$SIGNATURE" "$DMG" "$GATEKEEPER"

if [ "$SIGN_IDENTITY" = "-" ]; then
  cat <<'MSG'

==> 打包完成；公证未执行
本机没有 Developer ID Application 证书，当前为 ad-hoc 签名，Gatekeeper 拒绝属预期。
发布前需要：
  1. 申请 Developer ID Application 证书并导入钥匙串；
  2. export DULLDOC_SIGN_IDENTITY="Developer ID Application: <团队名> (<TEAMID>)" 后重新运行本脚本；
  3. 保存公证凭据：xcrun notarytool store-credentials dulldoc-notary --apple-id <AppleID> --team-id <TEAMID> --password <app-specific-password>；
  4. 分别公证 .app 与 .dmg：xcrun notarytool submit <文件> --keychain-profile dulldoc-notary --wait；
  5. 成功后再 staple：xcrun stapler staple DullDoc.app 与 DullDoc_<版本>_aarch64.dmg，最后 spctl 复核。
MSG
  exit 20
fi

log "公证与 staple"
xcrun notarytool submit "$APP" --keychain-profile "${DULLDOC_NOTARY_PROFILE:-dulldoc-notary}" --wait
xcrun stapler staple "$APP"
xcrun notarytool submit "$DMG" --keychain-profile "${DULLDOC_NOTARY_PROFILE:-dulldoc-notary}" --wait
xcrun stapler staple "$DMG"
spctl --assess --type execute --verbose "$APP"
