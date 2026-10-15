# DCinVox

[한국어](README.ko.md) · [English](README.md) · [日本語](README.ja.md)

영상 속 목소리를 음절 단위로 잘라 가사로 이어 붙이고, 음계 블록으로 노래로 만드는 데스크톱 앱 (Tauri 2 + Rust). © HolicJin

이 저장소에는 **영상·음성 소재가 없습니다.** 불러온 파일과 결과물의 저작권·초상권은 사용자 책임입니다. 사칭·허위 정보·비동의 합성에 쓰지 마세요. 자세한 내용은 [`docs/EULA.md`](docs/EULA.md)를 보세요.

## 필요한 것

- Rust (stable)
- 개발용 FFmpeg (`PATH`의 `ffmpeg` / `ffprobe`)
- `models/` 가중치 — git에 넣지 않습니다. 받는 방법: [`models/README.ko.md`](models/README.ko.md)

## 실행 (개발)

```
cd src-tauri
cargo run
```

- UI(`ui/`)는 빌드 도구 없는 순수 HTML/CSS/JS — Node 불필요
- 배포 시에는 `tools/build_ffmpeg_*.sh`로 만든 LGPL FFmpeg를 실행 파일 옆에 둡니다

## 테스트

```
cd src-tauri
cargo test
```

## 배포 빌드

```
tools/build_ffmpeg_macos.sh          # 한 번만 (LGPL ffmpeg → src-tauri/binaries/)
cd src-tauri && cargo tauri build --bundles app
```

Windows는 `.github/workflows/build.yml` (태그 `v*` 또는 수동 실행). 산출물은 Actions 아티팩트입니다. 모델은 앱 **옆** `models/` 폴더에 둡니다.

## 문서

| | 한국어 | English | 日本語 |
|---|---|---|---|
| 사용권 | [EULA.md](docs/EULA.md) | [EULA.en.md](docs/EULA.en.md) | [EULA.ja.md](docs/EULA.ja.md) |
| 출시 | [RELEASE.md](docs/RELEASE.md) | [RELEASE.en.md](docs/RELEASE.en.md) | [RELEASE.ja.md](docs/RELEASE.ja.md) |
| 스토어 문구 | [STORE.md](docs/STORE.md) | [STORE.en.md](docs/STORE.en.md) | [STORE.ja.md](docs/STORE.ja.md) |
| 공개 소프트웨어 | [THIRD_PARTY_NOTICES.md](docs/THIRD_PARTY_NOTICES.md) | 동일 | [THIRD_PARTY_NOTICES.ja.md](docs/THIRD_PARTY_NOTICES.ja.md) |
| 모델 | [README.ko.md](models/README.ko.md) | [README.md](models/README.md) | [README.ja.md](models/README.ja.md) |

## 라이선스

앱 소스: [`LICENSE`](LICENSE) (열람·개인 빌드). 포함된 공개 소프트웨어·모델: [`docs/THIRD_PARTY_NOTICES.md`](docs/THIRD_PARTY_NOTICES.md).

## 진행 상황

- [x] 마일스톤 1: 프로젝트 열기/만들기, 영상 불러오기(Assets 복사 + 48kHz WAV 추출), 재생, 단계 버튼, 저작권 고지
- [x] 마일스톤 2a: Whisper large-v3 음성 인식 + 단어 타임스탬프(DTW) + SQLite 색인, 진행률·취소·영상별 체크포인트
- [x] 마일스톤 2b: 보컬 분리(배경음 제거) — 불러오기·분석 때 스테레오 가운데(보컬) + 말소리 대역만 남김. 추가 신경망 모델 없음
- [x] 마일스톤 3: 한국어 표준발음 변환, 가사 매칭(구>단어>음절>비슷한 소리), 무음 다듬기·음량 맞춤, 미리보기 렌더(조각 캐시), 후보 바꾸기
- [x] 마일스톤 4a: 한국어 음소 강제정렬(wav2vec2 음소 모델, ONNX int8 정적 링크) → 음절·모음 경계, 정렬된 후보 우선·짧은 음절 감점
- [x] 마일스톤 4+: 알아듣기 개선 — 후보 재검증(잘라 놓고 다시 듣기, 분석 때 미리 계산), 단어 안 위치 일치 가산, 단어 사이 40ms 쉼. 측정(`bench.rs`, Whisper 재인식 CER): 51.3% → 23.7%
- [x] 마일스톤 4b: 일본어·영어 음소 정렬 — 가나 모라 / 영어 음절을 같은 wav2vec2 정렬기에 연결. 한자만 있는 어절은 건너뜀
- [x] 마일스톤 5: 음계 블록 편집기(도레미 7음 스냅·드래그/↑↓·길이·빠르기·키·프리셋·되돌리기·재생 커서), WORLD 변환 — 기본 엔진은 비교 청취로 고른 '부드러운 보코더'(미세 떨림 유지·프레임 보간·자음 원음), 고급에서 기계음/PSOLA 선택 (유성음만 목표 음, 모음만 늘이기, 모음 시작=박, 화자별 옥타브 자동 맞춤), 늘인 영상
- [x] 마일스톤 6: 반주(MR) 불러오기 — 빠르기·첫 박·조성 자동 감지(단조는 나란한 장조 도레미), 한 박씩 시작 위치 조정, 반주 크기, 스테레오 섞기 / MIDI 멜로디 불러오기(가장 음표 많은 트랙, 화음은 윗음, 쉼 반영)·내보내기(가사 포함) / 내보내기: MP4 1280×720, WAV(반주 포함/목소리만), MIDI
- [x] 원곡 따라 부르기: 멜로디는 로봇 오토튠을 눌러야 적용(그전엔 목소리 그대로의 음계). 원곡 가수가 부른 음절 위치·음을 찾아(Whisper+음소 정렬, 반주마다 한 번) 사용자 가사 음절을 처음부터 순서대로 얹음 — 가사 내용과 무관, 긴 전주는 한 마디 전부터. 측정(`melody_eval.rs`): 음 일치 21.5% → 72.7%, 시작 오차 4s → 0ms / 노래가 끝나면 반주도 끝 / 쉼 넣기 / 영역 크기 조절
- [x] 마일스톤 7: GitHub 공개 — 대용량 모델·샘플 음원은 저장소에 넣지 않음. (Steam 관련 초안은 `docs/`·`steam/`에 남아 있음)
