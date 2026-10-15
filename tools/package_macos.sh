#!/bin/bash
# macOS 배포 zip은 반드시 Mac에서 만든다. Linux zip은 .app 번들 서명을 깨서
# Gatekeeper가 실행 직후 SIGKILL로 죽일 수 있다.
# 사용: tools/package_macos.sh [dist폴더] [출력zip]
# 기본: dist/ → DCinVox-macos-arm64.zip
set -euo pipefail
if [ "$(uname -s)" != "Darwin" ]; then
  echo "이 스크립트는 macOS에서만 실행하세요. Linux에서 .app을 zip하면 서명이 깨집니다." >&2
  exit 1
fi
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DIST="$(cd "${1:-$ROOT/dist}" && pwd)"
OUT="${2:-$ROOT/DCinVox-macos-arm64.zip}"
OUT_DIR="$(cd "$(dirname "$OUT")" && pwd)"
OUT="$OUT_DIR/$(basename "$OUT")"
APP="$DIST/DCinVox.app"
if [ ! -d "$APP" ]; then
  echo "없음: $APP" >&2
  exit 1
fi

mkdir -p "$DIST/models"
if [ -f "$ROOT/models/phoneme-aligner-vocab.json" ]; then
  cp "$ROOT/models/phoneme-aligner-vocab.json" "$DIST/models/"
fi
cp -f "$ROOT/docs/THIRD_PARTY_NOTICES.md" "$DIST/" 2>/dev/null || true
cp -f "$ROOT/docs/EULA.md" "$DIST/" 2>/dev/null || true
cp -f "$ROOT/tools/export_aligner.py" "$DIST/"

# Mach-O adhoc만 있으면 CodeResources가 없어 검증이 실패한다. 번들 단위로 다시 서명.
codesign --force --deep --sign - "$APP"
codesign --verify --verbose=2 "$APP"

rm -f "$OUT"
# dist 내용이 zip 루트가 되게 한다 (dist/ 한 겹이 생기면 안 됨).
# --sequesterRsrc 는 리소스 포크를 보존한다. Linux zip은 번들 서명을 깨뜨린다.
(cd "$DIST" && ditto -c -k --sequesterRsrc . "$OUT")
echo "wrote $OUT"
echo "codesign:"
codesign -dv --verbose=2 "$APP" 2>&1 | head -n 20
