//! 후보 재검증: 잘라낸 조각을 음소 인식 모델에 다시 들려 주고,
//! 목표 소리로 가장 또렷하게 들리는 후보를 고른다 ("어디서 왔나"가 아니라 "잘라 놓고 들으면 어떤가"로 판단).
//! (측정 결과: 앞뒤 이웃과 이어 붙여 평가하는 방식은 오히려 나빠져서 쓰지 않음)

use std::collections::HashMap;
use std::io::BufReader;
use std::path::Path;

use crate::align::Aligner;
use crate::compose::{Clip, ClipKind};
use crate::{analyze, stt};

/// 재검증할 후보 수 (많을수록 정확하지만 느림)
const TOP_K: usize = crate::compose::MAX_CANDIDATES;
/// 조각 앞뒤에 붙이는 무음 (모델이 짧은 조각도 안정적으로 듣게)
const PAD_SECS: f64 = 0.1;

type Wav16 = hound::WavReader<BufReader<std::fs::File>>;

fn read16(wav: &mut Wav16, start: f64, end: f64) -> Result<Vec<f32>, String> {
    let sr = stt::SAMPLE_RATE as f64;
    let total = wav.duration();
    let s = ((start.max(0.0) * sr) as u32).min(total);
    let e = ((end.max(0.0) * sr) as u32).min(total);
    wav.seek(s).map_err(|e| e.to_string())?;
    let pad = (PAD_SECS * sr) as usize;
    let mut out = vec![0.0f32; pad];
    for v in wav.samples::<i16>().take(e.saturating_sub(s) as usize) {
        out.push(v.map_err(|e| e.to_string())? as f32 / 32768.0);
    }
    out.extend(std::iter::repeat_n(0.0, pad));
    Ok(out)
}

/// 측정(bench)상 10 이상이면 효과가 포화 — 사실상 재검증 점수가 선택을 결정
pub const WEIGHT: f32 = 10.0;

fn open16<'a>(wavs: &'a mut HashMap<String, Wav16>, root: &Path, id: &str) -> Result<&'a mut Wav16, String> {
    if !wavs.contains_key(id) {
        let w = hound::WavReader::open(analyze::stt_wav(root, id)).map_err(|e| e.to_string())?;
        wavs.insert(id.to_string(), w);
    }
    Ok(wavs.get_mut(id).unwrap())
}

/// `weight`: 재검증 점수를 기존 점수에 얼마나 반영할지
pub fn rerank(root: &Path, clips: &mut [Clip], aligner: &mut Aligner, weight: f32, mut on_progress: impl FnMut(usize, usize)) -> Result<(), String> {
    let mut wavs: HashMap<String, Wav16> = HashMap::new();
    // 같은 소리·같은 구간은 한 번만 평가 (가사에 같은 음절이 반복될 때)
    let mut cache: HashMap<(String, String, i64, i64), Option<f32>> = HashMap::new();
    let total = clips.len();
    for (i, clip) in clips.iter_mut().enumerate() {
        on_progress(i, total);
        if clip.kind == ClipKind::Missing || clip.candidates.len() < 2 {
            continue;
        }
        let k = clip.candidates.len().min(TOP_K);
        let span = |c: &crate::compose::Source| (c.start + (c.end - c.start) * c.frac.0, c.start + (c.end - c.start) * c.frac.1);
        let key = |c: &crate::compose::Source| {
            let (s, e) = span(c);
            (clip.pron.clone(), c.asset_id.clone(), (s * 1000.0) as i64, (e * 1000.0) as i64)
        };
        // 분석 때 미리 들어 본 점수가 있으면 그대로 사용, 없는 것만 지금 계산
        for c in clip.candidates.iter().take(k) {
            if let Some(m) = c.check {
                cache.entry(key(c)).or_insert(Some(m));
            }
        }
        let todo: Vec<usize> = (0..k).filter(|&j| !cache.contains_key(&key(&clip.candidates[j]))).collect();
        let mut pieces = Vec::with_capacity(todo.len());
        for &j in &todo {
            let c = &clip.candidates[j];
            let (s, e) = span(c);
            pieces.push(read16(open16(&mut wavs, root, &c.asset_id)?, s, e)?);
        }
        let scores = aligner.match_scores(&pieces, &clip.pron)?;
        for (&j, m) in todo.iter().zip(scores) {
            cache.insert(key(&clip.candidates[j]), m);
        }
        for cand in clip.candidates.iter_mut().take(k) {
            if let Some(m) = cache[&key(cand)] {
                cand.check = Some(m);
                cand.score += weight * m;
            }
        }
        clip.candidates[..k].sort_by(|a, b| b.score.total_cmp(&a.score));
        clip.choice = 0;
    }
    on_progress(total, total);
    Ok(())
}
