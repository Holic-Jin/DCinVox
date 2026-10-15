//! 음소 강제정렬 (CTC forced alignment).
//! Whisper가 알아낸 "무슨 말을 했는지"를 음소 열로 바꾸고, 다국어 음소 인식 모델
//! (wav2vec2-xlsr-53-espeak-cv-ft, Apache-2.0)의 프레임별 확률 위에서 가장 그럴듯한 시간 배치를 찾는다.
//! 결과: 단어·음절 경계와 음절 안 모음 구간(마일스톤 5의 "모음만 늘이기"에 사용).

use std::collections::HashMap;
use std::path::Path;

use ort::session::Session;
use ort::value::Tensor;

use crate::g2p;

pub const MODEL_FILE: &str = "phoneme-aligner-int8.onnx";
pub const VOCAB_FILE: &str = "phoneme-aligner-vocab.json";
/// wav2vec2 프레임 간격 (16kHz에서 320샘플)
pub const FRAME_SECS: f64 = 0.02;
/// 자음이 모음 앞뒤로 차지할 수 있는 최대 길이(프레임). 앞 쉼이나 잡음이 자음으로 잡혀 늘어지는 것 방지
const MAX_ONSET_FRAMES: usize = 8; // 160ms
const MAX_CODA_FRAMES: usize = 10; // 200ms
const BLANK: usize = 0;

pub struct Aligner {
    session: Session,
    vocab: HashMap<String, usize>,
}

/// 정렬할 단어 하나 (색인의 단어 id와 발음형)
pub struct WordIn {
    pub token_id: i64,
    pub pron: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SyllableOut {
    pub unit: String,
    pub start: f64,
    pub end: f64,
    /// 모음(유성 중심) 구간 — 노래로 늘일 때 이 구간만 늘인다
    pub v_start: f64,
    pub v_end: f64,
}

#[derive(Debug, Clone)]
pub struct WordOut {
    pub token_id: i64,
    pub start: f64,
    pub end: f64,
    pub syllables: Vec<SyllableOut>,
}

/// 정렬용 음소 하나가 어느 단어·음절 소속인지
#[derive(Debug, Clone, Copy)]
struct PhoneRef {
    /// 음소 후보 집합 번호 (emission 표의 열, 0은 blank)
    class: usize,
    word: usize,
    syl: usize,
    is_vowel: bool,
}

impl Aligner {
    pub fn load(model: &Path, vocab: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(vocab).map_err(|e| tr!("align_vocab_failed", e))?;
        let vocab: HashMap<String, usize> = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
        let err = |e: &dyn std::fmt::Display| tr!("align_model_failed", e);
        let mut builder = Session::builder()
            .map_err(|e| err(&e))?
            .with_intra_threads(threads.saturating_sub(1).max(1))
            .map_err(|e| err(&e))?;
        let session = builder.commit_from_file(model).map_err(|e| err(&e))?;
        Ok(Self { session, vocab })
    }

    /// 16kHz 오디오 → 프레임별 로그확률 (frames × vocab)
    fn emissions(&mut self, pcm: &[f32]) -> Result<(Vec<f32>, usize, usize), String> {
        // wav2vec2 전처리: 구간별 평균 0, 분산 1
        let n = pcm.len().max(1) as f32;
        let mean = pcm.iter().sum::<f32>() / n;
        let var = pcm.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / n;
        let std = (var + 1e-7).sqrt();
        let input: Vec<f32> = pcm.iter().map(|x| (x - mean) / std).collect();

        let tensor = Tensor::from_array(([1usize, input.len()], input.into_boxed_slice())).map_err(|e| e.to_string())?;
        let outputs = self.session.run(ort::inputs![tensor]).map_err(|e| tr!("align_run_failed", e))?;
        let (shape, logits) = outputs[0].try_extract_tensor::<f32>().map_err(|e| e.to_string())?;
        let frames = shape[1] as usize;
        let v = shape[2] as usize;
        Ok((log_softmax(logits, frames, v), frames, v))
    }

    /// 한 문장 구간(`pcm`은 `offset`초부터 시작)의 단어들을 정렬. 음소로 못 바꾸는 단어가 있으면 None.
    pub fn align(&mut self, pcm: &[f32], offset: f64, words: &[WordIn]) -> Result<Option<Vec<WordOut>>, String> {
        let Some((phones, classes)) = self.phones(words) else { return Ok(None) };
        if phones.is_empty() {
            return Ok(None);
        }
        let (emis, frames, v) = self.emissions(pcm)?;
        let (ce, c) = class_emissions(&emis, frames, v, &classes);
        let labels: Vec<usize> = phones.iter().map(|p| p.class).collect();
        let Some((spans, _)) = viterbi(&ce, frames, c, &labels) else { return Ok(None) };
        Ok(Some(collect(words, &phones, &spans, frames, offset)))
    }

    /// 여러 조각을 한 번에(배치) 평가. 각 조각은 따로 정규화 후 뒤를 무음으로 채워 길이를 맞춘다
    /// (조각끼리 섞이지 않으므로 하나씩 평가하는 것과 같은 판단, 모델 호출은 1번).
    pub fn match_scores(&mut self, pieces: &[Vec<f32>], pron: &str) -> Result<Vec<Option<f32>>, String> {
        let words = [WordIn { token_id: 0, pron: pron.to_string() }];
        let Some((phones, classes)) = self.phones(&words) else { return Ok(vec![None; pieces.len()]) };
        if phones.is_empty() || pieces.is_empty() {
            return Ok(vec![None; pieces.len()]);
        }
        let len = pieces.iter().map(|p| p.len()).max().unwrap_or(0).max(400);
        let mut batch = vec![0.0f32; pieces.len() * len];
        for (b, p) in pieces.iter().enumerate() {
            let n = p.len().max(1) as f32;
            let mean = p.iter().sum::<f32>() / n;
            let std = (p.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / n + 1e-7).sqrt();
            for (o, x) in batch[b * len..].iter_mut().zip(p) {
                *o = (x - mean) / std;
            }
        }
        let tensor = Tensor::from_array(([pieces.len(), len], batch.into_boxed_slice())).map_err(|e| e.to_string())?;
        let outputs = self.session.run(ort::inputs![tensor]).map_err(|e| tr!("align_run_failed", e))?;
        let (shape, logits) = outputs[0].try_extract_tensor::<f32>().map_err(|e| e.to_string())?;
        let (frames, v) = (shape[1] as usize, shape[2] as usize);
        let labels: Vec<usize> = phones.iter().map(|p| p.class).collect();
        let mut out = Vec::with_capacity(pieces.len());
        for b in 0..pieces.len() {
            let raw = &logits[b * frames * v..(b + 1) * frames * v];
            let emis = log_softmax(raw, frames, v);
            let (ce, c) = class_emissions(&emis, frames, v, &classes);
            let score = match viterbi(&ce, frames, c, &labels) {
                Some((_, target)) => {
                    let free: f32 = (0..frames).map(|t| emis[t * v..(t + 1) * v].iter().cloned().fold(f32::NEG_INFINITY, f32::max)).sum();
                    (target - free) / frames.max(1) as f32
                }
                None => -10.0,
            };
            out.push(Some(score));
        }
        Ok(out)
    }

    /// 단어들 → 음소 열 + 후보 집합 목록. 한자만 있는 일본어 등 못 바꾸는 단어는 건너뛴다.
    fn phones(&self, words: &[WordIn]) -> Option<(Vec<PhoneRef>, Vec<Vec<usize>>)> {
        let mut classes: Vec<Vec<usize>> = Vec::new();
        let mut out = Vec::new();
        for (wi, w) in words.iter().enumerate() {
            let units = g2p::units(&w.pron);
            if units.is_empty() {
                continue;
            }
            let mut local = Vec::new();
            let mut ok = true;
            'word: for (si, unit) in units.iter().enumerate() {
                let Some((onset, vowel, coda)) = g2p::phones_unit(unit) else {
                    ok = false;
                    break;
                };
                for (list, is_vowel) in [(onset, false), (vowel, true), (coda, false)] {
                    for alts in list {
                        let ids: Vec<usize> = alts.iter().filter_map(|p| self.vocab.get(*p).copied()).collect();
                        if ids.is_empty() {
                            ok = false;
                            break 'word;
                        }
                        let class = match classes.iter().position(|c| *c == ids) {
                            Some(k) => k + 1,
                            None => {
                                classes.push(ids);
                                classes.len()
                            }
                        };
                        local.push(PhoneRef { class, word: wi, syl: si, is_vowel });
                    }
                }
            }
            if ok {
                out.extend(local);
            }
        }
        if out.is_empty() {
            return None;
        }
        Some((out, classes))
    }
}

fn log_softmax(logits: &[f32], frames: usize, v: usize) -> Vec<f32> {
    let mut out = vec![0.0f32; frames * v];
    for t in 0..frames {
        let row = &logits[t * v..(t + 1) * v];
        let max = row.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let lse = max + row.iter().map(|x| (x - max).exp()).sum::<f32>().ln();
        for (o, x) in out[t * v..(t + 1) * v].iter_mut().zip(row) {
            *o = x - lse;
        }
    }
    out
}

/// 후보 집합별 확률 = 후보들 확률의 합 (log-sum-exp). 열 0은 blank
fn class_emissions(emis: &[f32], frames: usize, v: usize, classes: &[Vec<usize>]) -> (Vec<f32>, usize) {
    let c = classes.len() + 1;
    let mut ce = vec![0.0f32; frames * c];
    for t in 0..frames {
        let row = &emis[t * v..(t + 1) * v];
        ce[t * c] = row[BLANK];
        for (k, ids) in classes.iter().enumerate() {
            let max = ids.iter().map(|&i| row[i]).fold(f32::NEG_INFINITY, f32::max);
            ce[t * c + k + 1] = max + ids.iter().map(|&i| (row[i] - max).exp()).sum::<f32>().ln();
        }
    }
    (ce, c)
}

/// CTC 강제정렬. 반환: (라벨별 (시작 프레임, 끝 프레임+1), 최적 경로 로그확률)
fn viterbi(emis: &[f32], frames: usize, v: usize, labels: &[usize]) -> Option<(Vec<(usize, usize)>, f32)> {
    // 확장 라벨: blank, l1, blank, l2, ..., blank
    let s_len = labels.len() * 2 + 1;
    let ext = |s: usize| if s % 2 == 0 { BLANK } else { labels[s / 2] };
    if frames < labels.len() {
        return None; // 음소보다 프레임이 적으면 불가능
    }
    const NEG: f32 = f32::NEG_INFINITY;
    let mut prev = vec![NEG; s_len];
    let mut cur = vec![NEG; s_len];
    let mut back = vec![0u8; frames * s_len]; // 0: 그대로, 1: 한 칸, 2: 두 칸
    prev[0] = emis[ext(0)];
    if s_len > 1 {
        prev[1] = emis[ext(1)];
    }
    for t in 1..frames {
        let row = &emis[t * v..(t + 1) * v];
        for s in 0..s_len {
            let mut best = prev[s];
            let mut step = 0u8;
            if s >= 1 && prev[s - 1] > best {
                best = prev[s - 1];
                step = 1;
            }
            if s >= 2 && s % 2 == 1 && ext(s) != ext(s - 2) && prev[s - 2] > best {
                best = prev[s - 2];
                step = 2;
            }
            cur[s] = if best == NEG { NEG } else { best + row[ext(s)] };
            back[t * s_len + s] = step;
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    let mut s = if s_len >= 2 && prev[s_len - 2] > prev[s_len - 1] { s_len - 2 } else { s_len - 1 };
    if prev[s] == NEG {
        return None;
    }
    let score = prev[s];
    let mut spans = vec![(usize::MAX, 0usize); labels.len()];
    for t in (0..frames).rev() {
        if s % 2 == 1 {
            let k = s / 2;
            spans[k].0 = spans[k].0.min(t);
            spans[k].1 = spans[k].1.max(t + 1);
        }
        if t > 0 {
            s -= back[t * s_len + s] as usize;
        }
    }
    spans.iter().all(|sp| sp.0 != usize::MAX).then_some((spans, score))
}

/// 음소 구간 → 단어·음절 시간.
/// CTC는 음소를 1~2프레임에 뾰족하게 찍으므로, 음절 경계 = 다음 음절 첫 음소 시작.
fn collect(words: &[WordIn], phones: &[PhoneRef], spans: &[(usize, usize)], frames: usize, offset: f64) -> Vec<WordOut> {
    let t = |f: usize| offset + f as f64 * FRAME_SECS;
    // 음절별 첫/마지막 음소 인덱스
    let mut syls: Vec<(usize, usize, usize, usize)> = Vec::new(); // (word, syl, first, last)
    for (i, p) in phones.iter().enumerate() {
        match syls.last_mut() {
            Some(last) if last.0 == p.word && last.1 == p.syl => last.3 = i,
            _ => syls.push((p.word, p.syl, i, i)),
        }
    }
    // 마지막 음소 뒤로 최대 3프레임(60ms)까지만 꼬리로 인정 (쉼까지 늘어지지 않게)
    let tail = |last_phone: usize, next_start: Option<usize>| {
        let end = spans[last_phone].1 + 3;
        next_start.map_or(end.min(frames), |n| end.min(n))
    };

    let mut out: Vec<WordOut> = words
        .iter()
        .map(|w| WordOut { token_id: w.token_id, start: 0.0, end: 0.0, syllables: Vec::new() })
        .collect();
    // 음절 시작 (모음 기준으로 자음 길이 제한)
    let syl_start = |first: usize, last: usize| {
        let v = (first..=last).find(|&i| phones[i].is_vowel).map_or(spans[first].0, |i| spans[i].0);
        spans[first].0.max(v.saturating_sub(MAX_ONSET_FRAMES))
    };
    for (k, &(wi, si, first, last)) in syls.iter().enumerate() {
        let start_f = syl_start(first, last);
        let next = syls.get(k + 1);
        let vowel: Vec<usize> = (first..=last).filter(|&i| phones[i].is_vowel).collect();
        let v_start = vowel.first().map_or(start_f, |&i| spans[i].0);
        let vowel_last_end = vowel.last().map_or(spans[last].1, |&i| spans[i].1);
        // 같은 단어 안이면 다음 음절 시작까지, 단어 끝이면 꼬리만. 단, 모음 뒤로 너무 길게는 안 감
        let end_f = match next {
            Some(n) if n.0 == wi => syl_start(n.2, n.3),
            Some(n) => tail(last, Some(syl_start(n.2, n.3))),
            None => tail(last, None),
        }
        .min(vowel_last_end + MAX_CODA_FRAMES)
        .max(start_f + 1);
        let coda = (first..=last).find(|&i| i > *vowel.last().unwrap_or(&first) && !phones[i].is_vowel);
        let v_end = coda.map_or(end_f, |i| spans[i].0).max(v_start + 1).min(end_f.max(v_start + 1));
        let unit = g2p::units(&words[wi].pron).get(si).cloned().unwrap_or_default();
        out[wi].syllables.push(SyllableOut { unit, start: t(start_f), end: t(end_f), v_start: t(v_start), v_end: t(v_end) });
    }
    for w in &mut out {
        if let (Some(f), Some(l)) = (w.syllables.first(), w.syllables.last()) {
            w.start = f.start;
            w.end = l.end;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 인위적인 확률표에서 라벨 위치를 정확히 찾는지
    #[test]
    fn viterbi_finds_label_positions() {
        let v = 4; // 0=blank, 1,2,3
        // 프레임: b 1 1 b 2 b b 3 b
        let seq = [0, 1, 1, 0, 2, 0, 0, 3, 0];
        let mut emis = vec![-10.0f32; seq.len() * v];
        for (t, &l) in seq.iter().enumerate() {
            emis[t * v + l] = -0.1;
        }
        let (spans, _) = viterbi(&emis, seq.len(), v, &[1, 2, 3]).unwrap();
        assert_eq!(spans, [(1, 3), (4, 5), (7, 8)]);
    }

    #[test]
    fn viterbi_handles_repeated_labels() {
        // "n n" (안나의 받침 ㄴ + 초성 ㄴ)은 사이에 blank가 반드시 필요
        let v = 2;
        let seq = [1, 0, 1];
        let mut emis = vec![-10.0f32; seq.len() * v];
        for (t, &l) in seq.iter().enumerate() {
            emis[t * v + l] = -0.1;
        }
        assert_eq!(viterbi(&emis, 3, v, &[1, 1]).unwrap().0, [(0, 1), (2, 3)]);
        assert!(viterbi(&emis[..2 * v], 2, v, &[1, 1]).is_none());
    }

    #[test]
    fn syllable_boundaries_from_phone_spans() {
        // "사랑" = s a | ɾ a ŋ
        let words = [WordIn { token_id: 7, pron: "사랑".into() }];
        let p = |syl, is_vowel| PhoneRef { class: 1, word: 0, syl, is_vowel };
        let phones = [p(0, false), p(0, true), p(1, false), p(1, true), p(1, false)];
        let spans = [(0, 1), (2, 3), (8, 9), (10, 11), (15, 16)];
        let out = collect(&words, &phones, &spans, 100, 10.0);
        let s = &out[0].syllables;
        assert_eq!(s[0].unit, "사");
        assert!((s[0].start - 10.0).abs() < 1e-9 && (s[0].end - 10.16).abs() < 1e-9);
        assert!((s[0].v_start - 10.04).abs() < 1e-9);
        assert!((s[1].start - 10.16).abs() < 1e-9);
        assert!((s[1].v_end - 10.30).abs() < 1e-9); // 받침 ㅇ 시작에서 모음 끝
        assert!((s[1].end - 10.38).abs() < 1e-9); // 마지막 음소 + 3프레임 꼬리
        assert_eq!(out[0].token_id, 7);
    }

    #[test]
    fn caps_long_consonants_before_vowel() {
        // "주": 자음 ㅈ이 20프레임(0.4초) 전부터 잡힌 경우 → 모음 8프레임 전까지만
        let words = [WordIn { token_id: 1, pron: "주".into() }];
        let p = |is_vowel| PhoneRef { class: 1, word: 0, syl: 0, is_vowel };
        let phones = [p(false), p(true)];
        let spans = [(0, 1), (20, 24)];
        let out = collect(&words, &phones, &spans, 100, 0.0);
        let s = &out[0].syllables[0];
        assert!((s.start - 0.24).abs() < 1e-9, "{}", s.start);
        assert!((s.v_start - 0.40).abs() < 1e-9);
    }
}

