//! 클립 목록 → 미리보기 영상(cache/preview.mp4).
//! - 오디오: 원본 48kHz WAV에서 샘플 단위로 직접 잘라 붙임(정확한 하드컷, 음량 맞춤)
//! - 영상: 같은 구간을 클립별로 짧은 영상으로 인코딩(캐시) 후 이어 붙임
//! - 영상 프레임은 누적 시간 기준으로 잘라 클립이 많아도 소리와 어긋나지 않음

use std::collections::HashMap;
use std::fs;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use serde::Serialize;

use crate::compose::{Clip, ClipKind};
use crate::{media, project};

pub const FPS: u32 = 30;
const SR: u32 = project::OUTPUT_SAMPLE_RATE;
/// 미리보기 해상도 (가볍게)
pub const PREVIEW_SIZE: (u32, u32) = (854, 480);
/// 내보내기 해상도
pub const EXPORT_SIZE: (u32, u32) = (1280, 720);
const LINE_GAP: f64 = 0.3;
/// 가사 단어 사이 쉼(ms). 단어 안 음절끼리는 하드컷 그대로 (bench로 조정)
/// 측정: 0ms 30.6% → 40ms 26.3% (80ms는 오히려 33.2%)
pub static WORD_GAP_MS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(40);
const MISSING_LEN: f64 = 0.25;
const MIN_LEN: f64 = 0.04;
/// 하드컷이 '틱' 소리만 안 나게 하는 최소 페이드 (합성 티는 그대로)
const FADE_SECS: f64 = 0.003;
const TARGET_RMS: f32 = 0.1; // -20 dBFS

pub const PREVIEW_FILE: &str = "preview.mp4";

/// 미리보기 영상 위의 클립 위치 (UI에서 클립 누르면 그 위치로 이동)
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct TimelineItem {
    pub clip: Option<usize>,
    pub start: f64,
    pub end: f64,
}

pub enum Piece {
    Audio { asset_id: String, start: f64, end: f64 },
    Silence(f64),
}

pub type Wav = hound::WavReader<BufReader<fs::File>>;

pub fn read_range(wav: &mut Wav, start: f64, end: f64) -> Result<Vec<f32>, String> {
    let total = wav.duration();
    let s = ((start.max(0.0) * SR as f64) as u32).min(total);
    let e = ((end.max(0.0) * SR as f64) as u32).min(total);
    wav.seek(s).map_err(|e| e.to_string())?;
    wav.samples::<i16>()
        .take((e.saturating_sub(s)) as usize)
        .map(|v| v.map(|x| x as f32 / 32768.0).map_err(|e| e.to_string()))
        .collect()
}

/// 단어 구간 앞뒤의 조용한 부분 제거 (Whisper 단어 끝이 뒤 쉼까지 늘어지는 문제 보정)
fn trim_silence(wav: &mut Wav, start: f64, end: f64) -> Result<(f64, f64), String> {
    let samples = read_range(wav, start, end)?;
    let frame = (SR / 100) as usize; // 10ms
    let rms: Vec<f32> = samples
        .chunks(frame)
        .map(|c| (c.iter().map(|x| x * x).sum::<f32>() / c.len() as f32).sqrt())
        .collect();
    let peak = rms.iter().cloned().fold(0.0, f32::max);
    let thr = (peak * 0.0316).max(0.003); // 최대 대비 -30dB
    let first = rms.iter().position(|&r| r > thr);
    let last = rms.iter().rposition(|&r| r > thr);
    match (first, last) {
        (Some(f), Some(l)) => {
            let s = start + (f as f64 * 0.01 - 0.02).max(0.0);
            let e = (start + (l + 1) as f64 * 0.01 + 0.02).min(end);
            Ok((s, e.max(s + MIN_LEN)))
        }
        _ => Ok((start, end)),
    }
}

pub struct RenderOutput {
    pub timeline: Vec<TimelineItem>,
}

pub fn render(
    root: &Path,
    clips: &[Clip],
    on_progress: impl FnMut(usize, usize),
    cancel: Arc<AtomicBool>,
) -> Result<RenderOutput, String> {
    let cache = root.join(project::CACHE_DIR);
    let (audio, timeline, pieces) = build_audio(root, clips)?;
    let wav_path = cache.join("preview.wav");
    write_wav(&wav_path, &audio)?;
    let segs: Vec<VideoSeg> = pieces
        .iter()
        .zip(&timeline)
        .map(|((_, piece), item)| VideoSeg {
            t0: item.start,
            t1: item.end,
            src: match piece {
                Piece::Silence(_) => None,
                Piece::Audio { asset_id, start, end } => Some((asset_id.clone(), *start, *end)),
            },
        })
        .collect();
    render_video(root, &segs, &wav_path, PREVIEW_FILE, PREVIEW_SIZE, on_progress, cancel)?;
    Ok(RenderOutput { timeline })
}

/// 출력 영상의 한 구간: [t0, t1)초에 원본 (영상, 시작, 끝)을 늘이거나 줄여 채움. None이면 검은 화면
pub struct VideoSeg {
    pub t0: f64,
    pub t1: f64,
    pub src: Option<(String, f64, f64)>,
}

/// 구간들 → 영상 조각 인코딩(캐시) → 이어 붙이고 `wav_path` 소리를 입혀 cache/`out_name`으로.
/// 프레임은 누적 시간 기준으로 잘라 조각이 많아도 소리와 어긋나지 않음
pub fn render_video(
    root: &Path,
    segs: &[VideoSeg],
    wav_path: &Path,
    out_name: &str,
    size: (u32, u32),
    mut on_progress: impl FnMut(usize, usize),
    cancel: Arc<AtomicBool>,
) -> Result<(), String> {
    let (width, height) = size;
    // 내보내기는 화질을 올림 (같은 옵션은 뒤에 온 값이 적용됨)
    let quality: Vec<String> = if size == PREVIEW_SIZE { Vec::new() } else { vec!["-b:v".into(), "6M".into()] };
    let cache = root.join(project::CACHE_DIR);
    let files: HashMap<String, PathBuf> = project::open_or_create(root)?
        .assets
        .into_iter()
        .map(|a| (a.id, root.join(project::ASSETS_DIR).join(a.file_name)))
        .collect();
    let clip_dir = cache.join("clips");
    fs::create_dir_all(&clip_dir).map_err(|e| e.to_string())?;
    let mut jobs: Vec<(PathBuf, Vec<String>)> = Vec::new();
    let mut segments: Vec<PathBuf> = Vec::new();
    for seg in segs {
        let f0 = (seg.t0 * FPS as f64).round() as u64;
        let f1 = (seg.t1 * FPS as f64).round() as u64;
        let n = f1.saturating_sub(f0);
        if n == 0 {
            continue; // 1프레임보다 짧은 조각은 소리만
        }
        let scale = format!("fps={FPS},scale={width}:{height}:force_original_aspect_ratio=decrease,pad={width}:{height}:(ow-iw)/2:(oh-ih)/2,setsar=1,format=yuv420p");
        let (name, input, vf): (String, Vec<String>, String) = match &seg.src {
            None => (
                format!("black_{n}_{height}.mp4"),
                vec!["-f".into(), "lavfi".into(), "-i".into(), format!("color=c=black:s={width}x{height}:r={FPS}")],
                scale,
            ),
            Some((asset_id, start, end)) => {
                let file = files.get(asset_id).ok_or("원본 영상을 찾을 수 없습니다")?;
                // 원본 길이와 출력 길이가 다르면 늘이거나 줄임 (노래로 늘인 음절)
                let stretch = ((seg.t1 - seg.t0) / (end - start).max(0.001)).clamp(0.05, 50.0);
                let vf = if (stretch - 1.0).abs() < 0.01 { scale } else { format!("setpts={stretch:.4}*PTS,{scale}") };
                (
                    format!("{asset_id}_{}_{n}_{}_{height}.mp4", (start * 1000.0).round() as i64, (stretch * 1000.0).round() as i64),
                    vec!["-ss".into(), format!("{start:.3}"), "-i".into(), file.to_string_lossy().into_owned()],
                    vf,
                )
            }
        };
        let out = clip_dir.join(&name);
        segments.push(out.clone());
        if out.is_file() || jobs.iter().any(|(p, _)| *p == out) {
            continue; // 이미 만든 조각 재사용 (후보 바꾸기·재생성이 빨라짐)
        }
        let mut args: Vec<String> = vec!["-y".into(), "-loglevel".into(), "error".into()];
        args.extend(input);
        args.extend(["-frames:v".into(), n.to_string(), "-vf".into(), vf, "-an".into()]);
        args.extend(media::h264_args().iter().map(|s| s.to_string()));
        args.extend(quality.iter().cloned());
        args.extend(["-video_track_timescale".into(), "30000".into()]);
        let tmp = out.with_extension("part.mp4");
        args.push(tmp.to_string_lossy().into_owned());
        jobs.push((out, args));
    }

    let total = jobs.len() + 1;
    on_progress(0, total);
    run_parallel(jobs, &cancel, |done| on_progress(done, total))?;

    // 이어 붙이기 + 소리 합치기
    let stem = out_name.trim_end_matches(".mp4");
    let list = cache.join(format!("{stem}_list.txt"));
    let body: String = segments
        .iter()
        .map(|p| format!("file '{}'\n", p.to_string_lossy().replace('\'', "'\\''")))
        .collect();
    fs::write(&list, body).map_err(|e| e.to_string())?;
    let out = cache.join(out_name);
    let tmp = cache.join(format!("{stem}.part.mp4"));
    let mut args: Vec<String> = vec!["-y".into(), "-loglevel".into(), "error".into()];
    if segments.is_empty() {
        args.extend(["-f".into(), "lavfi".into(), "-i".into(), format!("color=c=black:s={width}x{height}:r={FPS}:d=0.1")]);
    } else {
        args.extend(["-f".into(), "concat".into(), "-safe".into(), "0".into(), "-i".into(), list.to_string_lossy().into_owned()]);
    }
    args.extend(
        ["-i", &wav_path.to_string_lossy(), "-map", "0:v", "-map", "1:a", "-c:v", "copy", "-c:a", "aac", "-b:a", "192k", "-movflags", "+faststart"]
            .iter()
            .map(|s| s.to_string()),
    );
    args.push(tmp.to_string_lossy().into_owned());
    media::run(&args)?;
    fs::rename(&tmp, &out).map_err(|e| e.to_string())?;
    on_progress(total, total);
    Ok(())
}

/// 1) 클립 → 잘라낼 구간, 2) 오디오 이어 붙이기. 반환: (48kHz 오디오, 타임라인, 조각)
pub fn build_audio(root: &Path, clips: &[Clip]) -> Result<(Vec<f32>, Vec<TimelineItem>, Vec<(Option<usize>, Piece)>), String> {
    let mut wavs: HashMap<String, Wav> = HashMap::new();
    // 1) 클립 → 실제 잘라낼 구간
    let mut pieces: Vec<(Option<usize>, Piece)> = Vec::new();
    for (i, clip) in clips.iter().enumerate() {
        if i > 0 && clips[i - 1].line != clip.line {
            pieces.push((None, Piece::Silence(LINE_GAP)));
        } else if i > 0 && clips[i - 1].word != clip.word {
            let gap = WORD_GAP_MS.load(Ordering::Relaxed) as f64 / 1000.0;
            if gap > 0.0 {
                pieces.push((None, Piece::Silence(gap)));
            }
        }
        match (clip.kind, clip.source()) {
            (ClipKind::Missing, _) | (_, None) => pieces.push((Some(i), Piece::Silence(MISSING_LEN))),
            (_, Some(src)) => {
                let wav = wav_for(&mut wavs, root, &src.asset_id)?;
                // 정렬된 구간은 이미 정확하므로 그대로 (다듬으면 ㅅ·ㅎ 같은 약한 자음이 잘림)
                // (측정 결과 정렬 경계를 조용한 지점으로 옮기거나 앞뒤를 늘이고 줄이면 모두 알아듣기 나빠짐)
                let (s, e) = if src.aligned { (src.start, src.end) } else { trim_silence(wav, src.start, src.end)? };
                let a = s + (e - s) * src.frac.0;
                let b = (s + (e - s) * src.frac.1).max(a + MIN_LEN);
                pieces.push((Some(i), Piece::Audio { asset_id: src.asset_id.clone(), start: a, end: b }));
            }
        }
    }

    // 2) 오디오 이어 붙이기
    let mut audio: Vec<f32> = Vec::new();
    let mut timeline = Vec::new();
    let fade = (FADE_SECS * SR as f64) as usize;
    for (clip, piece) in &pieces {
        let t0 = audio.len() as f64 / SR as f64;
        match piece {
            Piece::Silence(d) => audio.extend(std::iter::repeat_n(0.0, (d * SR as f64) as usize)),
            Piece::Audio { asset_id, start, end } => {
                let mut s = read_range(wavs.get_mut(asset_id).unwrap(), *start, *end)?;
                let rms = (s.iter().map(|x| x * x).sum::<f32>() / s.len().max(1) as f32).sqrt();
                let gain = if rms > 1e-5 { (TARGET_RMS / rms).clamp(0.25, 8.0) } else { 1.0 };
                let n = s.len();
                for (k, x) in s.iter_mut().enumerate() {
                    let edge = k.min(n - 1 - k);
                    let f = if edge < fade { edge as f32 / fade as f32 } else { 1.0 };
                    *x = (*x * gain * f).clamp(-1.0, 1.0);
                }
                audio.extend(s);
            }
        }
        let t1 = audio.len() as f64 / SR as f64;
        timeline.push(TimelineItem { clip: *clip, start: t0, end: t1 });
    }
    Ok((audio, timeline, pieces))
}


/// ffmpeg 작업을 여러 개 동시에 (8GB에서도 무리 없게 최대 4개)
fn run_parallel(jobs: Vec<(PathBuf, Vec<String>)>, cancel: &AtomicBool, mut on_done: impl FnMut(usize)) -> Result<(), String> {
    let workers = std::thread::available_parallelism().map(|n| n.get() / 2).unwrap_or(2).clamp(1, 4);
    let queue = Arc::new(Mutex::new(jobs.into_iter()));
    let done = Arc::new(AtomicUsize::new(0));
    let error: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    std::thread::scope(|scope| {
        for _ in 0..workers {
            let (queue, done, error) = (queue.clone(), done.clone(), error.clone());
            scope.spawn(move || loop {
                if cancel.load(Ordering::Relaxed) || error.lock().unwrap().is_some() {
                    break;
                }
                let Some((out, args)) = queue.lock().unwrap().next() else { break };
                let tmp = PathBuf::from(args.last().unwrap());
                let result = media::run(&args).and_then(|_| fs::rename(&tmp, &out).map_err(|e| e.to_string()));
                if let Err(e) = result {
                    let _ = fs::remove_file(&tmp);
                    *error.lock().unwrap() = Some(e);
                    break;
                }
                done.fetch_add(1, Ordering::Relaxed);
            });
        }
        // 진행률 보고는 호출 스레드에서
        let mut last = usize::MAX;
        loop {
            let d = done.load(Ordering::Relaxed);
            if d != last {
                on_done(d);
                last = d;
            }
            let queue_empty = queue.lock().unwrap().len() == 0;
            if (queue_empty && Arc::strong_count(&done) == 1) || error.lock().unwrap().is_some() || cancel.load(Ordering::Relaxed) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    });
    if cancel.load(Ordering::Relaxed) {
        return Err(crate::stt::CANCELLED.into());
    }
    let err = error.lock().unwrap().take();
    match err {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

pub fn wav_for<'a>(wavs: &'a mut HashMap<String, Wav>, root: &Path, id: &str) -> Result<&'a mut Wav, String> {
    if !wavs.contains_key(id) {
        let w = hound::WavReader::open(project::cache_wav(root, id)).map_err(|e| format!("오디오 캐시 열기 실패: {e}"))?;
        wavs.insert(id.to_string(), w);
    }
    Ok(wavs.get_mut(id).unwrap())
}

pub fn write_wav(path: &Path, samples: &[f32]) -> Result<(), String> {
    write_wav_ch(path, samples, 1)
}

/// `channels`가 2면 `samples`는 L, R 번갈아
pub fn write_wav_ch(path: &Path, samples: &[f32], channels: u16) -> Result<(), String> {
    let spec = hound::WavSpec { channels, sample_rate: SR, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
    let mut w = hound::WavWriter::create(path, spec).map_err(|e| e.to_string())?;
    for &x in samples {
        w.write_sample((x * 32767.0) as i16).map_err(|e| e.to_string())?;
    }
    w.finalize().map_err(|e| e.to_string())
}

/// 실제 프로젝트로 확인: DCINVOX_TEST_PROJECT=<폴더> cargo test --release render_real -- --ignored --nocapture
#[cfg(test)]
mod real_project {
    use super::*;

    #[test]
    #[ignore]
    fn render_real() {
        let root = PathBuf::from(std::env::var("DCINVOX_TEST_PROJECT").unwrap());
        let words = crate::index::load_words(&crate::index::open(&root).unwrap()).unwrap();
        let lyrics = std::env::var("DCINVOX_TEST_LYRICS").unwrap_or_else(|_| "우리 모두 사랑해".into());
        let t0 = std::time::Instant::now();
        let clips = crate::compose::compose(&lyrics, &words);
        println!("compose: {} words indexed, {:.0}ms", words.len(), t0.elapsed().as_secs_f64() * 1000.0);
        for c in &clips {
            let s = c.source().map(|s| format!("{} [{:.2}-{:.2}] frac {:.2}-{:.2} score {:.2} (+{} alt)", s.said, s.start, s.end, s.frac.0, s.frac.1, s.score, c.candidates.len() - 1)).unwrap_or_default();
            println!("  L{} {:?} '{}' ← {}", c.line, c.kind, c.lyric, s);
        }
        let t1 = std::time::Instant::now();
        let out = render(&root, &clips, |d, t| eprint!("\r{d}/{t}"), Arc::new(AtomicBool::new(false))).unwrap();
        println!("\nrender {:.1}s → {} ({:.2}s)", t1.elapsed().as_secs_f64(), root.join(project::CACHE_DIR).join(PREVIEW_FILE).display(), out.timeline.last().unwrap().end);
    }
}
