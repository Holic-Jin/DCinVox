#!/bin/bash
# macOS Steam depot 폴더 만들기: DCinVox.app + models/ + 문서 → steam/content/mac/
# 먼저: tools/build_ffmpeg_macos.sh && (cd src-tauri && cargo tauri build --bundles app)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/steam/content/mac"
APP="$ROOT/src-tauri/target/release/bundle/macos/DCinVox.app"
[ -d "$APP" ] || { echo "먼저 배포 빌드를 하세요 (cargo tauri build --bundles app)"; exit 1; }
rm -rf "$OUT" && mkdir -p "$OUT/models"
cp -R "$APP" "$OUT/"
cp "$ROOT"/models/ggml-large-v3-q5_0.bin "$ROOT"/models/phoneme-aligner-int8.onnx "$ROOT"/models/phoneme-aligner-vocab.json "$OUT/models/"
cp "$ROOT"/docs/THIRD_PARTY_NOTICES.md "$ROOT"/docs/EULA.md "$OUT/"
# 설치 상태 점검 (모델·ffmpeg를 찾는지)
"$OUT/DCinVox.app/Contents/MacOS/dcinvox" --self-check
du -sh "$OUT"
