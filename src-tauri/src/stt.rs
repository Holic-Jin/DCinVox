//! Whisper(whisper.cpp) 음성 인식 → 단어 단위 타임스탬프.
//! 단어 경계는 DTW 토큰 타임스탬프로 잡고, 음절 단위 정밀 정렬은 align 단계(마일스톤 4)에서 보정한다.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use whisper_rs::{
    DtwMode, DtwModelPreset, DtwParameters, FullParams, SamplingStrategy, WhisperContext,
    WhisperContextParameters,
};

pub const SAMPLE_RATE: u32 = 16_000;

#[derive(Debug, Clone, serde::Serialize)]
pub struct Word {
    pub text: String,
    pub start: f64,
    pub end: f64,
    /// 단어를 이루는 토큰 확률의 평균 (0~1)
    pub prob: f32,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Segment {
    pub start: f64,
    pub end: f64,
    pub text: String,
    pub words: Vec<Word>,
}

pub struct Transcript {
    pub lang: String,
    pub segments: Vec<Segment>,
}

/// 모델을 한 번 올려 여러 파일에 재사용. 분석이 끝나면 drop되어 메모리 해제(8GB 대응).
pub struct Recognizer {
    ctx: WhisperContext,
}

impl Recognizer {
    pub fn load(model: &Path) -> Result<Self, String> {
        let mut params = WhisperContextParameters::default();
        params.dtw_parameters(DtwParameters {
            mode: DtwMode::ModelPreset { model_preset: DtwModelPreset::LargeV3 },
            ..Default::default()
        });
        let ctx = WhisperContext::new_with_params(model, params)
            .map_err(|e| format!("음성 인식 모델 로드 실패: {e}"))?;
        Ok(Self { ctx })
    }

    /// `language`: None이면 자동 감지
    pub fn transcribe(
        &self,
        pcm: &[f32],
        language: Option<&str>,
        on_progress: impl FnMut(i32) + 'static,
        cancel: Arc<AtomicBool>,
    ) -> Result<Transcript, String> {
        self.transcribe_hinted(pcm, language, None, on_progress, cancel)
    }

    /// `prompt`: 들릴 것으로 예상되는 문장(예: 노래 가사) — 알아듣기를 그쪽으로 유도
    pub fn transcribe_hinted(
        &self,
        pcm: &[f32],
        language: Option<&str>,
        prompt: Option<&str>,
        mut on_progress: impl FnMut(i32) + 'static,
        cancel: Arc<AtomicBool>,
    ) -> Result<Transcript, String> {
        // 품질 우선: 빔서치. 느려도 8GB에서 동작하도록 스레드는 코어 수 - 1
        let mut params = FullParams::new(SamplingStrategy::BeamSearch { beam_size: 5, patience: -1.0 });
        let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
        params.set_n_threads(threads.saturating_sub(1).max(1) as i32);
        params.set_language(Some(language.unwrap_or("auto")));
        if let Some(p) = prompt {
            params.set_initial_prompt(p);
        }
        params.set_token_timestamps(true);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_special(false);
        params.set_print_timestamps(false);
        params.set_suppress_nst(true);
        params.set_progress_callback_safe(move |p: i32| on_progress(p));
        // whisper-rs 0.16의 set_abort_callback_safe는 user_data 타입이 어긋나 즉시 중단되는 버그가 있어
        // AtomicBool 포인터를 직접 넘긴다. `cancel` Arc는 full()이 끝날 때까지 이 함수가 붙잡고 있음.
        unsafe {
            params.set_abort_callback(Some(abort_trampoline));
            params.set_abort_callback_user_data(Arc::as_ptr(&cancel) as *mut std::ffi::c_void);
        }

        let mut state = self.ctx.create_state().map_err(|e| e.to_string())?;
        let result = state.full(params, pcm);
        if cancel.load(Ordering::Relaxed) {
            return Err(CANCELLED.into());
        }
        result.map_err(|e| format!("음성 인식 실패: {e}"))?;

        let lang = whisper_rs::get_lang_str(state.full_lang_id_from_state()).unwrap_or("").to_string();
        let eot = self.ctx.token_eot();
        let total_secs = pcm.len() as f64 / SAMPLE_RATE as f64;

        let mut segments = Vec::new();
        for seg in state.as_iter() {
            let seg_start = seg.start_timestamp() as f64 / 100.0;
            let seg_end = (seg.end_timestamp() as f64 / 100.0).min(total_secs);
            let mut pieces = Vec::new();
            for i in 0..seg.n_tokens() {
                let Some(tok) = seg.get_token(i) else { continue };
                if tok.token_id() >= eot {
                    continue; // 특수 토큰(타임스탬프·언어 등)
                }
                let data = tok.token_data();
                let bytes = tok.to_bytes().map(|b| b.to_vec()).unwrap_or_default();
                pieces.push(TokenPiece { bytes, t: data.t_dtw as f64 / 100.0, p: data.p });
            }
            let words = group_words(&pieces, seg_start, seg_end, lang == "ja" || lang == "zh");
            let text = seg.to_str_lossy().map(|s| s.trim().to_string()).unwrap_or_default();
            if !words.is_empty() {
                segments.push(Segment { start: seg_start, end: seg_end, text, words });
            }
        }
        Ok(Transcript { lang, segments: drop_hallucinations(segments) })
    }
}

pub const CANCELLED: &str = "취소됨";

/// 사람이 말할 수 없는 속도(글자/초)의 문장은 Whisper 반복 환각으로 보고 버린다.
/// 한국어·일본어 빠른 말도 초당 10~12글자 안팎이라 25면 넉넉한 기준.
const MAX_CHARS_PER_SEC: f64 = 25.0;

/// Whisper가 같은 문장을 길이 0에 가깝게 반복 출력하거나 타임스탬프가 무너진 문장 제거.
/// 정상 속도로 반복된 말은 실제 발화일 수 있어(합성 소재로 유용) 남긴다.
fn drop_hallucinations(segments: Vec<Segment>) -> Vec<Segment> {
    segments
        .into_iter()
        .filter(|seg| {
            let chars = seg.text.chars().filter(|c| c.is_alphanumeric()).count() as f64;
            chars / (seg.end - seg.start).max(0.01) <= MAX_CHARS_PER_SEC
        })
        .collect()
}

unsafe extern "C" fn abort_trampoline(data: *mut std::ffi::c_void) -> bool {
    (*(data as *const AtomicBool)).load(Ordering::Relaxed)
}

struct TokenPiece {
    bytes: Vec<u8>,
    /// DTW로 추정한 토큰 시작 시각(초)
    t: f64,
    p: f32,
}

/// BPE 토큰 → 단어. 공백으로 시작하는 토큰이 새 단어의 시작.
/// 한글·가나는 한 글자가 여러 토큰에 걸쳐 쪼개질 수 있어 바이트로 모은 뒤 디코딩한다.
/// 띄어쓰기 없는 언어(ja/zh)는 여기서는 문장부호 단위로만 나누고, 형태소 분리는 tokenize 단계에서 한다.
fn group_words(pieces: &[TokenPiece], seg_start: f64, seg_end: f64, no_spaces: bool) -> Vec<Word> {
    struct Acc {
        bytes: Vec<u8>,
        start: f64,
        probs: Vec<f32>,
    }
    let mut accs: Vec<Acc> = Vec::new();
    for piece in pieces {
        let starts_new = accs.is_empty()
            || piece.bytes.first() == Some(&b' ')
            || (no_spaces && is_punct_bytes(&piece.bytes));
        if starts_new {
            accs.push(Acc { bytes: Vec::new(), start: piece.t, probs: Vec::new() });
        }
        let acc = accs.last_mut().unwrap();
        acc.bytes.extend_from_slice(&piece.bytes);
        acc.probs.push(piece.p);
    }

    let mut words: Vec<Word> = Vec::new();
    for (i, acc) in accs.iter().enumerate() {
        let text = clean_word(&String::from_utf8_lossy(&acc.bytes));
        if text.is_empty() {
            continue;
        }
        let start = acc.start.clamp(seg_start, seg_end);
        let next = accs.get(i + 1).map(|n| n.start).unwrap_or(seg_end);
        let end = next.clamp(start, seg_end);
        let prob = acc.probs.iter().sum::<f32>() / acc.probs.len() as f32;
        words.push(Word { text, start, end, prob });
    }
    // DTW가 단어 길이를 0으로 잡는 경우 최소 길이 보장(뒤 단어와 겹치지 않는 범위에서)
    for i in 0..words.len() {
        if words[i].end - words[i].start < 0.03 {
            let limit = words.get(i + 1).map(|w| w.start).unwrap_or(seg_end);
            words[i].end = (words[i].start + 0.03).min(limit.max(words[i].start));
        }
    }
    words
}

fn is_punct_bytes(b: &[u8]) -> bool {
    std::str::from_utf8(b).map(|s| !s.is_empty() && s.trim().chars().all(is_punct)).unwrap_or(false)
}

fn is_punct(c: char) -> bool {
    // 장음 기호 'ー'는 발음의 일부이므로 문장부호가 아님
    c.is_ascii_punctuation() || "、。，．！？「」『』（）…・〜―\u{201c}\u{201d}\u{2018}\u{2019}".contains(c)
}

/// 앞뒤 공백·문장부호 제거 (단어 내부 아포스트로피 등은 유지)
fn clean_word(s: &str) -> String {
    s.trim().trim_matches(|c: char| is_punct(c) && c != '\'').trim().to_string()
}

/// 16kHz mono WAV 읽기
pub fn read_wav(path: &Path) -> Result<Vec<f32>, String> {
    let mut reader = hound::WavReader::open(path).map_err(|e| format!("WAV 읽기 실패: {e}"))?;
    let spec = reader.spec();
    if spec.channels != 1 || spec.sample_rate != SAMPLE_RATE {
        return Err(format!("16kHz mono WAV가 아닙니다 ({}Hz, {}ch)", spec.sample_rate, spec.channels));
    }
    match spec.sample_format {
        hound::SampleFormat::Int => reader
            .samples::<i16>()
            .map(|s| s.map(|v| v as f32 / 32768.0))
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string()),
        hound::SampleFormat::Float => reader.samples::<f32>().collect::<Result<_, _>>().map_err(|e| e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn piece(s: &str, t: f64) -> TokenPiece {
        TokenPiece { bytes: s.as_bytes().to_vec(), t, p: 0.9 }
    }

    #[test]
    fn groups_bpe_tokens_into_words() {
        let pieces = vec![piece(" 안녕", 0.0), piece("하세요", 0.3), piece(",", 0.6), piece(" 여러분", 0.7)];
        let words = group_words(&pieces, 0.0, 1.5, false);
        let texts: Vec<_> = words.iter().map(|w| w.text.as_str()).collect();
        assert_eq!(texts, ["안녕하세요", "여러분"]);
        assert_eq!(words[0].start, 0.0);
        assert_eq!(words[0].end, 0.7);
        assert_eq!(words[1].end, 1.5);
    }

    #[test]
    fn joins_split_utf8_bytes() {
        // '한'(ED 95 9C)이 두 토큰으로 쪼개진 경우
        let pieces = vec![
            TokenPiece { bytes: vec![b' ', 0xED, 0x95], t: 0.0, p: 0.8 },
            TokenPiece { bytes: vec![0x9C], t: 0.1, p: 0.8 },
        ];
        let words = group_words(&pieces, 0.0, 1.0, false);
        assert_eq!(words[0].text, "한");
    }

    #[test]
    fn drops_repeated_zero_length_segments() {
        let seg = |t: &str, s: f64, e: f64| Segment {
            start: s,
            end: e,
            text: t.into(),
            words: vec![Word { text: t.into(), start: s, end: e, prob: 0.9 }],
        };
        let out = drop_hallucinations(vec![
            seg("아주 성능 좋은 무기를 개발해야 된다", 3753.2, 3759.8),
            seg("아주 성능 좋은 무기를 개발해야 된다", 3759.8, 3759.8),
            seg("아주 성능 좋은 무기를 개발해야 된다", 3759.8, 3759.9),
            seg("슬픕니다", 3760.0, 3760.9),
            seg("슬픕니다", 3761.0, 3761.9), // 정상 속도로 반복된 말은 유지
        ]);
        assert_eq!(out.len(), 3);
        assert_eq!(out[0].end, 3759.8);
    }

    #[test]
    fn japanese_splits_on_punctuation_only() {
        let pieces = vec![piece("こんにち", 0.0), piece("は", 0.4), piece("。", 0.6), piece("元気", 0.8)];
        let words = group_words(&pieces, 0.0, 1.2, true);
        let texts: Vec<_> = words.iter().map(|w| w.text.as_str()).collect();
        assert_eq!(texts, ["こんにちは", "元気"]);
    }
}

/// 실제 모델로 확인: DCINVOX_TEST_WAV=<16k wav> cargo test --release stt_real -- --ignored --nocapture
#[cfg(test)]
mod real_model {
    use super::*;

    #[test]
    #[ignore]
    fn stt_real() {
        let wav = std::env::var("DCINVOX_TEST_WAV").expect("DCINVOX_TEST_WAV");
        let pcm = read_wav(Path::new(&wav)).unwrap();
        let rec = Recognizer::load(&crate::models::find(crate::models::WHISPER_MODEL).unwrap()).unwrap();
        let t0 = std::time::Instant::now();
        let t = rec.transcribe(&pcm, None, |_| {}, Arc::new(AtomicBool::new(false))).unwrap();
        println!("lang={} audio={:.0}s took={:.1}s", t.lang, pcm.len() as f64 / 16000.0, t0.elapsed().as_secs_f64());
        for s in &t.segments {
            println!("[{:6.2}-{:6.2}] {}", s.start, s.end, s.text);
            let ws: Vec<String> = s.words.iter().map(|w| format!("{}({:.2}-{:.2})", w.text, w.start, w.end)).collect();
            println!("    {}", ws.join(" "));
        }
    }
}
