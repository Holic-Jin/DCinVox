//! MIDI 멜로디 불러오기·내보내기.
//! 불러오기: 노래 멜로디 MIDI의 음표를 가사 음절에 순서대로 배정 → 음악을 몰라도 노래가 됨.
//! 내보내기: 음계 블록을 MIDI로 (가사 포함) → 다른 음악 프로그램에서 이어서 작업.

use midly::num::{u15, u24, u28, u4, u7};
use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};

use crate::compose::Clip;
use crate::song::Song;

const PPQ: u16 = 480;
const MAJOR: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];

/// MIDI에서 뽑은 멜로디 (한 줄)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Melody {
    /// (시작 박, 길이 박, MIDI 음 번호)
    pub notes: Vec<(f32, f32, u8)>,
    pub bpm: Option<f32>,
    /// 조표의 장조 으뜸음 (0=C … 11=B)
    pub key: Option<i32>,
}

pub fn read_melody(bytes: &[u8]) -> Result<Melody, String> {
    let smf = Smf::parse(bytes).map_err(|e| tr!("midi_unreadable", e))?;
    let ppq = match smf.header.timing {
        Timing::Metrical(t) => t.as_int() as f32,
        Timing::Timecode(..) => return Err(tr!("midi_smpte").into()),
    };
    let mut bpm = None;
    let mut key = None;
    // 트랙별 음표 (시작 틱, 끝 틱, 음)
    let mut tracks: Vec<Vec<(u64, u64, u8)>> = Vec::new();
    for track in &smf.tracks {
        let mut t: u64 = 0;
        let mut open: Vec<(u8, u8, u64)> = Vec::new(); // (채널, 음, 시작)
        let mut notes = Vec::new();
        for ev in track {
            t += ev.delta.as_int() as u64;
            match ev.kind {
                TrackEventKind::Meta(MetaMessage::Tempo(us)) if bpm.is_none() => bpm = Some(60_000_000.0 / us.as_int() as f32),
                TrackEventKind::Meta(MetaMessage::KeySignature(sharps, _minor)) if key.is_none() => {
                    // 같은 조표의 장조 (단조면 나란한조 장조 = 같은 7음)
                    key = Some((7 * sharps as i32).rem_euclid(12));
                }
                TrackEventKind::Midi { channel, message } => {
                    let ch = channel.as_int();
                    // 드럼 채널(10번)은 멜로디가 아님
                    if ch == 9 {
                        continue;
                    }
                    let (k, on) = match message {
                        MidiMessage::NoteOn { key, vel } => (key.as_int(), vel.as_int() > 0),
                        MidiMessage::NoteOff { key, .. } => (key.as_int(), false),
                        _ => continue,
                    };
                    if on {
                        open.push((ch, k, t));
                    } else if let Some(i) = open.iter().position(|o| o.0 == ch && o.1 == k) {
                        let (_, k, s) = open.remove(i);
                        notes.push((s, t, k));
                    }
                }
                _ => {}
            }
        }
        tracks.push(notes);
    }
    // 음표가 가장 많은 트랙을 멜로디로
    let mut notes = tracks.into_iter().max_by_key(|t| t.len()).unwrap_or_default();
    if notes.is_empty() {
        return Err(tr!("midi_no_notes").into());
    }
    notes.sort_by_key(|n| (n.0, std::cmp::Reverse(n.2)));
    // 화음이면 같은 시작의 가장 높은 음만, 겹치면 앞 음을 다음 음 시작에서 끊음 (한 목소리)
    let mut mono: Vec<(u64, u64, u8)> = Vec::new();
    for n in notes {
        if let Some(last) = mono.last_mut() {
            if last.0 == n.0 {
                continue;
            }
            if last.1 > n.0 {
                last.1 = n.0;
            }
        }
        mono.push(n);
    }
    Ok(Melody {
        notes: mono.into_iter().map(|(s, e, k)| (s as f32 / ppq, (e - s) as f32 / ppq, k)).collect(),
        bpm,
        key,
    })
}

/// 키 안의 계이름으로 (키에 없는 음은 아래 음 + 반음 올림)
pub fn degree_of(key: i32, midi: i32) -> (i32, bool) {
    let rel = midi - (60 + key);
    let oct = rel.div_euclid(12);
    let pc = rel.rem_euclid(12);
    match MAJOR.iter().position(|&s| s == pc) {
        Some(d) => (d as i32 + 7 * oct, false),
        None => {
            let d = MAJOR.iter().position(|&s| s == pc - 1).unwrap_or(0);
            (d as i32 + 7 * oct, true)
        }
    }
}

/// 멜로디를 음계 블록에 순서대로 배정. 반환: 배정한 음 수
pub fn apply_melody(song: &mut Song, m: &Melody) -> usize {
    if let Some(bpm) = m.bpm {
        song.bpm = bpm.round().clamp(40.0, 240.0);
    }
    if let Some(k) = m.key {
        song.key = k;
    } else {
        // 조표가 없으면 음 분포로 키 추정
        let mut chroma = [0.0f32; 12];
        for &(_, d, k) in &m.notes {
            chroma[(k % 12) as usize] += d;
        }
        let (tonic, minor) = crate::music::key_from_chroma(&chroma);
        song.key = if minor { (tonic + 3) % 12 } else { tonic };
    }
    // MIDI의 쉼표를 그대로 쓰므로 줄 사이 자동 쉼은 끔
    song.line_rest = 0.0;
    let mut prev_end = 0.0f32;
    let n = song.notes.len().min(m.notes.len());
    for (note, &(start, dur, k)) in song.notes.iter_mut().zip(&m.notes) {
        let (degree, sharp) = degree_of(song.key, k as i32);
        note.degree = degree;
        note.sharp = sharp;
        note.beats = (dur * 4.0).round().max(1.0) / 4.0; // 16분음표 단위
        note.gap = ((start - prev_end) * 4.0).round().max(0.0) / 4.0;
        prev_end = start + dur;
    }
    // 블록이 화면 가운데(도 근처)에 오도록 옥타브 정리 (실제 음높이는 화자에 맞춰 자동 조정됨)
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

/// 음계 블록 → MIDI (가사 포함, 형식 0)
pub fn write_midi(song: &Song, clips: &[Clip]) -> Result<Vec<u8>, String> {
    let starts = song.start_beats(clips);
    let tick = |beats: f32| (beats * PPQ as f32).round() as u64;
    let lyrics: Vec<Vec<u8>> = song.notes.iter().map(|n| n.lyric.as_bytes().to_vec()).collect();
    // (틱, 순서, 이벤트) — 같은 틱이면 끄기를 켜기보다 먼저
    let mut events: Vec<(u64, u8, TrackEventKind)> = Vec::new();
    events.push((0, 0, TrackEventKind::Meta(MetaMessage::Tempo(u24::new((60_000_000.0 / song.bpm.max(1.0)) as u32)))));
    let sharps = [0i8, 7, 2, -3, 4, -1, 6, 1, -4, 3, -2, 5][song.key.rem_euclid(12) as usize];
    events.push((0, 0, TrackEventKind::Meta(MetaMessage::KeySignature(sharps, false))));
    for (i, n) in song.notes.iter().enumerate() {
        if n.rest {
            continue;
        }
        let key = u7::new(song.midi(n).clamp(0, 127) as u8);
        let (s, e) = (tick(starts[i]), tick(starts[i] + n.beats));
        events.push((s, 1, TrackEventKind::Meta(MetaMessage::Lyric(&lyrics[i]))));
        events.push((s, 2, TrackEventKind::Midi { channel: u4::new(0), message: MidiMessage::NoteOn { key, vel: u7::new(100) } }));
        events.push((e, 0, TrackEventKind::Midi { channel: u4::new(0), message: MidiMessage::NoteOff { key, vel: u7::new(0) } }));
    }
    events.sort_by_key(|e| (e.0, e.1));
    let end = events.last().map_or(0, |e| e.0);
    events.push((end, 3, TrackEventKind::Meta(MetaMessage::EndOfTrack)));

    let mut track = Vec::with_capacity(events.len());
    let mut last = 0;
    for (t, _, kind) in events {
        track.push(TrackEvent { delta: u28::new((t - last) as u32), kind });
        last = t;
    }
    let smf = Smf { header: Header::new(Format::SingleTrack, Timing::Metrical(u15::new(PPQ))), tracks: vec![track] };
    let mut out = Vec::new();
    smf.write_std(&mut out).map_err(|e| e.to_string())?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::song::Note;

    fn song_with(n: usize) -> Song {
        let notes = (0..n)
            .map(|i| Note { clip: 0, syl: i, pron: String::new(), lyric: "라".into(), degree: 0, sharp: false, beats: 1.0, gap: 0.0, rest: false, source_hz: None })
            .collect();
        Song { notes, ..Song::default() }
    }

    #[test]
    fn degrees_from_midi_pitch() {
        assert_eq!(degree_of(0, 60), (0, false)); // C → 도
        assert_eq!(degree_of(0, 67), (4, false)); // G → 솔
        assert_eq!(degree_of(0, 61), (0, true)); // C# → 도#
        assert_eq!(degree_of(0, 59), (-1, false)); // B3 → 낮은 시
        assert_eq!(degree_of(7, 67), (0, false)); // 사장조에서 G → 도
    }

    #[test]
    fn midi_round_trip_keeps_melody_and_rhythm() {
        // 작은 별 앞부분, 마지막 음 2박, 중간에 반 박 쉼
        let mut src = song_with(7);
        src.bpm = 96.0;
        src.key = 7;
        src.line_rest = 0.0;
        for (n, d) in src.notes.iter_mut().zip([0, 0, 4, 4, 5, 5, 4]) {
            n.degree = d;
        }
        src.notes[6].beats = 2.0;
        src.notes[4].gap = 0.5;
        let bytes = write_midi(&src, &[]).unwrap();

        let m = read_melody(&bytes).unwrap();
        assert_eq!(m.notes.len(), 7);
        assert_eq!(m.bpm.map(|b| b.round()), Some(96.0));
        assert_eq!(m.key, Some(7));

        let mut dst = song_with(9); // 음절이 더 많아도 앞에서부터 배정
        let assigned = apply_melody(&mut dst, &m);
        assert_eq!(assigned, 7);
        assert_eq!((dst.bpm, dst.key), (96.0, 7));
        let degs: Vec<i32> = dst.notes[..7].iter().map(|n| n.degree).collect();
        assert_eq!(degs, [0, 0, 4, 4, 5, 5, 4]);
        assert_eq!(dst.notes[6].beats, 2.0);
        assert_eq!(dst.notes[4].gap, 0.5);
    }

    /// 시험용 MIDI 만들기: DCINVOX_OUT=<폴더> cargo test sample_midi -- --ignored
    #[test]
    #[ignore]
    fn sample_midi() {
        // 작은 별 전체 (6마디 × 7음, 마디 끝 음은 2박)
        let phrases: [[i32; 7]; 6] = [
            [0, 0, 4, 4, 5, 5, 4],
            [3, 3, 2, 2, 1, 1, 0],
            [4, 4, 3, 3, 2, 2, 1],
            [4, 4, 3, 3, 2, 2, 1],
            [0, 0, 4, 4, 5, 5, 4],
            [3, 3, 2, 2, 1, 1, 0],
        ];
        let mut song = song_with(42);
        song.bpm = 100.0;
        song.line_rest = 0.0;
        for (i, n) in song.notes.iter_mut().enumerate() {
            n.degree = phrases[i / 7][i % 7];
            n.beats = if i % 7 == 6 { 2.0 } else { 1.0 };
            n.lyric = "라".into();
        }
        let dir = std::path::PathBuf::from(std::env::var("DCINVOX_OUT").unwrap());
        std::fs::write(dir.join("작은별_시험용.mid"), write_midi(&song, &[]).unwrap()).unwrap();
    }
}
