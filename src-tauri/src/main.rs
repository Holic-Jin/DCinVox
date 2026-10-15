// 릴리스 빌드에서 Windows 콘솔 창이 뜨지 않도록
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[macro_use]
mod i18n;
mod align;
mod analyze;
mod bench;
mod compose;
mod convert;
mod g2p;
mod index;
mod media;
mod media_server;
mod melody_eval;
mod midi;
mod models;
mod music;
mod project;
mod render;
mod song;
mod stt;
mod verify;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use media_server::MediaServer;
use project::{Project, ProjectView};
use tauri::{Emitter, Manager, State};

#[derive(Default)]
struct AppState {
    current: Mutex<Option<(PathBuf, Project)>>,
    /// 진행 중인 작업 취소 플래그
    cancel: Arc<AtomicBool>,
}

#[derive(Clone, serde::Serialize)]
struct AnalyzeProgress {
    /// "stt" (음성 인식) | "align" (발음 정렬) | "check" (조각 미리 듣기)
    stage: &'static str,
    index: usize,
    total: usize,
    file: String,
    percent: i32,
}

#[derive(Clone, serde::Serialize)]
struct GenerateProgress {
    /// "verify" (잘 들리는 후보 고르기) | "render" (영상 조각 만들기)
    stage: &'static str,
    done: usize,
    total: usize,
}

#[derive(serde::Serialize)]
struct AnalyzeResult {
    project: ProjectView,
    errors: Vec<String>,
    cancelled: bool,
}

#[derive(Clone, serde::Serialize)]
struct ImportProgress {
    done: usize,
    total: usize,
    file: String,
}

#[derive(serde::Serialize)]
struct ImportResult {
    project: ProjectView,
    errors: Vec<String>,
}

#[tauri::command]
fn open_project(path: String, state: State<AppState>, server: State<MediaServer>) -> Result<ProjectView, String> {
    let root = PathBuf::from(path);
    let project = project::open_or_create(&root)?;
    server.set_root(&root);
    let view = project::view(&root, &project);
    *state.current.lock().unwrap() = Some((root, project));
    Ok(view)
}

#[tauri::command]
async fn import_videos(
    paths: Vec<String>,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<ImportResult, String> {
    let (root, mut proj) = state
        .current
        .lock()
        .unwrap()
        .clone()
        .ok_or(tr!("no_project"))?;

    // ffmpeg 처리는 오래 걸리므로 UI 스레드를 막지 않게 블로킹 스레드에서 실행
    let (root, proj, errors) = tauri::async_runtime::spawn_blocking(move || {
        let total = paths.len();
        let mut errors = Vec::new();
        for (i, p) in paths.iter().enumerate() {
            let src = PathBuf::from(p);
            let file = src.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let _ = app.emit("import-progress", ImportProgress { done: i, total, file: file.clone() });
            if let Err(e) = project::import_video(&root, &mut proj, &src) {
                errors.push(format!("{file}: {e}"));
            }
        }
        let _ = app.emit("import-progress", ImportProgress { done: total, total, file: String::new() });
        (root, proj, errors)
    })
    .await
    .map_err(|e| e.to_string())?;

    project::save(&root, &proj)?;
    let view = project::view(&root, &proj);
    *state.current.lock().unwrap() = Some((root, proj));
    Ok(ImportResult { project: view, errors })
}

#[tauri::command]
fn remove_asset(asset_id: String, state: State<AppState>) -> Result<ProjectView, String> {
    let mut guard = state.current.lock().unwrap();
    let (root, proj) = guard.as_mut().ok_or(tr!("no_project"))?;
    project::remove_asset(root, proj, &asset_id)?;
    index::remove_asset(&index::open(root)?, &asset_id)?;
    project::save(root, proj)?;
    Ok(project::view(root, proj))
}

/// 아직 분석되지 않은 영상을 모두 분석. 영상 하나가 끝날 때마다 저장(중간에 꺼져도 이어하기 가능).
#[tauri::command]
async fn analyze(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<AnalyzeResult, String> {
    let (root, proj) = state.current.lock().unwrap().clone().ok_or(tr!("no_project"))?;
    let pending: Vec<_> = proj
        .assets
        .iter()
        .filter(|a| a.analysis != project::StepStatus::Done)
        .map(|a| (a.id.clone(), a.file_name.clone()))
        .collect();
    let mut to_align: Vec<_> = proj
        .assets
        .iter()
        .filter(|a| a.analysis == project::StepStatus::Done && !a.aligned)
        .map(|a| (a.id.clone(), a.file_name.clone()))
        .collect();
    let mut to_check: Vec<_> = proj
        .assets
        .iter()
        .filter(|a| a.analysis == project::StepStatus::Done && a.aligned && !a.checked)
        .map(|a| (a.id.clone(), a.file_name.clone()))
        .collect();
    let cancel = state.cancel.clone();
    cancel.store(false, Ordering::Relaxed);

    let app2 = app.clone();
    let (errors, cancelled) = tauri::async_runtime::spawn_blocking(move || {
        let mut errors = Vec::new();
        // 영상마다 상태 저장 (체크포인트 — 중간에 꺼져도 이어하기)
        let checkpoint = |id: &str, f: &dyn Fn(&mut project::Asset), errors: &mut Vec<String>| {
            let st = app2.state::<AppState>();
            let mut guard = st.current.lock().unwrap();
            if let Some((r, p)) = guard.as_mut().filter(|(r, _)| *r == root) {
                if let Some(a) = p.assets.iter_mut().find(|a| a.id == id) {
                    f(a);
                }
                if let Err(e) = project::save(r, p) {
                    errors.push(e);
                }
            }
        };
        let progress_for = |stage: &'static str, i: usize, total: usize, file: &str| {
            let emit_app = app2.clone();
            let f = file.to_string();
            let _ = app2.emit("analyze-progress", AnalyzeProgress { stage, index: i, total, file: f.clone(), percent: 0 });
            move |p: i32| {
                let _ = emit_app.emit("analyze-progress", AnalyzeProgress { stage, index: i, total, file: f.clone(), percent: p });
            }
        };

        // 1) 음성 인식 — 모델은 이 블록 안에서만 메모리에 존재 (8GB 대응: 끝나면 해제 후 정렬 모델 로드)
        if !pending.is_empty() {
            let recognizer = match models::find(models::WHISPER_MODEL).and_then(|m| stt::Recognizer::load(&m)) {
                Ok(r) => r,
                Err(e) => return (vec![e], false),
            };
            let total = pending.len();
            for (i, (id, file)) in pending.into_iter().enumerate() {
                let result = analyze::analyze_asset(&root, &id, &recognizer, progress_for("stt", i, total, &file), cancel.clone());
                if cancel.load(Ordering::Relaxed) {
                    return (errors, true);
                }
                match result {
                    Ok(_) => {
                        checkpoint(&id, &|a| { a.analysis = project::StepStatus::Done; a.aligned = false; }, &mut errors);
                        to_align.push((id, file));
                    }
                    Err(e) => {
                        errors.push(format!("{file}: {e}"));
                        checkpoint(&id, &|a| a.analysis = project::StepStatus::Error, &mut errors);
                    }
                }
            }
        }

        // 2) 음소 정렬 (음절 단위 정밀 자르기) → 3) 조각 미리 듣기 (같은 모델 사용)
        if !to_align.is_empty() || !to_check.is_empty() {
            let mut aligner = match models::find(align::MODEL_FILE)
                .and_then(|m| models::find(align::VOCAB_FILE).map(|v| (m, v)))
                .and_then(|(m, v)| align::Aligner::load(&m, &v))
            {
                Ok(a) => a,
                Err(e) => return (vec![e], false),
            };
            let total = to_align.len();
            for (i, (id, file)) in to_align.into_iter().enumerate() {
                let result = analyze::align_asset(&root, &id, &mut aligner, progress_for("align", i, total, &file), cancel.clone());
                if cancel.load(Ordering::Relaxed) {
                    return (errors, true);
                }
                match result {
                    Ok(_) => {
                        checkpoint(&id, &|a| { a.aligned = true; a.checked = false; }, &mut errors);
                        to_check.push((id, file));
                    }
                    Err(e) => errors.push(tr!("align_step_failed", file, e)),
                }
            }
            let total = to_check.len();
            for (i, (id, file)) in to_check.into_iter().enumerate() {
                let result = analyze::check_asset(&root, &id, &mut aligner, progress_for("check", i, total, &file), cancel.clone());
                if cancel.load(Ordering::Relaxed) {
                    return (errors, true);
                }
                match result {
                    Ok(_) => checkpoint(&id, &|a| a.checked = true, &mut errors),
                    Err(e) => errors.push(tr!("check_step_failed", file, e)),
                }
            }
        }
        (errors, false)
    })
    .await
    .map_err(|e| e.to_string())?;

    let guard = state.current.lock().unwrap();
    let (root, proj) = guard.as_ref().ok_or(tr!("project_closed"))?;
    Ok(AnalyzeResult { project: project::view(root, proj), errors, cancelled })
}

#[tauri::command]
fn set_language(lang: String) {
    i18n::set(&lang);
}

#[tauri::command]
fn cancel_job(state: State<AppState>) {
    state.cancel.store(true, Ordering::Relaxed);
}

#[tauri::command]
fn get_transcript(asset_id: String, state: State<AppState>) -> Result<index::TranscriptView, String> {
    let guard = state.current.lock().unwrap();
    let (root, _) = guard.as_ref().ok_or(tr!("no_project"))?;
    index::transcript(&index::open(root)?, &asset_id)
}

/// 가사 → 클립 매칭 → 미리보기 렌더
#[tauri::command]
async fn generate(lyrics: String, app: tauri::AppHandle, state: State<'_, AppState>) -> Result<ProjectView, String> {
    let (root, proj) = state.current.lock().unwrap().clone().ok_or(tr!("no_project"))?;
    let prev_song = proj.song.clone();
    let cancel = state.cancel.clone();
    cancel.store(false, Ordering::Relaxed);
    let r = root.clone();
    let l = lyrics.clone();
    let (generation, song) = tauri::async_runtime::spawn_blocking(move || -> Result<(project::Generation, song::Song), String> {
        let words = index::load_words(&index::open(&r)?)?;
        if words.is_empty() {
            return Err(tr!("need_analyze").into());
        }
        let mut clips = compose::compose(&l, &words);
        if clips.is_empty() {
            return Err(tr!("need_lyrics").into());
        }
        // 잘라 놓고 들었을 때 가장 또렷한 후보 고르기 (정렬 모델은 이 블록 안에서만 메모리에 존재)
        {
            let mut aligner = models::find(align::MODEL_FILE)
                .and_then(|m| models::find(align::VOCAB_FILE).map(|v| (m, v)))
                .and_then(|(m, v)| align::Aligner::load(&m, &v))?;
            verify::rerank(&r, &mut clips, &mut aligner, verify::WEIGHT, |done, total| {
                let _ = app.emit("generate-progress", GenerateProgress { stage: "verify", done, total });
            })?;
        }
        let out = render::render(&r, &clips, |done, total| {
            let _ = app.emit("generate-progress", GenerateProgress { stage: "render", done, total });
        }, cancel)?;
        // 음절마다 음계 블록 만들기 (기본: 원래 말투에 가까운 로봇 노래). 템포·키 등은 이전 설정 유지
        let song = convert::init_song(&r, &clips, prev_song.as_ref(), song::Preset::Robot)?;
        Ok((project::Generation { lyrics: l, clips, timeline: out.timeline }, song))
    })
    .await
    .map_err(|e| e.to_string())??;

    let mut guard = state.current.lock().unwrap();
    let (root2, proj) = guard.as_mut().filter(|(r, _)| *r == root).ok_or(tr!("project_changed"))?;
    proj.lyrics = lyrics;
    proj.generation = Some(generation);
    proj.song = Some(song);
    proj.conversion = None;
    project::save(root2, proj)?;
    Ok(project::view(root2, proj))
}

/// 클립의 원본을 다음 후보로 바꾸고 다시 렌더
#[tauri::command]
async fn swap_clip(index: usize, state: State<'_, AppState>) -> Result<ProjectView, String> {
    let (root, proj) = state.current.lock().unwrap().clone().ok_or(tr!("no_project"))?;
    let mut generation = proj.generation.ok_or(tr!("no_generation"))?;
    let clip = generation.clips.get_mut(index).ok_or(tr!("bad_clip"))?;
    if clip.candidates.len() < 2 {
        return Err(tr!("no_alternative").into());
    }
    clip.choice = (clip.choice + 1) % clip.candidates.len();
    let cancel = state.cancel.clone();
    cancel.store(false, Ordering::Relaxed);
    let r = root.clone();
    let prev_song = proj.song.clone();
    let (generation, song) = tauri::async_runtime::spawn_blocking(move || -> Result<(project::Generation, Option<song::Song>), String> {
        let out = render::render(&r, &generation.clips, |_, _| {}, cancel)?;
        generation.timeline = out.timeline;
        // 원본이 바뀐 음절의 원래 음높이만 다시 재고, 사용자가 고른 음계는 유지
        let song = match prev_song {
            Some(prev) => {
                let fresh = convert::init_song(&r, &generation.clips, Some(&prev), song::Preset::Robot)?;
                let mut s = prev;
                for (n, f) in s.notes.iter_mut().zip(&fresh.notes) {
                    if n.clip == index {
                        n.source_hz = f.source_hz;
                    }
                }
                Some(s)
            }
            None => None,
        };
        Ok((generation, song))
    })
    .await
    .map_err(|e| e.to_string())??;

    let mut guard = state.current.lock().unwrap();
    let (root2, proj) = guard.as_mut().filter(|(r, _)| *r == root).ok_or(tr!("project_changed"))?;
    proj.generation = Some(generation);
    if song.is_some() {
        proj.song = song;
    }
    proj.conversion = None;
    project::save(root2, proj)?;
    Ok(project::view(root2, proj))
}

/// 음계 블록 편집 저장 (블록 옮기기·길이·템포·키 등). 변환 결과는 무효화
#[tauri::command]
fn update_song(song: song::Song, state: State<AppState>) -> Result<(), String> {
    let mut guard = state.current.lock().unwrap();
    let (root, proj) = guard.as_mut().ok_or(tr!("no_project"))?;
    proj.song = Some(song);
    proj.conversion = None;
    project::save(root, proj)
}

/// 프리셋 적용: robot | monotone | speech
#[tauri::command]
fn apply_preset(preset: song::Preset, state: State<AppState>) -> Result<song::Song, String> {
    let mut guard = state.current.lock().unwrap();
    let (root, proj) = guard.as_mut().ok_or(tr!("no_project"))?;
    let s = proj.song.as_mut().ok_or(tr!("need_generate"))?;
    song::apply_preset(s, preset);
    let out = s.clone();
    proj.conversion = None;
    project::save(root, proj)?;
    Ok(out)
}

/// 반주(MR) 불러오기: Assets/로 복사 → 분석(빠르기·첫 박·키) → 노래 설정에 반영
#[tauri::command]
async fn import_mr(path: String, state: State<'_, AppState>) -> Result<song::Song, String> {
    let (root, proj) = state.current.lock().unwrap().clone().ok_or(tr!("no_project"))?;
    let mut s = proj.song.ok_or(tr!("need_generate"))?;
    let r = root.clone();
    let s = tauri::async_runtime::spawn_blocking(move || -> Result<song::Song, String> {
        let src = PathBuf::from(&path);
        let assets = r.join(project::ASSETS_DIR);
        let name = project::unique_file_name(&assets, &src)?;
        let dest = assets.join(&name);
        if src.canonicalize().ok() != dest.canonicalize().ok() {
            std::fs::copy(&src, &dest).map_err(|e| tr!("mr_copy_failed", e))?;
        }
        let cache = r.join(project::CACHE_DIR);
        media::extract_wav_ch(&dest, &cache.join(convert::MR_CACHE), project::OUTPUT_SAMPLE_RATE, 2)?;
        let probe = cache.join("mr_analysis.wav");
        media::extract_wav_ch(&dest, &probe, 22_050, 2)?;
        let stereo: Vec<f32> = hound::WavReader::open(&probe)
            .map_err(|e| e.to_string())?
            .samples::<i16>()
            .map(|v| v.map(|x| x as f32 / 32768.0))
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string())?;
        let (left, right): (Vec<f32>, Vec<f32>) = stereo.chunks(2).map(|c| (c[0], *c.get(1).unwrap_or(&c[0]))).unzip();
        let mono: Vec<f32> = left.iter().zip(&right).map(|(a, b)| 0.5 * (a + b)).collect();
        let a = music::analyze(&mono, 22_050);
        // 원곡 목소리가 들어 있으면 멜로디를 따고, 조성도 멜로디 기준이 더 정확
        let melody = music::clean_melody(music::extract_melody(&left, &right, 22_050));
        let (key, tonic, minor) = music::key_from_melody(&melody).unwrap_or((a.key, a.tonic, a.minor));
        s.bpm = a.bpm.round();
        s.key = key;
        let found = !melody.is_empty();
        s.mr = Some(song::Mr { file: name, start: a.first_beat, skip: 0.0, gain: 1.0, detected_bpm: a.bpm, tonic, minor, melody, syllables: Vec::new(), syllables_for: String::new() });
        // 멜로디는 로봇 오토튠을 누를 때 적용. 지금은 바뀐 키 기준으로 목소리 그대로의 음계만 다시 잡음
        if found {
            s.midi = None; // 나중에 불러온 쪽 멜로디를 따름
        }
        song::voice_degrees(&mut s);
        Ok(s)
    })
    .await
    .map_err(|e| e.to_string())??;
    let mut guard = state.current.lock().unwrap();
    let (root2, proj) = guard.as_mut().filter(|(r, _)| *r == root).ok_or(tr!("project_changed"))?;
    proj.song = Some(s.clone());
    proj.conversion = None;
    project::save(root2, proj)?;
    Ok(s)
}

#[derive(Clone, serde::Serialize)]
struct ConvertProgress {
    stage: &'static str,
    done: usize,
    total: usize,
}

/// 음계대로 노래로 변환 + 영상 만들기 → cache/converted.mp4
#[tauri::command]
async fn convert_song(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<ProjectView, String> {
    let (root, proj) = state.current.lock().unwrap().clone().ok_or(tr!("no_project"))?;
    let generation = proj.generation.ok_or(tr!("need_generate"))?;
    let song = proj.song.ok_or(tr!("need_generate"))?;
    let cancel = state.cancel.clone();
    cancel.store(false, Ordering::Relaxed);
    let r = root.clone();
    let spans = tauri::async_runtime::spawn_blocking(move || -> Result<Vec<convert::NoteSpan>, String> {
        let out = convert::convert(&r, &generation.clips, &song, |done, total| {
            let _ = app.emit("convert-progress", ConvertProgress { stage: "sing", done, total });
        })?;
        let cache = r.join(project::CACHE_DIR);
        // 목소리만 / 반주 섞은 소리 둘 다 저장 (내보내기에서 고를 수 있게)
        render::write_wav(&cache.join(convert::VOCAL_WAV), &out.audio)?;
        let (wav, total) = match &song.mr {
            Some(mr) => {
                let mix = convert::mix_with_mr(&r, &out.audio, mr)?;
                let path = cache.join(convert::MIX_WAV);
                render::write_wav_ch(&path, &mix, 2)?;
                (path, mix.len() as f64 / 2.0 / project::OUTPUT_SAMPLE_RATE as f64)
            }
            None => {
                let _ = std::fs::remove_file(cache.join(convert::MIX_WAV));
                (cache.join(convert::VOCAL_WAV), out.audio.len() as f64 / project::OUTPUT_SAMPLE_RATE as f64)
            }
        };
        render::render_video(&r, &convert::video_segments(&out, total), &wav, convert::OUTPUT_FILE, render::PREVIEW_SIZE, |done, total| {
            let _ = app.emit("convert-progress", ConvertProgress { stage: "video", done, total });
        }, cancel)?;
        Ok(out.spans)
    })
    .await
    .map_err(|e| e.to_string())??;

    let mut guard = state.current.lock().unwrap();
    let (root2, proj) = guard.as_mut().filter(|(r, _)| *r == root).ok_or(tr!("project_changed"))?;
    proj.conversion = Some(spans);
    project::save(root2, proj)?;
    Ok(project::view(root2, proj))
}

#[derive(serde::Serialize)]
struct MidiResult {
    song: song::Song,
    /// 가사 음절에 배정한 음 수
    assigned: usize,
    midi_notes: usize,
    syllables: usize,
}

/// MIDI 멜로디 불러오기 → 음절에 순서대로 배정
#[tauri::command]
fn import_midi(path: String, state: State<AppState>) -> Result<MidiResult, String> {
    let bytes = std::fs::read(&path).map_err(|e| tr!("midi_read_failed", e))?;
    let melody = midi::read_melody(&bytes)?;
    let mut guard = state.current.lock().unwrap();
    let (root, proj) = guard.as_mut().ok_or(tr!("no_project"))?;
    let s = proj.song.as_mut().ok_or(tr!("need_generate"))?;
    // 바로 바꾸지 않고 보관 → 로봇 오토튠을 누르면 적용
    let assigned = melody.notes.len().min(s.notes.len());
    let midi_notes = melody.notes.len();
    s.midi = Some(melody);
    let out = MidiResult { song: s.clone(), assigned, midi_notes, syllables: s.notes.len() };
    proj.conversion = None;
    project::save(root, proj)?;
    Ok(out)
}

/// 내보내기: kind = "mp4" | "wav_mix" | "wav_vocal" | "midi"
#[tauri::command]
async fn export(kind: String, dest: String, app: tauri::AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let (root, proj) = state.current.lock().unwrap().clone().ok_or(tr!("no_project"))?;
    let cancel = state.cancel.clone();
    cancel.store(false, Ordering::Relaxed);
    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let cache = root.join(project::CACHE_DIR);
        let dest = PathBuf::from(dest);
        let mix = cache.join(convert::MIX_WAV);
        let vocal = cache.join(convert::VOCAL_WAV);
        let need_converted = || if proj.conversion.is_some() { Ok(()) } else { Err(tr!("need_convert").to_string()) };
        match kind.as_str() {
            "midi" => {
                let generation = proj.generation.as_ref().ok_or(tr!("need_generate"))?;
                let s = proj.song.as_ref().ok_or(tr!("need_generate"))?;
                std::fs::write(&dest, midi::write_midi(s, &generation.clips)?).map_err(|e| e.to_string())
            }
            "wav_vocal" => {
                need_converted()?;
                std::fs::copy(&vocal, &dest).map(|_| ()).map_err(|e| e.to_string())
            }
            "wav_mix" => {
                need_converted()?;
                std::fs::copy(if mix.is_file() { &mix } else { &vocal }, &dest).map(|_| ()).map_err(|e| e.to_string())
            }
            "mp4" => {
                need_converted()?;
                // 변환을 다시 돌려 고화질 영상 구간으로 렌더 (소리는 변환 결과 그대로)
                let generation = proj.generation.as_ref().ok_or(tr!("need_generate"))?;
                let s = proj.song.as_ref().ok_or(tr!("need_generate"))?;
                let out = convert::convert(&root, &generation.clips, s, |_, _| {})?;
                let wav = if mix.is_file() { mix } else { vocal };
                let total = hound::WavReader::open(&wav).map(|r| r.duration() as f64 / project::OUTPUT_SAMPLE_RATE as f64).map_err(|e| e.to_string())?;
                render::render_video(&root, &convert::video_segments(&out, total), &wav, "export.mp4", render::EXPORT_SIZE, |done, total| {
                    let _ = app.emit("export-progress", GenerateProgress { stage: "video", done, total });
                }, cancel)?;
                std::fs::copy(cache.join("export.mp4"), &dest).map(|_| ()).map_err(|e| e.to_string())
            }
            other => Err(tr!("unknown_format", other)),
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

/// 로봇 오토튠(원곡 멜로디대로): 원곡 가수가 부른 음절 위치·음을 찾아, 사용자 가사 음절을 처음부터 순서대로 얹음.
/// 가사 내용은 상관없음 ("짝반짝작은별"도 "반짝반짝작은" 멜로디로). 원곡 음절은 반주마다 한 번만 찾고 다시 씀
#[tauri::command]
async fn follow_original(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<song::Song, String> {
    let (root, proj) = state.current.lock().unwrap().clone().ok_or(tr!("no_project"))?;
    let mut s = proj.song.ok_or(tr!("need_generate"))?;
    let cancel = state.cancel.clone();
    cancel.store(false, Ordering::Relaxed);
    let r = root.clone();
    let s = tauri::async_runtime::spawn_blocking(move || -> Result<song::Song, String> {
        let mr = s.mr.clone().ok_or(tr!("no_mr"))?;
        if mr.syllables.is_empty() || mr.syllables_for != ORIGINAL_SYLLABLES_V {
            let say = |stage: &'static str| {
                let _ = app.emit("follow-progress", stage);
            };
            let cache = r.join(project::CACHE_DIR);
            say("reading");
            let stereo: Vec<f32> = hound::WavReader::open(cache.join("mr_analysis.wav"))
                .map_err(|e| tr!("mr_reload", e))?
                .samples::<i16>()
                .map(|v| v.map(|x| x as f32 / 32768.0))
                .collect::<Result<_, _>>()
                .map_err(|e| e.to_string())?;
            let (left, right): (Vec<f32>, Vec<f32>) = stereo.chunks(2).map(|c| (c[0], *c.get(1).unwrap_or(&c[0]))).unzip();
            // 빠르기·박 위치를 곡 전체로 정밀하게 다시 (반올림 없이 그대로 사용)
            let mono: Vec<f32> = left.iter().zip(&right).map(|(a, b)| 0.5 * (a + b)).collect();
            let a = music::analyze(&mono, 22_050);
            {
                let m = s.mr.as_mut().unwrap();
                m.detected_bpm = a.bpm;
                m.start = a.first_beat;
            }
            let contour = music::extract_contour(&left, &right, 22_050).ok_or(tr!("mr_too_short"))?;
            // 가운데(보컬) 강조 신호 → 16kHz
            let center = music::center_signal(&left, &right);
            let c22 = cache.join("mr_center22.wav");
            let c16 = cache.join("mr_center16.wav");
            let spec = hound::WavSpec { channels: 1, sample_rate: 22_050, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
            let mut w = hound::WavWriter::create(&c22, spec).map_err(|e| e.to_string())?;
            for v in &center {
                w.write_sample((v.clamp(-1.0, 1.0) * 32767.0) as i16).map_err(|e| e.to_string())?;
            }
            w.finalize().map_err(|e| e.to_string())?;
            media::extract_wav(&c22, &c16, stt::SAMPLE_RATE)?;
            let mono16 = stt::read_wav(&c16)?;
            say("lyrics");
            // 모델은 이 안에서만 메모리에 (8GB 대응: 인식 → 해제 → 정렬)
            let transcript_syllables = {
                let rec = stt::Recognizer::load(&models::find(models::WHISPER_MODEL)?)?;
                let mut aligner = align::Aligner::load(&models::find(align::MODEL_FILE)?, &models::find(align::VOCAB_FILE)?)?;
                say("locating");
                music::original_syllables(&mono16, &contour, &rec, &mut aligner, None, cancel)?
            };
            let m = s.mr.as_mut().unwrap();
            m.syllables = transcript_syllables;
            m.syllables_for = ORIGINAL_SYLLABLES_V.into();
        }
        if s.mr.as_ref().is_some_and(|m| m.syllables.is_empty()) {
            // 원곡 가사를 못 알아들으면 음표 방식으로
            song::apply_preset(&mut s, song::Preset::Robot);
        } else {
            // 반주 박자에 정확히 맞추기: 감지한 빠르기를 반올림하지 않고 그대로
            let m = s.mr.clone().unwrap();
            s.bpm = m.detected_bpm;
            let spb = s.secs_per_beat();
            // 원곡에서 처음 부른 음절 직전 박을 노래 첫 박으로, 전주는 한 마디 전부터만
            let first = m.syllables.first().map(|x| x.v_start);
            // 노래 첫 박은 반주의 박자 격자 위에서 고름. 격자의 가장 앞 박부터 계산해야
            // 이전에 옮겨 둔 시작 위치와 상관없이 앞쪽으로도 갈 수 있음
            let grid0 = m.start.rem_euclid(spb);
            let mut start = grid0;
            if let Some(t) = first {
                let k = ((t - 0.05 - grid0) / spb).floor();
                start = grid0 + k.max(0.0) * spb;
            }
            let mr = s.mr.as_mut().unwrap();
            mr.start = start;
            mr.skip = (start - 4.0 * spb).max(0.0);
            music::apply_syllables(&mut s, &m.syllables, start);
            s.flat = true;
        }
        Ok(s)
    })
    .await
    .map_err(|e| e.to_string())??;
    let mut guard = state.current.lock().unwrap();
    let (root2, proj) = guard.as_mut().filter(|(r, _)| *r == root).ok_or(tr!("project_changed"))?;
    proj.song = Some(s.clone());
    proj.conversion = None;
    project::save(root2, proj)?;
    Ok(s)
}

/// 원곡 음절 찾기 방식 버전 (바뀌면 저장된 결과를 다시 계산)
const ORIGINAL_SYLLABLES_V: &str = "auto-v6";

/// 반주 빼기
#[tauri::command]
fn remove_mr(state: State<AppState>) -> Result<song::Song, String> {
    let mut guard = state.current.lock().unwrap();
    let (root, proj) = guard.as_mut().ok_or(tr!("no_project"))?;
    let s = proj.song.as_mut().ok_or(tr!("need_generate"))?;
    s.mr = None;
    let out = s.clone();
    proj.conversion = None;
    project::save(root, proj)?;
    Ok(out)
}

/// 영상 재생용 로컬 서버 주소. UI는 `<base>/<프로젝트 기준 상대경로>`로 재생한다.
#[tauri::command]
fn media_base(server: State<MediaServer>) -> String {
    server.base_url()
}

#[tauri::command]
fn save_lyrics(lyrics: String, state: State<AppState>) -> Result<(), String> {
    let mut guard = state.current.lock().unwrap();
    let (root, proj) = guard.as_mut().ok_or(tr!("no_project"))?;
    proj.lyrics = lyrics;
    project::save(root, proj)
}

/// 설치 상태 점검 (`dcinvox --self-check`): 같이 설치돼야 하는 ffmpeg와 AI 모델을 찾는지 출력. 지원 문의 대응용
fn self_check() -> bool {
    let mut ok = true;
    let ff = media::ffmpeg_path();
    let ff_ok = std::process::Command::new(&ff).arg("-version").output().map(|o| o.status.success()).unwrap_or(false);
    println!("ffmpeg: {} {}", ff.display(), if ff_ok { "OK" } else { "FAILED" });
    ok &= ff_ok;
    let mut missing = Vec::new();
    for name in [models::WHISPER_MODEL, align::MODEL_FILE, align::VOCAB_FILE] {
        match models::find(name) {
            Ok(p) => println!("model {name}: {} OK", p.display()),
            Err(e) => {
                println!("model {name}: {e}");
                missing.push(name);
                ok = false;
            }
        }
    }
    if !missing.is_empty() {
        println!("looked in:");
        for d in models::search_dirs() {
            println!("  {}", d.display());
        }
        if missing.contains(&align::MODEL_FILE) {
            println!(
                "note: {onnx} is not in the GitHub zip. Convert it with tools/export_aligner.py (docs/INSTALL.md).",
                onnx = align::MODEL_FILE
            );
        }
    }
    println!("{}", if ok { "SELF-CHECK OK" } else { "SELF-CHECK FAILED" });
    ok
}

fn main() {
    if std::env::args().any(|a| a == "--self-check") {
        std::process::exit(if self_check() { 0 } else { 1 });
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .manage(MediaServer::start().expect("재생 서버 시작 실패"))
        .invoke_handler(tauri::generate_handler![set_language, open_project, import_videos, remove_asset, save_lyrics, media_base, analyze, cancel_job, get_transcript, generate, swap_clip, update_song, apply_preset, convert_song, import_mr, remove_mr, import_midi, export, follow_original])
        .run(tauri::generate_context!())
        .expect("DCinVox 실행 실패");
}
