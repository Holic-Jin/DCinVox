//! 영어 표기 → 음절 덩어리 + 정렬용 음소 (규칙 기반, CMUdict 없이).

use super::ko::Phone;

fn is_vowel_letter(c: char) -> bool {
    matches!(c, 'a' | 'e' | 'i' | 'o' | 'u' | 'y')
}

/// 단어 하나를 음절 덩어리로. "hello" → hel / lo
pub fn syllables(word: &str) -> Vec<String> {
    let w: Vec<char> = word.chars().filter(|c| c.is_ascii_alphabetic()).map(|c| c.to_ascii_lowercase()).collect();
    if w.is_empty() {
        return Vec::new();
    }
    let mut nuclei = Vec::new();
    let mut i = 0;
    while i < w.len() {
        if is_vowel_letter(w[i]) {
            let start = i;
            i += 1;
            while i < w.len() && is_vowel_letter(w[i]) {
                i += 1;
            }
            nuclei.push(start..i);
        } else {
            i += 1;
        }
    }
    if nuclei.is_empty() {
        return vec![w.into_iter().collect()];
    }
    let mut bounds = vec![0usize];
    for pair in nuclei.windows(2) {
        let a_end = pair[0].end;
        let b_start = pair[1].start;
        let cons = b_start - a_end;
        let split = if cons <= 1 { b_start } else { a_end + cons - 1 };
        bounds.push(split);
    }
    bounds.push(w.len());
    bounds.windows(2).map(|b| w[b[0]..b[1]].iter().collect()).filter(|s: &String| !s.is_empty()).collect()
}

pub fn phones(syl: &str) -> Option<(Vec<Phone>, Vec<Phone>, Vec<Phone>)> {
    let s: String = syl.chars().filter(|c| c.is_ascii_alphabetic()).map(|c| c.to_ascii_lowercase()).collect();
    if s.is_empty() {
        return None;
    }
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    let mut onset = Vec::new();
    while i < chars.len() && !is_vowel_letter(chars[i]) {
        let (p, n) = cons(&chars[i..])?;
        onset.push(p);
        i += n;
    }
    if i >= chars.len() {
        return Some((onset, vec![&["ə", "ʌ"]], vec![]));
    }
    let (vowel, n) = vowel(&chars[i..]);
    i += n;
    let mut coda = Vec::new();
    while i < chars.len() {
        let (p, n) = cons(&chars[i..])?;
        coda.push(p);
        i += n;
    }
    Some((onset, vowel, coda))
}

fn cons(rest: &[char]) -> Option<(Phone, usize)> {
    let two: String = rest.iter().take(2).collect();
    let one = rest[0];
    match two.as_str() {
        "ch" => Some((&["tʃ", "tɕ", "ts"], 2)),
        "sh" => Some((&["ʃ", "ɕ", "s"], 2)),
        "th" => Some((&["θ", "ð", "t"], 2)),
        "ng" => Some((&["ŋ", "n"], 2)),
        "ph" => Some((&["f"], 2)),
        "ck" => Some((&["k", "ɡ"], 2)),
        "wh" => Some((&["w", "h"], 2)),
        "qu" => Some((&["k"], 2)),
        "kn" => Some((&["n"], 2)),
        "wr" => Some((&["ɹ", "r"], 2)),
        _ => match one {
            'b' => Some((&["b", "p"], 1)),
            'c' => Some((&["k", "s", "tʃ"], 1)),
            'd' => Some((&["d", "t"], 1)),
            'f' => Some((&["f"], 1)),
            'g' => Some((&["ɡ", "dʒ", "k"], 1)),
            'h' => Some((&["h"], 1)),
            'j' => Some((&["dʒ", "j", "ʒ"], 1)),
            'k' => Some((&["k", "ɡ"], 1)),
            'l' => Some((&["l", "ɫ", "ɾ"], 1)),
            'm' => Some((&["m"], 1)),
            'n' => Some((&["n", "ŋ"], 1)),
            'p' => Some((&["p", "b"], 1)),
            'q' => Some((&["k"], 1)),
            'r' => Some((&["ɹ", "r", "ɚ"], 1)),
            's' => Some((&["s", "z", "ʃ"], 1)),
            't' => Some((&["t", "d", "ɾ"], 1)),
            'v' => Some((&["v", "f"], 1)),
            'w' => Some((&["w"], 1)),
            'x' => Some((&["k"], 1)),
            'z' => Some((&["z", "s"], 1)),
            _ => None,
        },
    }
}

fn vowel(rest: &[char]) -> (Vec<Phone>, usize) {
    let three: String = rest.iter().take(3).collect();
    let two: String = rest.iter().take(2).collect();
    if three == "igh" {
        return (vec![&["aɪ", "a", "i"]], 3);
    }
    match two.as_str() {
        "ee" | "ea" | "ey" | "ie" => (vec![&["i", "iː", "ɪ"]], 2),
        "oo" => (vec![&["u", "uː", "ʊ"]], 2),
        "oa" | "ow" => (vec![&["oʊ", "o", "aʊ"]], 2),
        "ou" => (vec![&["aʊ", "u", "ʌ"]], 2),
        "ai" | "ay" => (vec![&["eɪ", "e", "a"]], 2),
        "oi" | "oy" => (vec![&["ɔɪ", "oɪ"]], 2),
        "au" | "aw" => (vec![&["ɔ", "ɔː", "ɑ"]], 2),
        "ew" => (vec![&["u", "ju"]], 2),
        "ar" => (vec![&["ɑː", "ɑ", "ɚ"]], 2),
        "or" => (vec![&["ɔː", "ɔ", "ɚ"]], 2),
        "er" | "ir" | "ur" => (vec![&["ɚ", "ɜ", "ə"]], 2),
        _ => match rest[0] {
            'a' => (vec![&["æ", "eɪ", "ɑ", "ə", "a"]], 1),
            'e' => (vec![&["ɛ", "i", "ə", "e"]], 1),
            'i' | 'y' => (vec![&["ɪ", "aɪ", "i", "ə"]], 1),
            'o' => (vec![&["ɒ", "oʊ", "ɔ", "ə", "ʌ"]], 1),
            'u' => (vec![&["ʌ", "u", "ʊ", "ə", "ju"]], 1),
            _ => (vec![&["ə"]], 1),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_english_syllables() {
        assert_eq!(syllables("hello"), ["hel", "lo"]);
        assert_eq!(syllables("cat"), ["cat"]);
        assert_eq!(syllables("singing"), ["sing", "ing"]);
        assert!(phones("hel").unwrap().1.iter().any(|p| p.contains(&"ɛ") || p.contains(&"e") || p.contains(&"ə") || p.contains(&"ɪ") || p.contains(&"æ")));
        assert!(phones("lo").is_some());
    }
}
