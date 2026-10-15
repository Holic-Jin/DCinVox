# Steam 스토어 문구 (초안)

[한국어](STORE.md) · [English](STORE.en.md) · [日本語](STORE.ja.md)

GitHub 배포가 기본입니다. 아래는 Steam용으로 남겨 둔 초안입니다.

## 짧은 설명 (300자 이내)

영상 속 목소리를 음절 단위로 잘라 내 가사로 이어 붙이고, 블록을 위아래로 옮겨 노래로! 원곡 반주를 넣으면 멜로디도 자동으로 따라 부릅니다. 합성 티 팍팍 나는 그 맛, 이제 클릭 몇 번으로.

## 상세 설명

- 🎬 **영상만 넣으면 끝** — 음성 인식으로 말소리를 단어·음절 단위로 정리 (한국어 중심, 일본어·영어 인식)
- ✍️ **가사대로 이어 붙이기** — 없는 단어는 음절을 모아서, 없는 음절은 비슷한 소리로. 잘라 놓고 다시 들어 보고 가장 또렷한 조각을 자동 선택
- 🎹 **도레미 블록만 옮기면 노래** — 음악을 몰라도 키 안의 음만 쓰니 틀릴 일 없음. 자음은 그대로, 모음만 늘여서 박자에 맞춤
- 🎵 **원곡 따라 부르기** — 반주 음원을 넣으면 빠르기·키·원곡 멜로디를 찾아 내 가사를 그 멜로디로
- 🎞️ **영상도 함께** — 각 음절의 원래 장면이 노래에 맞춰 늘어나는 합성 영상
- 💾 **내보내기** — MP4(720p), WAV, MIDI(가사 포함)
- 🔒 **모두 내 컴퓨터에서** — 인터넷 연결·업로드 없음
- 🌐 **화면 언어** — 한국어·English·日本語 (처음엔 컴퓨터 언어를 따르고, 오른쪽 위에서 바꿈)

**시스템 요구 사항:** macOS 11 이상(Apple Silicon 권장) / Windows 10·11 64비트, RAM 8GB 이상, 저장 공간 약 2GB. 처리 시간은 컴퓨터 성능에 따라 다릅니다(1시간 영상 분석 약 40분, M3 기준).

**알림:** 프로그램에는 영상·음성이 들어 있지 않습니다. 불러오는 소재와 결과물의 권리·책임은 사용자에게 있습니다. 결과물은 의도적으로 "합성 티"가 나는 소리이며 자연스러운 가창 합성기가 아닙니다.

## Steam AI 콘텐츠 공개 (Content Survey → AI Generated Content) 초안

**Live-Generated Content:**

> DCinVox uses AI models that run locally on the user's computer: speech recognition (OpenAI Whisper, MIT license) to transcribe the speech in videos the user imports, and a phoneme recognition model (wav2vec2, Apache-2.0) to locate syllables. Pitch changes use a conventional signal-processing vocoder (WORLD). The app does not generate new voices or images with generative AI; output is assembled from the user's own imported media. No content is included with the app, and no user data is uploaded. Users are responsible for the rights to the media they import; the app shows a notice about this before every generation and export.

**가드레일:** 생성할 때마다·내보낼 때마다 저작권 책임 고지, 앱에 소재 미포함, EULA에 사칭·허위 정보 금지 조항.

## 스크린샷·영상 주의

스토어 이미지와 예고편에는 **직접 촬영했거나 사용 허락을 받은 영상·음원만** 쓰세요 (예: 직접 녹화한 영상, 저작권이 만료된 음원).
