# モデル（git には含めません）

[한국어](README.ko.md) · [English](README.md) · [日本語](README.ja.md)

重みは GitHub には大きすぎます（約 1.3 GB）。このフォルダに置いてください。アプリは実行ファイルの横と、リポジトリ直下の `models/` を探します。

## Whisper large-v3 (MIT)

[ggerganov/whisper.cpp](https://huggingface.co/ggerganov/whisper.cpp) から `ggml-large-v3-q5_0.bin` をこのフォルダへダウンロードします。

```
# リポジトリ直下で
mkdir -p models
# 続けて ggml-large-v3-q5_0.bin を models/ へ
```

## 音素アライナー (Apache-2.0)

`phoneme-aligner-vocab.json` は git にあります。ONNX の重みはありません。

1. [facebook/wav2vec2-xlsr-53-espeak-cv-ft](https://huggingface.co/facebook/wav2vec2-xlsr-53-espeak-cv-ft) を入手します。
2. 変換:

```
python tools/export_aligner.py /path/to/wav2vec2-xlsr-53-espeak-cv-ft models
```

`phoneme-aligner-int8.onnx` が出力されます（vocab JSON も更新されることがあります）。
