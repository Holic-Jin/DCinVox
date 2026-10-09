//! 한국어 표기 → 발음 (표준발음법의 주요 규칙).
//! 잘라 붙일 때 철자가 아니라 실제 소리로 맞춰야 하므로 색인과 가사 모두 이 형태로 비교한다.
//! 예) 같이→가치, 국물→궁물, 음악→으막, 좋아→조아, 학교→학꾜
//!
//! 한 어절(띄어쓰기 단위) 안에서만 적용한다. 잘라 쓰는 단위가 어절이기 때문.

const CHO: [char; 19] = [
    'ㄱ', 'ㄲ', 'ㄴ', 'ㄷ', 'ㄸ', 'ㄹ', 'ㅁ', 'ㅂ', 'ㅃ', 'ㅅ', 'ㅆ', 'ㅇ', 'ㅈ', 'ㅉ', 'ㅊ', 'ㅋ', 'ㅌ', 'ㅍ', 'ㅎ',
];
const JONG: [char; 28] = [
    '\0', 'ㄱ', 'ㄲ', 'ㄳ', 'ㄴ', 'ㄵ', 'ㄶ', 'ㄷ', 'ㄹ', 'ㄺ', 'ㄻ', 'ㄼ', 'ㄽ', 'ㄾ', 'ㄿ', 'ㅀ', 'ㅁ', 'ㅂ', 'ㅄ',
    'ㅅ', 'ㅆ', 'ㅇ', 'ㅈ', 'ㅊ', 'ㅋ', 'ㅌ', 'ㅍ', 'ㅎ',
];

// 중성 인덱스
const V_YE: u32 = 7; // ㅖ
const V_E: u32 = 5; // ㅔ
const V_YEO: u32 = 6; // ㅕ
const V_EO: u32 = 4; // ㅓ
const V_UI: u32 = 19; // ㅢ
const V_I: u32 = 20; // ㅣ

#[derive(Clone, Copy, Debug)]
struct Syl {
    cho: char,
    jung: u32,
    /// '\0' = 받침 없음
    jong: char,
}

fn decompose(c: char) -> Option<Syl> {
    let code = c as u32;
    if !(0xAC00..=0xD7A3).contains(&code) {
        return None;
    }
    let idx = code - 0xAC00;
    Some(Syl {
        cho: CHO[(idx / (21 * 28)) as usize],
        jung: (idx / 28) % 21,
        jong: JONG[(idx % 28) as usize],
    })
}

fn compose(s: Syl) -> char {
    let cho = CHO.iter().position(|&c| c == s.cho).unwrap_or(11) as u32;
    let jong = JONG.iter().position(|&c| c == s.jong).unwrap_or(0) as u32;
    char::from_u32(0xAC00 + (cho * 21 + s.jung) * 28 + jong).unwrap_or('?')
}

/// 겹받침 → (앞, 뒤). 홑받침은 ('\0', 받침)
fn split(j: char) -> (char, char) {
    match j {
        'ㄳ' => ('ㄱ', 'ㅅ'),
        'ㄵ' => ('ㄴ', 'ㅈ'),
        'ㄶ' => ('ㄴ', 'ㅎ'),
        'ㄺ' => ('ㄹ', 'ㄱ'),
        'ㄻ' => ('ㄹ', 'ㅁ'),
        'ㄼ' => ('ㄹ', 'ㅂ'),
        'ㄽ' => ('ㄹ', 'ㅅ'),
        'ㄾ' => ('ㄹ', 'ㅌ'),
        'ㄿ' => ('ㄹ', 'ㅍ'),
        'ㅀ' => ('ㄹ', 'ㅎ'),
        'ㅄ' => ('ㅂ', 'ㅅ'),
        _ => ('\0', j),
    }
}

/// 받침 대표음 (7종성)
fn neutralize(j: char, next_cho: Option<char>) -> char {
    match j {
        'ㄲ' | 'ㅋ' | 'ㄳ' => 'ㄱ',
        // 맑게[말께]: ㄺ은 ㄱ 앞에서 ㄹ
        'ㄺ' => if next_cho == Some('ㄱ') { 'ㄹ' } else { 'ㄱ' },
        'ㅅ' | 'ㅆ' | 'ㅈ' | 'ㅊ' | 'ㅌ' | 'ㅎ' => 'ㄷ',
        'ㅍ' | 'ㅄ' | 'ㄿ' => 'ㅂ',
        'ㄵ' | 'ㄶ' => 'ㄴ',
        'ㄼ' | 'ㄽ' | 'ㄾ' | 'ㅀ' => 'ㄹ',
        'ㄻ' => 'ㅁ',
        other => other,
    }
}

fn class(j: char) -> Option<char> {
    match j {
        'ㄱ' | 'ㄲ' | 'ㅋ' => Some('ㄱ'),
        'ㄷ' | 'ㅅ' | 'ㅆ' | 'ㅈ' | 'ㅊ' | 'ㅌ' => Some('ㄷ'),
        'ㅂ' | 'ㅍ' => Some('ㅂ'),
        _ => None,
    }
}

fn tense(c: char) -> char {
    match c {
        'ㄱ' => 'ㄲ',
        'ㄷ' => 'ㄸ',
        'ㅂ' => 'ㅃ',
        'ㅅ' => 'ㅆ',
        'ㅈ' => 'ㅉ',
        other => other,
    }
}

fn nasal(j: char) -> char {
    match j {
        'ㄱ' => 'ㅇ',
        'ㄷ' => 'ㄴ',
        'ㅂ' => 'ㅁ',
        other => other,
    }
}

const PLAIN: [char; 5] = ['ㄱ', 'ㄷ', 'ㅂ', 'ㅅ', 'ㅈ'];

/// 앞 음절 받침 `a`와 뒤 음절 `b` 사이의 음운 변동 적용
fn apply_pair(a: &mut Syl, b: &mut Syl) {
    if a.jong == '\0' {
        return;
    }
    let c = b.cho;
    let (first, last) = split(a.jong);

    // 1) ㅎ 받침: 좋다[조타], 좋소[조쏘], 좋아[조아], 놓는[논는]
    if last == 'ㅎ' {
        match c {
            'ㄱ' | 'ㄷ' | 'ㅈ' => {
                b.cho = match c { 'ㄱ' => 'ㅋ', 'ㄷ' => 'ㅌ', _ => 'ㅊ' };
                a.jong = first;
                return;
            }
            'ㅅ' => {
                b.cho = 'ㅆ';
                a.jong = first;
                return;
            }
            'ㄴ' => a.jong = if first == '\0' { 'ㄴ' } else { first },
            _ => a.jong = first,
        }
        if a.jong == '\0' {
            return;
        }
    }

    // 2) 뒤 ㅎ: 축하[추카], 입학[이팍], 못해[모태], 닫히다[다치다]
    if c == 'ㅎ' {
        let (first, last) = split(a.jong);
        if let Some(k) = class(last) {
            b.cho = match k {
                'ㄱ' => 'ㅋ',
                'ㅂ' => 'ㅍ',
                _ if last == 'ㅈ' || last == 'ㅊ' => 'ㅊ',
                _ if b.jung == V_I && (last == 'ㄷ' || last == 'ㅌ') => 'ㅊ',
                _ => 'ㅌ',
            };
            a.jong = first;
        }
        return;
    }

    // 3) 연음: 음악[으막], 있어[이써], 닭이[달기], 값이[갑씨], 구개음화 같이[가치]
    if c == 'ㅇ' && a.jong != 'ㅇ' {
        let (first, last) = split(a.jong);
        let mut moved = last;
        if b.jung == V_I {
            moved = match last { 'ㄷ' => 'ㅈ', 'ㅌ' => 'ㅊ', x => x };
        }
        if last == 'ㅅ' && matches!(first, 'ㄱ' | 'ㅂ') {
            moved = 'ㅆ';
        }
        b.cho = moved;
        a.jong = first;
        return;
    }

    // 4) 대표음 + 비음화/유음화/경음화
    let original = a.jong;
    let j = neutralize(original, Some(c));
    a.jong = j;
    match (j, c) {
        ('ㄱ' | 'ㄷ' | 'ㅂ', 'ㄴ' | 'ㅁ') => a.jong = nasal(j), // 국물[궁물]
        ('ㄱ' | 'ㄷ' | 'ㅂ', 'ㄹ') => {
            a.jong = nasal(j); // 독립[동닙]
            b.cho = 'ㄴ';
        }
        ('ㅁ' | 'ㅇ', 'ㄹ') => b.cho = 'ㄴ', // 심리[심니]
        ('ㄴ', 'ㄹ') => a.jong = 'ㄹ',         // 신라[실라]
        ('ㄹ', 'ㄴ') => b.cho = 'ㄹ',          // 설날[설랄]
        ('ㄱ' | 'ㄷ' | 'ㅂ', _) if PLAIN.contains(&c) => b.cho = tense(c), // 학교[학꾜]
        // 넓게[널께], 맑게[말께], 핥다[할따]
        ('ㄹ', _) if matches!(original, 'ㄼ' | 'ㄾ' | 'ㄺ') && PLAIN.contains(&c) => b.cho = tense(c),
        _ => {}
    }
}

fn apply_vowels(s: &mut Syl) {
    if s.jung == V_UI && s.cho != 'ㅇ' {
        s.jung = V_I; // 희망[히망]
    }
    if s.jung == V_YE && !matches!(s.cho, 'ㅇ' | 'ㄹ') {
        s.jung = V_E; // 계속[게속]
    }
    if s.jung == V_YEO && matches!(s.cho, 'ㅈ' | 'ㅉ' | 'ㅊ') {
        s.jung = V_EO; // 가져[가저]
    }
}

/// 한 어절의 발음형. 한글이 아닌 글자는 그대로 둔다.
pub fn pronounce(word: &str) -> String {
    let mut items: Vec<Result<Syl, char>> = word.chars().map(|c| decompose(c).ok_or(c)).collect();
    for i in 0..items.len() {
        let (left, right) = items.split_at_mut(i + 1);
        match (left.last_mut(), right.first_mut()) {
            (Some(Ok(a)), Some(Ok(b))) => apply_pair(a, b),
            (Some(Ok(a)), _) => a.jong = neutralize(a.jong, None), // 어절 끝
            _ => {}
        }
    }
    items
        .into_iter()
        .map(|it| match it {
            Ok(mut s) => {
                apply_vowels(&mut s);
                compose(s)
            }
            Err(c) => c,
        })
        .collect()
}

/// 음절의 모음 (중성 번호)
pub fn vowel_of(c: char) -> Option<u32> {
    decompose(c).map(|s| s.jung)
}

pub fn is_hangul(c: char) -> bool {
    decompose(c).is_some()
}

/// 음소 하나 = 정렬 모델이 낼 수 있는 기호 후보들 (모델이 한국어를 여러 표기로 적기 때문)
pub type Phone = &'static [&'static str];

/// 발음형 음절 1개 → 음소 열 : (초성, 중성, 종성)
/// 된소리·거센소리·울림소리 차이는 후보 집합으로 흡수 (정렬 경계를 찾는 데는 충분)
pub fn phones(syllable: char) -> Option<(Vec<Phone>, Vec<Phone>, Vec<Phone>)> {
    const K: Phone = &["k", "ɡ", "kh", "kʰ", "kː", "q"];
    const T: Phone = &["t", "d", "th", "tʰ", "tː", "t̪", "ɾ"];
    const P: Phone = &["p", "b", "ph", "pʰ", "pː", "β"];
    const S: Phone = &["s", "ɕ", "sʲ", "s̪", "ʃ"];
    const J: Phone = &["tɕ", "dʒ", "dʑ", "tʃ", "ts", "tɕh", "tʃʰ", "ʑ"];
    const R: Phone = &["ɾ", "r", "l", "ɹ", "ɭ", "d"];
    const H: Phone = &["h", "x", "ç", "ɦ"];
    let onset: Vec<Phone> = match decompose(syllable)?.cho {
        'ㄱ' | 'ㄲ' | 'ㅋ' => vec![K],
        'ㄴ' => vec![&["n", "nʲ", "ɲ"]],
        'ㄷ' | 'ㄸ' | 'ㅌ' => vec![T],
        'ㄹ' => vec![R],
        'ㅁ' => vec![&["m", "mʲ"]],
        'ㅂ' | 'ㅃ' | 'ㅍ' => vec![P],
        'ㅅ' | 'ㅆ' => vec![S],
        'ㅈ' | 'ㅉ' | 'ㅊ' => vec![J],
        'ㅎ' => vec![H],
        _ => vec![], // ㅇ
    };
    const A: Phone = &["a", "ɐ", "aː", "ɑ", "ä"];
    const AE: Phone = &["ɛ", "e", "eː", "e̞", "ɛː"];
    const EO: Phone = &["ʌ", "ə", "ɔ", "ɒ", "o"];
    const O: Phone = &["o", "oː", "o̞", "ɔ"];
    const U: Phone = &["u", "uː", "ʊ", "ɯ"];
    const EU: Phone = &["ɯ", "ɨ", "ɯᵝ", "ʊ", "u", "ə"];
    const I: Phone = &["i", "iː", "ɪ", "i."];
    const YJ: Phone = &["j"];
    const W: Phone = &["w", "ʋ", "u"];
    const V: [&[Phone]; 21] = [
        &[A], &[AE], &[YJ, A], &[YJ, AE], &[EO], &[AE], &[YJ, EO], &[YJ, AE], &[O], &[W, A],
        &[W, AE], &[W, AE], &[YJ, O], &[U], &[W, EO], &[W, AE], &[W, I], &[YJ, U], &[EU],
        &[EU, I], &[I],
    ];
    let s = decompose(syllable)?;
    let vowel = V[s.jung as usize].to_vec();
    let coda: Vec<Phone> = match neutralize(s.jong, None) {
        'ㄱ' => vec![K],
        'ㄴ' => vec![&["n", "nʲ", "ŋ"]],
        'ㄷ' => vec![T],
        'ㄹ' => vec![&["l", "ɾ", "r", "ɭ"]],
        'ㅁ' => vec![&["m", "n"]],
        'ㅂ' => vec![P],
        'ㅇ' => vec![&["ŋ", "n"]],
        _ => vec![],
    };
    Some((onset, vowel, coda))
}

/// 영상에 없는 음절을 대신할 비슷한 소리 (가까운 순).
/// 된소리→예사소리(빤→반), 받침 없애기(랑→라), 둘 다(빵→바)
pub fn similar(syllable: &str) -> Vec<String> {
    let mut chars = syllable.chars();
    let (Some(c), None) = (chars.next(), chars.next()) else { return Vec::new() };
    let Some(s) = decompose(c) else { return Vec::new() };
    let plain = match s.cho {
        'ㄲ' => 'ㄱ',
        'ㄸ' => 'ㄷ',
        'ㅃ' => 'ㅂ',
        'ㅆ' => 'ㅅ',
        'ㅉ' => 'ㅈ',
        x => x,
    };
    let mut out: Vec<String> = Vec::new();
    let mut push = |v: Syl| {
        let ch = compose(v).to_string();
        if ch != syllable && !out.contains(&ch) {
            out.push(ch);
        }
    };
    push(Syl { cho: plain, ..s });
    push(Syl { jong: '\0', ..s });
    push(Syl { cho: plain, jong: '\0', ..s });
    out
}

#[cfg(test)]
mod tests {
    use super::pronounce;

    #[test]
    fn standard_pronunciation_rules() {
        let cases = [
            ("같이", "가치"),
            ("굳이", "구지"),
            ("국물", "궁물"),
            ("밥만", "밤만"),
            ("읽는", "잉는"),
            ("음악", "으막"),
            ("있어", "이써"),
            ("앉아", "안자"),
            ("닭이", "달기"),
            ("값이", "갑씨"),
            ("좋아", "조아"),
            ("않아", "아나"),
            ("싫어", "시러"),
            ("좋다", "조타"),
            ("놓고", "노코"),
            ("놓는", "논는"),
            ("축하", "추카"),
            ("입학", "이팍"),
            ("못해", "모태"),
            ("닫히다", "다치다"),
            ("신라", "실라"),
            ("설날", "설랄"),
            ("독립", "동닙"),
            ("심리", "심니"),
            ("학교", "학꾜"),
            ("했다", "핻따"),
            ("넓게", "널께"),
            ("희망", "히망"),
            ("계속", "게속"),
            ("가져", "가저"),
            ("사랑해", "사랑해"),
            ("꽃", "꼳"),
            ("부엌", "부억"),
        ];
        for (w, want) in cases {
            assert_eq!(pronounce(w), want, "{w}");
        }
    }

    #[test]
    fn similar_sounds() {
        assert_eq!(super::similar("빤"), ["반", "빠", "바"]);
        assert_eq!(super::similar("랑"), ["라"]);
        assert!(super::similar("가").is_empty());
    }

    #[test]
    fn leaves_non_hangul_alone() {
        assert_eq!(pronounce("love"), "love");
        assert_eq!(pronounce("2006년"), "2006년");
    }
}
