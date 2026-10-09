# 출시 체크리스트

## A. HolicJin님이 직접 해야 하는 것 (계정·비용)
| 순서 | 할 일 | 비용 | 메모 |
|---|---|---|---|
| 1 | **Steamworks 파트너 가입** (partner.steamgames.com) | 앱당 US$100 (매출 $1,000 넘으면 환급) | 세금·은행 정보 입력, 본인 확인. 승인까지 며칠 |
| 2 | 스토어 페이지 작성 | — | `docs/STORE.md` 문구 사용. 출시 최소 2주 전 "곧 출시" 공개 필요 |
| 3 | Content Survey의 **AI 콘텐츠 공개** 작성 | — | `docs/STORE.md`의 초안 사용 |
| 4 | **Apple Developer Program** 가입 | 연 US$99 | macOS 서명·공증용. 없으면 macOS에서 "확인되지 않은 개발자" 경고로 실행이 막힘 |
| 5 | (선택) Windows 코드서명 인증서 | 연 수십~수백 달러 | Steam으로만 배포하면 생략 가능 (Steam이 실행하므로 SmartScreen 경고가 대체로 안 뜸) |
| 6 | **GitHub 저장소 만들기** (비공개 권장) | 무료 | Windows 빌드를 GitHub Actions에서 함. 만들면 알려 주세요 → 연결·첫 빌드 |
| 7 | EULA 법률 검토 | — | `docs/EULA.md`는 초안 |
| 8 | 스토어 이미지·예고편 | — | 직접 촬영했거나 허락받은 소재만 |

## B. 빌드 (자동화됨)
- macOS: `tools/build_ffmpeg_macos.sh` → `cargo tauri build --bundles app`
- Windows: `.github/workflows/build.yml` (GitHub Actions, windows-latest)
- 모델은 앱 안이 아니라 **옆 폴더 `models/`** 에 둠 (서명된 앱을 건드리지 않기 위해). 앱이 자동으로 찾음.

## C. Steam 배포 폴더 구성 (depot)
```
macOS depot                     Windows depot
├─ DCinVox.app                  ├─ dcinvox.exe
├─ models/                      ├─ ffmpeg.exe
│  ├─ ggml-large-v3-q5_0.bin    ├─ WebView2 설치 안내 (Steam 재배포 스크립트)
│  ├─ phoneme-aligner-int8.onnx └─ models/ (macOS와 동일)
│  └─ phoneme-aligner-vocab.json
├─ THIRD_PARTY_NOTICES.md
└─ EULA.md
```

## D. 출시 전 확인
- [ ] 개발 도구가 없는 깨끗한 Mac/Windows에서, **인터넷을 끄고** 설치 → 분석 → 생성 → 변환 → 내보내기
- [ ] 8GB RAM 컴퓨터에서 1시간 영상 분석 (메모리 3GB 이하)
- [ ] 음악 경험 없는 사람 2~3명이 설명 없이 10분 안에 노래 만들기
- [ ] FFmpeg 소스 tarball과 빌드 스크립트를 보관 (LGPL 요청 대응)
