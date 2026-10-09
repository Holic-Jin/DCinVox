//! 원곡 멜로디 따기 정확도 측정 (개발용).
//! 정답을 아는 노래를 만든다: 분석된 목소리로 알려진 멜로디를 부르게 하고(변환), 반주(좌우 화음·가운데 베이스·킥)를 섞음.
//! 그 음원에서 멜로디를 따서 가사 음절에 배정한 결과가 정답 음과 얼마나 맞는지 음절 단위로 잰다.
//!
//! 측정 기록 (3곡, 음절 단위): 음표로 나눠 순서대로 21.5% → 원곡 음절(가사 힌트 + 가사 맞춤) 72.7%, 시작 오차 4s → 0ms.
//! 이후 사용자 요구로 가사 맞춤은 빼고 "원곡에서 부른 음절에 처음부터 순서대로"로 바뀜. 이 측정용 노래는
//! 기계음 목소리라 힌트 없는 Whisper가 잘 못 알아들음 → 실제 가수 곡(밤양갱)에서는 힌트 없이 300음절을 정확히 찾음.
//! (모음 확률로 음절 찾기는 반주 속에서 모음이 거의 안 잡혀 실패 — 제거)
//!
//! DCINVOX_TEST_PROJECT=<분석된 프로젝트 복사본> cargo test --release melody_eval -- --ignored --nocapture

#![cfg(test)]

use std::path::{Path, PathBuf};

use crate::song::{self, Song};
use crate::{compose, convert, index, media, music, project};

pub struct TestSong {
    pub name: &'static str,
    pub lyrics: &'static str,
    /// (계이름, 박)
    pub melody: Vec<(i32, f32)>,
}

pub fn test_songs() -> Vec<TestSong> {
    let twinkle: Vec<(i32, f32)> = [[0, 0, 4, 4, 5, 5, 4], [3, 3, 2, 2, 1, 1, 0], [4, 4, 3, 3, 2, 2, 1], [4, 4, 3, 3, 2, 2, 1], [0, 0, 4, 4, 5, 5, 4], [3, 3, 2, 2, 1, 1, 0]]
        .iter()
        .flat_map(|p| p.iter().enumerate().map(|(i, &d)| (d, if i == 6 { 2.0 } else { 1.0 })))
        .collect();
    let school: Vec<(i32, f32)> = vec![
        (4, 1.0), (4, 1.0), (5, 1.0), (5, 1.0), (4, 1.0), (4, 1.0), (2, 2.0),
        (4, 1.0), (4, 1.0), (2, 1.0), (2, 1.0), (1, 3.0),
        (4, 1.0), (4, 1.0), (5, 1.0), (5, 1.0), (4, 1.0), (4, 1.0), (2, 2.0),
        (4, 1.0), (2, 1.0), (1, 1.0), (2, 1.0), (0, 3.0),
    ];
    let rabbit: Vec<(i32, f32)> = vec![
        (4, 1.0), (2, 0.5), (2, 1.5), (4, 1.0), (2, 0.5), (0, 1.5),
        (1, 1.0), (2, 0.5), (1, 0.5), (0, 1.0), (2, 1.0), (4, 2.0),
        (7, 0.5), (4, 0.5), (7, 0.5), (4, 0.5), (7, 1.0), (4, 0.5), (2, 1.5),
        (4, 1.0), (1, 0.5), (3, 0.5), (2, 1.0), (1, 1.0), (0, 2.0),
    ];
    vec![
        TestSong {
            name: "작은별",
            lyrics: "반짝반짝 작은 별 아름답게 비치네\n동쪽 하늘에서도 서쪽 하늘에서도\n반짝반짝 작은 별 아름답게 비치네",
            melody: twinkle,
        },
        TestSong { name: "학교종", lyrics: "학교 종이 땡땡땡 어서 모이자\n선생님이 우리를 기다리신다", melody: school },
        TestSong { name: "산토끼", lyrics: "산토끼 토끼야 어디를 가느냐\n깡충깡충 뛰면서 어디를 가느냐", melody: rabbit },
    ]
}

pub struct Rendered {
    /// 22050Hz 스테레오 (L, R)
    pub left: Vec<f32>,
    pub right: Vec<f32>,
    /// 음절별 정답: (박 시각 초, MIDI 음)
    pub truth: Vec<Option<(f64, f64)>>,
    /// 노래 첫 박 (초)
    pub start: f64,
    pub bpm: f32,
    /// 가사 음절 수만큼의 노래 (멜로디 적용 전)
    pub song: Song,
}

/// 정답 멜로디로 부른 노래 + 반주 → 22050Hz 스테레오
pub fn render_test_song(root: &Path, t: &TestSong, tmp: &Path) -> Rendered {
    let words = index::load_words(&index::open(root).unwrap()).unwrap();
    let clips = compose::compose(t.lyrics, &words);
    let mut s = convert::init_song(root, &clips, None, song::Preset::Robot).unwrap();
    s.bpm = 100.0;
    s.key = 0;
    s.line_rest = 0.0;
    s.engine = song::Engine::Smooth;
    s.flat = true;
    assert_eq!(s.notes.len(), t.melody.len(), "{}: 음절 수와 멜로디 길이가 다름", t.name);
    let blank = s.clone();
    for (n, &(d, b)) in s.notes.iter_mut().zip(&t.melody) {
        n.degree = d;
        n.beats = b;
    }
    let out = convert::convert(root, &clips, &s, |_, _| {}).unwrap();
    let targets = song::fold_octaves(&s, &clips);
    let truth: Vec<Option<(f64, f64)>> = out.spans.iter().zip(&targets).map(|(sp, t)| t.map(|hz| (sp.beat_at, song::hz_to_midi(hz)))).collect();

    // 반주 (48kHz): 박마다 그 음이 들어간 화음(I·IV·V), 좌우로 다른 자리바꿈, 가운데 베이스와 킥
    let sr = 48_000.0;
    let n = out.audio.len();
    let (mut l, mut r) = (vec![0.0f32; n], vec![0.0f32; n]);
    let spb = s.secs_per_beat();
    let starts = s.start_beats(&clips);
    let chord_of = |pc: i32| -> [i32; 3] {
        for c in [[0, 4, 7], [5, 9, 0], [7, 11, 2]] {
            if c.contains(&pc.rem_euclid(12)) {
                return c;
            }
        }
        [0, 4, 7]
    };
    let tone = |midi: f64, t: f64| -> f32 {
        let f = 440.0 * 2f64.powf((midi - 69.0) / 12.0);
        ((1..=4).map(|h| (2.0 * std::f64::consts::PI * f * h as f64 * t).sin() / h as f64).sum::<f64>()) as f32
    };
    for (i, note) in s.notes.iter().enumerate() {
        let a = ((0.3 + starts[i] as f64 * spb) * sr) as usize;
        let b = (((0.3 + (starts[i] + note.beats) as f64 * spb) * sr) as usize).min(n);
        let chord = chord_of(s.midi(note) - 60);
        for k in a..b {
            let t = k as f64 / sr;
            let tt = (k - a) as f64 / sr;
            l[k] += chord.iter().map(|&c| tone(48.0 + c as f64, t)).sum::<f32>() * 0.025;
            r[k] += chord.iter().map(|&c| tone(60.0 + c as f64, t)).sum::<f32>() * 0.02;
            let bass = tone(36.0 + chord[0] as f64, t) * 0.06;
            let kick = ((2.0 * std::f64::consts::PI * 55.0 * tt).sin() * (-tt * 25.0).exp() * 0.3) as f32;
            l[k] += bass + kick;
            r[k] += bass + kick;
        }
    }
    let mut stereo = Vec::with_capacity(n * 2);
    for k in 0..n {
        let v = out.audio[k] * 0.8;
        stereo.push(l[k] + v);
        stereo.push(r[k] + v);
    }
    let a = tmp.join("eval48.wav");
    let b = tmp.join("eval22.wav");
    crate::render::write_wav_ch(&a, &stereo.iter().map(|v| v.tanh()).collect::<Vec<_>>(), 2).unwrap();
    media::extract_wav_ch(&a, &b, 22_050, 2).unwrap();
    let mut rd = hound::WavReader::open(&b).unwrap();
    let samples: Vec<f32> = rd.samples::<i16>().map(|v| v.unwrap() as f32 / 32768.0).collect();
    let (left, right): (Vec<f32>, Vec<f32>) = samples.chunks(2).map(|c| (c[0], c[1])).unzip();
    Rendered { left, right, truth, start: 0.3, bpm: s.bpm, song: blank }
}

/// 배정 결과 채점: (음 이름 일치 %, ±1반음 이내 %, 시작 시각 오차 중앙값 ms)
pub fn score(result: &Song, r: &Rendered) -> (f64, f64, f64) {
    let spb = 60.0 / result.bpm as f64;
    let starts = result.start_beats(&[]);
    let (mut exact, mut near, mut total) = (0, 0, 0);
    let mut timing = Vec::new();
    for (i, n) in result.notes.iter().enumerate() {
        let Some((t_true, m_true)) = r.truth.get(i).copied().flatten() else { continue };
        total += 1;
        let d = ((result.midi(n) as f64 - m_true).round() as i64).rem_euclid(12);
        let pc = d.min(12 - d);
        exact += (pc == 0) as usize;
        near += (pc <= 1) as usize;
        timing.push(((r.start + starts[i] as f64 * spb) - t_true).abs() * 1000.0);
    }
    timing.sort_by(f64::total_cmp);
    let pct = |x: usize| 100.0 * x as f64 / total.max(1) as f64;
    (pct(exact), pct(near), timing.get(timing.len() / 2).copied().unwrap_or(0.0))
}

/// 지금 방식: 음표로 나눠 순서대로 배정
fn method_notes(r: &Rendered) -> Song {
    let notes = music::clean_melody(music::extract_melody(&r.left, &r.right, 22_050));
    let mut s = r.song.clone();
    s.mr = Some(song::Mr { file: String::new(), start: r.start, skip: 0.0, gain: 1.0, detected_bpm: r.bpm, tonic: 0, minor: false, melody: notes, syllables: Vec::new(), syllables_for: String::new() });
    song::apply_preset(&mut s, song::Preset::Robot);
    s
}

/// 음절 방식: 원곡 가사를 받아 적고 음절 위치를 찾아 그 음절에서 부른 음을 배정
fn method_syllables(r: &Rendered, rec: &crate::stt::Recognizer, aligner: &mut crate::align::Aligner, tmp: &Path, hint: Option<&str>) -> (Song, usize) {
    let contour = music::extract_contour(&r.left, &r.right, 22_050).unwrap();
    // 16kHz 모노: 가운데(보컬) 강조 신호
    let mono = music::center_signal(&r.left, &r.right);
    let a = tmp.join("eval_mono22.wav");
    let b = tmp.join("eval_mono16.wav");
    let spec = hound::WavSpec { channels: 1, sample_rate: 22_050, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
    let mut w = hound::WavWriter::create(&a, spec).unwrap();
    for v in &mono {
        w.write_sample((v * 32767.0) as i16).unwrap();
    }
    w.finalize().unwrap();
    media::extract_wav(&a, &b, crate::stt::SAMPLE_RATE).unwrap();
    let mono16 = crate::stt::read_wav(&b).unwrap();
    let syls = music::original_syllables(&mono16, &contour, rec, aligner, hint, std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false))).unwrap();
    let mut s = r.song.clone();
    music::apply_syllables(&mut s, &syls, r.start);
    (s, syls.len())
}

#[test]
#[ignore]
fn melody_eval() {
    let root = PathBuf::from(std::env::var("DCINVOX_TEST_PROJECT").unwrap());
    let tmp = std::env::temp_dir().join(format!("dcinvox-meval-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).unwrap();
    let rec = crate::stt::Recognizer::load(&crate::models::find(crate::models::WHISPER_MODEL).unwrap()).unwrap();
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../models");
    let mut aligner = crate::align::Aligner::load(&dir.join(crate::align::MODEL_FILE), &dir.join(crate::align::VOCAB_FILE)).unwrap();
    let (mut a1, mut a2, mut a3) = ((0.0, 0.0, 0.0), (0.0, 0.0, 0.0), (0.0, 0.0, 0.0));
    let songs = test_songs();
    for t in &songs {
        let r = render_test_song(&root, t, &tmp);
        // 점검 1: 정답을 그대로 넣으면 100%여야 함
        let mut oracle = r.song.clone();
        for (n, &(d, b)) in oracle.notes.iter_mut().zip(&t.melody) {
            n.degree = d;
            n.beats = b;
        }
        let (oe, _, oms) = score(&oracle, &r);
        // 점검 2: 정답 위치에서 음높이 선이 정답 음을 가리키는지 (위치 문제와 음높이 문제 분리)
        let contour = music::extract_contour(&r.left, &r.right, 22_050).unwrap();
        let (mut ok, mut tot, mut voiced_ok) = (0, 0, 0);
        for &(t0, m) in r.truth.iter().flatten() {
            let fr: Vec<usize> = (contour.frame(t0 + 0.03)..=contour.frame(t0 + 0.25)).collect();
            let v: Vec<f32> = fr.iter().filter(|&&f| contour.voiced[f]).map(|&f| contour.pitch[f]).collect();
            tot += 1;
            if v.len() * 2 >= fr.len() {
                voiced_ok += 1;
            }
            if let Some(&p) = { let mut x = v.clone(); x.sort_by(f32::total_cmp); x.get(x.len() / 2).copied() }.as_ref() {
                let d = ((p as f64 - m).round() as i64).rem_euclid(12);
                ok += (d.min(12 - d) == 0) as usize;
            }
        }
        println!("{:<6} 점검: 정답 넣기 {oe:.0}% ({oms:.0}ms) · 정답 위치 음높이 일치 {:.0}% · 유성 판정 {:.0}%", t.name, 100.0 * ok as f64 / tot as f64, 100.0 * voiced_ok as f64 / tot as f64);
        let (e, n, ms) = score(&method_notes(&r), &r);
        println!("{:<6} 음표방식: 음 일치 {e:5.1}%  ±1반음 {n:5.1}%  시작 오차 {ms:4.0}ms", t.name);
        a1 = (a1.0 + e, a1.1 + n, a1.2 + ms);
        let (song2, found) = method_syllables(&r, &rec, &mut aligner, &tmp, None);
        let (e, n, ms) = score(&song2, &r);
        println!("{:<6} 음절(가운데):   음 일치 {e:5.1}%  ±1반음 {n:5.1}%  시작 오차 {ms:4.0}ms  (원곡 음절 {found}/{})", t.name, t.melody.len());
        a2 = (a2.0 + e, a2.1 + n, a2.2 + ms);
        let (song3, found) = method_syllables(&r, &rec, &mut aligner, &tmp, Some(t.lyrics));
        let (e, n, ms) = score(&song3, &r);
        println!("{:<6} 음절(가운데+가사): 음 일치 {e:5.1}%  ±1반음 {n:5.1}%  시작 오차 {ms:4.0}ms  (원곡 음절 {found}/{})", t.name, t.melody.len());
        a3 = (a3.0 + e, a3.1 + n, a3.2 + ms);
    }
    let k = songs.len() as f64;
    println!("EVAL 음표방식 평균: 음 일치 {:.1}%  ±1반음 {:.1}%  시작 오차 {:.0}ms", a1.0 / k, a1.1 / k, a1.2 / k);
    println!("EVAL 음절(가운데) 평균: 음 일치 {:.1}%  ±1반음 {:.1}%  시작 오차 {:.0}ms", a2.0 / k, a2.1 / k, a2.2 / k);
    println!("EVAL 음절(가운데+가사) 평균: 음 일치 {:.1}%  ±1반음 {:.1}%  시작 오차 {:.0}ms", a3.0 / k, a3.1 / k, a3.2 / k);
    let _ = std::fs::remove_dir_all(&tmp);
    let _ = project::CACHE_DIR;
}
