# ストア文言（草案）

[한국어](STORE.md) · [English](STORE.en.md) · [日本語](STORE.ja.md)

現在の配布は GitHub です。以下はプロジェクトページ向けの草案です。

## 短い説明（300字以内）

動画の声を音節ごとに切り出して、好きな歌詞につなぎ合わせ、ブロックを上下に動かすだけで歌に。原曲の伴奏を読み込めばメロディーも自動でなぞります。あの音MADの味を数クリックで。

## 詳しい説明

- 🎬 **動画を入れるだけ** — 音声認識で話し声を単語・音節に整理（韓国語中心、日本語・英語も認識）
- ✍️ **歌詞どおりにつなぐ** — ない単語は音節を集め、ない音節は近い音で。切って聞き直し、いちばん明瞭な断片を自動選択
- 🎹 **ドレミブロックを動かすだけ** — 音楽が分からなくてもキー内の音だけなので外れにくい。子音はそのまま、母音だけ伸ばして拍に合わせる
- 🎵 **原曲に合わせて歌う** — 伴奏を入れるとテンポ・キー・原曲メロディを探し、そのメロディで歌詞を載せる
- 🎞️ **映像も一緒に** — 各音節のもとの場面が歌に合わせて伸びる合成映像
- 💾 **書き出し** — MP4（720p）、WAV、MIDI（歌詞付き）
- 🔒 **すべて自分のパソコンで** — ネット接続・アップロードなし
- 🌐 **画面言語** — 한국어・English・日本語（最初はパソコンの言語、右上で変更）

**システム要件:** macOS 11 以降（Apple Silicon 推奨）/ Windows 10・11 64ビット、RAM 8GB 以上、空き容量 約 2GB。処理時間はパソコンによる（1時間の動画解析は M3 で約40分）。

**お知らせ:** プログラムに映像・音声は入っていません。読み込む素材と成果物の権利・責任は利用者にあります。成果物は意図的に「合成くさい」音で、自然な歌声シンセではありません。

## AI コンテンツ案内（草案）

**Live-Generated Content:**

> DCinVox uses AI models that run locally on the user's computer: speech recognition (OpenAI Whisper, MIT license) to transcribe the speech in videos the user imports, and a phoneme recognition model (wav2vec2, Apache-2.0) to locate syllables. Pitch changes use a conventional signal-processing vocoder (WORLD). The app does not generate new voices or images with generative AI; output is assembled from the user's own imported media. No content is included with the app, and no user data is uploaded. Users are responsible for the rights to the media they import; the app shows a notice about this before every generation and export.

**ガードレール:** 生成ごと・書き出しごとに著作権の責任表示、アプリに素材を同梱しない、EULA になりすまし・虚偽情報の禁止。

## スクリーンショット・映像の注意

紹介画像と予告編には、**自分で撮影した、または使用許諾を得た映像・音源だけ**を使ってください（例: 自分で撮った映像、著作権が切れた音源）。
