# かんたんインストール案内

[한국어](INSTALL.md) · [English](INSTALL.en.md) · [日本語](INSTALL.ja.md)

このプログラムは**動画をネットに上げません。** 解析は自分のパソコンで行います。

以下は**配布 zip** 向けです。ソースから動かす手順は末尾です。

---

## 用意するもの

- **macOS** 11 以降、Apple Silicon（M1・M2・M3…）推奨
- または **Windows** 10/11 64ビット
- RAM **8GB** 以上
- 空き容量 **約 3GB**
- インターネット（初回のモデル入手時）

zip に**大きな AI ファイルは入っていません。** ブラウザで1つ、変換で1つ用意します。

---

## macOS — この順で

展開したフォルダがダウンロードの `DCinVox-macos-arm64` である前提です。場所が違えば `cd` だけ変えてください。

### 1. アプリを入手

1. [GitHub Releases](https://github.com/Holic-Jin/DCinVox/releases) から**最新**の `DCinVox-macos-arm64.zip` をダウンロード（1.1.0 以降）。
2. 展開し、**フォルダごと**置きます。`DCinVox.app` だけ移さないでください。
3. `DCinVox.app`、`models/`、`export_aligner.py` が見えます。`export_aligner.py` が無ければ 1.0.1 以前です。最新 zip を取り直してください。

### 2. すぐ終了しないように（必須）

Safari・Chrome で受け取ると、macOS が警告なしでアプリを止めることがあります。アイコンが一度跳ねて消えるのは**アプリの故障ではありません。**

「ターミナル」を開き（Spotlight で「ターミナル」）:

```
cd ~/Downloads/DCinVox-macos-arm64
xattr -cr DCinVox.app
```

そのあと Finder で `DCinVox.app` を開きます。**右クリック → 開くだけでは足りないことが多いです。**

### 3. Whisper ファイルを入れる（約 1GB）

解析に必要です。zip にはありません。

1. この URL をブラウザに貼ります。ページではなく**ファイル**が保存されるはずです。  
   https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-q5_0.bin
2. アプリと**同じフォルダ**の `models/` へ移します。
3. 名前は必ず `ggml-large-v3-q5_0.bin` です。
4. 大きさは**約 1GB**です。数MBなら HTML を保存しています。やり直してください。

`DCinVox.app` の**中ではなく横**です。

### 4. `phoneme-aligner-int8.onnx` を作る

Hugging Face からこの名前の `.onnx` を落とすファイルでは**ありません。** zip の `export_aligner.py` が一度変換します。

**Python 3.12 または 3.13** が必要です。**3.15 は不可**です（torch に 3.15 用の配布がありません）。

Homebrew があるとき:

```
brew install python@3.13
```

無ければ [python.org](https://www.python.org/downloads/) から **3.13** を入れてください。

ターミナルでは、Homebrew の例文（`xyz` や `path/to/venv`）を**コピーせず**、次を実行します。

```
cd ~/Downloads/DCinVox-macos-arm64
python3.13 -m venv .venv
source .venv/bin/activate
```

先頭に `(.venv)` が出たら:

```
python3 -m pip install -r aligner-requirements.txt
python3 export_aligner.py facebook/wav2vec2-xlsr-53-espeak-cv-ft ./models
```

zip に `requirements.txt` しか無いときはその名前を使います。

初回はダウンロードのため時間がかかります。終わると `models/phoneme-aligner-int8.onnx`（数百 MB）ができます。

`pipx install torch` は使わないでください。Homebrew の Python に直接 `pip install` すると `externally-managed-environment` が出るのは正常です。だから **`.venv`** を使います。

### 5. 起動

Finder で `DCinVox.app` を開きます。解析には `models/` に次の3つが必要です。

- `ggml-large-v3-q5_0.bin`
- `phoneme-aligner-vocab.json`（zip に入っている）
- `phoneme-aligner-int8.onnx`

---

## Windows — この順で（初めてでも）

Python を使ったことがない人向けです。**コマンドプロンプト** (`cmd`) を使ってください。PowerShell は実行ポリシーで `Activate.ps1` が止まりやすいので、ここでは使いません。

展開フォルダがダウンロードの `DCinVox-windows-x64` である前提です。場所が違えば `cd` だけ変えてください。

### 1. アプリを入手

1. [GitHub Releases](https://github.com/Holic-Jin/DCinVox/releases) から**最新**の `DCinVox-windows-x64.zip` をダウンロード（1.1.0 以降）。
2. 展開し、**フォルダごと**置きます。`dcinvox.exe` だけデスクトップへ移さないでください。`ffmpeg.exe` が隣に無いと動画が開きません。
3. `dcinvox.exe`、`ffmpeg.exe`、`models\`、`export_aligner.py` が見えます。`export_aligner.py` が無ければ 1.0.1 以前です。最新 zip を取り直してください。

### 2. Windows に止められたとき（SmartScreen）

**Windows によって PC が保護されました** はウイルスではありません。署名の無いビルドだからです。

1. **詳細情報**（More info）を押します。
2. **実行**（Run anyway）を押します。

白い画面だけなら手順 6 の WebView2 を見てください。解析はモデル3つを入れてからです。

### 3. Whisper ファイルを入れる（約 1GB）

解析に必要です。zip にはありません。

1. この URL をブラウザに貼ります。ページではなく**ファイル**が保存されるはずです。  
   https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-q5_0.bin
2. `dcinvox.exe` と**同じフォルダ**の `models` へ移します。
3. 名前は必ず `ggml-large-v3-q5_0.bin` です。
4. 大きさは**約 1GB**です。数MBなら HTML を保存しています。やり直してください。

### 4. Python 3.13 を入れる（一度だけ）

`phoneme-aligner-int8.onnx` は Hugging Face からこの名前で落とすファイルでは**ありません。** zip の `export_aligner.py` が一度変換します。

**Python 3.12 または 3.13** が必要です。**3.14・3.15 は不可**です。Microsoft Store の「Python」ボタンは使わないでください。`python` と打つと Store が開くときは下の表の “Python was not found” を見てください。

1. https://www.python.org/downloads/windows/ から **3.13.x** の **Windows installer (64-bit)** だけ選びます。
2. 最初の画面の一番下 **Add python.exe to PATH** を**必ずオン**にします。
3. **Install Now**。管理者確認が出たら許可します。
4. すでに開いているコマンドプロンプトは**閉じて、新しい窓**を開きます（PATH が古い窓に残ることがあります）。

確認: スタートで `cmd` と検索し、**コマンドプロンプト**で:

```
py -3.13 --version
```

`Python 3.13.` が出れば成功です。`py` が無いときは PATH を入れていません。インストーラの **Modify → Add Python to PATH**、または削除して入れ直してください。

### 5. `phoneme-aligner-int8.onnx` を作る

**コマンドプロンプト**で**1行ずつ**実行します。例示の `xyz` や `path/to/venv` はコピーしないでください。PowerShell の `$env:...` や `Activate.ps1` と混ぜないでください。

```
cd %USERPROFILE%\Downloads\DCinVox-windows-x64
dir export_aligner.py
```

`export_aligner.py` が見える必要があります。無いときは、展開した本当のフォルダへ `cd` を直します（エクスプローラーのアドレスバーをクリックするとパスが出ます）。

続けて:

```
py -3.13 -m venv .venv
.venv\Scripts\activate.bat
```

先頭に `(.venv)` が出たら:

```
python -m pip install -r aligner-requirements.txt
python export_aligner.py facebook/wav2vec2-xlsr-53-espeak-cv-ft .\models
```

zip に `requirements.txt` しか無いときはその名前を使います。

初回はダウンロードのため時間がかかります。終わると `models\phoneme-aligner-int8.onnx`（数百 MB）ができます。

`pipx install torch` は使わないでください。Store の Python や 3.15 で作った `.venv` は消して、3.13 で**新しく**作ってください。古い venv はバージョンが変わりません。

### 6. 起動

`dcinvox.exe` を開きます。解析には `models\` に次の3つが必要です。

- `ggml-large-v3-q5_0.bin`
- `phoneme-aligner-vocab.json`（zip に入っている）
- `phoneme-aligner-int8.onnx`

窓が**真っ白、またはすぐ閉じる**ときは、Windows 10 では [WebView2 ランタイム](https://developer.microsoft.com/microsoft-edge/webview2/)（Evergreen x64）が必要なことがあります。入れてから開き直してください。

---

## できたフォルダ

macOS:

```
DCinVox-macos-arm64/
├─ DCinVox.app
├─ export_aligner.py
├─ aligner-requirements.txt   （または requirements.txt）
├─ models/
│  ├─ ggml-large-v3-q5_0.bin
│  ├─ phoneme-aligner-int8.onnx
│  └─ phoneme-aligner-vocab.json
└─ .venv/                     （変換用）
```

Windows:

```
DCinVox-windows-x64/
├─ dcinvox.exe
├─ ffmpeg.exe
├─ export_aligner.py
├─ aligner-requirements.txt   （または requirements.txt）
├─ models\
│  ├─ ggml-large-v3-q5_0.bin
│  ├─ phoneme-aligner-int8.onnx
│  └─ phoneme-aligner-vocab.json
└─ .venv\                     （変換用）
```

---

## 確認

展開フォルダで（macOS は `xattr` のあと）:

```
# macOS
DCinVox.app/Contents/MacOS/dcinvox --self-check

# Windows
dcinvox.exe --self-check
```

`ffmpeg: … OK` が出れば ffmpeg は準備できています。モデル3つがまだなら `SELF-CHECK FAILED` は正常です。**何も出さずに終わる**ときは、macOS は `xattr -cr DCinVox.app` をやり直します。Windows では **コマンドプロンプト**で `dcinvox.exe` があるフォルダへ `cd` してください。

読み込むファイルの権利は利用者の責任です。[EULA.ja.md](EULA.ja.md)

---

## うまくいかないとき

| 表示 | 意味 |
|---|---|
| アイコンが跳ねて終了 | `xattr -cr DCinVox.app` のあと、展開フォルダのアプリを開き直す |
| `externally-managed-environment` | Homebrew の Python に直接 pip。`.venv` を作って有効化してから `python3 -m pip` |
| `xyz` が見つからない | 説明の例示 `xyz` をそのまま打った。`aligner-requirements.txt` を入れる |
| `No matching distribution found for torch` / `cp315` | Python **3.15**。3.13 で**新しい** `.venv` を作る。古い venv はバージョンが変わらない |
| `numpy` が無い | pip が失敗したあとスクリプトを実行した。先に torch/venv を直す |
| `pipx` で torch が失敗 | pipx は用途が違う。`.venv` + `pip install -r aligner-requirements.txt` |
| ggml が無い | アプリ**横**の `models/` か、名前と約 1GB か |
| onnx だけ無い | zip に無い。macOS 手順 4 / Windows 手順 5 を `models` 向けに実行 |
| 変換が長い | 初回は Hugging Face から取得します。終わるまで待つ |
| Windows によって PC が保護されました / SmartScreen | **詳細情報 → 実行**。未署名のビルドだからです |
| `Python was not found` / Store が開く | Store の偽 python です。python.org の **3.13** を **Add to PATH** 付きで入れる。設定 → アプリ実行エイリアスで `python.exe` をオフにしてもよい |
| `py` が見つからない | インストール時に PATH を入れていません。Modify するか入れ直し。**新しい**コマンドプロンプトを開く |
| PowerShell で `Activate.ps1` が実行できない | **コマンドプロンプト**と `.venv\Scripts\activate.bat` を使う |
| `%USERPROFILE%` が展開されない | PowerShell に cmd の変数を書いた。cmd を使うか、PowerShell なら `cd $env:USERPROFILE\Downloads\DCinVox-windows-x64` |
| 白い画面 / 起動しない | [WebView2](https://developer.microsoft.com/microsoft-edge/webview2/) Evergreen x64 を入れる |
| Windows で ffmpeg / 動画エラー | `dcinvox.exe` だけ移した。`ffmpeg.exe` を**同じフォルダ**に置く |

---

## ソースから（開発）

```
git clone https://github.com/Holic-Jin/DCinVox.git
cd DCinVox
# models/ にモデル3つを置いてから
cd src-tauri
cargo run
```

Rust（stable）と `ffmpeg` / `ffprobe` が必要です。公開手順は [RELEASE.ja.md](RELEASE.ja.md)。

閲覧・個人ビルド: [`LICENSE`](../LICENSE)
