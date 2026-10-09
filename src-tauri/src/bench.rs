//! 알아듣기 점수 측정 (개발용).
//! 생성된 소리를 Whisper가 다시 듣고 받아 적은 결과를 가사와 비교해 음절 오류율(CER)을 잰다.
//! 낮을수록 잘 들림. 개선 전후 비교에 사용.
//!
//! DCINVOX_TEST_PROJECT=<분석된 프로젝트> cargo test --release bench_intelligibility -- --ignored --nocapture

#![cfg(test)]

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crate::{compose, g2p, index, media, models, render, stt};

/// 영상에 그대로 없는 말 위주 (음절 조각을 많이 쓰게)
const LYRICS: &[&str] = &[
    "야 기분 좋다 우리 대한민국 파이팅",
    "디시 인사이드 파이팅 정부가 전부 다 사랑해",
    "반짝반짝 작은 별 아름답게 비치네",
    "학교 종이 땡땡땡 어서 모이자",
    "산토끼 토끼야 어디를 가느냐",
    "동해물과 백두산이 마르고 닳도록",
    "오늘도 열심히 일하는 당신을 응원합니다",
    "나비야 나비야 이리 날아 오너라",
    "곰 세 마리가 한 집에 있어 아빠 곰 엄마 곰 애기 곰",
    "떴다 떴다 비행기 날아라 날아라",
    "고향의 봄 나의 살던 고향은 꽃피는 산골",
    "졸업식 날 친구들과 사진을 찍었습니다",
    "라면 먹고 갈래 아니면 커피 마실래",
    "오늘 저녁 메뉴는 김치찌개 입니다",
    "시간이 없어서 빨리 가야 합니다",
    "고양이가 생선을 물고 도망갔다",
];

fn edit_distance(a: &[char], b: &[char]) -> usize {
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut cur = vec![i + 1; b.len() + 1];
        for (j, cb) in b.iter().enumerate() {
            cur[j + 1] = (prev[j] + (ca != cb) as usize).min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        prev = cur;
    }
    prev[b.len()]
}

/// 발음형 기준 음절 열 (철자 차이는 무시)
fn syllables(text: &str) -> Vec<char> {
    text.split_whitespace().flat_map(|w| g2p::pron(w).chars().collect::<Vec<_>>()).collect()
}

fn to_16k(audio48: &[f32], dir: &Path) -> Vec<f32> {
    let a = dir.join("bench48.wav");
    let b = dir.join("bench16.wav");
    let spec = hound::WavSpec { channels: 1, sample_rate: 48_000, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
    let mut w = hound::WavWriter::create(&a, spec).unwrap();
    for &x in audio48 {
        w.write_sample((x * 32767.0) as i16).unwrap();
    }
    w.finalize().unwrap();
    media::extract_wav(&a, &b, stt::SAMPLE_RATE).unwrap();
    stt::read_wav(&b).unwrap()
}

#[test]
#[ignore]
fn bench_intelligibility() {
    let root = PathBuf::from(std::env::var("DCINVOX_TEST_PROJECT").unwrap());
    let tag = std::env::var("DCINVOX_BENCH_TAG").unwrap_or_default();
    let words = index::load_words(&index::open(&root).unwrap()).unwrap();
    let weight: f32 = std::env::var("DCINVOX_VERIFY_WEIGHT").ok().and_then(|v| v.parse().ok()).unwrap_or(crate::verify::WEIGHT);
    if let Some(v) = std::env::var("DCINVOX_WORD_GAP_MS").ok().and_then(|v| v.parse::<u32>().ok()) {
        render::WORD_GAP_MS.store(v, std::sync::atomic::Ordering::Relaxed);
    }
    if let Some(v) = std::env::var("DCINVOX_POS_BONUS").ok().and_then(|v| v.parse::<u32>().ok()) {
        compose::POSITION_BONUS_X100.store(v, std::sync::atomic::Ordering::Relaxed);
    }
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../models");
    let mut aligner = crate::align::Aligner::load(&dir.join(crate::align::MODEL_FILE), &dir.join(crate::align::VOCAB_FILE)).unwrap();
    let rec = stt::Recognizer::load(&models::find(models::WHISPER_MODEL).unwrap()).unwrap();
    let tmp = std::env::temp_dir().join(format!("dcinvox-bench-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).unwrap();

    let (mut errs, mut total) = (0, 0);
    let mut verify_secs = 0.0;
    for line in LYRICS {
        let mut clips = compose::compose(line, &words);
        if weight > 0.0 {
            let t = std::time::Instant::now();
            crate::verify::rerank(&root, &mut clips, &mut aligner, weight, |_, _| {}).unwrap();
            verify_secs += t.elapsed().as_secs_f64();
        }
        let (audio, _, _) = render::build_audio(&root, &clips).unwrap();
        // 앞뒤 무음: Whisper가 첫 단어를 놓치지 않게 (실제 사용 환경과 비슷하게)
        let pad = vec![0.0f32; 48_000 / 2];
        let pcm = to_16k(&[pad.clone(), audio, pad].concat(), &tmp);
        let t = rec.transcribe(&pcm, Some("ko"), |_| {}, Arc::new(AtomicBool::new(false))).unwrap();
        let hyp: String = t.segments.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join(" ");
        let (r, h) = (syllables(line), syllables(&hyp));
        // Whisper가 반복 환각을 내면 한 줄이 점수를 지배하므로 줄당 최대 100%
        let e = edit_distance(&r, &h).min(r.len());
        errs += e;
        total += r.len();
        println!("[{:>5.1}%] {line}\n         → {hyp}", 100.0 * e as f64 / r.len() as f64);
    }
    println!("verify time {verify_secs:.1}s for {} lines", LYRICS.len());
    println!("BENCH {tag} CER {:.1}% ({errs}/{total})", 100.0 * errs as f64 / total as f64);
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn cer_basics() {
    assert_eq!(edit_distance(&syllables("같이 가자"), &syllables("가치 가자")), 0);
    assert_eq!(edit_distance(&['가', '나'], &['가', '다', '나']), 1);
}
