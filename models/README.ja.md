# モデル（git には含めません）

[한국어](README.ko.md) · [English](README.md) · [日本語](README.ja.md)

重みは GitHub には大きすぎます（約 1.3 GB）。アプリ（`DCinVox.app`）と**同じフォルダ**の `models/` に置いてください。`.app` の中ではありません。

解析には次の3つがすべて必要です。

## Whisper large-v3 (MIT) — `ggml-large-v3-q5_0.bin`（約 1 GB）

直接ファイルのリンク（約 1GB か、HTML ではないか確認）:

https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-q5_0.bin

## 音素アライナー (Apache-2.0)

`phoneme-aligner-vocab.json` は git と配布 zip の `models/` にあります。

`phoneme-aligner-int8.onnx` は**配布ファイルではありません。** zip にも無く、アプリも取得しません。[facebook/wav2vec2-xlsr-53-espeak-cv-ft](https://huggingface.co/facebook/wav2vec2-xlsr-53-espeak-cv-ft) を一度変換します。

```
python3 -m pip install -r requirements.txt
python3 tools/export_aligner.py facebook/wav2vec2-xlsr-53-espeak-cv-ft models
```

配布 zip では zip 内の `export_aligner.py` を使い、出力を**アプリ横**の `models` にします:

```
python3 export_aligner.py facebook/wav2vec2-xlsr-53-espeak-cv-ft /path/to/folder-next-to-app/models
```

`phoneme-aligner-int8.onnx`（数百 MB 程度）ができます。vocab JSON も更新されることがあります。
