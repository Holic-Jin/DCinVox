# Easy install guide

[한국어](INSTALL.md) · [English](INSTALL.en.md) · [日本語](INSTALL.ja.md)

The app **does not upload your video**. Analysis and generation run on your computer. This repository includes no video or audio, and the large AI weights are not in git.

## Requirements

- **macOS** 11 or later (Apple Silicon recommended)
- **Windows** 10 or 11, 64-bit
- **8 GB** RAM or more
- About **2 GB** free disk (app + models)
- Time depends on the machine (about 40 minutes to analyze a 1-hour video on M3)

## In short

1. Get the app (A or B below).
2. Create a `models/` folder **next to** the app and put three model files in it.
3. Launch the app.

That is the whole install. You do not need Node or a compiler if you use a prebuilt build.

---

## A. Prebuilt app (easiest)

1. Download the latest build from [GitHub Releases](https://github.com/Holic-Jin/DCinVox/releases).
   - If there is no Release yet, use the `build` workflow artifacts (`DCinVox-macos` / `DCinVox-windows`) on the repo **Actions** tab. Artifacts expire after a while.
2. Unzip and keep **the whole folder** together. Do not move only the executable (FFmpeg must sit beside it).

### macOS layout

```
My folder/
├─ DCinVox.app
└─ models/
   ├─ ggml-large-v3-q5_0.bin
   ├─ phoneme-aligner-int8.onnx
   └─ phoneme-aligner-vocab.json
```

If macOS says the developer cannot be verified:

- In Finder, **right-click `DCinVox.app` → Open**, or
- Terminal: `xattr -d com.apple.quarantine DCinVox.app`

### Windows layout

```
My folder\
├─ dcinvox.exe
├─ ffmpeg.exe
└─ models\
   ├─ ggml-large-v3-q5_0.bin
   ├─ phoneme-aligner-int8.onnx
   └─ phoneme-aligner-vocab.json
```

If SmartScreen appears, choose **More info → Run anyway**. Windows 10 may need WebView2.

---

## B. Run from source (development)

Use this only when you have no prebuilt app.

1. Install [Rust](https://rustup.rs/) (stable).
2. Put a development FFmpeg on `PATH` (`ffmpeg`, `ffprobe`).
3. Clone the repo and prepare models (see **Get the models**).
4. Run:

```
git clone https://github.com/Holic-Jin/DCinVox.git
cd DCinVox
# put the three model files in models/, then
cd src-tauri
cargo run
```

macOS release build:

```
tools/build_ffmpeg_macos.sh
cd src-tauri && cargo tauri build --bundles app
```

Windows release builds use `.github/workflows/build.yml`. Shipping details: [RELEASE.en.md](RELEASE.en.md).

---

## Get the models (required for both)

The app never downloads weights. Put them in `models/` **next to** the executable (or `.app`).

| File | Rough size | How to get it |
|---|---|---|
| `ggml-large-v3-q5_0.bin` | ~1 GB | Download from [ggerganov/whisper.cpp](https://huggingface.co/ggerganov/whisper.cpp) (MIT) |
| `phoneme-aligner-vocab.json` | small | Already in this repo under `models/` — copy it |
| `phoneme-aligner-int8.onnx` | the rest | Convert the Hugging Face model once (Apache-2.0) |

Whisper example (repo root or next to the app):

```
mkdir -p models
# download in a browser, or:
# https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-q5_0.bin
```

Phoneme ONNX (Python, once):

```
python tools/export_aligner.py /path/to/wav2vec2-xlsr-53-espeak-cv-ft models
```

Source model: [facebook/wav2vec2-xlsr-53-espeak-cv-ft](https://huggingface.co/facebook/wav2vec2-xlsr-53-espeak-cv-ft)

More detail: [models/README.md](../models/README.md).

---

## First launch

1. The UI follows the OS language. Switch **한국어 / English / 日本語** from the top right.
2. You are responsible for copyright and portrait rights in files you import. Do not use the app for impersonation, disinformation, or non-consensual synthesis. [EULA.en.md](EULA.en.md)
3. To verify the install, in a terminal:

```
# macOS (binary inside the app)
DCinVox.app/Contents/MacOS/dcinvox --self-check

# Windows
dcinvox.exe --self-check
```

You want `ffmpeg: … OK` and valid model paths.

---

## If something fails

| Symptom | Check |
|---|---|
| Missing model | `models/` is **beside** the app, and file names match the table **exactly** |
| Video will not open / convert | On Windows, `ffmpeg.exe` is next to `dcinvox.exe`. On macOS, keep the `.app` intact |
| macOS will not open the app | Right-click → Open, or remove the quarantine attribute |
| First launch is slow | Normal — it is reading a large model from disk |
| Out of memory | On 8 GB machines, close other programs |

Viewing and personal builds: [`LICENSE`](../LICENSE).
