# かんたんインストール案内

[한국어](INSTALL.md) · [English](INSTALL.en.md) · [日本語](INSTALL.ja.md)

このプログラムは**動画をネットに上げません。** 解析と生成はすべて自分のパソコンで行います。リポジトリに映像・音声素材はなく、大きな AI モデルも git にはありません。

## 必要な環境

- **macOS** 11 以降（Apple Silicon 推奨）
- **Windows** 10 または 11、64ビット
- RAM **8GB** 以上
- 空き容量 **約 2GB**（アプリ + モデル）
- 処理時間はパソコンによります（1時間の動画解析は M3 で約40分）

## 手順の要約

1. アプリを入手します（下の A または B）。
2. アプリの**横**に `models/` を作り、モデルファイルを3つ入れます。
3. アプリを起動します。

これでインストールは終わりです。配布ビルドを使う場合、Node や開発ツールは不要です。

---

## A. 配布ビルドで入れる（いちばん簡単）

1. [GitHub Releases](https://github.com/Holic-Jin/DCinVox/releases) から最新版をダウンロードします。
   - Release がまだ無いときは、リポジトリの **Actions** にある `build` の成果物（`DCinVox-macos` / `DCinVox-windows`）を使います。成果物は一定期間で期限切れになります。
2. 展開し、**フォルダごと**同じ場所に置きます。実行ファイルだけ別の場所へ移さないでください（FFmpeg が隣にある必要があります）。

### macOS の配置例

```
自分のフォルダ/
├─ DCinVox.app
└─ models/
   ├─ ggml-large-v3-q5_0.bin
   ├─ phoneme-aligner-int8.onnx
   └─ phoneme-aligner-vocab.json
```

「開発元を確認できない」と出るときは:

- Finder で `DCinVox.app` を **右クリック → 開く**、または
- ターミナル: `xattr -d com.apple.quarantine DCinVox.app`

### Windows の配置例

```
自分のフォルダ\
├─ dcinvox.exe
├─ ffmpeg.exe
└─ models\
   ├─ ggml-large-v3-q5_0.bin
   ├─ phoneme-aligner-int8.onnx
   └─ phoneme-aligner-vocab.json
```

SmartScreen が出たら **詳細情報 → 実行** を選びます。Windows 10 では WebView2 が必要なことがあります。

---

## B. ソースから動かす（開発用）

配布ビルドが無いときだけこの手順です。

1. [Rust](https://rustup.rs/)（stable）を入れます。
2. 開発用 FFmpeg を `PATH` に置きます（`ffmpeg`、`ffprobe`）。
3. リポジトリをクローンし、モデルを用意します（下の **モデルの入手**）。
4. 実行:

```
git clone https://github.com/Holic-Jin/DCinVox.git
cd DCinVox
# models/ にモデル3つを置いてから
cd src-tauri
cargo run
```

macOS の配布ビルド:

```
tools/build_ffmpeg_macos.sh
cd src-tauri && cargo tauri build --bundles app
```

Windows の配布ビルドは `.github/workflows/build.yml` を使います。公開手順の詳細は [RELEASE.ja.md](RELEASE.ja.md) です。

---

## モデルの入手（どちらでも必須）

アプリはモデルを自動ダウンロードしません。実行ファイル（または `.app`）の**横**の `models/` に置いてください。

| ファイル | おおよその大きさ | 入手方法 |
|---|---|---|
| `ggml-large-v3-q5_0.bin` | 約 1 GB | [ggerganov/whisper.cpp](https://huggingface.co/ggerganov/whisper.cpp) からダウンロード（MIT） |
| `phoneme-aligner-vocab.json` | 小さい | このリポジトリの `models/` にあるのでコピー |
| `phoneme-aligner-int8.onnx` | 残り | Hugging Face のモデルを一度変換（Apache-2.0） |

Whisper の例（リポジトリ直下、またはアプリの横）:

```
mkdir -p models
# ブラウザで入手するか:
# https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-q5_0.bin
```

音素アライナーの ONNX（Python、一度だけ）:

```
python tools/export_aligner.py /path/to/wav2vec2-xlsr-53-espeak-cv-ft models
```

元モデル: [facebook/wav2vec2-xlsr-53-espeak-cv-ft](https://huggingface.co/facebook/wav2vec2-xlsr-53-espeak-cv-ft)

詳しくは [models/README.ja.md](../models/README.ja.md) です。

---

## 最初の起動

1. 画面言語はパソコンの言語に従います。右上で **한국어 / English / 日本語** に切り替えられます。
2. 読み込む映像・音源の**著作権・肖像権は利用者の責任**です。なりすまし・虚偽情報・同意のない合成に使わないでください。[EULA.ja.md](EULA.ja.md)
3. 入れ方が正しいかは、ターミナルで確認できます:

```
# macOS（アプリ内の実行ファイル）
DCinVox.app/Contents/MacOS/dcinvox --self-check

# Windows
dcinvox.exe --self-check
```

`ffmpeg: … OK` とモデルのパスが出れば準備完了です。

---

## うまくいかないとき

| 症状 | 確認 |
|---|---|
| モデルが無いと言われる | `models/` がアプリの**横**か、ファイル名が上の表と**完全に同じ**か |
| 動画が開けない / 変換できない | Windows は `ffmpeg.exe` が `dcinvox.exe` と同じフォルダか。macOS は `.app` をバラバラにしていないか |
| macOS でアプリが開かない | 右クリック → 開く、または隔離属性を削除 |
| 初回起動が遅い | 正常です。大きなモデルをディスクから読んでいます |
| メモリ不足 | 8GB RAM の機械では他のアプリを減らしてください |

閲覧・個人ビルドの条件は [`LICENSE`](../LICENSE) です。
