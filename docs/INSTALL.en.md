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
2. On macOS, after unzipping, run `xattr -cr DCinVox.app` **first** (otherwise the icon may bounce and quit).
3. Create a `models/` folder **next to** the app and put three model files in it.
4. Launch the app.

That is the whole install. You do not need Node or a compiler if you use a prebuilt app.

---

## A. Prebuilt app (easiest)

1. Download the latest build from [GitHub Releases](https://github.com/Holic-Jin/DCinVox/releases).
   - If there is no Release yet, use the `build` workflow artifacts (`DCinVox-macos-arm64` / `DCinVox-windows`) on the repo **Actions** tab. Artifacts expire after a while.
2. Unzip and keep **the whole folder** together. Do not move only the executable (FFmpeg must sit beside it).
3. The zip **does not include model weights.** Get Whisper and the ONNX file under **Get the models** below. Only `phoneme-aligner-vocab.json` ships in the zip under `models/`.

### macOS layout

```
My folder/
├─ DCinVox.app
└─ models/
   ├─ ggml-large-v3-q5_0.bin
   ├─ phoneme-aligner-int8.onnx
   └─ phoneme-aligner-vocab.json
```

This build is not signed or notarized with an Apple Developer ID. Downloads from Safari or Chrome get `com.apple.quarantine`, and on current macOS the app often **bounces once and quits with no warning**. That is Gatekeeper sending SIGKILL, not an app panic.

After unzipping, **before** you open the app, remove quarantine in Terminal:

```
# in the unzipped folder
xattr -cr DCinVox.app
```

Then open the app in Finder. **Right-click → Open** often does nothing on current macOS; the app just dies.

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

The app never downloads weights. Put them in `models/` **next to** the executable (or `.app`). Missing models do not crash at launch; they fail later during analysis or alignment.

| File | Rough size | How to get it |
|---|---|---|
| `ggml-large-v3-q5_0.bin` | ~1 GB | Download from [ggerganov/whisper.cpp](https://huggingface.co/ggerganov/whisper.cpp) (MIT). **Not in the zip** |
| `phoneme-aligner-vocab.json` | small | In the release zip under `models/`, or copy from this repo |
| `phoneme-aligner-int8.onnx` | the rest | Convert the Hugging Face model once (Apache-2.0). **Not in the zip — you convert it** |

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
3. To verify the install, in a terminal (on macOS, after removing quarantine):

```
# macOS (binary inside the app)
DCinVox.app/Contents/MacOS/dcinvox --self-check

# Windows
dcinvox.exe --self-check
```

You want `ffmpeg: … OK`. If Whisper or ONNX are not in place yet, `SELF-CHECK FAILED` plus the search paths is expected. If the command **dies with no output** or exit code 137, quarantine is still on — not an app bug. Run `xattr -cr DCinVox.app` again.

---

## If something fails

| Symptom | Check |
|---|---|
| Icon bounces once then quits / `--self-check` dies immediately | Remove quarantine with `xattr -cr DCinVox.app`. Right-click → Open is often not enough |
| Missing model | `models/` is **beside** the app, and file names match the table **exactly**. ONNX is not in the zip; convert it with `export_aligner.py` |
| Video will not open / convert | On Windows, `ffmpeg.exe` is next to `dcinvox.exe`. On macOS, keep the `.app` intact |
| macOS will not open the app | Remove quarantine (`xattr -cr`). This build has no Developer ID signature |
| First launch is slow | Normal — it is reading a large model from disk |
| Out of memory | On 8 GB machines, close other programs |

Viewing and personal builds: [`LICENSE`](../LICENSE).
