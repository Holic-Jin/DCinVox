# DCinVox 1.0.1

Cut speech from your videos into syllables, stitch it into lyrics, and turn it into a song with pitch blocks. All processing stays on your computer.

**1.0.1:** macOS zip is now packed on a Mac (`ditto` + adhoc bundle signature). v1.0.0 could quit immediately after download (Gatekeeper SIGKILL). After unzipping, still run `xattr -cr DCinVox.app` before opening — this build is not Developer ID signed.

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

- **macOS:** `DCinVox.app` + `models/` beside it. This zip is **not** Developer ID signed. After unzipping, run `xattr -cr DCinVox.app` **before** opening. On current macOS the icon often bounces once and quits with no dialog (Gatekeeper SIGKILL). Right-click → Open is usually not enough.
- **Windows:** `dcinvox.exe` + `ffmpeg.exe` + `models\` in the same folder.

## Models (required)

The zip includes `models/phoneme-aligner-vocab.json` only. Whisper and ONNX are not shipped.

| File | Where to get it |
|---|---|
| `ggml-large-v3-q5_0.bin` | [ggerganov/whisper.cpp](https://huggingface.co/ggerganov/whisper.cpp) |
| `phoneme-aligner-vocab.json` | this zip, `models/` |
| `phoneme-aligner-int8.onnx` | convert with `tools/export_aligner.py` (not a download) |

The program includes no video or audio. You are responsible for rights in files you import. See the EULA in `docs/`.
