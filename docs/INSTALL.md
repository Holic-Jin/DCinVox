# 쉬운 설치 안내

[한국어](INSTALL.md) · [English](INSTALL.en.md) · [日本語](INSTALL.ja.md)

이 프로그램은 **영상을 인터넷에 올리지 않습니다.** 분석은 내 컴퓨터에서만 합니다.

아래는 **배포 zip**을 받는 길을 기준으로 적었습니다. 개발용 소스 실행은 맨 아래에 있습니다.

---

## 준비물

- **macOS** 11 이상, Apple Silicon(M1·M2·M3…) 권장
- 또는 **Windows** 10/11 64비트
- RAM **8GB** 이상
- 빈 디스크 **약 3GB** (앱 + 모델)
- 인터넷 (처음 모델 받을 때만)

zip 안에 **큰 AI 파일은 없습니다.** 브라우저로 하나, 변환으로 하나를 따로 만듭니다.

---

## macOS — 그대로 따라 하기

Finder에서 연 폴더 이름이 `DCinVox-macos-arm64` 이고 `다운로드`에 있다고 가정합니다. 다른 곳이면 `cd` 경로만 바꾸세요.

### 1. 앱 받기

1. [GitHub Releases](https://github.com/Holic-Jin/DCinVox/releases)에서 **최신** `DCinVox-macos-arm64.zip`을 받습니다. (1.1.0 이상)
2. 압축을 풉니다. **폴더 통째로** 둡니다. `DCinVox.app`만 다른 곳으로 옮기지 마세요.
3. 폴더 안에 `DCinVox.app`, `models/`, `export_aligner.py`가 보여야 합니다. `export_aligner.py`가 없으면 1.0.1 이하입니다. 최신 zip을 다시 받으세요.

### 2. 앱이 바로 꺼지지 않게 (반드시)

Safari·Chrome으로 받으면 macOS가 앱을 막을 수 있습니다. 아이콘이 한 번 튕기고 꺼지는 것은 **프로그램이 고장난 것이 아닙니다.**

`터미널` 앱을 연 뒤 (Spotlight에서 “터미널”):

```
cd ~/Downloads/DCinVox-macos-arm64
xattr -cr DCinVox.app
```

그다음 Finder에서 `DCinVox.app`을 엽니다. **우클릭 → 열기만으로는 부족한 경우가 많습니다.**

### 3. Whisper 파일 넣기 (~1GB)

분석에 필요합니다. zip에 없습니다.

1. 이 주소를 **그대로** 브라우저에 붙여 넣으세요. 사이트 페이지가 아니라 **파일이 받아져야** 합니다.  
   https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-q5_0.bin
2. 받은 파일을 앱과 **같은 폴더**의 `models/` 안으로 옮깁니다.
3. 이름은 반드시 `ggml-large-v3-q5_0.bin` 이어야 합니다.
4. 크기가 **약 1GB**여야 합니다. 몇 MB면 웹페이지가 저장된 것입니다. 다시 받으세요.

`DCinVox.app` **속(Contents)이 아니라**, 앱 **옆** `models/` 입니다.

### 4. 음소 파일 만들기 (`phoneme-aligner-int8.onnx`)

이 이름은 Hugging Face에서 `.onnx`로 받는 파일이 **아닙니다.** zip의 `export_aligner.py`가 한 번 변환해 만듭니다.

**Python 3.12 또는 3.13**이 필요합니다. **3.15는 안 됩니다.** (torch가 3.15용 설치 파일을 안 올립니다.)

Homebrew가 있으면:

```
brew install python@3.13
```

없거나 잘 모르겠으면 [python.org](https://www.python.org/downloads/)에서 **3.13** 설치 파일을 받으세요. 설치할 때 “Add Python to PATH”가 있으면 켭니다.

그다음 터미널에서 **한 줄씩** 실행합니다. `xyz` 나 `path/to/venv` 같은 **예시 글자는 복사하지 마세요.**

```
cd ~/Downloads/DCinVox-macos-arm64
python3.13 -m venv .venv
source .venv/bin/activate
```

프롬프트 앞에 `(.venv)` 가 생기면 성공입니다. 이어서:

```
python3 -m pip install -r aligner-requirements.txt
python3 export_aligner.py facebook/wav2vec2-xlsr-53-espeak-cv-ft ./models
```

zip에 `aligner-requirements.txt` 대신 `requirements.txt`만 있으면 그 이름을 쓰세요.

첫 변환은 다운로드 때문에 **시간이 꽤 걸립니다.** 끝나면 `models/phoneme-aligner-int8.onnx` 가 생기고 크기는 수백 MB입니다.

`pipx install torch` 는 쓰지 마세요. Homebrew Python에 바로 `pip install` 하면 `externally-managed-environment` 가 뜨는 것이 정상입니다. 그래서 **가상환경(.venv)** 을 씁니다.

### 5. 앱 실행

Finder에서 `DCinVox.app`을 엽니다. 분석 버튼은 아래 세 파일이 `models/`에 있을 때 동작합니다.

- `ggml-large-v3-q5_0.bin`
- `phoneme-aligner-vocab.json` (zip에 이미 있음)
- `phoneme-aligner-int8.onnx`

---

## Windows — 그대로 따라 하기

### 1. 앱 받기

1. [GitHub Releases](https://github.com/Holic-Jin/DCinVox/releases)에서 `DCinVox-windows-x64.zip`을 받습니다.
2. 압축을 풀고 **폴더 통째로** 둡니다. `dcinvox.exe`만 옮기지 마세요. 같은 폴더에 `ffmpeg.exe`가 있어야 합니다.
3. SmartScreen이 뜨면 **추가 정보 → 실행**입니다.

### 2. Whisper 파일 넣기

macOS 3번과 같습니다.  
https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-q5_0.bin  
을 받아 `models\ggml-large-v3-q5_0.bin` 으로 둡니다. 약 1GB.

### 3. 음소 파일 만들기

[python.org](https://www.python.org/downloads/)에서 **Python 3.13**을 설치합니다. **3.15는 안 됩니다.** 설치 화면에서 **Add python.exe to PATH**를 켭니다.

`명령 프롬프트` 또는 PowerShell에서 (폴더를 푼 위치로 `cd`):

```
cd %USERPROFILE%\Downloads\DCinVox-windows-x64
py -3.13 -m venv .venv
.venv\Scripts\activate
python -m pip install -r aligner-requirements.txt
python export_aligner.py facebook/wav2vec2-xlsr-53-espeak-cv-ft .\models
```

### 4. 실행

`dcinvox.exe`를 엽니다. Windows 10은 WebView2가 필요할 수 있습니다.

---

## 설치가 끝난 폴더

macOS:

```
DCinVox-macos-arm64/
├─ DCinVox.app
├─ export_aligner.py
├─ aligner-requirements.txt   (또는 requirements.txt)
├─ models/
│  ├─ ggml-large-v3-q5_0.bin
│  ├─ phoneme-aligner-int8.onnx
│  └─ phoneme-aligner-vocab.json
└─ .venv/                     (변환용, 앱 실행에는 필요 없음)
```

Windows도 `dcinvox.exe` + `ffmpeg.exe` + 같은 `models\` 입니다.

---

## 확인

터미널, 압축 푼 폴더에서 (macOS는 2번의 `xattr` 이후):

```
# macOS
DCinVox.app/Contents/MacOS/dcinvox --self-check

# Windows
dcinvox.exe --self-check
```

`ffmpeg: … OK` 가 나오면 ffmpeg는 된 것입니다. 모델 세 개가 아직이면 `SELF-CHECK FAILED` 가 정상입니다. **글자 없이 바로 죽으면** `xattr -cr DCinVox.app` 을 다시 하세요.

불러온 영상의 저작권·초상권은 사용자 책임입니다. [EULA.md](EULA.md)

---

## 안 될 때

| 이런 메시지나 증상 | 의미와 할 일 |
|---|---|
| 아이콘이 튕기고 꺼짐 | `xattr -cr DCinVox.app` 후 압축 푼 폴더의 앱을 다시 열기 |
| `externally-managed-environment` | Homebrew Python에 직접 pip 한 것. `.venv`를 만들고 **활성화한 뒤** `python3 -m pip` |
| `xyz` 패키지를 찾을 수 없음 | 안내문의 예시 글자 `xyz`를 그대로 친 것. 실제로는 `aligner-requirements.txt` 를 설치 |
| `No matching distribution found for torch` / `cp315` | Python **3.15** 입니다. 3.13으로 **새** `.venv`를 만드세요. 이미 있는 venv는 버전이 안 바뀝니다 |
| `numpy 패키지가 없습니다` | pip가 실패한 뒤 스크립트를 실행한 것. 위 표의 torch/venv를 먼저 해결 |
| `pipx` 로 torch 설치 실패 | pipx는 이 용도가 아님. `.venv` + `pip install -r aligner-requirements.txt` |
| ggml이 없다고 함 | 파일이 앱 **옆** `models/` 인지, 이름·크기(~1GB) 확인. `.app` 속이 아님 |
| onnx만 없다고 함 | zip에 없음. 4번의 변환을 앱 옆 `./models` 로 실행 |
| 변환이 몇 시간 | 첫 실행은 모델 다운로드 포함. 중단하지 말고 기다리기 |

---

## 개발용 (소스에서 실행)

배포 zip이 없을 때만.

```
git clone https://github.com/Holic-Jin/DCinVox.git
cd DCinVox
# 위의 모델 3개를 models/ 에 둔 뒤
cd src-tauri
cargo run
```

Rust(stable)와 개발용 `ffmpeg`/`ffprobe`가 필요합니다. 출시 절차는 [RELEASE.md](RELEASE.md)입니다.

소스 열람·개인 빌드: [`LICENSE`](../LICENSE)
