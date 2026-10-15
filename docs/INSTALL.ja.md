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
2. macOS なら展開したあと**先に** `xattr -cr DCinVox.app`（しないとアイコンが跳ねて終了することがあります）。
3. アプリの**横**に `models/` を作り、モデルファイルを3つ入れます。
4. アプリを起動します。

これでインストールは終わりです。配布ビルドを使う場合、Node や開発ツールは不要です。

---

## A. 配布ビルドで入れる（いちばん簡単）

1. [GitHub Releases](https://github.com/Holic-Jin/DCinVox/releases) から最新版をダウンロードします。
   - Release がまだ無いときは、リポジトリの **Actions** にある `build` の成果物（`DCinVox-macos-arm64` / `DCinVox-windows`）を使います。成果物は一定期間で期限切れになります。
2. 展開し、**フォルダごと**同じ場所に置きます。実行ファイルだけ別の場所へ移さないでください（FFmpeg が隣にある必要があります）。
3. zip に**モデル重みは入っていません。** Whisper と ONNX は下の **モデルの入手** で別途用意します。zip の `models/` に入っているのは `phoneme-aligner-vocab.json` だけです。

### macOS の配置例

```
自分のフォルダ/
├─ DCinVox.app
└─ models/
   ├─ ggml-large-v3-q5_0.bin
   ├─ phoneme-aligner-int8.onnx
   └─ phoneme-aligner-vocab.json
```

この配布ビルドは Apple Developer ID で署名・公証されていません。Safari や Chrome でダウンロードすると `com.apple.quarantine` が付き、今の macOS では**警告が出ず、アイコンが一度跳ねてすぐ終了する**ことがあります。アプリのパニックではなく、Gatekeeper がプロセスを SIGKILL で止めたものです。

展開した**あと**、アプリを開く**前に**、ターミナルで隔離を外してください:

```
# 展開したフォルダで
xattr -cr DCinVox.app
```

そのあと Finder から開きます。以前のように「右クリック → 開く」だけでは警告が出ず、すぐ落ちることがあります。

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

アプリはモデルを自動ダウンロードしません。実行ファイル（または `.app`）の**横**の `models/` に置いてください。無くても起動直後のクラッシュ原因にはなりませんが、**解析ボタンは次の3つが揃わないと動きません。**

macOS では `DCinVox.app` と**同じフォルダ**の `models/` です。`.app` の中ではありません。

| ファイル | おおよその大きさ | 入手方法 |
|---|---|---|
| `ggml-large-v3-q5_0.bin` | 約 1 GB | 下の Whisper。**zip には無い** |
| `phoneme-aligner-vocab.json` | 小さい | 配布 zip の `models/` に入っている |
| `phoneme-aligner-int8.onnx` | 約 300 MB | 下の ONNX 変換。**zip には無い。`.onnx` として落とすファイルではない** |

### Whisper（`ggml-large-v3-q5_0.bin`）

1. この**直接リンク**で入手します（ページではなくファイル）:  
   https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-q5_0.bin
2. アプリの横の `models/ggml-large-v3-q5_0.bin` に置きます。名前は一字も違えてはいけません。
3. 大きさは**約 1GB**です。数MBなら HTML を保存しています。

### 音素アライナー ONNX（`phoneme-aligner-int8.onnx`）— 変換が必要

Hugging Face にこの名前の `.onnx` はありません。  
元は [facebook/wav2vec2-xlsr-53-espeak-cv-ft](https://huggingface.co/facebook/wav2vec2-xlsr-53-espeak-cv-ft)（Apache-2.0）で、配布 zip の `export_aligner.py`（リポジトリでは `tools/export_aligner.py`）が一度変換して `phoneme-aligner-int8.onnx` を作ります。解析の 2/3（発音合わせ）に必要です。

1. Python 3.10 以降を入れます。
2. ターミナル:

```
python3 -m pip install torch transformers onnx onnxruntime numpy huggingface_hub
```

3. zip を展開したフォルダ（またはリポジトリ直下）で、**出力をアプリ横の models に**します:

```
# macOS 例: アプリが ~/DCinVox/DCinVox.app のとき
python3 export_aligner.py facebook/wav2vec2-xlsr-53-espeak-cv-ft ~/DCinVox/models

# リポジトリで開発するとき
python3 tools/export_aligner.py facebook/wav2vec2-xlsr-53-espeak-cv-ft models
```

第1引数は手元のフォルダではなく**モデル ID そのままで**構いません。スクリプトが取得して変換します。初回は時間がかかります。

4. 終わると `models/phoneme-aligner-int8.onnx` ができます（数百 MB 程度）。これが無いと解析は始まりません。

詳しくは [models/README.ja.md](../models/README.ja.md) です。

---

## 最初の起動

1. 画面言語はパソコンの言語に従います。右上で **한국어 / English / 日本語** に切り替えられます。
2. 読み込む映像・音源の**著作権・肖像権は利用者の責任**です。なりすまし・虚偽情報・同意のない合成に使わないでください。[EULA.ja.md](EULA.ja.md)
3. 入れ方が正しいかは、ターミナルで確認できます（macOS は上で隔離を外してから）:

```
# macOS（アプリ内の実行ファイル）
DCinVox.app/Contents/MacOS/dcinvox --self-check

# Windows
dcinvox.exe --self-check
```

`ffmpeg: … OK` が出れば ffmpeg は準備できています。Whisper や ONNX をまだ入れていなければ `SELF-CHECK FAILED` と探索パスが出るのが正常です。コマンドが**何も出さずにすぐ終わる**、または終了コード 137 なら、アプリの不具合ではなく隔離が残っています。`xattr -cr DCinVox.app` をやり直してください。

---

## うまくいかないとき

| 症状 | 確認 |
|---|---|
| アイコンが一度跳ねてすぐ終了 / `--self-check` が即終了 | `xattr -cr DCinVox.app` で隔離を外す。右クリック → 開くだけでは足りないことが多い |
| モデルが無いと言われる | `models/` が `.app` / `exe` と**同じフォルダ**か。エラーに探したパスが出ます。ONNX は `export_aligner.py` で変換 |
| Whisper を入れたのに ggml が無いと言われる | `.app` の中ではなく**横**の `models/` か、サイズが約 1GB か。macOS は `xattr -cr` のあと展開フォルダから開き直す（一時コピーだと元の models が見えない） |
| ONNX が無いとだけ言われる | 正常です。zip にありません。上の変換をアプリ横 `models/` 向けに実行 |
| 動画が開けない / 変換できない | Windows は `ffmpeg.exe` が `dcinvox.exe` と同じフォルダか。macOS は `.app` をバラバラにしていないか |
| macOS でアプリが開かない | 隔離属性を削除（`xattr -cr`）。Developer ID 署名の無い配布です |
| 初回起動が遅い | 正常です。大きなモデルをディスクから読んでいます |
| メモリ不足 | 8GB RAM の機械では他のアプリを減らしてください |

閲覧・個人ビルドの条件は [`LICENSE`](../LICENSE) です。
