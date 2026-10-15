# Models (not in git)

[한국어](README.ko.md) · [English](README.md) · [日本語](README.ja.md)

Weights are too large for GitHub (about 1.3 GB). Put them in `models/` **in the same folder as the app** (beside `DCinVox.app`, not inside it).

Analyze needs all three files.

## Whisper large-v3 (MIT) — `ggml-large-v3-q5_0.bin` (~1 GB)

Direct file link (check the size is ~1 GB, not an HTML page):

https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-q5_0.bin

## Phoneme aligner (Apache-2.0)

`phoneme-aligner-vocab.json` is in git and in the release zip under `models/`.

`phoneme-aligner-int8.onnx` is **not a download**. It is not in the zip, and the app does not fetch it. Convert [facebook/wav2vec2-xlsr-53-espeak-cv-ft](https://huggingface.co/facebook/wav2vec2-xlsr-53-espeak-cv-ft) once:

```
python3 -m pip install -r requirements.txt
python3 tools/export_aligner.py facebook/wav2vec2-xlsr-53-espeak-cv-ft models
```

With a release zip, use `export_aligner.py` from the zip and write into `models` **next to the app**:

```
python3 export_aligner.py facebook/wav2vec2-xlsr-53-espeak-cv-ft /path/to/folder-next-to-app/models
```

That writes `phoneme-aligner-int8.onnx` (a few hundred MB) and can refresh the vocab JSON.
