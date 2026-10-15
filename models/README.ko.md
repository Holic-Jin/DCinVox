# 모델 (git에 없음)

[한국어](README.ko.md) · [English](README.md) · [日本語](README.ja.md)

가중치는 GitHub에 올리기엔 너무 큽니다(약 1.3 GB). 이 폴더에 두세요. 앱은 실행 파일 옆과 저장소 루트의 `models/`를 찾습니다.

## Whisper large-v3 (MIT)

[ggerganov/whisper.cpp](https://huggingface.co/ggerganov/whisper.cpp)에서 `ggml-large-v3-q5_0.bin`을 이 폴더로 받으세요.

```
# 저장소 루트에서
mkdir -p models
# 그다음 ggml-large-v3-q5_0.bin 을 models/ 에 둡니다
```

## 음소 정렬기 (Apache-2.0)

`phoneme-aligner-vocab.json`은 git에 있습니다. ONNX 가중치는 없습니다.

1. [facebook/wav2vec2-xlsr-53-espeak-cv-ft](https://huggingface.co/facebook/wav2vec2-xlsr-53-espeak-cv-ft)를 받습니다.
2. 변환:

```
python tools/export_aligner.py /path/to/wav2vec2-xlsr-53-espeak-cv-ft models
```

`phoneme-aligner-int8.onnx`가 생깁니다(vocab JSON도 갱신될 수 있음).
