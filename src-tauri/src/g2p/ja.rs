//! 가나 → 모라 단위 + 정렬 모델(espeak) 음소.
//! 한자만으로 된 어절은 사전 없이 읽지 못하므로 정렬에서 건너뛴다.

use super::ko::Phone;

pub fn is_kana(c: char) -> bool {
    matches!(c, '\u{3040}'..='\u{309F}' | '\u{30A0}'..='\u{30FF}' | 'ー' | '･')
}

pub fn to_hiragana(s: &str) -> String {
    s.chars()
        .map(|c| {
            if ('\u{30A1}'..='\u{30F6}').contains(&c) {
                char::from_u32(c as u32 - 0x60).unwrap_or(c)
            } else if c == 'ー' {
                'ー'
            } else {
                c
            }
        })
        .collect()
}

/// 발음형을 모라로 나눈다. きゃ / しゃ 등은 한 덩어리.
pub fn morae(pron: &str) -> Vec<String> {
    let s: Vec<char> = to_hiragana(pron).chars().filter(|&c| is_kana(c) || c == 'ー').collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < s.len() {
        if i + 1 < s.len() && is_youon(s[i], s[i + 1]) {
            out.push(format!("{}{}", s[i], s[i + 1]));
            i += 2;
        } else {
            out.push(s[i].to_string());
            i += 1;
        }
    }
    out
}

fn is_youon(a: char, b: char) -> bool {
    matches!(b, 'ゃ' | 'ゅ' | 'ょ' | 'ぁ' | 'ぃ' | 'ぅ' | 'ぇ' | 'ぉ')
        && !matches!(a, 'っ' | 'ん' | 'ー')
}

pub fn phones(mora: &str) -> Option<(Vec<Phone>, Vec<Phone>, Vec<Phone>)> {
    const K: Phone = &["k", "ɡ", "kh", "kʰ"];
    const G: Phone = &["ɡ", "k", "ɣ"];
    const S: Phone = &["s", "ɕ", "ʃ"];
    const Z: Phone = &["z", "dz", "ʑ", "dʑ"];
    const T: Phone = &["t", "d", "ts"];
    const D: Phone = &["d", "t"];
    const N: Phone = &["n", "nʲ", "ɲ"];
    const H: Phone = &["h", "ç", "ɸ"];
    const B: Phone = &["b", "β", "p"];
    const P: Phone = &["p", "b", "ph"];
    const M: Phone = &["m", "mʲ"];
    const R: Phone = &["ɾ", "r", "l", "ɹ"];
    const W: Phone = &["w", "ʋ", "u"];
    const Y: Phone = &["j"];
    const A: Phone = &["a", "ɐ", "ä", "ɑ"];
    const I: Phone = &["i", "iː", "ɪ"];
    const U: Phone = &["ɯ", "u", "ʊ", "ɨ"];
    const E: Phone = &["e", "eː", "ɛ", "e̞"];
    const O: Phone = &["o", "oː", "ɔ", "o̞"];
    let m = to_hiragana(mora);
    let yv = |y: char| -> Option<(Vec<Phone>, Vec<Phone>)> {
        let v = match y {
            'ゃ' => A,
            'ゅ' => U,
            'ょ' => O,
            'ぁ' => A,
            'ぃ' => I,
            'ぅ' => U,
            'ぇ' => E,
            'ぉ' => O,
            _ => return None,
        };
        Some((vec![Y], vec![v]))
    };
    match m.as_str() {
        "っ" => Some((vec![&["t", "k", "p", "s", "tɕ"]], vec![], vec![])),
        "ん" => Some((vec![], vec![&["n", "m", "ŋ", "ɴ"]], vec![])),
        "ー" => Some((vec![], vec![&["iː", "uː", "eː", "oː", "aː", "ɯ", "a"]], vec![])),
        "あ" => Some((vec![], vec![A], vec![])),
        "い" => Some((vec![], vec![I], vec![])),
        "う" => Some((vec![], vec![U], vec![])),
        "え" => Some((vec![], vec![E], vec![])),
        "お" => Some((vec![], vec![O], vec![])),
        "か" => Some((vec![K], vec![A], vec![])),
        "き" => Some((vec![K], vec![I], vec![])),
        "く" => Some((vec![K], vec![U], vec![])),
        "け" => Some((vec![K], vec![E], vec![])),
        "こ" => Some((vec![K], vec![O], vec![])),
        "が" => Some((vec![G], vec![A], vec![])),
        "ぎ" => Some((vec![G], vec![I], vec![])),
        "ぐ" => Some((vec![G], vec![U], vec![])),
        "げ" => Some((vec![G], vec![E], vec![])),
        "ご" => Some((vec![G], vec![O], vec![])),
        "さ" => Some((vec![S], vec![A], vec![])),
        "す" => Some((vec![S], vec![U], vec![])),
        "せ" => Some((vec![S], vec![E], vec![])),
        "そ" => Some((vec![S], vec![O], vec![])),
        "し" => Some((vec![&["ɕ", "ʃ", "s"]], vec![I], vec![])),
        "ざ" => Some((vec![Z], vec![A], vec![])),
        "ず" | "づ" => Some((vec![Z], vec![U], vec![])),
        "ぜ" => Some((vec![Z], vec![E], vec![])),
        "ぞ" => Some((vec![Z], vec![O], vec![])),
        "じ" | "ぢ" => Some((vec![&["dʑ", "ʑ", "dʒ", "ʒ"]], vec![I], vec![])),
        "た" => Some((vec![T], vec![A], vec![])),
        "て" => Some((vec![T], vec![E], vec![])),
        "と" => Some((vec![T], vec![O], vec![])),
        "ち" => Some((vec![&["tɕ", "tʃ", "ts"]], vec![I], vec![])),
        "つ" => Some((vec![&["ts", "t"]], vec![U], vec![])),
        "だ" => Some((vec![D], vec![A], vec![])),
        "で" => Some((vec![D], vec![E], vec![])),
        "ど" => Some((vec![D], vec![O], vec![])),
        "な" => Some((vec![N], vec![A], vec![])),
        "に" => Some((vec![N], vec![I], vec![])),
        "ぬ" => Some((vec![N], vec![U], vec![])),
        "ね" => Some((vec![N], vec![E], vec![])),
        "の" => Some((vec![N], vec![O], vec![])),
        "は" => Some((vec![H], vec![A], vec![])),
        "ひ" => Some((vec![&["ç", "h"]], vec![I], vec![])),
        "ふ" => Some((vec![&["ɸ", "f", "h"]], vec![U], vec![])),
        "へ" => Some((vec![H], vec![E], vec![])),
        "ほ" => Some((vec![H], vec![O], vec![])),
        "ば" => Some((vec![B], vec![A], vec![])),
        "び" => Some((vec![B], vec![I], vec![])),
        "ぶ" => Some((vec![B], vec![U], vec![])),
        "べ" => Some((vec![B], vec![E], vec![])),
        "ぼ" => Some((vec![B], vec![O], vec![])),
        "ぱ" => Some((vec![P], vec![A], vec![])),
        "ぴ" => Some((vec![P], vec![I], vec![])),
        "ぷ" => Some((vec![P], vec![U], vec![])),
        "ぺ" => Some((vec![P], vec![E], vec![])),
        "ぽ" => Some((vec![P], vec![O], vec![])),
        "ま" => Some((vec![M], vec![A], vec![])),
        "み" => Some((vec![M], vec![I], vec![])),
        "む" => Some((vec![M], vec![U], vec![])),
        "め" => Some((vec![M], vec![E], vec![])),
        "も" => Some((vec![M], vec![O], vec![])),
        "や" => Some((vec![Y], vec![A], vec![])),
        "ゆ" => Some((vec![Y], vec![U], vec![])),
        "よ" => Some((vec![Y], vec![O], vec![])),
        "ら" => Some((vec![R], vec![A], vec![])),
        "り" => Some((vec![R], vec![I], vec![])),
        "る" => Some((vec![R], vec![U], vec![])),
        "れ" => Some((vec![R], vec![E], vec![])),
        "ろ" => Some((vec![R], vec![O], vec![])),
        "わ" => Some((vec![W], vec![A], vec![])),
        "を" => Some((vec![], vec![O], vec![])),
        "ゔ" => Some((vec![&["v", "b"]], vec![U], vec![])),
        _ => {
            let ch: Vec<char> = m.chars().collect();
            if ch.len() != 2 {
                return None;
            }
            let on: Phone = match ch[0] {
                'き' => K,
                'ぎ' => G,
                'し' => &["ɕ", "ʃ", "s"],
                'じ' => &["dʑ", "ʑ", "dʒ"],
                'ち' => &["tɕ", "tʃ"],
                'に' => N,
                'ひ' => &["ç", "h"],
                'び' => B,
                'ぴ' => P,
                'み' => M,
                'り' => R,
                'ふ' => &["ɸ", "f"],
                _ => return None,
            };
            let Some((glide, v)) = yv(ch[1]) else { return None };
            let mut vowel = if matches!(ch[1], 'ゃ' | 'ゅ' | 'ょ') { glide } else { vec![] };
            vowel.extend(v);
            Some((vec![on], vowel, vec![]))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_morae_and_maps_phones() {
        assert_eq!(morae("こんにちは"), ["こ", "ん", "に", "ち", "は"]);
        assert_eq!(morae("きゃく"), ["きゃ", "く"]);
        assert_eq!(to_hiragana("コンニチハ"), "こんにちは");
        assert!(phones("か").is_some());
        assert!(phones("きゃ").is_some());
        assert!(phones("漢").is_none());
    }
}
