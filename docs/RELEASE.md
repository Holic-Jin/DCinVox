# 출시 체크리스트 (GitHub)

[한국어](RELEASE.md) · [English](RELEASE.en.md) · [日本語](RELEASE.ja.md)

배포 경로는 GitHub입니다.

## A. 계정

| 순서 | 할 일 | 메모 |
|---|---|---|
| 1 | GitHub 저장소 `Holic-Jin/DCinVox` | 소스만. 모델 가중치·샘플 음원·FFmpeg 바이너리는 git에 넣지 않음 |
| 2 | (선택) Apple Developer / Windows 코드서명 | GitHub에서 `.app`/`.exe`를 직접 받을 때 Gatekeeper·SmartScreen 경고를 줄이려면 필요 |
| 3 | EULA 법률 검토 | `docs/EULA.md`는 초안 |
| 4 | 스크린샷·예고편 | 직접 촬영했거나 허락받은 소재만 |

## B. 빌드

- macOS: `tools/build_ffmpeg_macos.sh` → `cargo tauri build --bundles app`
- Windows: `.github/workflows/build.yml` (workflow_dispatch 또는 태그 `v*`)
- 모델은 앱 안이 아니라 **옆 폴더 `models/`** (`models/README.ko.md`)

## C. GitHub Release에 올리는 폴더 예

```
macOS                         Windows
├─ DCinVox.app                ├─ dcinvox.exe
├─ models/                    ├─ ffmpeg.exe
│  ├─ ggml-large-v3-q5_0.bin  └─ models/ (동일)
│  ├─ phoneme-aligner-int8.onnx
│  └─ phoneme-aligner-vocab.json
├─ THIRD_PARTY_NOTICES.md
└─ EULA.md
```

모델은 Release 용량·GitHub 파일 크기 제한 때문에 Hugging Face 등 별도 호스트가 안전합니다. 저장소에는 올리지 마세요.

## D. 출시 전 확인

- [ ] 개발 도구가 없는 깨끗한 Mac/Windows에서, **인터넷을 끈 뒤** 분석 → 생성 → 변환 → 내보내기
- [ ] 8GB RAM에서 메모리 3GB 이하
- [ ] FFmpeg 소스 tarball과 빌드 스크립트 보관 (LGPL 요청 대응)
