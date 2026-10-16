# リリースチェックリスト (GitHub)

[한국어](RELEASE.md) · [English](RELEASE.en.md) · [日本語](RELEASE.ja.md)

配布経路は GitHub です。

## A. アカウント

| 順 | 作業 | メモ |
|---|---|---|
| 1 | GitHub リポジトリ `Holic-Jin/DCinVox` | ソースのみ。モデル重み・サンプル音源・FFmpeg バイナリは git に入れない |
| 2 | （任意）Apple Developer / Windows コード署名 | GitHub から `.app` / `.exe` を直接配るとき、Gatekeeper・SmartScreen を減らすために必要。Developer ID + 公証が無ければ利用者は `xattr -cr` が必須 |
| 3 | EULA の法律確認 | `docs/EULA.ja.md` は草案 |
| 4 | スクリーンショット・予告編 | 自分で撮った、または許諾のある素材だけ |

## B. ビルド

- macOS: `tools/build_ffmpeg_macos.sh` → `cargo tauri build --bundles app` → `tools/package_macos.sh`（Mac 上で ditto により zip、adhoc バンドル署名）
- Windows: `.github/workflows/build.yml`（`workflow_dispatch` またはタグ `v*`）
- **`.app` の zip は macOS 上でのみ作る。** Linux で再圧縮するとバンドル署名が壊れ、受け取った側の Gatekeeper が起動直後に SIGKILL する
- モデルはアプリの中ではなく**横の** `models/`（`models/README.ja.md`）。配布 zip には vocab JSON だけ入れ、Whisper・ONNX は入れない

## C. GitHub Release に載せるフォルダ例

```
macOS                         Windows
├─ DCinVox.app                ├─ dcinvox.exe
├─ models/                    ├─ ffmpeg.exe
│  ├─ ggml-large-v3-q5_0.bin  └─ models/（同じ）
│  ├─ phoneme-aligner-int8.onnx
│  └─ phoneme-aligner-vocab.json
├─ THIRD_PARTY_NOTICES.ja.md
└─ EULA.ja.md
```

モデルは Release 容量と GitHub のファイルサイズ制限のため、Hugging Face など別ホストの方が安全です。リポジトリには上げないでください。

## D. 公開前の確認

- [ ] 開発ツールのないきれいな Mac/Windows で、**ネットを切った状態**で解析 → 生成 → 変換 → 書き出し
- [ ] 8GB RAM でメモリ 3GB 以下
- [ ] FFmpeg のソース tarball とビルドスクリプトを保管（LGPL の請求に備える）
