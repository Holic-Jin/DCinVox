# Easy install guide

[한국어](INSTALL.md) · [English](INSTALL.en.md) · [日本語](INSTALL.ja.md)

The app **does not upload your video**. Analysis runs on your computer.

These steps are for the **release zip**. Running from source is at the bottom.

---

## What you need

- **macOS** 11 or later, Apple Silicon (M1/M2/M3…) recommended
- or **Windows** 10/11 64-bit
- **8 GB** RAM or more
- About **3 GB** free disk
- Internet (only to fetch models the first time)

The zip does **not** include the large AI files. You download one in a browser and convert one with a script.

---

## macOS — follow in order

These commands assume the unzipped folder is `DCinVox-macos-arm64` in Downloads. Change the `cd` path if you moved it.

### 1. Get the app

1. Download the **latest** `DCinVox-macos-arm64.zip` from [GitHub Releases](https://github.com/Holic-Jin/DCinVox/releases) (1.1.0 or newer).
2. Unzip and keep **the whole folder**. Do not move only `DCinVox.app`.
3. You should see `DCinVox.app`, `models/`, and `export_aligner.py`. If `export_aligner.py` is missing, you have 1.0.1 or older — download the latest zip.

### 2. Stop the app from quitting immediately (required)

Safari/Chrome mark the download, and macOS may kill the app with no dialog. The icon bouncing once is **not a crash in the program**.

Open **Terminal** (Spotlight: “Terminal”), then:

```
cd ~/Downloads/DCinVox-macos-arm64
xattr -cr DCinVox.app
```

Then open `DCinVox.app` in Finder. **Right-click → Open is often not enough.**

### 3. Put the Whisper file in (~1 GB)

Needed for Analyze. Not in the zip.

1. Paste this URL in a browser. A **file** should download, not a web page:  
   https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-q5_0.bin
2. Move it into `models/` in the **same folder as the app**.
3. The name must be exactly `ggml-large-v3-q5_0.bin`.
4. Size must be **about 1 GB**. A few MB means you saved the HTML page — download again.

Not inside `DCinVox.app`. Next to it.

### 4. Make `phoneme-aligner-int8.onnx`

This is **not** an `.onnx` you download from Hugging Face. `export_aligner.py` in the zip converts it once.

You need **Python 3.12 or 3.13**. **Not 3.15.** (torch has no 3.15 install files.)

With Homebrew:

```
brew install python@3.13
```

Otherwise install **3.13** from [python.org](https://www.python.org/downloads/).

In Terminal, run **these lines**, not the Homebrew example text (`xyz`, `path/to/venv`):

```
cd ~/Downloads/DCinVox-macos-arm64
python3.13 -m venv .venv
source .venv/bin/activate
```

When the prompt starts with `(.venv)`:

```
python3 -m pip install -r aligner-requirements.txt
python3 export_aligner.py facebook/wav2vec2-xlsr-53-espeak-cv-ft ./models
```

If the zip only has `requirements.txt`, use that filename.

The first convert takes a while (it downloads). You should get `models/phoneme-aligner-int8.onnx` (a few hundred MB).

Do not use `pipx install torch`. `externally-managed-environment` on Homebrew Python is expected — that is why you use **`.venv`**.

### 5. Open the app

Open `DCinVox.app` in Finder. Analyze needs all three files in `models/`:

- `ggml-large-v3-q5_0.bin`
- `phoneme-aligner-vocab.json` (already in the zip)
- `phoneme-aligner-int8.onnx`

---

## Windows — follow in order (from scratch)

These steps assume you have **never used Python**. Use **Command Prompt** (`cmd`). Do not use PowerShell here — script policy often blocks `Activate.ps1`.

Commands assume the unzipped folder is `DCinVox-windows-x64` in Downloads. Change the `cd` path if you moved it.

### 1. Get the app

1. Download the **latest** `DCinVox-windows-x64.zip` from [GitHub Releases](https://github.com/Holic-Jin/DCinVox/releases) (1.1.0 or newer).
2. Unzip and keep **the whole folder**. Do not move only `dcinvox.exe` to the desktop. `ffmpeg.exe` must stay next to it or videos will not open.
3. You should see `dcinvox.exe`, `ffmpeg.exe`, `models\`, and `export_aligner.py`. If `export_aligner.py` is missing, you have 1.0.1 or older — download the latest zip.

### 2. If Windows blocks the app (SmartScreen)

**Windows protected your PC** is **not** a virus warning. The build is unsigned.

1. Click **More info**.
2. Click **Run anyway**.

A blank white window means skip to step 6 (WebView2). Analyze needs the three model files first.

### 3. Put the Whisper file in (~1 GB)

Needed for Analyze. Not in the zip.

1. Paste this URL in a browser. A **file** should download, not a web page:  
   https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-q5_0.bin
2. Move it into `models` in the **same folder as** `dcinvox.exe`.
3. The name must be exactly `ggml-large-v3-q5_0.bin`.
4. Size must be **about 1 GB**. A few MB means you saved the HTML page — download again.

### 4. Install Python 3.13 (once)

`phoneme-aligner-int8.onnx` is **not** an `.onnx` you download from Hugging Face. `export_aligner.py` in the zip converts it once.

You need **Python 3.12 or 3.13**. **Not 3.14 or 3.15.** Do not use the Microsoft Store “Python” button. If Store opens when you type `python`, see “Python was not found” in the table below.

1. On https://www.python.org/downloads/windows/ download **Windows installer (64-bit)** for **3.13.x** only.
2. On the first installer screen, check **Add python.exe to PATH** (bottom of the window).
3. Click **Install Now**. Allow the admin prompt if it appears.
4. **Close any old Command Prompt windows and open a new one** so PATH is picked up.

Check: Start menu → search `cmd` → **Command Prompt**:

```
py -3.13 --version
```

You want `Python 3.13.`. If `py` is not found, PATH was not enabled — rerun the installer with **Modify → Add Python to PATH**, or uninstall and install again.

### 5. Make `phoneme-aligner-int8.onnx`

In **Command Prompt**, run **these lines**. Do not copy example words like `xyz` or `path/to/venv`. Do not mix PowerShell (`$env:...`, `Activate.ps1`) with these commands.

```
cd %USERPROFILE%\Downloads\DCinVox-windows-x64
dir export_aligner.py
```

You must see `export_aligner.py`. If not, `cd` to the real unzipped folder (click the Explorer address bar to copy the path).

Then:

```
py -3.13 -m venv .venv
.venv\Scripts\activate.bat
```

When the prompt starts with `(.venv)`:

```
python -m pip install -r aligner-requirements.txt
python export_aligner.py facebook/wav2vec2-xlsr-53-espeak-cv-ft .\models
```

If the zip only has `requirements.txt`, use that filename.

The first convert takes a while (it downloads). You should get `models\phoneme-aligner-int8.onnx` (a few hundred MB).

Do not use `pipx install torch`. If the venv was made with Store Python or 3.15, delete it and make a **new** `.venv` with 3.13. An old venv keeps its version.

### 6. Open the app

Open `dcinvox.exe`. Analyze needs all three files in `models\`:

- `ggml-large-v3-q5_0.bin`
- `phoneme-aligner-vocab.json` (already in the zip)
- `phoneme-aligner-int8.onnx`

If the window is **blank or closes immediately**, Windows 10 may need the [WebView2 runtime](https://developer.microsoft.com/microsoft-edge/webview2/) (Evergreen x64). Install it and open the app again.

---

## Finished folder

macOS:

```
DCinVox-macos-arm64/
├─ DCinVox.app
├─ export_aligner.py
├─ aligner-requirements.txt   (or requirements.txt)
├─ models/
│  ├─ ggml-large-v3-q5_0.bin
│  ├─ phoneme-aligner-int8.onnx
│  └─ phoneme-aligner-vocab.json
└─ .venv/                     (only for the converter)
```

Windows:

```
DCinVox-windows-x64/
├─ dcinvox.exe
├─ ffmpeg.exe
├─ export_aligner.py
├─ aligner-requirements.txt   (or requirements.txt)
├─ models\
│  ├─ ggml-large-v3-q5_0.bin
│  ├─ phoneme-aligner-int8.onnx
│  └─ phoneme-aligner-vocab.json
└─ .venv\                     (only for the converter)
```

---

## Check

From the unzipped folder (after `xattr` on macOS):

```
# macOS
DCinVox.app/Contents/MacOS/dcinvox --self-check

# Windows
dcinvox.exe --self-check
```

You want `ffmpeg: … OK`. `SELF-CHECK FAILED` is normal until all three models are in place. If the command **dies with no text** on macOS, run `xattr -cr DCinVox.app` again. On Windows, `cd` to the folder that contains `dcinvox.exe` in **Command Prompt**.

You are responsible for rights in files you import. [EULA.en.md](EULA.en.md)

---

## If something fails

| What you see | What it means |
|---|---|
| Icon bounces and quits | `xattr -cr DCinVox.app`, then open the app from the unzipped folder |
| `externally-managed-environment` | pip on Homebrew Python. Create `.venv`, activate it, then `python3 -m pip` |
| Cannot find package `xyz` | You copied the example word `xyz`. Install `-r aligner-requirements.txt` instead |
| `No matching distribution found for torch` / `cp315` | That Python is **3.15**. Make a **new** `.venv` with 3.13. An old venv keeps its version |
| `numpy` module not found | pip did not finish. Fix torch/venv first |
| `pipx` cannot install torch | pipx is the wrong tool. Use `.venv` + `pip install -r aligner-requirements.txt` |
| Missing ggml | File is in `models/` **beside** the app, name exact, size ~1 GB |
| Missing onnx only | Not in the zip. Run macOS step 4 / Windows step 5 with output `models` |
| Convert takes a long time | First run downloads the Hugging Face model. Let it finish |
| Windows protected your PC / SmartScreen | **More info → Run anyway**. The build is unsigned |
| `Python was not found` / Store opens | Store stub, not real Python. Install python.org **3.13** with **Add to PATH**. You can also turn off `python.exe` under Settings → App execution aliases |
| `py` is not recognized | PATH was not checked at install. Modify the installer or reinstall. Open a **new** Command Prompt |
| PowerShell cannot run `Activate.ps1` | Use **Command Prompt** and `.venv\Scripts\activate.bat` |
| `%USERPROFILE%` is not expanded | That is a cmd variable in PowerShell. Use cmd, or in PowerShell `cd $env:USERPROFILE\Downloads\DCinVox-windows-x64` |
| Blank window / app does not open | Install [WebView2](https://developer.microsoft.com/microsoft-edge/webview2/) Evergreen x64 |
| ffmpeg / video error on Windows | You moved only `dcinvox.exe`. Keep `ffmpeg.exe` in the **same folder** |

---

## From source (development)

```
git clone https://github.com/Holic-Jin/DCinVox.git
cd DCinVox
# put the three model files in models/, then
cd src-tauri
cargo run
```

Needs Rust (stable) and `ffmpeg`/`ffprobe` on `PATH`. Shipping: [RELEASE.en.md](RELEASE.en.md).

Viewing and personal builds: [`LICENSE`](../LICENSE)
