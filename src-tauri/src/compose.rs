//! 가사 → 잘라 붙일 조각(클립) 목록.
//! 우선순위: 구(연속된 여러 단어) > 단어 > 음절 조각. 모두 발음형으로 비교한다.
//! 음절 조각의 위치는 지금은 단어 길이를 음절 수로 나눈 근사값이고, 마일스톤 4의 강제정렬로 정밀해진다.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::g2p;
use crate::index::{IndexedWord, SylSpan};

/// 후보는 넉넉히 모아 두고 재검증(verify)에서 실제로 잘 들리는 것을 고른다
pub const MAX_CANDIDATES: usize = 20;

/// 단어 안 위치(처음·끝)가 가사와 같은 조각에 주는 가산점 ×100 (bench로 조정)
pub static POSITION_BONUS_X100: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(100); // 측정: 0.15 → 1.0에서 CER 32.3% → 30.6%

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClipKind {
    Phrase,
    Word,
    Syllable,
    Missing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub asset_id: String,
    /// 원본 단어(또는 구) 구간
    pub start: f64,
    pub end: f64,
    /// 단어 안에서 쓸 부분 (0~1 비율). 단어·구 전체면 (0, 1)
    pub frac: (f64, f64),
    /// 원본에서 실제로 말한 단어 (화면 표시용)
    pub said: String,
    pub score: f32,
    /// 음소 정렬된 정확한 구간이면 true (렌더에서 무음 다듬기 생략)
    #[serde(default)]
    pub aligned: bool,
    /// 구간 안 음절들의 시간·모음 구간 (변환 단계에서 모음만 늘일 때 사용)
    #[serde(default)]
    pub syls: Vec<SylSpan>,
    /// 재검증 점수: 잘라 놓고 들었을 때 목표 소리와 얼마나 같은지 (0에 가까울수록 좋음)
    #[serde(default)]
    pub check: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Clip {
    /// 가사에서 이 클립이 담당하는 글자
    pub lyric: String,
    /// 이 클립이 내야 할 소리 (발음형) — 후보 재검증에 사용
    #[serde(default)]
    pub pron: String,
    pub kind: ClipKind,
    /// 가사 줄 번호 (줄 사이에는 쉼이 들어감)
    pub line: usize,
    /// 줄 안 가사 단어 번호 (단어 사이 짧은 쉼에 사용)
    #[serde(default)]
    pub word: usize,
    /// 현재 선택된 후보 (사용자가 다른 후보로 바꿀 수 있음)
    pub choice: usize,
    pub candidates: Vec<Source>,
}

impl Clip {
    pub fn source(&self) -> Option<&Source> {
        self.candidates.get(self.choice)
    }
}

struct LyricWord {
    text: String,
    pron: String,
}

pub fn compose(lyrics: &str, words: &[IndexedWord]) -> Vec<Clip> {
    let units: Vec<Vec<String>> = words.iter().map(|w| g2p::units(&w.pron)).collect();
    let mut by_pron: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, w) in words.iter().enumerate() {
        by_pron.entry(w.pron.as_str()).or_default().push(i);
    }

    let mut clips = Vec::new();
    let lines = lyrics.lines().map(str::trim).filter(|l| !l.is_empty());
    for (line_no, line) in lines.enumerate() {
        let lw: Vec<LyricWord> = line
            .split_whitespace()
            .map(|t| LyricWord { text: t.to_string(), pron: g2p::pron(t) })
            .filter(|w| !w.pron.is_empty())
            .collect();

        let mut i = 0;
        while i < lw.len() {
            // 1) 구: 같은 문장에서 연속으로 말한 단어들
            let mut best_k = 1;
            let mut starts = Vec::new();
            for &idx in by_pron.get(lw[i].pron.as_str()).into_iter().flatten() {
                let mut k = 1;
                while i + k < lw.len()
                    && idx + k < words.len()
                    && words[idx + k].segment_id == words[idx].segment_id
                    && words[idx + k].pron == lw[i + k].pron
                {
                    k += 1;
                }
                if k > best_k {
                    best_k = k;
                    starts.clear();
                }
                if k == best_k && k >= 2 {
                    starts.push(idx);
                }
            }
            if best_k >= 2 {
                let candidates = rank(starts.iter().map(|&s| {
                    let span = &words[s..s + best_k];
                    let n_units: usize = (s..s + best_k).map(|j| units[j].len()).sum();
                    let prob = span.iter().map(|w| w.prob).sum::<f32>() / best_k as f32;
                    let said = span.iter().map(|w| w.text.as_str()).collect::<Vec<_>>().join(" ");
                    let mut src = source(&span[0], span[best_k - 1].end, (0.0, 1.0), said, prob, n_units, 0.0);
                    if span.iter().zip(&units[s..s + best_k]).all(|(w, u)| w.syls.len() == u.len()) {
                        let syls: Vec<SylSpan> = span.iter().flat_map(|w| w.syls.iter().copied()).collect();
                        set_aligned(&mut src, prob, 0.0, syls);
                    }
                    src
                }));
                let lyric = lw[i..i + best_k].iter().map(|w| w.text.as_str()).collect::<Vec<_>>().join(" ");
                let pron = lw[i..i + best_k].iter().map(|w| w.pron.as_str()).collect();
                clips.push(Clip { lyric: clean(&lyric), pron, kind: ClipKind::Phrase, line: line_no, word: i, choice: 0, candidates });
                i += best_k;
                continue;
            }

            // 2) 단어
            if let Some(idxs) = by_pron.get(lw[i].pron.as_str()) {
                let candidates = rank(idxs.iter().map(|&j| {
                    let w = &words[j];
                    let mut src = source(w, w.end, (0.0, 1.0), w.text.clone(), w.prob, units[j].len(), 0.0);
                    if w.syls.len() == units[j].len() {
                        set_aligned(&mut src, w.prob, 0.0, w.syls.clone());
                        src.check = w.check; // 단어 전체를 미리 들어 본 점수
                    }
                    src
                }));
                clips.push(Clip { lyric: clean(&lw[i].text), pron: lw[i].pron.clone(), kind: ClipKind::Word, line: line_no, word: i, choice: 0, candidates });
                i += 1;
                continue;
            }

            // 3) 음절 조각으로 이어 붙이기
            clips.extend(syllable_cover(&lw[i], line_no, i, words, &units));
            i += 1;
        }
    }
    clips
}

/// 단어가 통째로 없을 때: 가장 긴 음절 덩어리부터 다른 단어 속에서 찾아 덮는다.
/// 예) "사랑해" 없음 → "사랑"(사랑하는 앞부분) + "해"(해서 앞부분)
fn syllable_cover(lw: &LyricWord, line: usize, word: usize, words: &[IndexedWord], units: &[Vec<String>]) -> Vec<Clip> {
    let target = g2p::units(&lw.pron);
    // 화면에는 원래 표기를 보여준다 (한국어 발음형은 음절 수가 표기와 같음)
    let shown: Vec<String> = {
        let orig = g2p::units(&crate::index::normalize(&lw.text));
        if orig.len() == target.len() { orig } else { target.clone() }
    };
    let m = target.len();
    let mut out = Vec::new();
    let mut p = 0;
    while p < m {
        let mut found = None;
        for len in (1..=m - p).rev() {
            let cands = find_units(&target[p..p + len], p, m, words, units, true);
            if !cands.is_empty() {
                found = Some((len, rank(cands.into_iter())));
                break;
            }
        }
        // 3-2) 한 음절도 없으면 비슷한 소리로 대체 (빤→반, 랑→라)
        if found.is_none() {
            for alt in g2p::ko::similar(&target[p]) {
                let cands = find_units(&[alt], p, m, words, units, false);
                if !cands.is_empty() {
                    found = Some((1, rank(cands.into_iter())));
                    break;
                }
            }
        }
        match found {
            Some((len, candidates)) => {
                out.push(Clip { lyric: shown[p..p + len].concat(), pron: target[p..p + len].concat(), kind: ClipKind::Syllable, line, word, choice: 0, candidates });
                p += len;
            }
            None => {
                out.push(Clip { lyric: shown[p].clone(), pron: target[p].clone(), kind: ClipKind::Missing, line, word, choice: 0, candidates: Vec::new() });
                p += 1;
            }
        }
    }
    out
}

/// `needle` 음절열이 들어 있는 모든 색인 단어 위치 (p: 가사 단어 안 위치, m: 가사 단어 음절 수)
/// `exact`: 찾는 소리가 가사 소리 그대로인지 (비슷한 소리 대체면 false → 미리 계산한 점수를 못 씀)
fn find_units(needle: &[String], p: usize, m: usize, words: &[IndexedWord], units: &[Vec<String>], exact: bool) -> Vec<Source> {
    let len = needle.len();
    let mut cands = Vec::new();
    for (j, u) in units.iter().enumerate() {
        if u.len() < len {
            continue;
        }
        for q in 0..=u.len() - len {
            if u[q..q + len] != *needle {
                continue;
            }
            // 단어 경계와 맞을수록 자른 티가 덜 나고 정확함 (단어 안 위치: 처음/끝이 가사와 같으면 가산)
            let pos = POSITION_BONUS_X100.load(std::sync::atomic::Ordering::Relaxed) as f32 / 100.0;
            let mut bonus = 0.0;
            if u.len() == len {
                bonus += 0.2;
            }
            if q == 0 && p == 0 {
                bonus += pos;
            }
            if q + len == u.len() && p + len == m {
                bonus += pos;
            }
            // 가운데끼리도 위치가 맞으면 (앞뒤로 이어지는 소리가 있었던 조각)
            if q > 0 && p > 0 && q + len < u.len() && p + len < m {
                bonus += pos / 2.0;
            }
            let w = &words[j];
            if w.syls.len() == u.len() {
                // 정렬된 음절 경계로 정확히 자르기
                let mut src = source(w, w.end, (0.0, 1.0), w.text.clone(), w.prob, u.len(), bonus);
                set_aligned(&mut src, w.prob, bonus, w.syls[q..q + len].to_vec());
                if exact && len == 1 {
                    src.check = w.syls[q].check; // 음절 하나를 미리 들어 본 점수
                } else if exact && len == u.len() {
                    src.check = w.check;
                }
                cands.push(src);
            } else {
                let n = u.len() as f64;
                cands.push(source(w, w.end, (q as f64 / n, (q + len) as f64 / n), w.text.clone(), w.prob, u.len(), bonus));
            }
        }
    }
    cands
}

fn source(w: &IndexedWord, end: f64, frac: (f64, f64), said: String, prob: f32, n_units: usize, bonus: f32) -> Source {
    let per_unit = (end - w.start) / n_units.max(1) as f64;
    // 음절당 길이가 너무 짧으면(인식 오류) 또는 너무 길면(뒤 쉼 포함) 감점
    let penalty = if per_unit < 0.06 {
        (0.06 - per_unit) * 5.0
    } else if per_unit > 0.45 {
        (per_unit - 0.45).min(1.0)
    } else {
        0.0
    };
    Source {
        asset_id: w.asset_id.clone(),
        start: w.start,
        end,
        frac,
        said,
        score: prob + bonus - penalty as f32,
        aligned: false,
        syls: Vec::new(),
        check: None,
    }
}

/// 정렬된 음절 구간으로 바꾸고 점수를 실제 음절 길이로 다시 매긴다.
/// 너무 짧은 음절(<0.1초)은 알아듣기 어려워 감점, 모음이 길수록 가산(노래로 늘이기에도 유리)
fn set_aligned(src: &mut Source, prob: f32, bonus: f32, syls: Vec<SylSpan>) {
    let n = syls.len().max(1) as f64;
    let dur = syls.iter().map(|s| s.end - s.start).sum::<f64>() / n;
    let vowel = syls.iter().map(|s| s.v_end - s.v_start).sum::<f64>() / n;
    let penalty = if dur < 0.1 {
        (0.1 - dur) * 6.0
    } else if dur > 0.5 {
        (dur - 0.5).min(1.0)
    } else {
        0.0
    };
    src.start = syls.first().map_or(src.start, |s| s.start);
    src.end = syls.last().map_or(src.end, |s| s.end);
    src.frac = (0.0, 1.0);
    src.score = prob + bonus + vowel.min(0.15) as f32 - penalty as f32;
    src.aligned = true;
    src.syls = syls;
}

fn rank(it: impl Iterator<Item = Source>) -> Vec<Source> {
    let mut v: Vec<Source> = it
        .map(|mut s| {
            if s.aligned {
                s.score += 0.3; // 음절 경계가 정확한 후보 우선
            }
            s
        })
        .collect();
    v.sort_by(|a, b| b.score.total_cmp(&a.score));
    v.truncate(MAX_CANDIDATES);
    v
}

fn clean(s: &str) -> String {
    s.trim_matches(|c: char| !c.is_alphanumeric()).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn idx(words: &[(&str, i64, f64)]) -> Vec<IndexedWord> {
        words
            .iter()
            .map(|&(t, seg, s)| IndexedWord {
                id: 0,
                syls: Vec::new(),
                check: None,
                asset_id: "a0001".into(),
                segment_id: seg,
                text: t.into(),
                pron: g2p::pron(t),
                start: s,
                end: s + 0.2 * t.chars().count() as f64,
                prob: 0.9,
            })
            .collect()
    }

    #[test]
    fn prefers_phrase_then_word_then_syllables() {
        let words = idx(&[
            ("우리", 1, 0.0),
            ("모두", 1, 0.5),
            ("사랑하는", 2, 3.0),
            ("해서", 3, 6.0),
            ("모두", 4, 9.0),
        ]);
        let clips = compose("우리 모두 사랑해!\n모두 쀍", &words);
        let kinds: Vec<_> = clips.iter().map(|c| (c.lyric.as_str(), c.kind)).collect();
        assert_eq!(
            kinds,
            [
                ("우리 모두", ClipKind::Phrase),
                ("사랑", ClipKind::Syllable),
                ("해", ClipKind::Syllable),
                ("모두", ClipKind::Word),
                ("쀍", ClipKind::Missing),
            ]
        );
        // "사랑" = 사랑하는의 앞 절반
        let s = clips[1].source().unwrap();
        assert_eq!(s.said, "사랑하는");
        assert_eq!(s.frac, (0.0, 0.5));
        // "해" = 해서의 앞 절반 (단어 시작과 맞음)
        assert_eq!(clips[2].source().unwrap().frac, (0.0, 0.5));
        assert_eq!(clips[3].line, 1);
    }

    #[test]
    fn matches_by_pronunciation_not_spelling() {
        // 영상에서 "가치"라고 인식됐어도 가사 "같이"와 맞아야 함
        let words = idx(&[("가치", 1, 0.0)]);
        let clips = compose("같이", &words);
        assert_eq!(clips[0].kind, ClipKind::Word);
    }
}
