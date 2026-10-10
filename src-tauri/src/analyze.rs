//! 분석 단계: 영상 1개 = 16kHz 변환 → 음성 인식 → 색인 저장 → 음소 정렬.
//! (보컬 분리는 이후 마일스톤에서 이 파이프라인에 끼워 넣는다)

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::{align, index, media, project, stt};

pub fn stt_wav(root: &Path, asset_id: &str) -> PathBuf {
    root.join(project::CACHE_DIR).join(format!("{asset_id}.16k.wav"))
}

/// 반환: 색인된 단어 수
pub fn analyze_asset(
    root: &Path,
    asset_id: &str,
    recognizer: &stt::Recognizer,
    on_progress: impl FnMut(i32) + 'static,
    cancel: Arc<AtomicBool>,
) -> Result<usize, String> {
    let wav16 = stt_wav(root, asset_id);
    if !wav16.is_file() {
        media::extract_wav(&project::cache_wav(root, asset_id), &wav16, stt::SAMPLE_RATE)?;
    }
    let pcm = stt::read_wav(&wav16)?;
    let transcript = recognizer.transcribe(&pcm, None, on_progress, cancel)?;
    drop(pcm);

    let mut conn = index::open(root)?;
    index::replace_asset(&mut conn, asset_id, &transcript)
}

/// 음소 정렬: 문장마다 단어·음절 경계를 정밀하게 다시 잡는다. 반환: 정렬된 단어 수
pub fn align_asset(
    root: &Path,
    asset_id: &str,
    aligner: &mut align::Aligner,
    mut on_progress: impl FnMut(i32),
    cancel: Arc<AtomicBool>,
) -> Result<usize, String> {
    let wav16 = stt_wav(root, asset_id);
    if !wav16.is_file() {
        media::extract_wav(&project::cache_wav(root, asset_id), &wav16, stt::SAMPLE_RATE)?;
    }
    let pcm = stt::read_wav(&wav16)?;
    let mut conn = index::open(root)?;
    let segments = index::segments_for_align(&conn, asset_id)?;
    let sr = stt::SAMPLE_RATE as f64;
    let mut results = Vec::new();
    for (i, seg) in segments.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            return Err(tr!("cancelled").into());
        }
        on_progress((i * 100 / segments.len().max(1)) as i32);
        // Whisper 시간이 조금 어긋나 있어도 들어오도록 앞뒤 여유
        let from = (seg.iter().map(|w| w.2).fold(f64::INFINITY, f64::min) - 0.4).max(0.0);
        let to = seg.iter().map(|w| w.3).fold(0.0, f64::max) + 0.4;
        let a = ((from * sr) as usize).min(pcm.len());
        let b = ((to * sr) as usize).min(pcm.len());
        if b <= a {
            continue;
        }
        let words: Vec<align::WordIn> = seg.iter().map(|w| align::WordIn { token_id: w.0, pron: w.1.clone() }).collect();
        if let Some(out) = aligner.align(&pcm[a..b], a as f64 / sr, &words)? {
            results.extend(out);
        }
    }
    let n = results.len();
    index::store_alignment(&mut conn, &results)?;
    Ok(n)
}

/// 재검증 점수 미리 계산: 정렬된 음절·단어를 하나씩 잘라 모델에 들려 주고 자기 소리로 들리는 정도를 저장.
/// 생성할 때 이 점수로 바로 후보를 고를 수 있어 빨라진다. 반환: 계산한 조각 수
pub fn check_asset(
    root: &Path,
    asset_id: &str,
    aligner: &mut align::Aligner,
    mut on_progress: impl FnMut(i32),
    cancel: Arc<AtomicBool>,
) -> Result<usize, String> {
    let pcm = stt::read_wav(&stt_wav(root, asset_id))?;
    let mut conn = index::open(root)?;
    let todo = index::pieces_to_check(&conn, asset_id)?;
    let sr = stt::SAMPLE_RATE as f64;
    let pad = vec![0.0f32; (0.1 * sr) as usize];
    let mut done = Vec::new();
    for (i, (table, id, pron, start, end)) in todo.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            index::store_checks(&mut conn, &done)?; // 계산한 만큼은 저장 → 이어하기
            return Err(tr!("cancelled").into());
        }
        if i % 50 == 0 {
            on_progress((i * 100 / todo.len().max(1)) as i32);
        }
        let a = ((start * sr) as usize).min(pcm.len());
        let b = ((end * sr) as usize).min(pcm.len()).max(a);
        let piece = [pad.as_slice(), &pcm[a..b], &pad].concat();
        let score = aligner.match_scores(&[piece], pron)?.first().copied().flatten().unwrap_or(-1.0);
        done.push((*table, *id, score));
        if done.len() >= 500 {
            index::store_checks(&mut conn, &done)?;
            done.clear();
        }
    }
    index::store_checks(&mut conn, &done)?;
    Ok(todo.len())
}

/// 실제 프로젝트(복사본 권장)로 확인: DCINVOX_TEST_PROJECT=<폴더> cargo test --release align_real -- --ignored --nocapture
#[cfg(test)]
mod real_project {
    use super::*;

    #[test]
    #[ignore]
    fn align_real() {
        let root = PathBuf::from(std::env::var("DCINVOX_TEST_PROJECT").unwrap());
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../models");
        let mut aligner = align::Aligner::load(&dir.join(align::MODEL_FILE), &dir.join(align::VOCAB_FILE)).unwrap();
        let t0 = std::time::Instant::now();
        let n = align_asset(&root, "a0001", &mut aligner, |p| eprint!("\r{p}%"), Arc::new(AtomicBool::new(false))).unwrap();
        println!("\naligned words {n} in {:.0}s", t0.elapsed().as_secs_f64());
        let conn = index::open(&root).unwrap();
        let mut st = conn
            .prepare("SELECT t.text, t.start, t.end, group_concat(s.unit || printf('[%.2f-%.2f v%.2f-%.2f]', s.start, s.end, s.v_start, s.v_end), ' ')
                      FROM tokens t JOIN syllables s ON s.token_id = t.id
                      WHERE t.start BETWEEN 300 AND 330 GROUP BY t.id ORDER BY t.start")
            .unwrap();
        let rows = st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?, r.get::<_, f64>(2)?, r.get::<_, String>(3)?))).unwrap();
        for r in rows {
            let (t, s, e, syl) = r.unwrap();
            println!("{t} {s:.2}-{e:.2}: {syl}");
        }
        let (total, aligned): (i64, i64) = conn.query_row("SELECT count(*), sum(aligned) FROM tokens", [], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        println!("coverage {aligned}/{total}");
    }

    #[test]
    #[ignore]
    fn check_real() {
        let root = PathBuf::from(std::env::var("DCINVOX_TEST_PROJECT").unwrap());
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../models");
        let mut aligner = align::Aligner::load(&dir.join(align::MODEL_FILE), &dir.join(align::VOCAB_FILE)).unwrap();
        let t0 = std::time::Instant::now();
        let n = check_asset(&root, "a0001", &mut aligner, |p| eprint!("\r{p}%"), Arc::new(AtomicBool::new(false))).unwrap();
        println!("\nchecked pieces {n} in {:.0}s", t0.elapsed().as_secs_f64());
    }
}
