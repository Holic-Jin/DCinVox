# DCinVox — 第三者ソフトウェア表示

[한국어·English](THIRD_PARTY_NOTICES.md) · [日本語](THIRD_PARTY_NOTICES.ja.md)

DCinVox © HolicJin。DCinVox には次の第三者ソフトウェアとモデルが含まれます。Rust crate の一覧（452）は [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) と同じです。各ライセンス本文は https://spdx.org/licenses/ で読めます。

## 同梱コンポーネント

| 構成要素 | 用途 | ライセンス | 出典 |
|---|---|---|---|
| FFmpeg 9.0.2（LGPL ビルド、別実行ファイル） | 映像・音声の読み書き | LGPL-2.1-or-later | https://ffmpeg.org — ソースとビルドスクリプト（`tools/build_ffmpeg_macos.sh`）は請求があれば提供 |
| whisper.cpp / ggml | 音声認識エンジン | MIT | https://github.com/ggml-org/whisper.cpp |
| Whisper large-v3（ggml q5_0 変換） | 音声認識モデル | MIT (OpenAI) | https://huggingface.co/ggerganov/whisper.cpp |
| wav2vec2-xlsr-53-espeak-cv-ft（ONNX int8 変換） | 音素アライメント | Apache-2.0 (Meta AI) | https://huggingface.co/facebook/wav2vec2-xlsr-53-espeak-cv-ft |
| ONNX Runtime | アライメントモデルの実行 | MIT (Microsoft) | https://github.com/microsoft/onnxruntime |
| WORLD vocoder | ピッチ変換 | Modified BSD (M. Morise) | https://github.com/mmorise/World |
| SQLite | 索引の保存 | Public domain | https://sqlite.org |
| Tauri | アプリ枠 | MIT / Apache-2.0 | https://tauri.app |

### FFmpeg（LGPL）について

DCinVox は FFmpeg を別実行ファイル（`ffmpeg`）として含め、GPL・nonfree 構成要素なしでビルドしています。この実行ファイルは自分でビルドした FFmpeg に差し替えられます。該当バージョンのソースコードとビルド設定は請求があれば提供します。
