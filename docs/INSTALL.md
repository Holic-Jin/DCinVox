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
2. macOS면 압축을 푼 뒤 **먼저** `xattr -cr DCinVox.app` (안 하면 아이콘만 튕기고 꺼질 수 있음).
3. `models/` 폴더를 앱 **옆**에 만들고, 모델 파일 3개를 넣습니다.
4. 앱을 실행합니다.

끝입니다. Node나 개발 도구는 배포본을 쓸 때 필요 없습니다.

---

## A. 배포본으로 설치 (가장 쉬움)

1. [GitHub Releases](https://github.com/Holic-Jin/DCinVox/releases)에서 최신 버전을 받습니다.
   - 아직 Release가 없으면 저장소 **Actions**의 `build` 작업 산출물(`DCinVox-macos-arm64` / `DCinVox-windows`)을 씁니다. 산출물은 일정 기간 후 만료됩니다.
2. 압축을 풀고 **폴더 전체를** 한곳에 둡니다. 실행 파일만 다른 곳으로 옮기지 마세요 (FFmpeg가 옆에 있어야 합니다).
3. zip에는 **모델 가중치가 없습니다.** Whisper와 ONNX는 아래 **모델 받기**에서 따로 준비합니다. `phoneme-aligner-vocab.json`만 배포 zip의 `models/`에 들어 있습니다.

### macOS 폴더 예

```
내 폴더/
├─ DCinVox.app
└─ models/
   ├─ ggml-large-v3-q5_0.bin
   ├─ phoneme-aligner-int8.onnx
   └─ phoneme-aligner-vocab.json
```

이 배포본은 Apple Developer ID로 서명·공증되어 있지 않습니다. Safari나 Chrome으로 받으면 `com.apple.quarantine`가 붙고, **경고창 없이 아이콘만 튕기다 바로 꺼지는** 경우가 많습니다. 프로그램이 패닉한 것이 아니라 macOS Gatekeeper가 프로세스를 강제 종료(SIGKILL)한 것입니다.

압축을 푼 **뒤**, 앱을 열기 **전에** 터미널에서 격리를 제거하세요:

```
# 압축을 푼 폴더에서
xattr -cr DCinVox.app
```

그다음에 Finder에서 앱을 엽니다. 예전처럼 “우클릭 → 열기”만으로는 경고가 안 뜨고 바로 죽는 경우가 있습니다.

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

앱은 모델을 내려받지 않습니다. **실행 파일(또는 `.app`) 옆** `models/`에 직접 둡니다. 없어도 실행 직후 크래시의 원인은 아니고, **분석 버튼은 아래 3개가 모두 있어야** 동작합니다.

macOS는 `DCinVox.app`과 **같은 폴더**의 `models/`입니다. `.app` 안(Contents)이 아닙니다.

| 파일 | 대략 크기 | 받는 방법 |
|---|---|---|
| `ggml-large-v3-q5_0.bin` | ~1 GB | 아래 Whisper. **zip에 없음** |
| `phoneme-aligner-vocab.json` | 작음 | 배포 zip의 `models/` (이미 있음) |
| `phoneme-aligner-int8.onnx` | ~300 MB | 아래 ONNX 변환. **zip에 없음. `.onnx`로 받는 파일이 아님** |

### Whisper (`ggml-large-v3-q5_0.bin`)

1. 이 **직접 링크**로 받으세요 (페이지가 아니라 파일):  
   https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-q5_0.bin
2. 받은 파일을 앱 옆 `models/ggml-large-v3-q5_0.bin`으로 두세요. 이름은 한 글자도 같아야 합니다.
3. 크기가 **약 1GB**여야 합니다. 몇 MB면 브라우저가 HTML을 저장한 것입니다.

### 음소 정렬 ONNX (`phoneme-aligner-int8.onnx`) — 변환이 필요함

Hugging Face에 이 이름 그대로 올라온 `.onnx`는 없습니다.  
원본은 [facebook/wav2vec2-xlsr-53-espeak-cv-ft](https://huggingface.co/facebook/wav2vec2-xlsr-53-espeak-cv-ft) (Apache-2.0)이고, 배포 zip의 `export_aligner.py`(저장소에서는 `tools/export_aligner.py`)가 한 번 변환해 `phoneme-aligner-int8.onnx`를 만듭니다. 분석 2/3(발음 정렬)에 필요합니다.

1. Python 3.10 이상을 설치합니다.
2. 터미널:

```
python3 -m pip install -r requirements.txt
```

3. zip을 푼 폴더(또는 저장소 루트)에서, **출력 경로를 앱 옆 models 로** 지정합니다:

```
# macOS 예: 앱이 ~/DCinVox/DCinVox.app 일 때
python3 export_aligner.py facebook/wav2vec2-xlsr-53-espeak-cv-ft ~/DCinVox/models

# 저장소에서 개발할 때
python3 tools/export_aligner.py facebook/wav2vec2-xlsr-53-espeak-cv-ft models
```

첫 인자는 폴더가 아니라 **모델 ID 그대로** 두면 됩니다. 스크립트가 받아 변환합니다. 첫 실행은 다운로드·변환에 시간이 걸립니다.

4. 끝나면 `models/phoneme-aligner-int8.onnx`가 생깁니다 (대략 수백 MB). 이 파일이 없으면 분석이 시작되지 않습니다.

더 자세한 설명은 [models/README.ko.md](../models/README.ko.md)입니다.

---

## 처음 실행

1. 화면 언어는 컴퓨터 언어를 따릅니다. 오른쪽 위에서 **한국어 / English / 日本語**로 바꿀 수 있습니다.
2. 불러오는 영상·음원의 **저작권·초상권은 사용자 책임**입니다. 사칭·허위 정보·비동의 합성에 쓰지 마세요. [EULA.md](EULA.md)
3. 설치가 맞는지 확인하려면 터미널에서 (macOS는 위에서 격리를 제거한 뒤):

```
# macOS (앱 안 실행 파일)
DCinVox.app/Contents/MacOS/dcinvox --self-check

# Windows
dcinvox.exe --self-check
```

`ffmpeg: … OK`가 나와야 합니다. Whisper·ONNX를 아직 안 넣었으면 `SELF-CHECK FAILED`와 찾은 경로가 출력되는 것이 정상입니다. 명령이 **아무 글자 없이 바로 죽거나** 종료 코드 137이면, 앱 버그가 아니라 격리가 남아 있는 것입니다. `xattr -cr DCinVox.app`을 다시 하세요.

---

## 안 될 때

| 증상 | 확인 |
|---|---|
| 아이콘이 한 번 튕기고 바로 꺼짐 / `--self-check`가 즉시 죽음 | `xattr -cr DCinVox.app`으로 격리 제거. 우클릭 → 열기만으로는 부족한 경우가 많음 |
| 모델이 없다고 함 | `models/`가 `.app`/`exe`와 **같은 폴더**인지. 찾아본 경로가 에러에 나옴. ONNX는 `export_aligner.py`로 변환해야 함 |
| Whisper를 넣었는데 ggml이 없다고 함 | 파일이 `.app` 안이 아니라 **옆** `models/`인지, 크기가 ~1GB인지. macOS면 `xattr -cr` 후 앱을 다시 열기 (임시 폴더 실행이면 원본 옆 models가 안 보임) |
| ONNX가 없다고만 함 | 정상입니다. zip에 없음. 위 변환 명령을 앱 옆 `models/`를 출력으로 실행 |
| 영상이 안 열림 / 변환 실패 | Windows는 `ffmpeg.exe`가 `dcinvox.exe`와 같은 폴더인지. macOS는 `.app`을 통째로 두었는지 |
| macOS에서 앱이 안 열림 | 격리 속성 제거 (`xattr -cr`). Developer ID 서명이 없는 배포본입니다 |
| 처음 실행이 느림 | 정상입니다. 큰 모델을 디스크에서 읽습니다 |
| 메모리 부족 | 8GB RAM 기기에서는 다른 프로그램을 줄이세요 |

소스 열람·개인 빌드 조건은 [`LICENSE`](../LICENSE)입니다.
