//! 반주(MR) 분석: 빠르기(BPM), 첫 박 위치, 조성(키).
//! 음악 비전공자가 숫자를 몰라도 반주에 맞는 도레미 줄과 박자 격자가 자동으로 잡히게 하는 용도.

use rustfft::{num_complex::Complex, FftPlanner};

const FFT: usize = 2048;
const HOP: usize = 512;

pub struct MrAnalysis {
    pub bpm: f32,
    /// 첫 박 시각(초)
    pub first_beat: f64,
    /// 장조로 환산한 으뜸음 (0=C … 11=B). 단조면 나란한조의 장조 (예: A단조 → C)
    pub key: i32,
    pub minor: bool,
    /// 원래 단조의 으뜸음 (표시용)
    pub tonic: i32,
}

/// 모노 오디오 → 크기 스펙트로그램 (프레임 × 빈)
fn spectrogram(x: &[f32]) -> Vec<Vec<f32>> {
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(FFT);
    let window: Vec<f32> = (0..FFT).map(|i| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / FFT as f32).cos()).collect();
    let mut frames = Vec::new();
    let mut pos = 0;
    let mut buf = vec![Complex::new(0.0f32, 0.0); FFT];
    while pos + FFT <= x.len() {
        for i in 0..FFT {
            buf[i] = Complex::new(x[pos + i] * window[i], 0.0);
        }
        fft.process(&mut buf);
        frames.push(buf[..FFT / 2].iter().map(|c| c.norm()).collect());
        pos += HOP;
    }
    frames
}

/// 빠르기와 첫 박: 소리가 새로 시작되는 세기(스펙트럼 증가량)의 주기성으로 추정
fn tempo(spec: &[Vec<f32>], sr: f64) -> (f32, f64) {
    let hop_secs = HOP as f64 / sr;
    // 온셋 세기: 로그 크기가 늘어난 양의 합
    let mut env: Vec<f32> = spec
        .windows(2)
        .map(|w| w[1].iter().zip(&w[0]).map(|(b, a)| ((1.0 + b).ln() - (1.0 + a).ln()).max(0.0)).sum())
        .collect();
    if env.len() < 64 {
        return (100.0, 0.0);
    }
    // 느린 변화 제거
    let k = 16;
    let means: Vec<f32> = (0..env.len())
        .map(|i| {
            let (lo, hi) = (i.saturating_sub(k), (i + k + 1).min(env.len()));
            env[lo..hi].iter().sum::<f32>() / (hi - lo) as f32
        })
        .collect();
    for (e, m) in env.iter_mut().zip(&means) {
        *e = (*e - m).max(0.0);
    }
    // 60~180 BPM 범위 자기상관 (120 근처를 약간 선호 — 사람이 느끼는 박)
    let lag_of = |bpm: f64| 60.0 / bpm / hop_secs;
    let (lo, hi) = (lag_of(180.0).floor() as usize, lag_of(60.0).ceil() as usize);
    let ac = |lag: usize| -> f32 { env.iter().zip(&env[lag..]).map(|(a, b)| a * b).sum::<f32>() / (env.len() - lag) as f32 };
    let mut best = (lo, f32::MIN);
    for lag in lo..=hi.min(env.len() / 2) {
        let bpm = 60.0 / (lag as f64 * hop_secs);
        let prior = (-0.5 * ((bpm / 120.0).log2() / 0.9).powi(2)).exp() as f32;
        // 두 배 주기도 함께 맞으면 진짜 박일 가능성이 높음
        let score = (ac(lag) + 0.5 * ac((lag * 2).min(env.len() - 1))) * prior;
        if score > best.1 {
            best = (lag, score);
        }
    }
    // 포물선 보간으로 소수 단위 주기
    let l = best.0;
    let (y0, y1, y2) = (ac(l.saturating_sub(1)), ac(l), ac(l + 1));
    let denom = y0 - 2.0 * y1 + y2;
    let frac = if denom.abs() > 1e-9 { (0.5 * (y0 - y2) / denom).clamp(-0.5, 0.5) } else { 0.0 };
    let coarse = l as f64 + frac as f64;

    // 정밀 조정: 곡 전체에 박 빗(comb)을 대 보고 박마다 온셋 세기의 합이 가장 큰 간격·위상.
    // 곡이 길수록 작은 빠르기 오차도 뒤에서 크게 어긋나므로 0.02 BPM 단위까지 가려낼 수 있음
    let smooth: Vec<f32> = (0..env.len()).map(|i| env[i.saturating_sub(1)..(i + 2).min(env.len())].iter().sum::<f32>() / 3.0).collect();
    let at = |t: f64| -> f32 {
        let i = t.floor() as usize;
        if i + 1 >= smooth.len() {
            return 0.0;
        }
        let w = (t - i as f64) as f32;
        smooth[i] * (1.0 - w) + smooth[i + 1] * w
    };
    let comb = |lag: f64, phase: f64| -> f32 {
        let mut s = 0.0;
        let mut t = phase;
        while t < (smooth.len() - 1) as f64 {
            s += at(t);
            t += lag;
        }
        s
    };
    let bpm0 = 60.0 / (coarse * hop_secs);
    let mut best_fit = (coarse, 0.0f64, f32::MIN);
    let mut b = bpm0 - 2.5;
    while b <= bpm0 + 2.5 {
        let lag = 60.0 / b / hop_secs;
        let mut ph = 0.0;
        while ph < lag {
            let sc = comb(lag, ph);
            if sc > best_fit.2 {
                best_fit = (lag, ph, sc);
            }
            ph += 0.5;
        }
        b += 0.02;
    }
    let (lag, phase0, _) = best_fit;
    let bpm = (60.0 / (lag * hop_secs)) as f32;
    let phase_best = (phase0.round() as usize, 0.0f32);
    // 같은 위상의 박 중 실제로 소리가 시작된 첫 박 (조용한 도입부의 빈 박은 건너뜀)
    let beat_strength = |t: f64| -> f32 {
        let i = t.round() as usize;
        env[i.saturating_sub(1)..(i + 2).min(env.len())].iter().cloned().fold(0.0, f32::max)
    };
    let _ = phase_best;
    let beats: Vec<f64> = std::iter::successors(Some(phase0), |t| Some(t + lag)).take_while(|t| (*t as usize) < env.len()).collect();
    let mean = beats.iter().map(|&t| beat_strength(t)).sum::<f32>() / beats.len().max(1) as f32;
    let first_frame = beats.iter().copied().find(|&t| beat_strength(t) >= 0.3 * mean).unwrap_or(phase0);
    // 스펙트로그램 프레임 중심 기준 시각 (+1은 차분으로 한 칸 밀린 것)
    let first = (first_frame + 1.0) * hop_secs + FFT as f64 / 2.0 / sr;
    (bpm, first)
}

/// Krumhansl-Kessler 조성 프로파일
const MAJOR_PROFILE: [f32; 12] = [6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88];
const MINOR_PROFILE: [f32; 12] = [6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17];

/// 음높이 분포(12음) → (으뜸음, 단조 여부)
pub fn key_from_chroma(chroma: &[f32; 12]) -> (i32, bool) {
    let corr = |profile: &[f32; 12], root: usize| -> f32 {
        let pm = profile.iter().sum::<f32>() / 12.0;
        let cm = chroma.iter().sum::<f32>() / 12.0;
        let (mut num, mut dp, mut dc) = (0.0, 0.0, 0.0);
        for i in 0..12 {
            let p = profile[(i + 12 - root) % 12] - pm;
            let c = chroma[i] - cm;
            num += p * c;
            dp += p * p;
            dc += c * c;
        }
        num / (dp * dc).sqrt().max(1e-9)
    };
    let mut best = (0, false, f32::MIN);
    for root in 0..12 {
        for (minor, prof) in [(false, &MAJOR_PROFILE), (true, &MINOR_PROFILE)] {
            let c = corr(prof, root);
            if c > best.2 {
                best = (root as i32, minor, c);
            }
        }
    }
    (best.0, best.1)
}

fn chroma(spec: &[Vec<f32>], sr: f64) -> [f32; 12] {
    let mut c = [0.0f32; 12];
    for frame in spec {
        for (bin, &mag) in frame.iter().enumerate().skip(1) {
            let hz = bin as f64 * sr / FFT as f64;
            if !(55.0..=2000.0).contains(&hz) {
                continue;
            }
            let midi = 69.0 + 12.0 * (hz / 440.0).log2();
            c[(midi.round() as i64).rem_euclid(12) as usize] += mag * mag;
        }
    }
    c
}

/// 반주 분석 (모노 샘플). 빠르기는 곡 전체로 정밀하게, 조성은 앞 90초로
pub fn analyze(mono: &[f32], sr: u32) -> MrAnalysis {
    let spec = spectrogram(mono);
    let (bpm, first_beat) = tempo(&spec, sr as f64);
    let key_frames = ((90.0 * sr as f64) as usize / HOP).min(spec.len());
    let (tonic, minor) = key_from_chroma(&chroma(&spec[..key_frames], sr as f64));
    let key = if minor { (tonic + 3) % 12 } else { tonic };
    MrAnalysis { bpm, first_beat, key, minor, tonic }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// C장조 화음(C-F-G-C) + 120BPM 박자 소리로 만든 가짜 반주
    fn fake_mr(sr: u32, bpm: f64, offset: f64, secs: f64) -> Vec<f32> {
        let n = (secs * sr as f64) as usize;
        let beat = 60.0 / bpm;
        let chords: [[f64; 3]; 4] = [[60.0, 64.0, 67.0], [65.0, 69.0, 72.0], [67.0, 71.0, 74.0], [60.0, 64.0, 67.0]];
        (0..n)
            .map(|i| {
                let t = i as f64 / sr as f64;
                let bar = (((t / (beat * 4.0)) as usize) % 4).min(3);
                let mut v: f64 = chords[bar].iter().map(|m| (2.0 * std::f64::consts::PI * 440.0 * 2f64.powf((m - 69.0) / 12.0) * t).sin() * 0.08).sum();
                // 박마다 짧은 타격음
                if t >= offset {
                    let since = (t - offset) % beat;
                    if since < 0.03 {
                        let noise = ((i as u64).wrapping_mul(2654435761) % 1000) as f64 / 1000.0 - 0.5;
                        v += noise * (1.0 - since / 0.03) * 0.8;
                    }
                }
                v as f32
            })
            .collect()
    }

    #[test]
    fn detects_tempo_beat_and_key() {
        let sr = 22050;
        // 박 소리는 1.0초부터 (앞 1초는 화음만 있는 도입부)
        let a = analyze(&fake_mr(sr, 120.0, 1.0, 20.0), sr);
        assert!((a.bpm - 120.0).abs() < 2.0, "bpm {}", a.bpm);
        assert!((a.first_beat - 1.0).abs() < 0.05, "first beat {}", a.first_beat);
        assert_eq!((a.key, a.minor), (0, false), "key");
    }

    #[test]
    fn tempo_is_precise_over_whole_song() {
        let sr = 22050;
        let a = analyze(&fake_mr(sr, 90.5, 1.0, 120.0), sr);
        assert!((a.bpm - 90.5).abs() < 0.1, "bpm {}", a.bpm);
    }

    #[test]
    fn minor_maps_to_relative_major() {
        // A단조 음 분포 (A C E 강하게) → 장조 환산 C
        let mut c = [0.0f32; 12];
        for (pc, w) in [(9, 6.0), (0, 4.0), (4, 5.0), (2, 2.0), (11, 2.0), (5, 2.5), (7, 2.0)] {
            c[pc] = w;
        }
        let (tonic, minor) = key_from_chroma(&c);
        assert_eq!((tonic, minor), (9, true));
    }
}

// ---------------------------------------------------------------------------
// 원곡 멜로디 따기: 가수가 부르는 음을 음원에서 자동으로 뽑아 음표로.
// 1) 가운데 강조: 보컬은 보통 가운데 → |M| − |S| (M=(L+R)/2, S=(L−R)/2)로 좌우로 퍼진 반주를 줄임
// 2) 배음 합 세기: 후보 음마다 배음 위치 크기를 더해 가장 그럴듯한 음높이
// 3) 비터비로 음높이 선을 매끄럽게 잇고, 소리 크기가 작은 곳은 쉼
// 4) 음높이가 바뀌거나 크기가 꺼지는 곳에서 음표를 나눔 (같은 음 반복 "도도"도 구분)
// ---------------------------------------------------------------------------

const MEL_FFT: usize = 2048;
const MEL_HOP: usize = 256;
const MEL_LO: f32 = 45.0; // A2
const MEL_HI: f32 = 84.0; // C6
const MEL_STEP: f32 = 0.2; // 20cent 간격 후보
const MIN_NOTE_SECS: f64 = 0.09;

/// 뽑아낸 멜로디 음 (초 단위, MIDI 음 번호)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct MelodyNote {
    pub start: f64,
    pub end: f64,
    pub midi: u8,
}

/// 프레임별 멜로디 음높이 선
pub struct Contour {
    pub hop_secs: f64,
    /// 프레임 0의 중심 시각
    pub offset: f64,
    /// MIDI 음 번호(소수)
    pub pitch: Vec<f32>,
    pub voiced: Vec<bool>,
    pub energy: Vec<f32>,
}

impl Contour {
    pub fn time(&self, frame: usize) -> f64 {
        self.offset + frame as f64 * self.hop_secs
    }
    pub fn frame(&self, t: f64) -> usize {
        (((t - self.offset) / self.hop_secs).max(0.0) as usize).min(self.pitch.len().saturating_sub(1))
    }
}

/// 스테레오(L, R) 음원 → 멜로디 음표
pub fn extract_melody(left: &[f32], right: &[f32], sr: u32) -> Vec<MelodyNote> {
    match extract_contour(left, right, sr) {
        Some(c) => segment_notes(&c),
        None => Vec::new(),
    }
}

/// 스테레오(L, R) 음원 → 프레임별 멜로디 음높이
pub fn extract_contour(left: &[f32], right: &[f32], sr: u32) -> Option<Contour> {
    let n = left.len().min(right.len());
    if n < MEL_FFT {
        return None;
    }
    let sr_f = sr as f32;
    let hop_secs = MEL_HOP as f64 / sr as f64;
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(MEL_FFT);
    let window: Vec<f32> = (0..MEL_FFT).map(|i| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / MEL_FFT as f32).cos()).collect();
    let bin_hz = sr_f / MEL_FFT as f32;
    let cands: Vec<f32> = (0..=((MEL_HI - MEL_LO) / MEL_STEP) as usize).map(|i| MEL_LO + i as f32 * MEL_STEP).collect();
    let cand_hz: Vec<f32> = cands.iter().map(|m| 440.0 * 2f32.powf((m - 69.0) / 12.0)).collect();

    let mut sal: Vec<Vec<f32>> = Vec::new(); // 프레임 × 후보
    let mut energy: Vec<f32> = Vec::new();
    let mut bm = vec![Complex::new(0.0f32, 0.0); MEL_FFT];
    let mut bs = vec![Complex::new(0.0f32, 0.0); MEL_FFT];
    let mut pos = 0;
    while pos + MEL_FFT <= n {
        for i in 0..MEL_FFT {
            let (l, r) = (left[pos + i], right[pos + i]);
            bm[i] = Complex::new(0.5 * (l + r) * window[i], 0.0);
            bs[i] = Complex::new(0.5 * (l - r) * window[i], 0.0);
        }
        fft.process(&mut bm);
        fft.process(&mut bs);
        let mag: Vec<f32> = (0..MEL_FFT / 2).map(|k| (bm[k].norm() - bs[k].norm()).max(0.0)).collect();
        let lmag: Vec<f32> = mag.iter().map(|v| (1.0 + 50.0 * v).ln()).collect();
        let at = |hz: f32| -> f32 {
            let b = hz / bin_hz;
            let i = b as usize;
            if i + 1 >= lmag.len() {
                return 0.0;
            }
            // 가장 가까운 빈 주변의 최댓값 (음높이 오차 허용)
            lmag[i].max(lmag[i + 1])
        };
        let row: Vec<f32> = cand_hz
            .iter()
            .map(|&f0| (1..=10).map(|h| 0.8f32.powi(h - 1) * at(f0 * h as f32)).sum())
            .collect();
        let lo = (200.0 / bin_hz) as usize;
        let hi = ((3000.0 / bin_hz) as usize).min(mag.len());
        energy.push(mag[lo..hi].iter().map(|v| v * v).sum::<f32>().sqrt());
        sal.push(row);
        pos += MEL_HOP;
    }
    let frames = sal.len();
    let c = cands.len();

    // 비터비: 큰 음높이 점프에 벌점 → 매끄러운 선
    let jump_cost = 0.08f32;
    let max_jump = (7.0 / MEL_STEP) as isize; // 프레임 사이 최대 7반음
    let mut score = sal[0].clone();
    let mut back = vec![0u16; frames * c];
    for t in 1..frames {
        let mut next = vec![f32::NEG_INFINITY; c];
        for j in 0..c {
            let (lo, hi) = ((j as isize - max_jump).max(0) as usize, ((j as isize + max_jump) as usize).min(c - 1));
            let mut best = (lo, f32::NEG_INFINITY);
            for i in lo..=hi {
                let v = score[i] - jump_cost * (i as f32 - j as f32).abs();
                if v > best.1 {
                    best = (i, v);
                }
            }
            next[j] = best.1 + sal[t][j];
            back[t * c + j] = best.0 as u16;
        }
        // 숫자가 커지지 않게 정규화
        let m = next.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        score = next.into_iter().map(|v| v - m).collect();
    }
    let mut path = vec![0usize; frames];
    path[frames - 1] = (0..c).max_by(|&a, &b| score[a].total_cmp(&score[b])).unwrap_or(0);
    for t in (1..frames).rev() {
        path[t - 1] = back[t * c + path[t]] as usize;
    }

    // 유성(노래하는 중) 판정: 소리 크기와 배음 세기가 둘 다 충분할 때
    let mut sorted_e = energy.clone();
    sorted_e.sort_by(f32::total_cmp);
    // 큰 소리 구간(상위 5%) 대비 -20dB 이상이면 소리 있음
    let e_thr = sorted_e[sorted_e.len() * 95 / 100] * 0.1 + 1e-6;
    let peak_sal: Vec<f32> = (0..frames).map(|t| sal[t][path[t]]).collect();
    let mut sorted_s = peak_sal.clone();
    sorted_s.sort_by(f32::total_cmp);
    let s_thr = sorted_s[sorted_s.len() / 2] * 0.7;
    let voiced: Vec<bool> = (0..frames).map(|t| energy[t] > e_thr && peak_sal[t] > s_thr).collect();
    let pitch: Vec<f32> = path.iter().map(|&j| cands[j]).collect();
    Some(Contour { hop_secs, offset: MEL_FFT as f64 / 2.0 / sr as f64, pitch, voiced, energy })
}

/// 음높이 선 → 음표 (음높이가 바뀌거나 크기가 꺼지는 곳에서 나눔)
pub fn segment_notes(c: &Contour) -> Vec<MelodyNote> {
    let (pitch, voiced, energy, hop_secs) = (&c.pitch, &c.voiced, &c.energy, c.hop_secs);
    let frames = pitch.len();
    let mut notes = Vec::new();
    let mut t = 0;
    while t < frames {
        if !voiced[t] {
            t += 1;
            continue;
        }
        let start = t;
        let mut seg: Vec<f32> = vec![pitch[t]];
        t += 1;
        while t < frames && voiced[t] {
            let med = median_f32(&seg);
            // 3프레임 연속 0.8반음 이상 벗어나면 새 음
            let moved = (0..3).all(|k| t + k < frames && voiced[t + k] && (pitch[t + k] - med).abs() > 0.8);
            if moved {
                break;
            }
            seg.push(pitch[t]);
            t += 1;
        }
        // 같은 음 반복: 음 안에서 크기가 크게 꺼지는 곳에서 다시 나눔
        let mut cuts = vec![start];
        let smooth = |i: usize| -> f32 { energy[i.saturating_sub(1)..(i + 2).min(t)].iter().sum::<f32>() / 3.0 };
        let mut i = start + 3;
        while i + 3 < t {
            let left_peak = (cuts.last().copied().unwrap()..i).map(smooth).fold(0.0, f32::max);
            let right_peak = (i + 1..(i + 12).min(t)).map(smooth).fold(0.0, f32::max);
            let here = smooth(i);
            if here < 0.55 * left_peak.min(right_peak) && (i - cuts.last().unwrap()) as f64 * hop_secs >= MIN_NOTE_SECS {
                cuts.push(i);
                i += 4;
            } else {
                i += 1;
            }
        }
        cuts.push(t);
        for w in cuts.windows(2) {
            let (a, b) = (w[0], w[1]);
            let secs = (b - a) as f64 * hop_secs;
            if secs < MIN_NOTE_SECS {
                continue;
            }
            let midi = median_f32(&pitch[a..b]).round().clamp(0.0, 127.0) as u8;
            notes.push(MelodyNote { start: c.time(a), end: c.time(b), midi });
        }
    }
    notes
}

/// 멜로디 음역에서 크게 벗어난 음(베이스가 섞여 들어온 음 등) 제거
pub fn clean_melody(notes: Vec<MelodyNote>) -> Vec<MelodyNote> {
    if notes.len() < 5 {
        return notes;
    }
    let mut m: Vec<u8> = notes.iter().map(|n| n.midi).collect();
    m.sort();
    let mid = m[m.len() / 2] as i32;
    notes.into_iter().filter(|n| (mid - 10..=mid + 12).contains(&(n.midi as i32))).collect()
}

/// 멜로디 음 길이로 조성 추정 (반주 화음보다 정확). 반환: (장조 환산 으뜸음, 원래 으뜸음, 단조)
pub fn key_from_melody(notes: &[MelodyNote]) -> Option<(i32, i32, bool)> {
    if notes.len() < 20 {
        return None;
    }
    let mut c = [0.0f32; 12];
    for n in notes {
        c[(n.midi % 12) as usize] += (n.end - n.start) as f32;
    }
    let (tonic, minor) = key_from_chroma(&c);
    Some((if minor { (tonic + 3) % 12 } else { tonic }, tonic, minor))
}

/// 원곡 가수의 음절 하나 (가사 위치 + 그때 부른 음)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct OrigSyllable {
    pub start: f64,
    pub end: f64,
    /// 모음 구간 — 모음 시작이 박
    pub v_start: f64,
    pub v_end: f64,
    /// 이 음절에서 부른 음 (MIDI, 소수)
    pub midi: f32,
    /// 원곡에서 들린 소리 (발음형 음절) — 사용자 가사와 맞춰 보는 데 사용
    #[serde(default)]
    pub unit: String,
}

/// Whisper가 앞 줄(들)을 그대로 다시 적는 반복 오류 합치기.
/// 같은 내용이 바로 앞 1~2줄에 있고 부를 수 없을 만큼 빠르면(초당 5.5음절 초과) 가짜 → 그 시간을 진짜 줄에 붙임.
/// ("밤양갱 / 밤양갱"처럼 정상 속도로 실제 반복한 줄은 그대로 둠). 반환: (줄, 시작, 끝)
fn merge_repeated_lines(segs: &[crate::stt::Segment]) -> Vec<(&crate::stt::Segment, f64, f64)> {
    let norm = |s: &crate::stt::Segment| crate::index::normalize(&s.text);
    let mut kept: Vec<(&crate::stt::Segment, f64, f64)> = Vec::new();
    for seg in segs {
        let syl = crate::g2p::units(&crate::g2p::pron(&seg.text)).len() as f64;
        let too_fast = syl >= 4.0 && syl / (seg.end - seg.start).max(0.01) > 5.5;
        let text = norm(seg);
        let n = kept.len();
        if too_fast && n >= 1 && norm(kept[n - 1].0) == text {
            // A A' → A의 시간을 늘림
            kept[n - 1].2 = seg.end;
            continue;
        }
        if too_fast && n >= 2 && norm(kept[n - 2].0) == text {
            // A B A' → A'의 시간은 사실 B가 불린 시간
            kept[n - 1].2 = seg.end;
            continue;
        }
        kept.push((seg, seg.start, seg.end));
    }
    kept
}

/// 노래에서 나올 수 있는 최대 음절 빠르기 (랩 제외). 이보다 촘촘한 줄은 인식 오류로 봄
const MAX_SUNG_SYLLABLES_PER_SEC: f64 = 8.0;

/// 원곡 음절 찾기: Whisper로 원곡 가사를 받아 적고, 음소 정렬로 음절마다 위치를 찾은 뒤,
/// 모음 구간의 멜로디 음높이를 그 음절의 음으로. `mono16`은 16kHz 모노
/// `lyrics_hint`: 사용자가 쓴 가사 (원곡 가사와 같으면 알아듣기가 크게 좋아짐, 언어도 여기서 정함)
pub fn original_syllables(
    mono16: &[f32],
    contour: &Contour,
    recognizer: &crate::stt::Recognizer,
    aligner: &mut crate::align::Aligner,
    lyrics_hint: Option<&str>,
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> Result<Vec<OrigSyllable>, String> {
    let lang = lyrics_hint.and_then(|l| l.chars().any(crate::g2p::ko::is_hangul).then_some("ko"));
    let t = recognizer.transcribe_hinted(mono16, lang, lyrics_hint, |_| {}, cancel)?;
    if std::env::var("DCINVOX_DEBUG_LYRICS").is_ok() {
        eprintln!("[원곡 가사 {}] {}", t.lang, t.segments.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join(" / "));
        for seg in &t.segments {
            if (60.0..100.0).contains(&seg.start) {
                let ws: Vec<String> = seg.words.iter().map(|w| format!("{}@{:.1}-{:.1}", w.text, w.start, w.end)).collect();
                eprintln!("[구간] {:.2}-{:.2} {}", seg.start, seg.end, ws.join(" "));
            }
        }
    }
    let sr = crate::stt::SAMPLE_RATE as f64;
    let segs = merge_repeated_lines(&t.segments);
    // 줄(문장)마다 앞뒤 여유를 주되 이웃 줄과 겹치지 않게 — 겹치면 줄 끝 음절이 다음 줄 첫 음절 자리로 끌려감
    let first_of = |k: usize| segs.get(k).map(|s| s.1);
    let last_of = |k: usize| segs.get(k).map(|s| s.2);

    // 1차: 줄마다 정렬. 상태: 정상(Some, 듬성 아님) / 듬성(간주까지 잡힘) / 버림
    let mut lines: Vec<Option<Vec<OrigSyllable>>> = vec![None; segs.len()];
    let mut sparse = vec![false; segs.len()];
    let keys: Vec<String> = segs.iter().map(|x| crate::index::normalize(&x.0.text)).collect();
    for (k, &(seg, seg_from, seg_to)) in segs.iter().enumerate() {
        let words: Vec<crate::align::WordIn> = seg
            .words
            .iter()
            .map(|w| crate::align::WordIn { token_id: 0, pron: crate::g2p::pron(&w.text) })
            .filter(|w| !w.pron.is_empty())
            .collect();
        if words.is_empty() {
            continue;
        }
        // 반복 오류를 합친 줄은 합친 시간 전체를 씀
        let w_start = seg.words.iter().map(|w| w.start).fold(f64::INFINITY, f64::min).min(seg_from.max(seg.start));
        let w_end = seg.words.iter().map(|w| w.end).fold(0.0, f64::max).max(seg_to);
        let mut from = (w_start - 0.4).max(0.0);
        let mut to = w_end + 0.4;
        if let Some(prev_end) = k.checked_sub(1).and_then(last_of) {
            from = from.max((prev_end + 0.02).min(w_start));
        }
        if let Some(next_start) = first_of(k + 1) {
            to = to.min((next_start - 0.02).max(w_end));
        }
        let (a, b) = (((from * sr) as usize).min(mono16.len()), ((to * sr) as usize).min(mono16.len()));
        if b <= a {
            continue;
        }
        let Some(aligned) = aligner.align(&mono16[a..b], a as f64 / sr, &words)? else { continue };
        // 사람이 부를 수 없을 만큼 촘촘하면(초당 8음절 초과) Whisper가 같은 줄을 반복해 적은 환각 → 버림
        let syl_count: usize = aligned.iter().map(|w| w.syllables.len()).sum();
        let span = aligned.last().map_or(0.0, |w| w.end) - aligned.first().map_or(0.0, |w| w.start);
        if syl_count >= 4 && syl_count as f64 / span.max(0.01) > MAX_SUNG_SYLLABLES_PER_SEC {
            continue;
        }
        // 너무 듬성듬성(초당 1.5음절 미만) = 간주·악기 소리까지 가사로 잡은 것 → 2차에서 앞의 같은 줄로 고침
        sparse[k] = syl_count >= 4 && (syl_count as f64) / span.max(0.01) < 1.5;
        let mut line = Vec::new();
        for w in aligned {
            for syl in w.syllables {
                // 모음 구간에서 노래한 프레임들의 음높이 중앙값 (없으면 음절 전체)
                let pick = |s: f64, e: f64| -> Vec<f32> {
                    (contour.frame(s)..=contour.frame(e)).filter(|&f| contour.voiced[f]).map(|f| contour.pitch[f]).collect()
                };
                let mut p = pick(syl.v_start, syl.v_end);
                if p.is_empty() {
                    p = pick(syl.start, syl.end);
                }
                if p.is_empty() {
                    continue;
                }
                line.push(OrigSyllable { start: syl.start, end: syl.end, v_start: syl.v_start, v_end: syl.v_end, midi: median_f32(&p), unit: syl.unit });
            }
        }
        if !line.is_empty() {
            lines[k] = Some(line);
        }
    }

    // 2차: 듬성한 줄은 앞에서 제대로 찾은 같은 가사 줄(후렴 등)의 리듬·음을 옮겨 씀.
    // 자리는 이웃 줄 기준: 첫 번째 때 "다음 줄까지 걸린 시간"이 같도록 (간주 속에 잘못 놓이는 것 방지)
    let good = |k: usize, lines: &Vec<Option<Vec<OrigSyllable>>>, sparse: &Vec<bool>| !sparse[k] && lines[k].is_some();
    for k in 0..segs.len() {
        if !sparse[k] {
            continue;
        }
        let Some(j) = (0..k).find(|&j| keys[j] == keys[k] && good(j, &lines, &sparse)) else { continue };
        let tpl = lines[j].clone().unwrap();
        let first_v = |x: &Option<Vec<OrigSyllable>>| x.as_ref().and_then(|v| v.first()).map(|s| s.v_start);
        let last_v = |x: &Option<Vec<OrigSyllable>>| x.as_ref().and_then(|v| v.last()).map(|s| s.v_start);
        // ① 다음 줄이 같은 가사이고 둘 다 제대로 찾았으면, 다음 줄 첫 음절에서 거꾸로
        let by_next = (k + 1 < segs.len() && j + 1 < k && keys.get(j + 1) == keys.get(k + 1) && good(k + 1, &lines, &sparse) && good(j + 1, &lines, &sparse))
            .then(|| Some(first_v(&lines[k + 1])? - (first_v(&lines[j + 1])? - tpl[0].v_start)))
            .flatten();
        // ② 앞 줄이 같은 가사면 앞 줄 마지막 음절에서 앞으로
        let by_prev = (k >= 1 && j >= 1 && keys[j - 1] == keys[k - 1] && good(k - 1, &lines, &sparse) && good(j - 1, &lines, &sparse))
            .then(|| Some(last_v(&lines[k - 1])? + (tpl[0].v_start - last_v(&lines[j - 1])?)))
            .flatten();
        // ⓪ 가장 확실: 그 줄 시간 범위 안에서 같은 멜로디(음 패턴)가 실제로 들리는 곳
        let by_melody = melody_position(&tpl, contour, segs[k].1, segs[k].2);
        let anchor = by_melody.or(by_next).or(by_prev).or_else(|| first_v(&lines[k])).unwrap_or(segs[k].1);
        let shift = anchor - tpl[0].v_start;
        let shifted = |src: &[OrigSyllable], d: f64| -> Vec<OrigSyllable> {
            src.iter().map(|x| OrigSyllable { start: x.start + d, end: x.end + d, v_start: x.v_start + d, v_end: x.v_end + d, ..x.clone() }).collect()
        };
        lines[k] = Some(shifted(&tpl, shift));
        // 옮긴 줄과 다음 줄이 겹치면, 다음 줄도 첫 번째 때와 같은 간격으로 (다음 줄이 같은 가사일 때)
        if k + 1 < segs.len() && j + 1 < segs.len() && keys[j + 1] == keys[k + 1] && good(j + 1, &lines, &sparse) {
            let moved_last = last_v(&lines[k]).unwrap_or(anchor);
            if first_v(&lines[k + 1]).is_none_or(|t| t < moved_last + 0.1) {
                let next_tpl = lines[j + 1].clone().unwrap();
                let gap = next_tpl[0].v_start - last_v(&lines[j]).unwrap_or(tpl[0].v_start);
                let d = moved_last + gap - next_tpl[0].v_start;
                lines[k + 1] = Some(shifted(&next_tpl, d));
            }
        }
    }
    let mut out: Vec<OrigSyllable> = lines.into_iter().flatten().flatten().collect();
    out.sort_by(|a, b| a.v_start.total_cmp(&b.v_start));
    fix_late_starts(&mut out, contour);
    Ok(out)
}

/// 음절들의 음 패턴(`tpl`)이 [from, to] 안에서 시작한다고 할 때 실제 멜로디와 가장 잘 맞는 시작 음절 시각.
/// 음절의 70% 이상에서 노래가 들리고 평균 오차가 1.5반음 이하일 때만
pub fn melody_position(tpl: &[OrigSyllable], contour: &Contour, from: f64, to: f64) -> Option<f64> {
    let t0 = tpl.first()?.v_start;
    let dur = tpl.last()?.v_start - t0;
    let mut best: Option<(f64, f64)> = None;
    let _ = dur;
    // 시작점만 범위 안에 있으면 됨 (Whisper가 준 줄 범위가 실제 노래보다 짧게 끝나는 경우가 있음)
    let mut a = from;
    while a <= to {
        let (mut err, mut cnt) = (0.0, 0.0);
        for x in tpl {
            let f = contour.frame(x.v_start - t0 + a + 0.05);
            if contour.voiced[f] {
                let d = (contour.pitch[f] - x.midi).rem_euclid(12.0);
                err += d.min(12.0 - d) as f64;
                cnt += 1.0;
            }
        }
        if cnt / tpl.len() as f64 >= 0.7 {
            let e = err / cnt;
            if e <= 1.5 && best.is_none_or(|(_, b)| e < b) {
                best = Some((a, e));
            }
        }
        a += 0.02;
    }
    best.map(|(a, _)| a)
}

/// 같은 모음이 이어지면(예: "많아" = 마-나) 음소 정렬이 두 번째 음절 시작을 못 찾고 늦게 잡는다.
/// 두 음절 사이가 길고, 그 사이에 멜로디가 다음 음절의 음으로 바뀌는 곳이 있으면 거기를 다음 음절 시작으로
pub fn fix_late_starts(syls: &mut [OrigSyllable], contour: &Contour) {
    for i in 0..syls.len().saturating_sub(1) {
        let (a, b) = (syls[i].clone(), syls[i + 1].clone());
        if b.v_start - a.v_start <= 0.5 || (b.midi - a.midi).abs() < 0.8 {
            continue;
        }
        let (f0, f1) = (contour.frame(a.v_start + 0.15), contour.frame(b.v_start - 0.1));
        let near_b = |f: usize| contour.voiced[f] && (contour.pitch[f] - b.midi).abs() < 0.8;
        // 다음 음절 음으로 바뀌어 3프레임(약 35ms) 이상 유지되는 첫 곳
        if let Some(f) = (f0..f1.saturating_sub(2)).find(|&f| (0..3).all(|k| near_b(f + k))) {
            let t = contour.time(f);
            let nb = &mut syls[i + 1];
            nb.start = nb.start.min(t);
            nb.v_start = t;
        }
    }
}

/// 소수 MIDI 음 → 키 안의 가장 가까운 계이름
pub fn nearest_in_key(key: i32, midi: f32) -> i32 {
    const MAJOR: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];
    let rel = midi - (60 + key) as f32;
    let oct = (rel / 12.0).floor() as i32;
    (-1..=1)
        .flat_map(|o| (0..7).map(move |d| (oct + o, d)))
        .min_by(|a, b| {
            let pa = (12 * a.0 + MAJOR[a.1 as usize]) as f32;
            let pb = (12 * b.0 + MAJOR[b.1 as usize]) as f32;
            (pa - rel).abs().total_cmp(&(pb - rel).abs())
        })
        .map(|(o, d)| 7 * o + d)
        .unwrap_or(0)
}

/// 두 음절 소리가 얼마나 다른지 (0 같음, 0.5 모음만 같음, 1 다름)
fn syllable_cost(a: &str, b: &str) -> f32 {
    if a == b {
        return 0.0;
    }
    let vowel = |s: &str| s.chars().next().and_then(crate::g2p::ko::vowel_of);
    match (vowel(a), vowel(b)) {
        (Some(x), Some(y)) if x == y => 0.5,
        _ => 1.0,
    }
}

/// 사용자 음절 ↔ 원곡 음절을 **처음부터 순서대로** 짝짓기 (편집 거리 정렬, 시작은 고정·끝은 자유).
/// 기본은 i번째 ↔ i번째와 같고, 원곡 인식이 음절을 빠뜨리거나 더 넣은 곳에서만 소리를 보고 한두 칸 맞춤.
/// 가사가 원곡과 전혀 달라도(개사) 모두 "다름"이라 그냥 순서대로가 됨. 반환: 사용자 음절별 원곡 음절 번호
pub fn match_in_order(user: &[String], orig: &[String]) -> Vec<Option<usize>> {
    let (n, m) = (user.len(), orig.len());
    let mut d = vec![vec![0.0f32; m + 1]; n + 1];
    for i in 0..=n {
        d[i][0] = i as f32;
    }
    for j in 0..=m {
        d[0][j] = j as f32; // 원곡 앞부분을 건너뛰는 것도 감점 → 처음부터 시작
    }
    for i in 1..=n {
        for j in 1..=m {
            d[i][j] = (d[i - 1][j - 1] + syllable_cost(&user[i - 1], &orig[j - 1])).min(d[i - 1][j] + 1.0).min(d[i][j - 1] + 1.0);
        }
    }
    // 원곡 뒷부분이 남는 건 공짜 (가사가 원곡보다 짧아도 됨)
    let mut j = (0..=m).min_by(|&a, &b| d[n][a].total_cmp(&d[n][b])).unwrap_or(m);
    let mut i = n;
    let mut out = vec![None; n];
    while i > 0 && j > 0 {
        let diag = d[i - 1][j - 1] + syllable_cost(&user[i - 1], &orig[j - 1]);
        if (d[i][j] - diag).abs() < 1e-6 {
            out[i - 1] = Some(j - 1);
            i -= 1;
            j -= 1;
        } else if (d[i][j] - (d[i - 1][j] + 1.0)).abs() < 1e-6 {
            i -= 1;
        } else {
            j -= 1;
        }
    }
    out
}

/// 원곡 음절 → 음계 블록에 처음부터 순서대로 (사용자 음절 i ≈ 원곡에서 i번째로 부른 음절).
/// 모음 시작을 박으로, 그 음절에서 원곡 가수가 부른 음을 블록 음으로
pub fn apply_syllables(song: &mut crate::song::Song, syls: &[OrigSyllable], start: f64) -> usize {
    let spb = song.secs_per_beat();
    // 완전히 같은 시각에 두 번 잡힌 음절만 하나로 (가까이 붙은 음절을 지우면 뒤 음절이 전부 한 칸씩 밀림)
    let mut all: Vec<&OrigSyllable> = Vec::new();
    for s in syls.iter().filter(|s| s.v_start >= start - 0.1) {
        if all.last().is_none_or(|p| s.v_start - p.v_start >= 0.01) {
            all.push(s);
        }
    }
    let user: Vec<String> = song.notes.iter().map(|n| n.pron.clone()).collect();
    if user.iter().any(|u| u.is_empty()) || all.iter().any(|s| s.unit.is_empty()) {
        return assign(song, &all, start, spb);
    }
    let orig: Vec<String> = all.iter().map(|s| s.unit.clone()).collect();
    let pairs = match_in_order(&user, &orig);
    // 짝 없는 음절(원곡 인식이 빠뜨린 곳)은 앞뒤 짝 사이로 메우고, 음은 그 시각 근처 원곡 음절의 음
    let mut filled: Vec<OrigSyllable> = Vec::with_capacity(user.len());
    for (k, p) in pairs.iter().enumerate() {
        if let Some(j) = p {
            filled.push(all[*j].clone());
            continue;
        }
        let prev = pairs[..k].iter().rev().flatten().next().map(|&j| all[j]);
        let next = pairs[k + 1..].iter().flatten().next().map(|&j| all[j]);
        let t = match (prev, next) {
            (Some(a), Some(b)) => 0.5 * (a.v_start + b.v_start),
            (Some(a), None) => a.end,
            (None, Some(b)) => (b.v_start - 0.3).max(start),
            (None, None) => start,
        };
        let midi = prev.or(next).map_or(60.0, |s| s.midi);
        filled.push(OrigSyllable { start: t, end: t + 0.2, v_start: t, v_end: t + 0.2, midi, unit: String::new() });
    }
    // 시간 순서 유지
    for k in 1..filled.len() {
        if filled[k].v_start < filled[k - 1].v_start + 0.04 {
            let t = filled[k - 1].v_start + 0.04;
            filled[k].v_start = t;
            filled[k].start = t;
        }
    }
    assign(song, &filled.iter().collect::<Vec<_>>(), start, spb)
}

fn assign(song: &mut crate::song::Song, usable: &[&OrigSyllable], start: f64, spb: f64) -> usize {
    // 멜로디 가운데에서 한 옥타브 가까이 벗어난 음은 베이스 등이 섞인 것 → 가운데 쪽 옥타브로
    let mut ms: Vec<f32> = usable.iter().take(song.notes.len()).map(|s| s.midi).collect();
    ms.sort_by(f32::total_cmp);
    let center = ms.get(ms.len() / 2).copied().unwrap_or(60.0);
    let fold = |m: f32| -> f32 {
        let mut x = m;
        while x < center - 9.0 {
            x += 12.0;
        }
        while x > center + 9.0 {
            x -= 12.0;
        }
        x
    };
    song.line_rest = 0.0;
    let n = song.notes.len().min(usable.len());
    // 8분의 1박 격자. 각 음절은 원곡에서의 절대 위치를 기준으로 놓아 앞 음절의 지연이 뒤로 쌓이지 않게 함
    let q = |x: f64| ((x * 8.0).round() / 8.0) as f32;
    let mut cur_end = 0.0f32;
    for i in 0..n {
        let s = usable[i];
        let target = q((s.v_start - start).max(0.0) / spb);
        // 앞 음과 같은 칸으로 반올림됐으면 바로 뒤에 붙임 (지연은 이 음을 짧게 해서 흡수)
        let beat = target.max(cur_end);
        // 다음 음절까지가 이 음의 길이 (긴 쉼이 있으면 이 음절 끝에서 끊음)
        let next = usable.get(i + 1).map(|x| q((x.v_start - start) / spb));
        let own_end = q((s.end - start) / spb + 0.25);
        let end = next.map_or(own_end, |nx| if nx - own_end > 0.5 { own_end } else { nx }).max(beat + 0.125);
        let degree = nearest_in_key(song.key, fold(s.midi));
        let note = &mut song.notes[i];
        note.degree = degree;
        note.sharp = false;
        note.gap = beat - cur_end;
        note.beats = end - beat;
        cur_end = end;
    }
    let mut d: Vec<i32> = song.notes[..n].iter().map(|x| x.degree).collect();
    d.sort();
    if let Some(&mid) = d.get(d.len() / 2) {
        let shift = crate::song::center_shift(mid);
        for x in &mut song.notes {
            x.degree -= shift;
        }
    }
    n
}

/// 가운데(보컬) 강조 신호: 주파수마다 좌우 공통 성분만 남기는 마스크 (M × max(0, 1 − |S|/|M|))
pub fn center_signal(left: &[f32], right: &[f32]) -> Vec<f32> {
    let n = left.len().min(right.len());
    let (nfft, hop) = (2048usize, 512usize);
    if n < nfft {
        return left.iter().zip(right).map(|(a, b)| 0.5 * (a + b)).collect();
    }
    let mut planner = FftPlanner::<f32>::new();
    let fwd = planner.plan_fft_forward(nfft);
    let inv = planner.plan_fft_inverse(nfft);
    let win: Vec<f32> = (0..nfft).map(|i| (std::f32::consts::PI * i as f32 / nfft as f32).sin()).collect(); // √Hann
    let mut out = vec![0.0f32; n + nfft];
    let mut norm = vec![0.0f32; n + nfft];
    let mut m = vec![Complex::new(0.0f32, 0.0); nfft];
    let mut sd = vec![Complex::new(0.0f32, 0.0); nfft];
    let mut pos = 0;
    while pos + nfft <= n {
        for i in 0..nfft {
            m[i] = Complex::new(0.5 * (left[pos + i] + right[pos + i]) * win[i], 0.0);
            sd[i] = Complex::new(0.5 * (left[pos + i] - right[pos + i]) * win[i], 0.0);
        }
        fwd.process(&mut m);
        fwd.process(&mut sd);
        for k in 0..nfft {
            let mm = m[k].norm();
            let g = if mm > 1e-9 { (1.0 - sd[k].norm() / mm).max(0.0) } else { 0.0 };
            m[k] *= g;
        }
        inv.process(&mut m);
        for i in 0..nfft {
            out[pos + i] += m[i].re / nfft as f32 * win[i];
            norm[pos + i] += win[i] * win[i];
        }
        pos += hop;
    }
    out.truncate(n);
    out.iter().zip(&norm).map(|(v, w)| if *w > 1e-6 { v / w } else { 0.0 }).collect()
}

fn median_f32(v: &[f32]) -> f32 {
    let mut s = v.to_vec();
    s.sort_by(f32::total_cmp);
    s.get(s.len() / 2).copied().unwrap_or(0.0)
}

#[cfg(test)]
mod melody_tests {
    use super::*;

    /// 가운데 "목소리"(배음 많은 음 + 살짝 떨림)로 작은 별을 부르고, 좌우에는 반주 화음
    #[test]
    fn merges_whisper_repeated_lines() {
        let seg = |s: f64, e: f64, t: &str| crate::stt::Segment { start: s, end: e, text: t.into(), words: Vec::new() };
        let segs = vec![
            seg(51.74, 56.60, "나는 길에 네가 내게 말했지"),
            seg(56.60, 58.08, "너는 바라는 게 너무나 많아"),
            seg(58.08, 58.90, "나는 길에 네가 내게 말했지"), // 가짜 (0.8초에 12음절)
            seg(58.90, 60.70, "너는 바라는 게 너무나 많아"), // 가짜
            seg(60.70, 64.70, "아니야 내가 널 바란 건 하나야"),
            seg(64.70, 66.70, "밤양갱"),
            seg(66.70, 68.70, "밤양갱"), // 실제로 반복해 부른 줄 → 유지
        ];
        let m = merge_repeated_lines(&segs);
        let spans: Vec<(f64, f64)> = m.iter().map(|x| (x.1, x.2)).collect();
        assert_eq!(spans, [(51.74, 56.60), (56.60, 60.70), (60.70, 64.70), (64.70, 66.70), (66.70, 68.70)]);
    }

    #[test]
    fn in_order_matching_absorbs_missed_syllables() {
        let v = |s: &str| s.chars().map(|c| c.to_string()).collect::<Vec<_>>();
        // 원곡 인식이 "이야"와 "떠"를 빠뜨림 → 뒤 음절이 밀리지 않아야 함
        let user = v("밤양갱이야떠나는길에");
        let orig = v("밤양갱나는길에");
        let p = match_in_order(&user, &orig);
        assert_eq!(p[0..3], [Some(0), Some(1), Some(2)]);
        assert_eq!(p[6..], [Some(3), Some(4), Some(5), Some(6)]);
        // 가사가 완전히 달라도(개사) 처음부터 순서대로
        let parody = v("가나다라");
        assert_eq!(match_in_order(&parody, &v("하하하하하")), [Some(0), Some(1), Some(2), Some(3)]);
    }

    #[test]
    fn crowded_syllables_do_not_push_later_ones() {
        // 0.04~0.08초 간격으로 붙어 있는 음절 4개(같은 칸으로 반올림되는 것 포함) 뒤에, 10초 지점 음절
        let mut song = crate::song::Song { bpm: 90.0, ..Default::default() };
        song.notes = (0..5)
            .map(|i| crate::song::Note { clip: 0, syl: i, pron: String::new(), lyric: "라".into(), degree: 0, sharp: false, beats: 1.0, gap: 0.0, rest: false, source_hz: None })
            .collect();
        let s = |t: f64| OrigSyllable { start: t, end: t + 0.1, v_start: t, v_end: t + 0.1, midi: 60.0, unit: String::new() };
        let syls = [s(1.0), s(1.04), s(1.12), s(1.2), s(10.0)];
        apply_syllables(&mut song, &syls, 0.0);
        let starts = song.start_beats(&[]);
        let spb = 60.0 / 90.0;
        // 붙어 있어도 5음절 모두 배정되고, 마지막은 제자리
        assert!((starts[4] as f64 * spb - 10.0).abs() < spb / 8.0, "last starts at {}s", starts[4] as f64 * spb);
        assert!(starts.windows(2).all(|w| w[1] > w[0]));
    }

    #[test]
    fn moves_late_syllable_to_pitch_change() {
        // 12.5초로 늦게 잡힌 "나"(B=59): 멜로디는 11.78초부터 B
        let hop = 0.0116;
        let n = 1200;
        let pitch: Vec<f32> = (0..n).map(|f| if (f as f64 * hop) < 11.78 { 61.0 } else { 59.0 }).collect();
        let c = Contour { hop_secs: hop, offset: 0.0, pitch, voiced: vec![true; n], energy: vec![1.0; n] };
        let s = |t: f64, m: f32| OrigSyllable { start: t, end: t + 0.3, v_start: t, v_end: t + 0.3, midi: m, unit: String::new() };
        let mut syls = vec![s(11.5, 61.0), s(12.5, 59.0)];
        fix_late_starts(&mut syls, &c);
        assert!((syls[1].v_start - 11.78).abs() < 0.03, "{}", syls[1].v_start);
    }

    #[test]
    fn extracts_sung_melody_over_accompaniment() {
        let sr = 22050u32;
        let beat = 0.6;
        let melody = [60u8, 60, 67, 67, 69, 69, 67];
        let lens = [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 2.0];
        let lead = 0.5;
        let total = lead + lens.iter().sum::<f64>() * beat + 0.5;
        let n = (total * sr as f64) as usize;
        let mut left = vec![0.0f32; n];
        let mut right = vec![0.0f32; n];
        let mut t0 = lead;
        let mut phase = 0.0f64;
        for (k, (&m, &l)) in melody.iter().zip(&lens).enumerate() {
            let f = 440.0 * 2f64.powf((m as f64 - 69.0) / 12.0);
            let a = (t0 * sr as f64) as usize;
            let b = ((t0 + l * beat) * sr as f64) as usize;
            for i in a..b.min(n) {
                let tt = (i - a) as f64 / sr as f64;
                phase += 2.0 * std::f64::consts::PI * f * (1.0 + 0.004 * (2.0 * std::f64::consts::PI * 5.0 * tt).sin()) / sr as f64;
                // 음 앞뒤 30ms 페이드 (음절 사이 틈)
                let env = (tt / 0.03).min(1.0).min(((b - i) as f64 / sr as f64) / 0.03);
                let v: f64 = (1..=8).map(|h| (phase * h as f64).sin() / h as f64).sum::<f64>() * 0.15 * env;
                left[i] += v as f32;
                right[i] += v as f32;
            }
            t0 += l * beat;
            let _ = k;
        }
        // 반주: 왼쪽은 도미솔, 오른쪽은 파라도 (좌우로 퍼진 소리)
        for i in 0..n {
            let tt = i as f64 / sr as f64;
            let chord = |ms: [f64; 3]| -> f32 {
                ms.iter().map(|m| (2.0 * std::f64::consts::PI * 440.0 * 2f64.powf((m - 69.0) / 12.0) * tt).sin()).sum::<f64>() as f32 * 0.06
            };
            left[i] += chord([48.0, 52.0, 55.0]);
            right[i] += chord([53.0, 57.0, 60.0]);
        }
        let notes = extract_melody(&left, &right, sr);
        let got: Vec<u8> = notes.iter().map(|n| n.midi).collect();
        assert_eq!(got, melody, "notes {notes:?}");
        // 시작 시각 오차 60ms 이내
        let mut t = lead;
        for (n, l) in notes.iter().zip(lens) {
            assert!((n.start - t).abs() < 0.06, "start {} vs {}", n.start, t);
            t += l * beat;
        }
    }
}

/// 실제 음원: DCINVOX_TEST_SONG=<스테레오 22050Hz wav> cargo test --release melody_real -- --ignored --nocapture
#[cfg(test)]
mod melody_real {
    use super::*;

    #[test]
    #[ignore]
    fn melody_real() {
        let path = std::env::var("DCINVOX_TEST_SONG").unwrap();
        let mut r = hound::WavReader::open(path).unwrap();
        let s: Vec<f32> = r.samples::<i16>().map(|v| v.unwrap() as f32 / 32768.0).collect();
        let (l, rr): (Vec<f32>, Vec<f32>) = s.chunks(2).map(|c| (c[0], c[1])).unzip();
        let t = std::time::Instant::now();
        let notes = clean_melody(extract_melody(&l, &rr, 22050));
        println!("key from melody {:?}", key_from_melody(&notes));
        let mono: Vec<f32> = l.iter().zip(&rr).map(|(a, b)| 0.5 * (a + b)).collect();
        let a = analyze(&mono, 22050);
        println!("{} notes in {:.1}s  (bpm {:.2}, first beat {:.2}, key {} minor {})", notes.len(), t.elapsed().as_secs_f64(), a.bpm, a.first_beat, a.tonic, a.minor);
        let names = ["C", "C#", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B"];
        for n in notes.iter().take(60) {
            print!("{:.2}-{:.2} {}{}  ", n.start, n.end, names[(n.midi % 12) as usize], n.midi / 12 - 1);
        }
        println!();
        let durs: Vec<f64> = notes.iter().map(|n| n.end - n.start).collect();
        let mut d = durs.clone();
        d.sort_by(f64::total_cmp);
        println!("median dur {:.2}s, pitch range {}..{}", d[d.len() / 2], notes.iter().map(|n| n.midi).min().unwrap(), notes.iter().map(|n| n.midi).max().unwrap());
        let mut pc = [0usize; 12];
        for n in &notes {
            pc[(n.midi % 12) as usize] += 1;
        }
        println!("pitch classes {:?}", names.iter().zip(pc).collect::<Vec<_>>());
        // 32음절 노래에 로봇 오토튠(원곡 멜로디) 적용
        let mut song = crate::song::Song::default();
        song.notes = (0..32)
            .map(|i| crate::song::Note { clip: 0, syl: i, pron: String::new(), lyric: "라".into(), degree: 0, sharp: false, beats: 1.0, gap: 0.0, rest: false, source_hz: None })
            .collect();
        song.bpm = a.bpm.round();
        let (key, tonic, minor) = key_from_melody(&notes).unwrap();
        song.key = key;
        song.mr = Some(crate::song::Mr { file: "x".into(), start: a.first_beat, skip: 0.0, gain: 1.0, detected_bpm: a.bpm, tonic, minor, melody: notes.clone(), syllables: Vec::new(), syllables_for: String::new() });
        crate::song::apply_preset(&mut song, crate::song::Preset::Robot);
        let deg = ["도", "레", "미", "파", "솔", "라", "시"];
        let line: Vec<String> = song
            .notes
            .iter()
            .map(|n| format!("{}{}{}({}박{})", deg[n.degree.rem_euclid(7) as usize], if n.sharp { "#" } else { "" }, n.degree.div_euclid(7), n.beats, if n.gap > 0.0 { format!(" 쉼{}", n.gap) } else { String::new() }))
            .collect();
        println!("song key {} bpm {}: {}", song.key, song.bpm, line.join(" "));
    }
}

/// 가사 힌트 없이 원곡 음절 위치만: DCINVOX_TEST_SONG=<스테레오 22050 wav> cargo test --release unhinted_real -- --ignored --nocapture
#[cfg(test)]
mod unhinted_real {
    use super::*;

    #[test]
    #[ignore]
    fn unhinted_real() {
        let path = std::env::var("DCINVOX_TEST_SONG").unwrap();
        let mut r = hound::WavReader::open(&path).unwrap();
        let s: Vec<f32> = r.samples::<i16>().map(|v| v.unwrap() as f32 / 32768.0).collect();
        let (l, rr): (Vec<f32>, Vec<f32>) = s.chunks(2).map(|c| (c[0], c[1])).unzip();
        let contour = extract_contour(&l, &rr, 22050).unwrap();
        let center = center_signal(&l, &rr);
        let dir = std::env::temp_dir();
        let (a, b) = (dir.join("uh22.wav"), dir.join("uh16.wav"));
        let spec = hound::WavSpec { channels: 1, sample_rate: 22050, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
        let mut w = hound::WavWriter::create(&a, spec).unwrap();
        for v in &center {
            w.write_sample((v.clamp(-1.0, 1.0) * 32767.0) as i16).unwrap();
        }
        w.finalize().unwrap();
        crate::media::extract_wav(&a, &b, 16000).unwrap();
        let mono16 = crate::stt::read_wav(&b).unwrap();
        let rec = crate::stt::Recognizer::load(&crate::models::find(crate::models::WHISPER_MODEL).unwrap()).unwrap();
        let mdir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../models");
        let mut al = crate::align::Aligner::load(&mdir.join(crate::align::MODEL_FILE), &mdir.join(crate::align::VOCAB_FILE)).unwrap();
        let t = std::time::Instant::now();
        std::env::set_var("DCINVOX_DEBUG_LYRICS", "1");
        let syls = original_syllables(&mono16, &contour, &rec, &mut al, None, std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false))).unwrap();
        println!("took {:.0}s · 원곡 음절 {}", t.elapsed().as_secs_f64(), syls.len());
        let line: Vec<String> = syls.iter().filter(|s| (64.0..100.0).contains(&s.v_start)).map(|s| format!("{}@{:.2}", s.unit, s.v_start)).collect();
        println!("{}", line.join(" "));
        // 가장 촘촘한 1.5초 구간의 음절 수
        let densest = syls.iter().map(|a| syls.iter().filter(|b| b.v_start >= a.v_start && b.v_start < a.v_start + 1.5).count()).max().unwrap_or(0);
        println!("densest 1.5s window: {densest} syllables");
        // 첫 후렴(38초대 "달...") 음 패턴이 70~90초 사이 어디서 가장 잘 맞는지 (음높이 선과 비교)
        let tpl: Vec<&OrigSyllable> = syls.iter().filter(|x| (37.5..41.0).contains(&x.v_start)).collect();
        if !tpl.is_empty() {
            let t0 = tpl[0].v_start;
            let mut scores: Vec<(f64, f64)> = Vec::new();
            let mut a = 70.0;
            while a < 90.0 {
                let mut err = 0.0;
                let mut cnt = 0.0;
                for x in &tpl {
                    let f = contour.frame(x.v_start - t0 + a + 0.05);
                    if contour.voiced[f] {
                        let d = ((contour.pitch[f] - x.midi).rem_euclid(12.0)).min(12.0 - (contour.pitch[f] - x.midi).rem_euclid(12.0));
                        err += d as f64;
                        cnt += 1.0;
                    }
                }
                let voiced = cnt / tpl.len() as f64;
                scores.push((a, if voiced > 0.6 { err / cnt } else { 99.0 }));
                a += 0.05;
            }
            scores.sort_by(|x, y| x.1.total_cmp(&y.1));
            println!("후렴 음 패턴이 잘 맞는 위치(반음 오차): {:?}", scores.iter().take(8).map(|(a, e)| format!("{a:.2}s:{e:.2}")).collect::<Vec<_>>());
        }
        // 사용자 가사가 있으면 처음부터 순서대로 짝지은 결과를 O/. 로 (O = 같은 소리)
        if let Ok(f) = std::env::var("DCINVOX_TEST_LYRICS_FILE") {
            let lyrics = std::fs::read_to_string(f).unwrap();
            let user: Vec<String> = lyrics.split_whitespace().flat_map(|w| crate::g2p::units(&crate::g2p::pron(w))).collect();
            let orig: Vec<String> = syls.iter().filter(|x| x.v_start >= 3.5).map(|x| x.unit.clone()).collect();
            let pairs = match_in_order(&user, &orig);
            let row: String = pairs.iter().enumerate().map(|(k, p)| match p { None => '-', Some(j) => if syllable_cost(&user[k], &orig[*j]) < 1.0 { 'O' } else { '.' } }).collect();
            for (i, chunk) in row.chars().collect::<Vec<_>>().chunks(50).enumerate() {
                println!("{:3} {}", i * 50, chunk.iter().collect::<String>());
            }
            println!("일치율 {:.0}%", 100.0 * row.chars().filter(|&c| c == 'O').count() as f64 / row.len() as f64);
        }
    }
}
