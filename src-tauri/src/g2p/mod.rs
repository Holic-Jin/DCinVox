//! 표기 → 발음형, 발음형 → 음절 단위.
//! 한국어는 표준발음 규칙(ko.rs). 일본어(가나 읽기)·영어(CMUdict)는 마일스톤 4에서 추가 — 그전까지는 정규화 문자열 그대로.

pub mod ko;

use crate::index::normalize;

/// 매칭용 발음형. 언어와 무관하게 한글 어절에는 한국어 규칙을 적용한다.
pub fn pron(text: &str) -> String {
    ko::pronounce(&normalize(text))
}

/// 발음형을 잘라 붙일 수 있는 최소 단위로 나눈다.
/// 한글 = 음절 1글자, 가나·한자 = 1글자, 로마자·숫자 = 연속된 덩어리 하나(영어 음절 분리는 마일스톤 4).
pub fn units(pron: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut run = String::new();
    for c in pron.chars() {
        if c.is_ascii_alphanumeric() || (c.is_alphabetic() && (c as u32) < 0x0250) {
            run.push(c);
            continue;
        }
        if !run.is_empty() {
            out.push(std::mem::take(&mut run));
        }
        out.push(c.to_string());
    }
    if !run.is_empty() {
        out.push(run);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_units() {
        assert_eq!(units("사랑해"), ["사", "랑", "해"]);
        assert_eq!(units("loveこころ"), ["love", "こ", "こ", "ろ"]);
        assert_eq!(pron("같이!"), "가치");
    }
}
