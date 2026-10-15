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

The zip includes `models/phoneme-aligner-vocab.json` and `export_aligner.py`. Whisper and the ONNX weights are not shipped. Analyze needs all three model files next to the app.

| File | Where to get it |
|---|---|
| `ggml-large-v3-q5_0.bin` | direct file: [ggml-large-v3-q5_0.bin](https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-q5_0.bin) (~1 GB) |
| `phoneme-aligner-vocab.json` | this zip, `models/` |
| `phoneme-aligner-int8.onnx` | **not a download.** From the unzipped folder: `python3 export_aligner.py facebook/wav2vec2-xlsr-53-espeak-cv-ft ./models` |

The program includes no video or audio. You are responsible for rights in files you import. See the EULA in `docs/`.
