# DCinVox 1.0.0

Cut speech from your videos into syllables, stitch it into lyrics, and turn it into a song with pitch blocks. All processing stays on your computer.

**This download does not include AI models** (about 1.3 GB). Put them in a `models/` folder next to the app. Guides:

- [한국어](https://github.com/Holic-Jin/DCinVox/blob/main/docs/INSTALL.md)
- [English](https://github.com/Holic-Jin/DCinVox/blob/main/docs/INSTALL.en.md)
- [日本語](https://github.com/Holic-Jin/DCinVox/blob/main/docs/INSTALL.ja.md)

## Downloads

| File | Platform |
|---|---|
| `DCinVox-macos-arm64.zip` | macOS 11+ (Apple Silicon) |
| `DCinVox-windows-x64.zip` | Windows 10/11 64-bit |

Unzip and keep the folder together. Do not move only the executable.

- **macOS:** `DCinVox.app` + `models/` beside it. If Gatekeeper blocks it: right-click → Open.
- **Windows:** `dcinvox.exe` + `ffmpeg.exe` + `models\` in the same folder.

## Models (required)

| File | Where to get it |
|---|---|
| `ggml-large-v3-q5_0.bin` | [ggerganov/whisper.cpp](https://huggingface.co/ggerganov/whisper.cpp) |
| `phoneme-aligner-vocab.json` | this repo, `models/` |
| `phoneme-aligner-int8.onnx` | convert with `tools/export_aligner.py` |

The program includes no video or audio. You are responsible for rights in files you import. See the EULA in `docs/`.
