# DCinVox

[한국어](README.ko.md) · [English](README.md) · [日本語](README.ja.md)

A desktop app that cuts speech from your videos into syllables, stitches them into lyrics, and turns them into a song with pitch blocks (Tauri 2 + Rust). © HolicJin

This repository includes **no video or audio**. You are responsible for copyright and portrait rights in files you import and in anything you export. Do not use it for impersonation, disinformation, or non-consensual synthesis. See [`docs/EULA.en.md`](docs/EULA.en.md).

## Requirements

- Rust (stable)
- FFmpeg on `PATH` for development (`ffmpeg` / `ffprobe`)
- Weights in `models/` — not stored in git. How to get them: [`models/README.md`](models/README.md)

## Run (development)

```
cd src-tauri
cargo run
```

- UI (`ui/`) is plain HTML/CSS/JS — no Node
- For release builds, place an LGPL FFmpeg next to the executable (`tools/build_ffmpeg_*.sh`)

## Tests

```
cd src-tauri
cargo test
```

## Release build

```
tools/build_ffmpeg_macos.sh          # once (LGPL ffmpeg → src-tauri/binaries/)
cd src-tauri && cargo tauri build --bundles app
```

Windows: `.github/workflows/build.yml` (tag `v*` or run manually). Artifacts come from Actions. Put models in a `models/` folder **next to** the app.

## Docs

| | 한국어 | English | 日本語 |
|---|---|---|---|
| License (EULA) | [EULA.md](docs/EULA.md) | [EULA.en.md](docs/EULA.en.md) | [EULA.ja.md](docs/EULA.ja.md) |
| Release | [RELEASE.md](docs/RELEASE.md) | [RELEASE.en.md](docs/RELEASE.en.md) | [RELEASE.ja.md](docs/RELEASE.ja.md) |
| Store copy | [STORE.md](docs/STORE.md) | [STORE.en.md](docs/STORE.en.md) | [STORE.ja.md](docs/STORE.ja.md) |
| Third-party | [THIRD_PARTY_NOTICES.md](docs/THIRD_PARTY_NOTICES.md) | same | [THIRD_PARTY_NOTICES.ja.md](docs/THIRD_PARTY_NOTICES.ja.md) |
| Models | [README.ko.md](models/README.ko.md) | [README.md](models/README.md) | [README.ja.md](models/README.ja.md) |

## License

App source: [`LICENSE`](LICENSE) (view and personal builds). Bundled open-source software and models: [`docs/THIRD_PARTY_NOTICES.md`](docs/THIRD_PARTY_NOTICES.md).

## Status

- [x] Milestone 1: create/open a project, import video (copy into Assets + extract 48 kHz WAV), playback, step buttons, copyright notice
- [x] Milestone 2a: Whisper large-v3 STT + word timestamps (DTW) + SQLite index, progress, cancel, per-video checkpoints
- [ ] Milestone 2b: vocal isolation (remove backing)
- [x] Milestone 3: Korean standard pronunciation, lyric matching (phrase > word > syllable > similar sound), silence trim and loudness match, preview render (clip cache), swap candidates
- [x] Milestone 4a: Korean phoneme forced alignment (wav2vec2 phoneme model, statically linked ONNX int8) → syllable and vowel bounds; prefer aligned candidates; penalize very short syllables
- [x] Milestone 4+: intelligibility — re-check candidates (cut and listen again, precomputed at analysis), bonus for position inside the word, 40 ms gap between words. Measured (`bench.rs`, Whisper re-recognition CER): 51.3% → 23.7%
- [ ] Milestone 4b: Japanese and English phoneme alignment
- [x] Milestone 5: pitch-block editor (do-re-mi snap in key, drag / ↑↓, length, tempo, key, presets, undo, playhead), WORLD conversion — default engine is the “smooth vocoder” chosen by listening (keeps micro-tremor, frame interpolation, original consonants); Advanced can pick mechanical / PSOLA (voiced frames only to target pitch, stretch vowels only, vowel onset on the beat, per-speaker octave), stretched video
- [x] Milestone 6: import backing track — auto detect tempo, downbeat, key (minor mapped to relative major do-re-mi), nudge start by beats, backing level, stereo mix / import MIDI melody (busiest track, top note of chords, rests) and export with lyrics / export: MP4 1280×720, WAV (with backing or voice only), MIDI
- [x] Follow the original: pitch follows only after robot autotune is on (until then, the source pitch). Finds the original singer’s syllable times and pitches (Whisper + phoneme align, once per backing) and lays your lyric syllables in order from the start — independent of lyric text; long intros start one bar before. Measured (`melody_eval.rs`): pitch match 21.5% → 72.7%, onset error 4 s → 0 ms / backing ends with the song / insert rests / resize regions
- [x] Milestone 7: GitHub public source — large models and sample audio are not in the repo. (Steam drafts remain under `docs/` and `steam/`)
