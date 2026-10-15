# DCinVox

[한국어](README.ko.md) · [English](README.md) · [日本語](README.ja.md)

動画の声を音節ごとに切り出して歌詞につなぎ、音階ブロックで歌にするデスクトップアプリ (Tauri 2 + Rust)。© HolicJin

このリポジトリには**映像・音声素材は含まれません。** 読み込んだファイルと出力の著作権・肖像権は利用者の責任です。なりすまし・虚偽情報・同意のない合成に使わないでください。詳しくは [`docs/EULA.ja.md`](docs/EULA.ja.md) を参照してください。

## 必要なもの

- Rust (stable)
- 開発用 FFmpeg (`PATH` 上の `ffmpeg` / `ffprobe`)
- `models/` の重み — git には入れません。入手方法: [`models/README.ja.md`](models/README.ja.md)

## 実行（開発）

```
cd src-tauri
cargo run
```

- UI（`ui/`）はビルドツールなしの HTML/CSS/JS — Node 不要
- 配布時は `tools/build_ffmpeg_*.sh` で作った LGPL FFmpeg を実行ファイルの横に置きます

## テスト

```
cd src-tauri
cargo test
```

## 配布ビルド

```
tools/build_ffmpeg_macos.sh          # 一度だけ (LGPL ffmpeg → src-tauri/binaries/)
cd src-tauri && cargo tauri build --bundles app
```

Windows は `.github/workflows/build.yml`（タグ `v*` または手動実行）。成果物は Actions のアーティファクトです。モデルはアプリの**横**の `models/` に置きます。

かんたんインストール（配布ビルドがあればコンパイル不要）: [`docs/INSTALL.ja.md`](docs/INSTALL.ja.md)。

## ドキュメント

| | 한국어 | English | 日本語 |
|---|---|---|---|
| かんたんインストール | [INSTALL.md](docs/INSTALL.md) | [INSTALL.en.md](docs/INSTALL.en.md) | [INSTALL.ja.md](docs/INSTALL.ja.md) |
| 利用規約 | [EULA.md](docs/EULA.md) | [EULA.en.md](docs/EULA.en.md) | [EULA.ja.md](docs/EULA.ja.md) |
| リリース | [RELEASE.md](docs/RELEASE.md) | [RELEASE.en.md](docs/RELEASE.en.md) | [RELEASE.ja.md](docs/RELEASE.ja.md) |
| ストア文言 | [STORE.md](docs/STORE.md) | [STORE.en.md](docs/STORE.en.md) | [STORE.ja.md](docs/STORE.ja.md) |
| 第三者ソフトウェア | [THIRD_PARTY_NOTICES.md](docs/THIRD_PARTY_NOTICES.md) | 同じ | [THIRD_PARTY_NOTICES.ja.md](docs/THIRD_PARTY_NOTICES.ja.md) |
| モデル | [README.ko.md](models/README.ko.md) | [README.md](models/README.md) | [README.ja.md](models/README.ja.md) |

## ライセンス

アプリ本体: [`LICENSE`](LICENSE)（閲覧・個人ビルド）。同梱の公開ソフトウェアとモデル: [`docs/THIRD_PARTY_NOTICES.ja.md`](docs/THIRD_PARTY_NOTICES.ja.md)。

## 進捗

- [x] マイルストーン 1: プロジェクトの作成/開く、動画の読み込み（Assets へコピー + 48kHz WAV 抽出）、再生、ステップボタン、著作権表示
- [x] マイルストーン 2a: Whisper large-v3 音声認識 + 単語タイムスタンプ（DTW）+ SQLite 索引、進捗・キャンセル・動画ごとのチェックポイント
- [x] マイルストーン 2b: ボーカル分離（伴奏除去） — 読み込み・解析時にステレオ中央（ボーカル）と音声帯域だけ残す。追加のニューラルモデルなし
- [x] マイルストーン 3: 韓国語の標準発音変換、歌詞マッチ（句>単語>音節>近い音）、無音の整形・音量合わせ、プレビューレンダー（断片キャッシュ）、候補の差し替え
- [x] マイルストーン 4a: 韓国語音素の強制アライメント（wav2vec2 音素モデル、ONNX int8 静的リンク）→ 音節・母音境界。揃った候補を優先、短すぎる音節は減点
- [x] マイルストーン 4+: 聞き取りやすさ — 候補の再検証（切って聞き直し、解析時に先計算）、単語内位置の加点、単語間 40ms の休み。測定（`bench.rs`、Whisper 再認識 CER）: 51.3% → 23.7%
- [x] マイルストーン 4b: 日本語・英語の音素アライメント — 仮名モーラ / 英語音節を同じ wav2vec2 アライナーへ。漢字だけの語はスキップ
- [x] マイルストーン 5: 音階ブロック編集（ドレミ 7音スナップ、ドラッグ/↑↓、長さ、テンポ、キー、プリセット、取り消し、再生カーソル）、WORLD 変換 — 既定は試聴で選んだ「なめらかボコーダー」（微細な揺れを残す・フレーム補間・子音は原音）。詳細で機械音/PSOLA を選択（有声音だけ目標音、母音だけ伸ばす、母音開始=拍、話者ごとのオクターブ自動合わせ）、伸ばした映像
- [x] マイルストーン 6: 伴奏（MR）読み込み — テンポ・最初の拍・調の自動検出（短調は平行長調のドレミ）、拍単位で開始位置、伴奏音量、ステレオ混合 / MIDI メロディ読み込み（音符が最も多いトラック、和音は上の音、休符を反映）・書き出し（歌詞付き） / 書き出し: MP4 1280×720、WAV（伴奏あり/声のみ）、MIDI
- [x] 原曲に合わせて歌う: メロディはロボットオートチューンを押してから適用（それまでは声のままの音程）。原曲歌いの音節位置と音を探し（Whisper+音素アライメント、伴奏ごとに一度）、ユーザー歌詞の音節を先頭から順に載せる — 歌詞の内容とは無関係、長い前奏は1小節前から。測定（`melody_eval.rs`）: 音の一致 21.5% → 72.7%、開始誤差 4s → 0ms / 歌が終われば伴奏も終了 / 休符を入れる / 領域サイズ調整
- [x] マイルストーン 7: GitHub 公開 — 大きなモデルとサンプル音源はリポジトリに入れない
