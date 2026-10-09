//! 변환: 음절 조각 → 노래 (WORLD 보코더).
//! 음성학 검토에서 정한 원칙을 그대로 구현한다.
//! - 음높이: 유성(목소리가 울리는) 프레임만 목표 음으로 교체. 무성음(ㅅ·ㅎ)은 그대로
//! - 길이: 자음은 원래 길이 유지, **모음만** 늘이거나 줄여 음표 길이를 채움
//! - 박자: 모음이 시작하는 순간을 박에 맞추고, 앞 자음은 박보다 먼저 나오게 배치
//! - 포먼트(목소리 색)는 WORLD가 스펙트럼 포락선으로 유지 → 옥타브 자동 맞춤과 함께 다람쥐 소리 방지

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::compose::Clip;
use crate::render::{self, Wav};
use crate::song::{self, Song};
use crate::project;

const SR: i32 = project::OUTPUT_SAMPLE_RATE as i32;
pub const OUTPUT_FILE: &str = "converted.mp4";
pub const VOCAL_WAV: &str = "converted.wav";
pub const MIX_WAV: &str = "converted_mix.wav";
/// WORLD 분석 간격 (ms)
const FRAME_MS: f64 = 5.0;
/// 첫 음 자음이 들어갈 앞 여유 (초)
const LEAD_IN: f64 = 0.3;
const TARGET_RMS: f64 = 0.1;
const FADE_SECS: f64 = 0.005;

/// 변환 결과에서 음표 하나가 차지하는 위치 (UI 재생 커서·영상 늘이기에 사용)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteSpan {
    pub note: usize,
    /// 출력에서 이 음절 소리 전체 구간 (자음 포함)
    pub start: f64,
    pub end: f64,
    /// 모음 시작 = 박
    pub beat_at: f64,
}

/// 음표 하나가 원본에서 쓰는 구간
#[derive(Debug, Clone, Copy)]
pub struct SylSrc {
    pub start: f64,
    pub end: f64,
    pub v_start: f64,
    pub v_end: f64,
}

/// 음표 → 원본 음절 구간. 정렬된 음절이 있으면 그것, 없으면 조각을 음절 수로 나눠 근사
pub fn syllable_source(clips: &[Clip], song: &Song, note_idx: usize) -> Option<(String, SylSrc)> {
    let n = &song.notes[note_idx];
    if n.rest {
        return None;
    }
    let clip = clips.get(n.clip)?;
    let src = clip.source()?;
    if let Some(s) = src.syls.get(n.syl) {
        return Some((src.asset_id.clone(), SylSrc { start: s.start, end: s.end, v_start: s.v_start, v_end: s.v_end }));
    }
    let count = song.notes.iter().filter(|m| m.clip == n.clip).count().max(1) as f64;
    let (a, b) = (src.start + (src.end - src.start) * src.frac.0, src.start + (src.end - src.start) * src.frac.1);
    let len = (b - a) / count;
    let s = a + len * n.syl as f64;
    Some((src.asset_id.clone(), SylSrc { start: s, end: s + len, v_start: s + len * 0.25, v_end: s + len * 0.8 }))
}

/// 아주 작은 잡음을 섞은 f64 신호 (완전한 무음·완전한 주기음에서 WORLD 분석이 깨지는 것 방지)
fn dithered(samples: &[f32]) -> Vec<f64> {
    let mut seed: u32 = 0x1234_5678;
    samples
        .iter()
        .map(|&v| {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            v as f64 + (seed as f64 / u32::MAX as f64 - 0.5) * 2e-5
        })
        .collect()
}

/// 음절 하나의 원래 음높이(유성 프레임 중앙값, Hz)
pub fn measure_f0(samples: &[f32]) -> Option<f64> {
    if samples.len() < (SR as usize) / 50 {
        return None;
    }
    let x = dithered(samples);
    let mut opt = rsworld_sys::HarvestOption::new();
    opt.frame_period = FRAME_MS;
    opt.f0_floor = 60.0;
    opt.f0_ceil = 800.0;
    let (_, f0) = rsworld::harvest(&x, SR, &opt);
    median(f0.into_iter().filter(|&f| f > 0.0).collect())
}

fn median(mut v: Vec<f64>) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    v.sort_by(f64::total_cmp);
    Some(v[v.len() / 2])
}

struct Analysis {
    f0: Vec<f64>,
    sp: Vec<Vec<f64>>,
    ap: Vec<Vec<f64>>,
}

fn analyze_world(x: &[f64], with_envelope: bool) -> Analysis {
    let x = x.to_vec();
    let mut hopt = rsworld_sys::HarvestOption::new();
    hopt.frame_period = FRAME_MS;
    hopt.f0_floor = 60.0;
    hopt.f0_ceil = 800.0;
    let (tpos, f0) = rsworld::harvest(&x, SR, &hopt);
    if !with_envelope {
        return Analysis { f0, sp: Vec::new(), ap: Vec::new() };
    }
    let mut copt = rsworld_sys::CheapTrickOption::new(SR);
    let sp = rsworld::cheaptrick(&x, SR, &tpos, &f0, &mut copt);
    let mut ap = rsworld::d4c(&x, SR, &tpos, &f0, &rsworld_sys::D4COption::new());
    // D4C는 잡음이 거의 없는 구간에서 NaN을 낼 수 있음 → 완전 주기(0.001)로 간주
    for v in ap.iter_mut().flatten() {
        if !v.is_finite() {
            *v = 0.001;
        }
    }
    Analysis { f0, sp, ap }
}

/// 원래 음높이의 잔떨림 비율 (f0 / 부드럽게 다듬은 f0). 큰 억양은 없애고 사람 목소리의 미세한 흔들림만 남김
fn micro_wobble(f0: &[f64]) -> Vec<f64> {
    let half = 5; // ±25ms
    (0..f0.len())
        .map(|i| {
            if f0[i] <= 0.0 {
                return 1.0;
            }
            let lo = i.saturating_sub(half);
            let hi = (i + half + 1).min(f0.len());
            let smooth = median(f0[lo..hi].iter().copied().filter(|&v| v > 0.0).collect()).unwrap_or(f0[i]);
            (f0[i] / smooth).clamp(0.94, 1.06)
        })
        .collect()
}

fn vibrato_factor(t_in_vowel: f64) -> f64 {
    // 모음이 0.15초 이상 이어지면 ±30cent, 5.5Hz
    if t_in_vowel <= 0.15 {
        return 1.0;
    }
    2f64.powf(0.3 / 12.0 * (2.0 * std::f64::consts::PI * 5.5 * t_in_vowel).sin())
}

/// 자음·모음·받침 경계 (프레임)
struct Regions {
    fv0: usize,
    fv1: usize,
    frames: usize,
}

fn regions(syl: SylSrc, frames: usize) -> Regions {
    let fp = FRAME_MS / 1000.0;
    let fv0 = (((syl.v_start - syl.start) / fp).round().max(0.0) as usize).min(frames.saturating_sub(1));
    let fv1 = (((syl.v_end - syl.start) / fp).round() as usize).clamp(fv0 + 1, frames);
    Regions { fv0, fv1, frames }
}

/// 음절 하나를 노래로: 목표 음높이 `target_hz`, 모음 시작부터 음표 끝까지 `note_secs`.
/// 반환: (출력 샘플, 모음 시작 전 자음 길이(초))
pub fn sing_syllable(
    samples: &[f32],
    syl: SylSrc,
    target_hz: f64,
    note_secs: f64,
    flat: bool,
    vibrato: bool,
    engine: song::Engine,
) -> (Vec<f32>, f64) {
    if samples.len() < (SR as usize) / 50 {
        return (samples.to_vec(), 0.0);
    }
    match engine {
        song::Engine::Vocoder => sing_vocoder(samples, syl, target_hz, note_secs, flat, vibrato, false),
        song::Engine::Smooth => sing_vocoder(samples, syl, target_hz, note_secs, flat, vibrato, true),
        song::Engine::Psola => sing_psola(samples, syl, target_hz, note_secs, flat, vibrato)
            .unwrap_or_else(|| sing_vocoder(samples, syl, target_hz, note_secs, flat, vibrato, true)),
    }
}

/// A/B: WORLD 보코더. `smooth`면 미세 떨림 유지 + 프레임 보간 + 자음·무성 받침은 원음 그대로
fn sing_vocoder(samples: &[f32], syl: SylSrc, target_hz: f64, note_secs: f64, flat: bool, vibrato: bool, smooth: bool) -> (Vec<f32>, f64) {
    let fp = FRAME_MS / 1000.0;
    let x = dithered(samples);
    let Analysis { f0, sp, ap } = analyze_world(&x, true);
    let Regions { fv0, fv1, frames } = regions(syl, f0.len());
    let onset = fv0 as f64 * fp;
    let coda = (frames - fv1) as f64 * fp;
    let vowel_out = (((note_secs - coda) / fp).round() as usize).max(1);
    let vlen = fv1 - fv0;

    // 출력 프레임 → 원본 위치(소수). 자음·받침은 그대로, 모음만 늘이거나 줄임
    let mut pos: Vec<f64> = (0..fv0).map(|i| i as f64).collect();
    pos.extend((0..vowel_out).map(|j| {
        if smooth && vowel_out > 1 {
            fv0 as f64 + j as f64 * (vlen - 1) as f64 / (vowel_out - 1) as f64
        } else {
            (fv0 + (j * vlen / vowel_out).min(vlen - 1)) as f64
        }
    }));
    pos.extend((fv1..frames).map(|i| i as f64));

    // 모음 구간이 대부분 유성이면 전부 유성으로 (평평한 소리가 중간에 끊기지 않게)
    let voiced_ratio = (fv0..fv1).filter(|&i| f0[i] > 0.0).count() as f64 / vlen as f64;
    let src_med = median((fv0..fv1).map(|i| f0[i]).filter(|&f| f > 0.0).collect()).unwrap_or(target_hz);
    let ratio = target_hz / src_med;
    let wobble = micro_wobble(&f0);

    let mut new_f0 = Vec::with_capacity(pos.len());
    let mut new_sp = Vec::with_capacity(pos.len());
    let mut new_ap = Vec::with_capacity(pos.len());
    for (k, &p) in pos.iter().enumerate() {
        let i0 = p.floor() as usize;
        let i1 = (i0 + 1).min(frames - 1);
        let w = if smooth { p - i0 as f64 } else { 0.0 };
        let in_vowel = k >= fv0 && k < fv0 + vowel_out;
        let src_f0 = if w > 0.0 && f0[i0] > 0.0 && f0[i1] > 0.0 { f0[i0] * (1.0 - w) + f0[i1] * w } else { f0[i0] };
        let voiced = src_f0 > 0.0 || (in_vowel && voiced_ratio >= 0.5);
        let mut f = if !voiced {
            0.0
        } else if flat {
            target_hz * if smooth { wobble[i0] } else { 1.0 }
        } else {
            (if src_f0 > 0.0 { src_f0 } else { src_med }) * ratio
        };
        if vibrato && voiced && in_vowel {
            f *= vibrato_factor((k - fv0) as f64 * fp);
        }
        new_f0.push(f);
        // 스펙트럼은 로그 영역에서 섞어야 자연스러움
        new_sp.push(if w > 0.0 {
            sp[i0].iter().zip(&sp[i1]).map(|(a, b)| (a.ln() * (1.0 - w) + b.ln() * w).exp()).collect()
        } else {
            sp[i0].clone()
        });
        let mut a: Vec<f64> =
            if w > 0.0 { ap[i0].iter().zip(&ap[i1]).map(|(a, b)| a * (1.0 - w) + b * w).collect() } else { ap[i0].clone() };
        // 억지로 유성으로 만든 프레임은 비주기성을 낮춰 목소리가 울리게
        if voiced && src_f0 == 0.0 {
            a.iter_mut().for_each(|v| *v = v.min(0.1));
        }
        new_ap.push(a);
    }
    let y = rsworld::synthesis(&new_f0, &new_sp, &new_ap, FRAME_MS, SR);
    let mut y: Vec<f32> = y.into_iter().map(|v| v as f32).collect();

    if smooth {
        // 자음은 보코더를 거치지 않은 원음으로 (ㅅ·ㅊ·ㅎ 질감 유지)
        let spf = (fp * SR as f64) as usize;
        let n_on = (fv0 * spf).min(samples.len()).min(y.len());
        splice(&mut y, 0, &samples[..n_on], false, true);
        // 받침이 대부분 무성이면 원음으로
        let coda_unvoiced = (fv1..frames).filter(|&i| f0[i] <= 0.0).count() * 2 > frames - fv1;
        if coda_unvoiced && frames > fv1 {
            let src = &samples[(fv1 * spf).min(samples.len())..];
            let at = y.len().saturating_sub(src.len());
            let n = src.len().min(y.len() - at);
            splice(&mut y, at, &src[..n], true, false);
        }
    }
    (y, onset)
}

/// `y[at..]`에 `src`를 덮어쓰기. 경계는 10ms 교차 페이드
fn splice(y: &mut [f32], at: usize, src: &[f32], fade_in: bool, fade_out: bool) {
    let xf = ((0.01 * SR as f64) as usize).min(src.len() / 2).max(1);
    for (k, &v) in src.iter().enumerate() {
        let Some(out) = y.get_mut(at + k) else { break };
        let mut w = 1.0f32;
        if fade_in && k < xf {
            w = k as f32 / xf as f32;
        }
        if fade_out && k + xf >= src.len() {
            w = w.min((src.len() - k) as f32 / xf as f32);
        }
        *out = *out * (1.0 - w) + v * w;
    }
}

/// C: TD-PSOLA. 원래 파형을 음높이 한 주기(×2 창)씩 잘라, 목표 음높이 간격으로 다시 겹쳐 붙인다.
/// 모음은 시간 축을 늘이고(같은 주기를 여러 번 사용), 무성 자음은 원음 그대로. 유성 구간이 없으면 None
fn sing_psola(samples: &[f32], syl: SylSrc, target_hz: f64, note_secs: f64, flat: bool, vibrato: bool) -> Option<(Vec<f32>, f64)> {
    let sr = SR as f64;
    let spf = FRAME_MS / 1000.0 * sr;
    let x = dithered(samples);
    let Analysis { f0, .. } = analyze_world(&x, false);
    let Regions { fv0, fv1, frames } = regions(syl, f0.len());
    let n = samples.len();
    let frame_of = |s: f64| ((s / spf) as usize).min(frames - 1);

    // 분석 피치마크: 유성 구간에서 한 주기마다 파형 봉우리
    let mut marks: Vec<(usize, f64)> = Vec::new(); // (위치, 주기 샘플 수)
    let mut p = 0.0f64;
    while (p as usize) < n {
        let f = f0[frame_of(p)];
        if f > 0.0 {
            let period = sr / f;
            let lo = (p - period * 0.25).max(0.0) as usize;
            let hi = ((p + period * 0.25) as usize).min(n - 1);
            let peak = (lo..=hi).max_by(|&a, &b| samples[a].abs().total_cmp(&samples[b].abs())).unwrap_or(p as usize);
            marks.push((peak, period));
            p = peak as f64 + period;
        } else {
            p += spf;
        }
    }
    let vs = ((fv0 as f64 * spf) as usize).min(n);
    let ve = ((fv1 as f64 * spf) as usize).clamp(vs, n);
    if !marks.iter().any(|m| m.0 >= vs && m.0 < ve) {
        return None;
    }
    let src_med = median((fv0..fv1).map(|i| f0[i]).filter(|&f| f > 0.0).collect()).unwrap_or(target_hz);
    let wobble = micro_wobble(&f0);

    // 구간 [a, b)를 길이 out_len으로 다시 만들기. 유성이면 PSOLA, 무성 자음이면 원음 그대로
    let region = |a: usize, b: usize, out_len: usize, is_vowel: bool| -> Vec<f32> {
        if b <= a {
            return Vec::new();
        }
        let (fa, fb) = (frame_of(a as f64), frame_of((b - 1) as f64));
        let voiced = (fa..=fb).filter(|&i| f0[i] > 0.0).count() * 2 > fb - fa + 1;
        if !voiced && !is_vowel {
            return samples[a..b].to_vec();
        }
        let mut out = vec![0.0f32; out_len];
        let mut t = 0.0f64;
        while (t as usize) < out_len {
            let src_t = a as f64 + t * (b - a) as f64 / out_len.max(1) as f64;
            // 가장 가까운 분석 마크의 한 주기를 가져옴 (모음을 늘이면 같은 주기를 여러 번 사용)
            let idx = marks.partition_point(|m| (m.0 as f64) < src_t);
            let nearest = [idx.checked_sub(1), Some(idx)]
                .into_iter()
                .flatten()
                .filter_map(|i| marks.get(i))
                .min_by(|x, y| (x.0 as f64 - src_t).abs().total_cmp(&(y.0 as f64 - src_t).abs()));
            let Some(&(mpos, mper)) = nearest else { break };
            let fr = frame_of(src_t);
            let mut hz = if flat || f0[fr] <= 0.0 { target_hz * wobble[fr] } else { f0[fr] * target_hz / src_med };
            if vibrato && is_vowel {
                hz *= vibrato_factor(t / sr);
            }
            let out_period = (sr / hz).max(8.0);
            // 주기 2개 길이 Hann 창. 간격이 좁아지면 겹침이 늘어나므로 크기 보정
            let half = mper.round() as isize;
            let gain = (out_period / mper).min(1.5) as f32;
            for k in -half..half {
                let si = mpos as isize + k;
                let oi = t as isize + k;
                if si < 0 || si as usize >= n || oi < 0 || oi as usize >= out_len {
                    continue;
                }
                let w = (0.5 * (1.0 + (std::f64::consts::PI * k as f64 / half as f64).cos())) as f32;
                out[oi as usize] += samples[si as usize] * w * gain;
            }
            t += out_period;
        }
        out
    };

    let coda_len = n - ve;
    let vowel_len = ((note_secs - coda_len as f64 / sr) * sr).round().max(spf) as usize;
    let onset = region(0, vs, vs, false);
    let vowel = region(vs, ve, vowel_len, true);
    let coda = region(ve, n, coda_len, false);
    // 구간 경계는 짧게 교차 페이드
    let mut y = onset;
    let at = y.len();
    y.resize(at + vowel.len(), 0.0);
    splice(&mut y, at, &vowel, at > 0, false);
    let at = y.len();
    y.resize(at + coda.len(), 0.0);
    splice(&mut y, at, &coda, true, false);
    Some((y, vs as f64 / sr))
}

pub struct ConvertOutput {
    pub audio: Vec<f32>,
    pub spans: Vec<NoteSpan>,
    /// 음표별 사용한 원본 구간 (영상 늘이기용)
    pub sources: Vec<Option<(String, SylSrc)>>,
}

/// 노래 전체 변환 (48kHz 모노)
pub fn convert(root: &Path, clips: &[Clip], song: &Song, mut on_progress: impl FnMut(usize, usize)) -> Result<ConvertOutput, String> {
    let spb = song.secs_per_beat();
    let starts = song.start_beats(clips);
    let targets = song::fold_octaves(song, clips);
    let end_beat = starts.last().copied().unwrap_or(0.0) + song.notes.last().map_or(0.0, |n| n.beats);
    // 반주가 있으면 노래 첫 박 = 반주의 start초, 없으면 짧은 앞 여유
    let lead = song.mr.as_ref().map_or(LEAD_IN, |m| (m.start - m.skip).max(0.0));
    let total_secs = lead + end_beat as f64 * spb + 1.0;
    let mut audio = vec![0.0f32; (total_secs * SR as f64) as usize];
    let mut wavs: HashMap<String, Wav> = HashMap::new();
    let mut spans = Vec::with_capacity(song.notes.len());
    let mut sources = Vec::with_capacity(song.notes.len());

    for (i, note) in song.notes.iter().enumerate() {
        on_progress(i, song.notes.len());
        let beat_at = lead + starts[i] as f64 * spb;
        let note_secs = note.beats as f64 * spb;
        let src = syllable_source(clips, song, i);
        sources.push(src.clone());
        let (Some((asset, syl)), Some(target)) = (src, targets[i]) else {
            spans.push(NoteSpan { note: i, start: beat_at, end: beat_at + note_secs, beat_at });
            continue;
        };
        let wav = render::wav_for(&mut wavs, root, &asset)?;
        let samples = render::read_range(wav, syl.start, syl.end)?;
        let (mut y, onset) = sing_syllable(&samples, syl, target, note_secs, song.flat, song.vibrato, song.engine);

        // 음량 맞춤 + 끝부분 짧은 페이드
        let rms = (y.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / y.len().max(1) as f64).sqrt();
        let gain = if rms > 1e-5 { (TARGET_RMS / rms).clamp(0.25, 8.0) as f32 } else { 1.0 };
        let fade = ((FADE_SECS * SR as f64) as usize).min(y.len() / 2);
        let n = y.len();
        for (k, v) in y.iter_mut().enumerate() {
            let edge = k.min(n - 1 - k);
            let f = if edge < fade { edge as f32 / fade.max(1) as f32 } else { 1.0 };
            *v *= gain * f;
        }
        // 자음은 박보다 먼저 시작 (모음 시작 = 박)
        let start = (beat_at - onset).max(0.0);
        let at = (start * SR as f64) as usize;
        if audio.len() < at + y.len() {
            audio.resize(at + y.len(), 0.0);
        }
        for (k, v) in y.iter().enumerate() {
            audio[at + k] = (audio[at + k] + v).clamp(-1.0, 1.0);
        }
        spans.push(NoteSpan { note: i, start, end: start + y.len() as f64 / SR as f64, beat_at });
    }
    on_progress(song.notes.len(), song.notes.len());
    Ok(ConvertOutput { audio, spans, sources })
}

pub const MR_CACHE: &str = "mr.wav";
/// 반주를 목소리와 비슷한 크기로 맞출 때 기준 (-23 dBFS, 목소리보다 살짝 작게)
const MR_TARGET_RMS: f64 = 0.07;

/// 목소리(모노) + 반주(스테레오) → 스테레오 섞은 소리 (L, R 번갈아). 길이는 노래 길이
pub fn mix_with_mr(root: &Path, vocal: &[f32], mr: &song::Mr) -> Result<Vec<f32>, String> {
    let path = root.join(project::CACHE_DIR).join(MR_CACHE);
    let mut r = hound::WavReader::open(&path).map_err(|e| format!("반주 읽기 실패: {e}"))?;
    let samples: Vec<f32> = r.samples::<i16>().map(|v| v.map(|x| x as f32 / 32768.0)).collect::<Result<_, _>>().map_err(|e| e.to_string())?;
    let frames = samples.len() / 2;
    let rms = (samples.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / samples.len().max(1) as f64).sqrt();
    let g = if rms > 1e-5 { (MR_TARGET_RMS / rms) as f32 * mr.gain } else { mr.gain };
    // 노래(영상)가 끝나면 반주가 더 있어도 거기서 끝. 마지막 1초 동안 반주를 줄여 자연스럽게
    let len = vocal.len();
    let fade = (SR as usize).min(len);
    // 건너뛴 전주만큼 반주를 앞당겨 읽음
    let skip = ((mr.skip.max(0.0) * SR as f64) as usize).min(frames);
    let mut out = vec![0.0f32; len * 2];
    for i in 0..len {
        let v = vocal[i];
        let fo = if i + fade >= len { (len - i) as f32 / fade as f32 } else { 1.0 };
        for ch in 0..2 {
            let m = if i + skip < frames { samples[(i + skip) * 2 + ch] * g * fo } else { 0.0 };
            // 부드러운 클리핑 방지
            out[i * 2 + ch] = (v + m).tanh();
        }
    }
    Ok(out)
}

/// 생성 결과로 노래 초기화: 음절마다 블록, 원래 음높이 측정 후 프리셋 적용
pub fn init_song(root: &Path, clips: &[Clip], prev: Option<&Song>, preset: song::Preset) -> Result<Song, String> {
    // 템포·키·반주 등 설정은 이전 노래에서 이어받고 음표만 새로 만든다
    let mut s = Song { notes: song::notes_from_clips(clips), ..prev.cloned().unwrap_or_default() };
    let mut wavs: HashMap<String, Wav> = HashMap::new();
    for i in 0..s.notes.len() {
        if let Some((asset, syl)) = syllable_source(clips, &s, i) {
            let wav = render::wav_for(&mut wavs, root, &asset)?;
            let x = render::read_range(wav, syl.v_start, syl.v_end.max(syl.v_start + 0.03))?;
            s.notes[i].source_hz = measure_f0(&x).map(|f| f as f32);
        }
    }
    match preset {
        // 생성 직후에는 멜로디를 자동 적용하지 않고 목소리 그대로의 음계로
        song::Preset::Robot => song::voice_degrees(&mut s),
        p => song::apply_preset(&mut s, p),
    }
    Ok(s)
}

/// 변환 결과 → 영상 구간: 음표마다 원본 음절 영상을 다음 음표 시작까지 늘여 채움. 쉼은 검은 화면
pub fn video_segments(out: &ConvertOutput, total: f64) -> Vec<render::VideoSeg> {
    let mut segs = Vec::new();
    let mut t = 0.0;
    for (i, span) in out.spans.iter().enumerate() {
        if span.start > t + 1e-3 {
            segs.push(render::VideoSeg { t0: t, t1: span.start, src: None });
            t = span.start;
        }
        let next = out.spans.get(i + 1).map_or(span.end, |n| n.start).max(span.end.min(total));
        let t1 = out.spans.get(i + 1).map_or(next, |n| n.start.min(next.max(n.start)));
        if t1 <= t {
            continue;
        }
        let src = out.sources[i].as_ref().map(|(asset, syl)| (asset.clone(), syl.start, syl.end));
        segs.push(render::VideoSeg { t0: t, t1, src });
        t = t1;
    }
    if total > t + 1e-3 {
        segs.push(render::VideoSeg { t0: t, t1: total, src: None });
    }
    segs
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 220Hz 목소리 비슷한 음 → 목표 330Hz, 0.6초로 늘였을 때 세 엔진 모두 음높이·길이 확인
    #[test]
    fn sings_at_target_pitch_and_length() {
        let sr = SR as f64;
        let len = (0.25 * sr) as usize;
        // 실제 목소리처럼 음높이가 살짝 흔들리는 배음 (완전히 일정한 합성음은 Harvest가 무성으로 판단함)
        let mut phase = 0.0f64;
        let samples: Vec<f32> = (0..len)
            .map(|i| {
                let t = i as f64 / sr;
                phase += 2.0 * std::f64::consts::PI * 220.0 * (1.0 + 0.01 * (2.0 * std::f64::consts::PI * 4.0 * t).sin()) / sr;
                (0.3 * phase.sin() + 0.2 * (2.0 * phase).sin() + 0.1 * (3.0 * phase).sin()) as f32
            })
            .collect();
        // 앞 0.05초를 자음, 0.05~0.2를 모음으로 가정
        let syl = SylSrc { start: 0.0, end: 0.25, v_start: 0.05, v_end: 0.2 };
        for engine in [song::Engine::Vocoder, song::Engine::Smooth, song::Engine::Psola] {
            let (y, onset) = sing_syllable(&samples, syl, 330.0, 0.6, true, false, engine);
            assert!((onset - 0.05).abs() < 0.006, "{engine:?} onset {onset}");
            let out_secs = y.len() as f64 / sr;
            // 자음 0.05 + 음표 0.6 (받침 0.05 포함) ≈ 0.65
            assert!((out_secs - 0.65).abs() < 0.03, "{engine:?} length {out_secs}");
            // 늘인 모음 구간 음높이가 목표 ±25cent (B·C는 원래 잔떨림을 남기므로 조금 넓게)
            let mid: Vec<f64> = y[(0.15 * sr) as usize..(0.5 * sr) as usize].iter().map(|&v| v as f64).collect();
            let mut dopt = rsworld_sys::DioOption::new();
            dopt.frame_period = FRAME_MS;
            let (t, f) = rsworld::dio(&mid, SR, &dopt);
            let f = rsworld::stonemask(&mid, SR, &t, &f);
            let hz = median(f.into_iter().filter(|&v| v > 0.0).collect()).unwrap();
            let cents = 1200.0 * (hz / 330.0).log2();
            assert!(cents.abs() < 25.0, "{engine:?} pitch {hz}Hz ({cents:.1} cents)");
        }
    }
}

/// 실제 프로젝트로 확인: DCINVOX_TEST_PROJECT=<폴더> cargo test --release convert_real -- --ignored --nocapture
#[cfg(test)]
mod real_project {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    #[test]
    #[ignore]
    fn convert_real() {
        let root = PathBuf::from(std::env::var("DCINVOX_TEST_PROJECT").unwrap());
        let words = crate::index::load_words(&crate::index::open(&root).unwrap()).unwrap();
        let lyrics = std::env::var("DCINVOX_TEST_LYRICS").unwrap_or_else(|_| "반짝반짝 작은 별\n아름답게 비치네".into());
        let mut clips = crate::compose::compose(&lyrics, &words);
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../models");
        let mut aligner = crate::align::Aligner::load(&dir.join(crate::align::MODEL_FILE), &dir.join(crate::align::VOCAB_FILE)).unwrap();
        crate::verify::rerank(&root, &mut clips, &mut aligner, crate::verify::WEIGHT, |_, _| {}).unwrap();

        let mut song = init_song(&root, &clips, None, song::Preset::Robot).unwrap();
        // 작은 별: 도도솔솔라라솔- 파파미미레레도-
        let melody = [0, 0, 4, 4, 5, 5, 4, 3, 3, 2, 2, 1, 1, 0];
        for (n, d) in song.notes.iter_mut().zip(melody.iter().cycle()) {
            n.degree = *d;
        }
        if let Some(last) = song.notes.last_mut() {
            last.beats = 2.0;
        }
        let t0 = std::time::Instant::now();
        let out = convert(&root, &clips, &song, |_, _| {}).unwrap();
        println!("convert {:.1}s, {} notes, {:.2}s audio", t0.elapsed().as_secs_f64(), song.notes.len(), out.audio.len() as f64 / SR as f64);

        // 음표마다 실제 음높이가 목표와 얼마나 맞는지 (DIO, 모음 가운데 구간)
        let targets = song::fold_octaves(&song, &clips);
        let spb = song.secs_per_beat();
        let mut errs = Vec::new();
        for (i, span) in out.spans.iter().enumerate() {
            let Some(target) = targets[i] else { continue };
            let a = ((span.beat_at + 0.05) * SR as f64) as usize;
            let b = ((span.beat_at + song.notes[i].beats as f64 * spb * 0.7) * SR as f64) as usize;
            let seg: Vec<f64> = out.audio[a..b.min(out.audio.len())].iter().map(|&v| v as f64).collect();
            let mut dopt = rsworld_sys::DioOption::new();
            dopt.frame_period = FRAME_MS;
            let (t, f) = rsworld::dio(&seg, SR, &dopt);
            let f = rsworld::stonemask(&seg, SR, &t, &f);
            if let Some(hz) = median(f.into_iter().filter(|&v| v > 0.0).collect()) {
                let c = 1200.0 * (hz / target).log2();
                errs.push(c.abs());
                println!("  {} target {:.0}Hz (src {:?}) → {:.0}Hz ({:+.0} cents)", song.notes[i].lyric, target, song.notes[i].source_hz.map(|v| v as i32), hz, c);
            }
        }
        errs.sort_by(f64::total_cmp);
        println!("pitch error median {:.1} cents, max {:.1} cents ({} notes)", errs[errs.len() / 2], errs.last().unwrap(), errs.len());

        let wav = root.join(project::CACHE_DIR).join("converted.wav");
        render::write_wav(&wav, &out.audio).unwrap();
        let t1 = std::time::Instant::now();
        let total = out.audio.len() as f64 / SR as f64;
        render::render_video(&root, &video_segments(&out, total), &wav, "converted.mp4", render::PREVIEW_SIZE, |_, _| {}, Arc::new(AtomicBool::new(false))).unwrap();
        println!("video {:.1}s", t1.elapsed().as_secs_f64());
    }

    /// UI 확인용: 생성+음계가 들어간 프로젝트 화면 데이터를 JSON으로 저장
    #[test]
    #[ignore]
    fn dump_view() {
        let root = PathBuf::from(std::env::var("DCINVOX_TEST_PROJECT").unwrap());
        let words = crate::index::load_words(&crate::index::open(&root).unwrap()).unwrap();
        let lyrics = "반짝반짝 작은 별\n아름답게 비치네";
        let mut clips = crate::compose::compose(lyrics, &words);
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../models");
        let mut aligner = crate::align::Aligner::load(&dir.join(crate::align::MODEL_FILE), &dir.join(crate::align::VOCAB_FILE)).unwrap();
        crate::verify::rerank(&root, &mut clips, &mut aligner, crate::verify::WEIGHT, |_, _| {}).unwrap();
        let song = init_song(&root, &clips, None, song::Preset::Robot).unwrap();
        let mut proj = project::open_or_create(&root).unwrap();
        proj.lyrics = lyrics.into();
        proj.generation = Some(project::Generation { lyrics: lyrics.into(), clips, timeline: Vec::new() });
        proj.song = Some(song);
        let view = project::view(&root, &proj);
        std::fs::write(std::env::var("DCINVOX_DUMP").unwrap(), serde_json::to_string(&view).unwrap()).unwrap();
    }

    /// 같은 노래를 세 엔진으로: DCINVOX_TEST_PROJECT=<프로젝트> DCINVOX_OUT=<폴더> cargo test --release engines_ab -- --ignored --nocapture
    #[test]
    #[ignore]
    fn engines_ab() {
        let root = PathBuf::from(std::env::var("DCINVOX_TEST_PROJECT").unwrap());
        let out_dir = PathBuf::from(std::env::var("DCINVOX_OUT").unwrap());
        std::fs::create_dir_all(&out_dir).unwrap();
        let proj = project::open_or_create(&root).unwrap();
        let clips = proj.generation.unwrap().clips;
        let mut song = proj.song.unwrap();
        for (name, engine) in [("A_현재_보코더", song::Engine::Vocoder), ("B_부드러운_보코더", song::Engine::Smooth), ("C_PSOLA_원음질감", song::Engine::Psola)] {
            song.engine = engine;
            let t = std::time::Instant::now();
            let out = convert(&root, &clips, &song, |_, _| {}).unwrap();
            let wav = out_dir.join(format!("{name}.wav"));
            render::write_wav(&wav, &out.audio).unwrap();
            println!("{name}: {:.1}s ({} notes)", t.elapsed().as_secs_f64(), song.notes.len());
        }
    }

    /// 반주 → 분석 → 변환·섞기 → MIDI → 고화질 영상: DCINVOX_TEST_PROJECT=<복사본> DCINVOX_TEST_MR=<음원> cargo test --release mr_export_real -- --ignored --nocapture
    #[test]
    #[ignore]
    fn mr_export_real() {
        let root = PathBuf::from(std::env::var("DCINVOX_TEST_PROJECT").unwrap());
        let mr_src = PathBuf::from(std::env::var("DCINVOX_TEST_MR").unwrap());
        let cache = root.join(project::CACHE_DIR);
        crate::media::extract_wav_ch(&mr_src, &cache.join(MR_CACHE), project::OUTPUT_SAMPLE_RATE, 2).unwrap();
        crate::media::extract_wav_ch(&mr_src, &cache.join("mr_analysis.wav"), 22_050, 1).unwrap();
        let mono: Vec<f32> = hound::WavReader::open(cache.join("mr_analysis.wav")).unwrap().samples::<i16>().map(|v| v.unwrap() as f32 / 32768.0).collect();
        let a = crate::music::analyze(&mono, 22_050);
        println!("MR: bpm {:.1}, first beat {:.2}s, key {} (tonic {}, minor {})", a.bpm, a.first_beat, a.key, a.tonic, a.minor);

        let words = crate::index::load_words(&crate::index::open(&root).unwrap()).unwrap();
        let clips = crate::compose::compose("반짝반짝 작은 별", &words);
        let mut s = init_song(&root, &clips, None, song::Preset::Robot).unwrap();
        s.bpm = a.bpm.round();
        s.key = a.key;
        s.mr = Some(song::Mr { file: "fake_mr.mp3".into(), start: a.first_beat, skip: 0.0, gain: 1.0, detected_bpm: a.bpm, tonic: a.tonic, minor: a.minor, melody: Vec::new(), syllables: Vec::new(), syllables_for: String::new() });
        let out = convert(&root, &clips, &s, |_, _| {}).unwrap();
        // 첫 음표의 박이 반주 첫 박과 같은 시각인지
        println!("first note beat_at {:.3}s (MR first beat {:.3}s)", out.spans[0].beat_at, a.first_beat);
        let mix = mix_with_mr(&root, &out.audio, s.mr.as_ref().unwrap()).unwrap();
        let mix_path = cache.join(MIX_WAV);
        render::write_wav_ch(&mix_path, &mix, 2).unwrap();
        let total = mix.len() as f64 / 2.0 / SR as f64;
        let peak = mix.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        println!("mix {:.2}s stereo, peak {:.2}", total, peak);

        let midi_bytes = crate::midi::write_midi(&s, &clips).unwrap();
        let back = crate::midi::read_melody(&midi_bytes).unwrap();
        println!("midi {} bytes, {} notes, bpm {:?}", midi_bytes.len(), back.notes.len(), back.bpm);

        let t = std::time::Instant::now();
        render::render_video(&root, &video_segments(&out, total), &mix_path, "export.mp4", render::EXPORT_SIZE, |_, _| {}, Arc::new(AtomicBool::new(false))).unwrap();
        println!("export video {:.1}s", t.elapsed().as_secs_f64());
    }
}
