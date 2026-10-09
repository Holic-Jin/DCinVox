//! 노래 설정과 음표(음절 블록).
//! 사용자는 블록을 위아래로 옮겨 계이름(도레미)만 고른다. 실제 음높이(옥타브 포함)는 여기서 계산:
//! - 키(조성)의 7음만 사용 → 어떤 조합도 반주와 어긋나지 않음
//! - 화자별 옥타브 자동 맞춤 → 남자 목소리를 높은 '솔'에 올려 다람쥐 소리가 나는 일 방지

use serde::{Deserialize, Serialize};

use crate::compose::{Clip, ClipKind};

/// 장음계 음 간격 (도레미파솔라시)
const MAJOR: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    /// 생성 결과의 몇 번째 클립에서 왔는지
    pub clip: usize,
    /// 클립 안 몇 번째 음절인지
    pub syl: usize,
    /// 블록에 표시할 글자
    pub lyric: String,
    /// 이 음절의 발음 (원곡 가사와 맞춰 볼 때 사용)
    #[serde(default)]
    pub pron: String,
    /// 계이름 단계: 0=도, 1=레 … 6=시, 7=한 옥타브 위 도, -1=아래 시
    pub degree: i32,
    /// 반음 올림 (고급)
    #[serde(default)]
    pub sharp: bool,
    /// 길이 (박)
    pub beats: f32,
    /// 이 음 앞의 쉼 (박) — MIDI 멜로디를 불러오면 채워짐
    #[serde(default)]
    pub gap: f32,
    /// 영상에 없는 소리 → 쉼
    #[serde(default)]
    pub rest: bool,
    /// 원래 말할 때 음높이(Hz) — 옥타브 맞춤과 "말하기" 프리셋에 사용
    #[serde(default)]
    pub source_hz: Option<f32>,
}

/// 변환 엔진 (비교 청취로 고름)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Engine {
    /// A: WORLD 보코더, 완전히 평평한 음 + 프레임 반복 (가장 기계적)
    Vocoder,
    /// B: WORLD + 자연스러운 미세 떨림 + 프레임 보간 + 자음은 원음 (비교 청취로 기본값 선정)
    #[default]
    Smooth,
    /// C: PSOLA — 원래 파형을 한 주기씩 잘라 간격을 바꿔 다시 붙임 (원음 질감 유지)
    Psola,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Song {
    pub bpm: f32,
    /// 키의 으뜸음 (0=C, 1=C#, … 11=B)
    pub key: i32,
    /// true: 음마다 음높이를 평평하게 (로봇/오토튠), false: 원래 억양을 유지한 채 옮김
    pub flat: bool,
    pub vibrato: bool,
    /// 음 바꾸기 방식
    #[serde(default)]
    pub engine: Engine,
    /// 가사 줄 사이 쉼 (박)
    pub line_rest: f32,
    pub notes: Vec<Note>,
    /// 반주 (없으면 목소리만)
    #[serde(default)]
    pub mr: Option<Mr>,
    /// 불러온 MIDI 멜로디 (로봇 오토튠을 누를 때 적용)
    #[serde(default)]
    pub midi: Option<crate::midi::Melody>,
}

/// 반주 설정 + 자동 분석 결과
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mr {
    /// Assets/ 안의 파일 이름
    pub file: String,
    /// 노래 첫 박이 반주의 몇 초에 오는지 (박 단위로 앞뒤로 옮길 수 있음)
    pub start: f64,
    /// 반주 앞부분을 이만큼(초) 건너뛰고 시작 (원곡 따라 부르기에서 긴 전주 생략)
    #[serde(default)]
    pub skip: f64,
    /// 반주 크기 (1.0 = 목소리와 비슷한 크기)
    pub gain: f32,
    pub detected_bpm: f32,
    /// 감지한 원래 으뜸음과 단조 여부 (표시용)
    pub tonic: i32,
    pub minor: bool,
    /// 원곡에서 뽑은 멜로디 (초 단위). 로봇 오토튠을 누르면 이 멜로디를 음절에 배정
    #[serde(default)]
    pub melody: Vec<crate::music::MelodyNote>,
    /// 원곡 가수의 음절 위치·음 (가사를 맞춰 본 결과). `syllables_for` 가사로 만든 것
    #[serde(default)]
    pub syllables: Vec<crate::music::OrigSyllable>,
    #[serde(default)]
    pub syllables_for: String,
}

impl Song {
    pub fn secs_per_beat(&self) -> f64 {
        60.0 / self.bpm.max(20.0) as f64
    }

    /// 계이름 → 키 기준 반음 수 (가온 다(C4)=60 기준 MIDI 번호)
    pub fn midi(&self, n: &Note) -> i32 {
        let oct = n.degree.div_euclid(7);
        let step = n.degree.rem_euclid(7) as usize;
        60 + self.key + 12 * oct + MAJOR[step] + n.sharp as i32
    }

    /// 음표들의 시작 박 (순서대로 이어 놓고, 줄이 바뀌면 쉼)
    pub fn start_beats(&self, clips: &[Clip]) -> Vec<f32> {
        let mut t = 0.0;
        let mut out = Vec::with_capacity(self.notes.len());
        for (i, n) in self.notes.iter().enumerate() {
            if i > 0 && clips.get(self.notes[i - 1].clip).map(|c| c.line) != clips.get(n.clip).map(|c| c.line) {
                t += self.line_rest;
            }
            t += n.gap;
            out.push(t);
            t += n.beats;
        }
        out
    }
}

pub fn midi_to_hz(m: f64) -> f64 {
    440.0 * 2f64.powf((m - 69.0) / 12.0)
}

pub fn hz_to_midi(hz: f64) -> f64 {
    69.0 + 12.0 * (hz / 440.0).log2()
}

/// 원래 음높이에 가장 가까운 키 안의 계이름
pub fn nearest_degree(key: i32, hz: f64) -> i32 {
    let m = hz_to_midi(hz);
    (-21..=28)
        .min_by(|&a, &b| {
            let ma = midi_of(key, a) as f64;
            let mb = midi_of(key, b) as f64;
            (ma - m).abs().total_cmp(&(mb - m).abs())
        })
        .unwrap_or(0)
}

fn midi_of(key: i32, degree: i32) -> i32 {
    60 + key + 12 * degree.div_euclid(7) + MAJOR[degree.rem_euclid(7) as usize]
}

/// 화자(영상)별 옥타브 맞춤: 그 화자가 부르는 음들의 가운데가 화자 평소 음높이와 가장 가까워지도록
/// 옥타브 단위로만 옮긴다 (멜로디 모양은 그대로). 반환: 음표별 실제 목표 Hz
pub fn fold_octaves(song: &Song, clips: &[Clip]) -> Vec<Option<f64>> {
    use std::collections::HashMap;
    let mut by_asset: HashMap<&str, (Vec<f64>, Vec<f64>)> = HashMap::new(); // (목표 midi, 원래 midi)
    for n in &song.notes {
        let Some(src) = clips.get(n.clip).and_then(|c| c.source()) else { continue };
        let e = by_asset.entry(src.asset_id.as_str()).or_default();
        e.0.push(song.midi(n) as f64);
        if let Some(hz) = n.source_hz {
            e.1.push(hz_to_midi(hz as f64));
        }
    }
    let median = |v: &mut Vec<f64>| -> Option<f64> {
        if v.is_empty() {
            return None;
        }
        v.sort_by(f64::total_cmp);
        Some(v[v.len() / 2])
    };
    let shift: HashMap<&str, f64> = by_asset
        .iter_mut()
        .map(|(id, (target, source))| {
            let k = match (median(target), median(source)) {
                (Some(t), Some(s)) => ((s - t) / 12.0).round() * 12.0,
                _ => 0.0,
            };
            (*id, k)
        })
        .collect();
    song.notes
        .iter()
        .map(|n| {
            if n.rest {
                return None;
            }
            let src = clips.get(n.clip)?.source()?;
            Some(midi_to_hz(song.midi(n) as f64 + shift.get(src.asset_id.as_str()).copied().unwrap_or(0.0)))
        })
        .collect()
}

/// 블록 가운데값이 화면 가운데(도~레 근처)에서 너무 멀면 옥타브 단위로 옮길 양.
/// 실제 소리는 화자 옥타브 자동 맞춤으로 정해지므로 보기 위한 정리일 뿐
pub fn center_shift(mid: i32) -> i32 {
    7 * ((mid as f32 - 1.5) / 7.0).round() as i32
}

/// 생성 결과 → 음표 (음절마다 블록 하나). 원래 음높이는 나중에 채움.
pub fn notes_from_clips(clips: &[Clip]) -> Vec<Note> {
    let mut notes = Vec::new();
    for (ci, clip) in clips.iter().enumerate() {
        let units = crate::g2p::units(&clip.pron);
        let shown = crate::g2p::units(&crate::index::normalize(&clip.lyric));
        let labels = if shown.len() == units.len() { shown } else { units.clone() };
        let n = labels.len().max(1);
        for (si, label) in labels.into_iter().enumerate().take(n) {
            notes.push(Note {
                clip: ci,
                syl: si,
                pron: units.get(si).cloned().unwrap_or_default(),
                lyric: label,
                degree: 0,
                sharp: false,
                beats: 1.0,
                gap: 0.0,
                rest: clip.kind == ClipKind::Missing,
                source_hz: None,
            });
        }
    }
    notes
}

/// 프리셋
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Preset {
    /// 원래 말할 때 음높이에 가장 가까운 계이름으로 (말투가 살아 있는 로봇 노래)
    Robot,
    /// 전부 한 음 (도)
    Monotone,
    /// 원래 억양 그대로 (평평하게 하지 않음)
    Speech,
}

/// 원래 말할 때 음높이에 가장 가까운 계이름으로 (멜로디를 적용하기 전의 기본 상태)
pub fn voice_degrees(song: &mut Song) {
    for n in &mut song.notes {
        n.degree = n.source_hz.map_or(0, |hz| nearest_degree(song.key, hz as f64));
        n.sharp = false;
    }
    let mut d: Vec<i32> = song.notes.iter().filter(|n| !n.rest).map(|n| n.degree).collect();
    d.sort();
    let shift = center_shift(d.get(d.len() / 2).copied().unwrap_or(0));
    for n in &mut song.notes {
        n.degree -= shift;
    }
}

pub fn apply_preset(song: &mut Song, preset: Preset) {
    // 로봇 오토튠: 불러온 MIDI 멜로디 → 원곡에서 딴 멜로디 순으로 따라 부르기
    if matches!(preset, Preset::Robot) {
        if let Some(m) = song.midi.clone() {
            crate::midi::apply_melody(song, &m);
            song.flat = true;
            return;
        }
        // 원곡 음절을 찾아 둔 게 있으면 그것 (가장 정확), 없으면 음표로 나눈 멜로디
        if let Some(m) = song.mr.clone().filter(|m| !m.syllables.is_empty()) {
            crate::music::apply_syllables(song, &m.syllables, m.start);
            song.flat = true;
            return;
        }
        if let Some(m) = song.mr.as_ref().filter(|m| !m.melody.is_empty()) {
            let spb = song.secs_per_beat() as f32;
            let start = m.start as f32;
            let melody = crate::midi::Melody {
                notes: m
                    .melody
                    .iter()
                    .map(|n| ((n.start as f32 - start) / spb, (n.end - n.start) as f32 / spb, n.midi))
                    .filter(|n| n.0 >= -0.25)
                    .map(|(s, d, k)| (s.max(0.0), d, k))
                    .collect(),
                bpm: Some(song.bpm),
                key: Some(song.key),
            };
            crate::midi::apply_melody(song, &melody);
            song.flat = true;
            return;
        }
    }
    match preset {
        Preset::Robot | Preset::Speech => {
            voice_degrees(song);
            song.flat = matches!(preset, Preset::Robot);
        }
        Preset::Monotone => {
            for n in &mut song.notes {
                n.degree = 0;
                n.sharp = false;
            }
            song.flat = true;
        }
    }
}

impl Default for Song {
    fn default() -> Self {
        Song { bpm: 100.0, key: 0, flat: true, vibrato: false, engine: Engine::default(), line_rest: 1.0, notes: Vec::new(), mr: None, midi: None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(degree: i32) -> Note {
        Note { clip: 0, syl: 0, pron: String::new(), lyric: "가".into(), degree, sharp: false, beats: 1.0, gap: 0.0, rest: false, source_hz: None }
    }

    #[test]
    fn degrees_to_pitch() {
        let song = Song::default();
        assert_eq!(song.midi(&note(0)), 60); // 도 = C4
        assert_eq!(song.midi(&note(4)), 67); // 솔
        assert_eq!(song.midi(&note(7)), 72); // 높은 도
        assert_eq!(song.midi(&note(-1)), 59); // 낮은 시
        let g = Song { key: 7, ..Song::default() }; // 사장조: 도 = G
        assert_eq!(g.midi(&note(0)), 67);
        assert!((midi_to_hz(69.0) - 440.0).abs() < 1e-9);
    }

    #[test]
    fn nearest_degree_stays_in_key() {
        // 277Hz(C#4)는 다장조에 없음 → 도(C4) 또는 레(D4)
        let d = nearest_degree(0, 277.2);
        assert!(d == 0 || d == 1);
        assert_eq!(nearest_degree(0, 392.0), 4); // G4 = 솔
    }
}
