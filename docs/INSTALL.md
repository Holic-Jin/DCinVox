# 쉬운 설치 안내

[한국어](INSTALL.md) · [English](INSTALL.en.md) · [日本語](INSTALL.ja.md)

이 프로그램은 **인터넷에 영상을 올리지 않습니다.** 분석·생성은 모두 내 컴퓨터에서 합니다. 저장소에는 영상·음성 소재가 없고, 큰 AI 모델도 git에 없습니다.

## 필요한 환경

- **macOS** 11 이상 (Apple Silicon 권장)
- **Windows** 10 또는 11, 64비트
- RAM **8GB** 이상
- 빈 저장 공간 **약 2GB** (앱 + 모델)
- 처리 시간은 컴퓨터에 따라 다릅니다 (1시간 영상 분석은 M3 기준 약 40분)

## 한눈에 보기

1. 앱을 받습니다 (아래 A 또는 B).
2. `models/` 폴더를 앱 **옆**에 만들고, 모델 파일 3개를 넣습니다.
3. 앱을 실행합니다.

끝입니다. Node나 개발 도구는 배포본을 쓸 때 필요 없습니다.

---

## A. 배포본으로 설치 (가장 쉬움)

1. [GitHub Releases](https://github.com/Holic-Jin/DCinVox/releases)에서 최신 버전을 받습니다.
   - 아직 Release가 없으면 저장소 **Actions**의 `build` 작업 산출물(`DCinVox-macos` / `DCinVox-windows`)을 씁니다. 산출물은 일정 기간 후 만료됩니다.
2. 압축을 풀고 **폴더 전체를** 한곳에 둡니다. 실행 파일만 다른 곳으로 옮기지 마세요 (FFmpeg가 옆에 있어야 합니다).

### macOS 폴더 예

```
내 폴더/
├─ DCinVox.app
└─ models/
   ├─ ggml-large-v3-q5_0.bin
   ├─ phoneme-aligner-int8.onnx
   └─ phoneme-aligner-vocab.json
```

처음 열 때 “확인되지 않은 개발자” 경고가 나오면:

- Finder에서 `DCinVox.app`을 **우클릭 → 열기**, 또는
- 터미널: `xattr -d com.apple.quarantine DCinVox.app`

### Windows 폴더 예

```
내 폴더\
├─ dcinvox.exe
├─ ffmpeg.exe
└─ models\
   ├─ ggml-large-v3-q5_0.bin
   ├─ phoneme-aligner-int8.onnx
   └─ phoneme-aligner-vocab.json
```

SmartScreen 경고가 나오면 **추가 정보 → 실행**을 고릅니다. Windows 10은 WebView2가 필요할 수 있습니다.

---

## B. 소스에서 직접 돌리기 (개발용)

배포본이 없을 때만 이 경로를 씁니다.

1. [Rust](https://rustup.rs/) (stable)를 설치합니다.
2. 개발용 FFmpeg를 `PATH`에 둡니다 (`ffmpeg`, `ffprobe`).
3. 저장소를 받고 모델을 준비합니다 (아래 **모델 받기**).
4. 실행:

```
git clone https://github.com/Holic-Jin/DCinVox.git
cd DCinVox
# models/ 에 모델 3개를 둔 뒤
cd src-tauri
cargo run
```

macOS 배포 빌드:

```
tools/build_ffmpeg_macos.sh
cd src-tauri && cargo tauri build --bundles app
```

Windows 배포 빌드는 `.github/workflows/build.yml`을 쓰면 됩니다. 자세한 출시 절차는 [RELEASE.md](RELEASE.md)입니다.

---

## 모델 받기 (공통, 필수)

앱은 모델을 내려받지 않습니다. **실행 파일(또는 `.app`) 옆** `models/`에 직접 둡니다.

| 파일 | 대략 크기 | 받는 방법 |
|---|---|---|
| `ggml-large-v3-q5_0.bin` | ~1 GB | [ggerganov/whisper.cpp](https://huggingface.co/ggerganov/whisper.cpp)에서 다운로드 (MIT) |
| `phoneme-aligner-vocab.json` | 작음 | 이 저장소 `models/`에 이미 있음. 그대로 복사 |
| `phoneme-aligner-int8.onnx` | 나머지 | Hugging Face 모델을 한 번 변환 (Apache-2.0) |

Whisper 예 (저장소 루트 또는 앱 옆):

```
mkdir -p models
# 브라우저로 받거나:
# https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-q5_0.bin
```

음소 정렬 ONNX (Python, 한 번만):

```
python tools/export_aligner.py /path/to/wav2vec2-xlsr-53-espeak-cv-ft models
```

원본 모델: [facebook/wav2vec2-xlsr-53-espeak-cv-ft](https://huggingface.co/facebook/wav2vec2-xlsr-53-espeak-cv-ft)

더 자세한 설명은 [models/README.ko.md](../models/README.ko.md)입니다.

---

## 처음 실행

1. 화면 언어는 컴퓨터 언어를 따릅니다. 오른쪽 위에서 **한국어 / English / 日本語**로 바꿀 수 있습니다.
2. 불러오는 영상·음원의 **저작권·초상권은 사용자 책임**입니다. 사칭·허위 정보·비동의 합성에 쓰지 마세요. [EULA.md](EULA.md)
3. 설치가 맞는지 확인하려면 터미널에서:

```
# macOS (앱 안 실행 파일)
DCinVox.app/Contents/MacOS/dcinvox --self-check

# Windows
dcinvox.exe --self-check
```

`ffmpeg: … OK`와 모델 경로가 보이면 준비된 것입니다.

---

## 안 될 때

| 증상 | 확인 |
|---|---|
| 모델이 없다고 함 | `models/`가 앱 **옆**인지, 파일 이름이 위 표와 **완전히 같은지** |
| 영상이 안 열림 / 변환 실패 | Windows는 `ffmpeg.exe`가 `dcinvox.exe`와 같은 폴더인지. macOS는 `.app`을 통째로 두었는지 |
| macOS에서 앱이 안 열림 | 우클릭 → 열기, 또는 격리 속성 제거 |
| 처음 실행이 느림 | 정상입니다. 큰 모델을 디스크에서 읽습니다 |
| 메모리 부족 | 8GB RAM 기기에서는 다른 프로그램을 줄이세요 |

소스 열람·개인 빌드 조건은 [`LICENSE`](../LICENSE)입니다.
