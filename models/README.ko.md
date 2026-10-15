# 모델 (git에 없음)

[한국어](README.ko.md) · [English](README.md) · [日本語](README.ja.md)

가중치는 GitHub에 올리기엔 너무 큽니다(약 1.3 GB). **앱(또는 `DCinVox.app`)과 같은 폴더**의 `models/`에 두세요. `.app` 속이 아닙니다.

분석에는 아래 3개가 모두 필요합니다.

## Whisper large-v3 (MIT) — `ggml-large-v3-q5_0.bin` (~1 GB)

직접 파일 링크 (페이지 HTML이 아닌지 크기 ~1GB 확인):

https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-q5_0.bin

## 음소 정렬기 (Apache-2.0)

`phoneme-aligner-vocab.json`은 git과 배포 zip의 `models/`에 있습니다.

`phoneme-aligner-int8.onnx`는 **다운로드 파일이 아닙니다.** zip에도 없고, 앱이 받아 오지도 않습니다. Hugging Face의 [facebook/wav2vec2-xlsr-53-espeak-cv-ft](https://huggingface.co/facebook/wav2vec2-xlsr-53-espeak-cv-ft)를 한 번 변환합니다.

```
python3 -m pip install -r requirements.txt
python3 tools/export_aligner.py facebook/wav2vec2-xlsr-53-espeak-cv-ft models
```

배포 zip을 쓰면 zip 안의 `export_aligner.py`와, **앱 옆** `models`를 출력으로 지정합니다:

```
python3 export_aligner.py facebook/wav2vec2-xlsr-53-espeak-cv-ft /path/to/folder-next-to-app/models
```

끝나면 `phoneme-aligner-int8.onnx`(대략 수백 MB)가 생깁니다. vocab JSON도 갱신될 수 있습니다.
