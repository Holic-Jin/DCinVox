# Models (not in git)

Weights are too large for GitHub (about 1.3 GB). Put them in this folder. The app looks next to the executable and in `models/` at the repo root.

## Whisper large-v3 (MIT)

Download `ggml-large-v3-q5_0.bin` from [ggerganov/whisper.cpp](https://huggingface.co/ggerganov/whisper.cpp) into this folder.

```
# from the repo root
mkdir -p models
# then download ggml-large-v3-q5_0.bin into models/
```

## Phoneme aligner (Apache-2.0)

`phoneme-aligner-vocab.json` is in git. The ONNX weights are not.

1. Download [facebook/wav2vec2-xlsr-53-espeak-cv-ft](https://huggingface.co/facebook/wav2vec2-xlsr-53-espeak-cv-ft).
2. Convert:

```
python tools/export_aligner.py /path/to/wav2vec2-xlsr-53-espeak-cv-ft models
```

That writes `phoneme-aligner-int8.onnx` (and can refresh the vocab JSON).
