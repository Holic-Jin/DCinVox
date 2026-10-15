# Release checklist (GitHub)

[한국어](RELEASE.md) · [English](RELEASE.en.md) · [日本語](RELEASE.ja.md)

Distribution is GitHub.

## A. Accounts

| Step | Task | Notes |
|---|---|---|
| 1 | GitHub repo `Holic-Jin/DCinVox` | Source only. Do not commit model weights, sample audio, or FFmpeg binaries |
| 2 | (Optional) Apple Developer / Windows code signing | Needed to reduce Gatekeeper / SmartScreen warnings if people download `.app` / `.exe` from GitHub |
| 3 | Legal review of the EULA | `docs/EULA.en.md` is a draft |
| 4 | Screenshots and trailer | Only footage you shot or have permission to use |

## B. Build

- macOS: `tools/build_ffmpeg_macos.sh` → `cargo tauri build --bundles app`
- Windows: `.github/workflows/build.yml` (`workflow_dispatch` or tag `v*`)
- Models sit in a `models/` folder **beside** the app (`models/README.md`)

## C. Example GitHub Release layout

```
macOS                         Windows
├─ DCinVox.app                ├─ dcinvox.exe
├─ models/                    ├─ ffmpeg.exe
│  ├─ ggml-large-v3-q5_0.bin  └─ models/ (same)
│  ├─ phoneme-aligner-int8.onnx
│  └─ phoneme-aligner-vocab.json
├─ THIRD_PARTY_NOTICES.md
└─ EULA.en.md
```

Models are safer on Hugging Face or similar because of GitHub file-size limits. Do not put them in the git repo.

## D. Before shipping

- [ ] On a clean Mac/Windows with no dev tools, **offline**: analyze → generate → convert → export
- [ ] Stay under about 3 GB RAM on an 8 GB machine
- [ ] Keep the FFmpeg source tarball and build scripts (LGPL source offer)
