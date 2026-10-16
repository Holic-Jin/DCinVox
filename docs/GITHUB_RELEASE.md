# DCinVox 1.1.0

Cut speech from your videos into syllables, stitch it into lyrics, and turn it into a song with pitch blocks. All processing stays on your computer.

**1.1.0** (use this zip): macOS Gatekeeper fix, `export_aligner.py`, `requirements.txt`, `INSTALL.md` in the zip, clearer model errors, **Windows from-scratch install steps** (Command Prompt, Python 3.13, SmartScreen, WebView2).

**1.0.2** zip is usable but its notes said `requirements.txt` while the file was named `aligner-requirements.txt`. Prefer 1.1.0.

This build is **not** Apple Developer ID signed. After unzipping on a Mac, **before** opening the app:

```
cd ~/Downloads/DCinVox-macos-arm64
xattr -cr DCinVox.app
```

The icon bouncing once is Gatekeeper, not an app crash.

On Windows the build is unsigned: if **Windows protected your PC** appears, click **More info → Run anyway**. Keep `ffmpeg.exe` next to `dcinvox.exe`. A blank window on Windows 10 usually means install [WebView2](https://developer.microsoft.com/microsoft-edge/webview2/) (Evergreen x64).

## Downloads

| File | Platform |
|---|---|
| `DCinVox-macos-arm64.zip` | macOS 11+ (Apple Silicon) |
| `DCinVox-windows-x64.zip` | Windows 10/11 64-bit |

Unzip and keep the folder together.

Full copy-paste guide (also in the zip as `INSTALL.md`):

- [한국어](https://github.com/Holic-Jin/DCinVox/blob/main/docs/INSTALL.md)
- [English](https://github.com/Holic-Jin/DCinVox/blob/main/docs/INSTALL.en.md)
- [日本語](https://github.com/Holic-Jin/DCinVox/blob/main/docs/INSTALL.ja.md)

## Models (required for Analyze)

The zip does **not** include Whisper or ONNX weights. It does include `models/phoneme-aligner-vocab.json`, `export_aligner.py`, `requirements.txt`, `aligner-requirements.txt`, and `INSTALL.md`.

| File | How |
|---|---|
| `ggml-large-v3-q5_0.bin` (~1 GB) | Direct file: [download](https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-q5_0.bin) → `models/` next to the app (not inside `.app`) |
| `phoneme-aligner-vocab.json` | already in this zip, `models/` |
| `phoneme-aligner-int8.onnx` | **not a download.** Convert once (Python **3.12 or 3.13**, not 3.14/3.15): |

macOS:

```
cd ~/Downloads/DCinVox-macos-arm64
python3.13 -m venv .venv
source .venv/bin/activate
python3 -m pip install -r requirements.txt
python3 export_aligner.py facebook/wav2vec2-xlsr-53-espeak-cv-ft ./models
```

Windows — use **Command Prompt** (`cmd`), not PowerShell. Install [Python 3.13](https://www.python.org/downloads/windows/) (64-bit), check **Add python.exe to PATH**, then open a **new** cmd window. Do not use Microsoft Store Python.

```
cd %USERPROFILE%\Downloads\DCinVox-windows-x64
py -3.13 -m venv .venv
.venv\Scripts\activate.bat
python -m pip install -r requirements.txt
python export_aligner.py facebook/wav2vec2-xlsr-53-espeak-cv-ft .\models
```

Do not `pip install` onto Homebrew Python (`externally-managed-environment`). Do not copy example words like `xyz`. Do not `pipx install torch`.

The program includes no video or audio. You are responsible for rights in files you import. See the EULA in the zip.
