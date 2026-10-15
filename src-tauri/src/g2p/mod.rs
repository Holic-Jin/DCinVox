//! 표기 → 발음형, 발음형 → 음절 단위, 음절 → 정렬 음소.
//! 한국어 표준발음(ko), 일본어 가나(ja), 영어 음절 규칙(en).

pub mod en;
pub mod ja;
pub mod ko;

use crate::index::normalize;

pub use ko::Phone;

/// 매칭용 발음형. 한글은 표준발음, 가나는 히라가나, 그 외는 정규화 문자열.
pub fn pron(text: &str) -> String {
    let n = normalize(text);
    if n.chars().any(ko::is_hangul) {
        return ko::pronounce(&n);
    }
    if n.chars().any(ja::is_kana) {
        return ja::to_hiragana(&n);
    }
    n
}

/// 잘라 붙일 최소 단위: 한글 1글자, 일본어 모라, 영어 음절, 숫자 덩어리.
pub fn units(pron: &str) -> Vec<String> {
    let chars: Vec<char> = pron.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if ko::is_hangul(c) {
            out.push(c.to_string());
            i += 1;
        } else if ja::is_kana(c) {
            let start = i;
            while i < chars.len() && ja::is_kana(chars[i]) {
                i += 1;
            }
            out.extend(ja::morae(&chars[start..i].iter().collect::<String>()));
        } else if c.is_ascii_alphabetic() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_alphabetic() {
                i += 1;
            }
            out.extend(en::syllables(&chars[start..i].iter().collect::<String>()));
        } else if c.is_ascii_digit() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            out.push(chars[start..i].iter().collect());
        } else {
            i += 1;
        }
    }
    out
}

/// 단위 하나 → (초성, 중성, 종성) 음소 후보. 못 바꾸면 None (한자만 있는 일본어 등).
pub fn phones_unit(unit: &str) -> Option<(Vec<Phone>, Vec<Phone>, Vec<Phone>)> {
    let mut it = unit.chars();
    let Some(c) = it.next() else { return None };
    if ko::is_hangul(c) && it.next().is_none() {
        return ko::phones(c);
    }
    if unit.chars().all(ja::is_kana) {
        return ja::phones(unit);
    }
    if unit.chars().all(|ch| ch.is_ascii_alphabetic()) {
        return en::phones(unit);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_units() {
        assert_eq!(units("사랑해"), ["사", "랑", "해"]);
        assert_eq!(units("loveこころ"), ["lov", "e", "こ", "こ", "ろ"]);
        assert_eq!(units("hello"), ["hel", "lo"]);
        assert_eq!(units("きゃく"), ["きゃ", "く"]);
        assert_eq!(pron("같이!"), "가치");
        assert_eq!(pron("コンニチハ"), "こんにちは");
        assert_eq!(pron("Hello"), "hello");
        assert!(phones_unit("사").is_some());
        assert!(phones_unit("こ").is_some());
        assert!(phones_unit("hel").is_some());
        assert!(phones_unit("漢").is_none());
    }
}
