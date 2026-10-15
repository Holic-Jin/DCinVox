//! 사용자에게 보이는 오류·안내 문구 (한국어·영어·일본어). 화면 언어를 바꾸면 프론트엔드가 set_language로 알려 줌
use std::sync::atomic::{AtomicU8, Ordering};

static LANG: AtomicU8 = AtomicU8::new(0); // 0=ko 1=en 2=ja

pub fn set(code: &str) {
    LANG.store(match code { "en" => 1, "ja" => 2, _ => 0 }, Ordering::Relaxed);
}

/// (키, 한국어, 영어, 일본어). `{}` 자리는 순서대로 채움
const TEXTS: &[(&str, &str, &str, &str)] = &[
    ("no_project", "먼저 프로젝트를 열어 주세요", "Open a project first", "先にプロジェクトを開いてください"),
    ("project_closed", "프로젝트가 닫혔습니다", "The project was closed", "プロジェクトが閉じられました"),
    ("project_changed", "프로젝트가 바뀌었습니다", "The project was switched", "プロジェクトが切り替わりました"),
    ("need_analyze", "분석된 단어가 없습니다. 먼저 분석을 해 주세요.", "No analyzed words yet. Run Analyze first.", "解析済みの単語がありません。先に解析してください。"),
    ("need_lyrics", "가사를 입력해 주세요", "Please enter lyrics", "歌詞を入力してください"),
    ("need_generate", "먼저 생성해 주세요", "Please generate first", "先に生成してください"),
    ("need_convert", "먼저 변환해 주세요", "Please convert first", "先に変換してください"),
    ("no_generation", "생성 결과가 없습니다", "There is no generated result", "生成結果がありません"),
    ("bad_clip", "잘못된 클립", "Invalid clip", "無効なクリップ"),
    ("no_alternative", "바꿀 수 있는 다른 후보가 없습니다", "There are no other candidates to swap in", "交換できる他の候補がありません"),
    ("align_step_failed", "{} (발음 정렬): {}", "{} (alignment): {}", "{} (発音合わせ): {}"),
    ("check_step_failed", "{} (조각 미리 듣기): {}", "{} (piece check): {}", "{} (試聴チェック): {}"),
    ("mr_copy_failed", "반주 복사 실패: {}", "Couldn't copy the backing track: {}", "伴奏をコピーできませんでした: {}"),
    ("mr_read_failed", "반주 읽기 실패: {}", "Couldn't read the backing track: {}", "伴奏を読み込めませんでした: {}"),
    ("mr_reload", "반주를 다시 불러와 주세요 ({})", "Please load the backing track again ({})", "伴奏をもう一度読み込んでください ({})"),
    ("mr_too_short", "반주가 너무 짧습니다", "The backing track is too short", "伴奏が短すぎます"),
    ("no_mr", "반주가 없습니다", "No backing track is loaded", "伴奏がありません"),
    ("midi_read_failed", "MIDI 파일 읽기 실패: {}", "Couldn't read the MIDI file: {}", "MIDIファイルを読み込めませんでした: {}"),
    ("midi_unreadable", "MIDI 파일을 읽을 수 없습니다: {}", "This MIDI file can't be read: {}", "このMIDIファイルは読み込めません: {}"),
    ("midi_smpte", "지원하지 않는 MIDI 시간 형식입니다 (SMPTE)", "Unsupported MIDI timing format (SMPTE)", "対応していないMIDIの時間形式です (SMPTE)"),
    ("midi_no_notes", "MIDI 파일에 음표가 없습니다", "The MIDI file has no notes", "MIDIファイルに音符がありません"),
    ("unknown_format", "알 수 없는 형식: {}", "Unknown format: {}", "不明な形式: {}"),
    ("align_vocab_failed", "정렬 어휘 읽기 실패: {}", "Couldn't read the alignment vocabulary: {}", "発音合わせの語彙を読み込めませんでした: {}"),
    ("align_model_failed", "발음 정렬 모델 로드 실패: {}", "Couldn't load the alignment model: {}", "発音合わせモデルを読み込めませんでした: {}"),
    ("align_run_failed", "정렬 모델 실행 실패: {}", "The alignment model failed: {}", "発音合わせモデルの実行に失敗しました: {}"),
    ("stt_model_failed", "음성 인식 모델 로드 실패: {}", "Couldn't load the speech recognition model: {}", "音声認識モデルを読み込めませんでした: {}"),
    ("stt_failed", "음성 인식 실패: {}", "Speech recognition failed: {}", "音声認識に失敗しました: {}"),
    ("model_missing", "모델 파일({})을 찾을 수 없습니다. 앱 옆 models/ 폴더에 두세요. (docs/INSTALL.md)", "Model file ({}) not found. Put it in the models/ folder next to the app. (docs/INSTALL.md)", "モデルファイル({})が見つかりません。アプリの横の models/ に置いてください。(docs/INSTALL.md)"),
    ("db_error", "색인 DB 오류: {}", "Index database error: {}", "索引データベースのエラー: {}"),
    ("ffmpeg_run_failed", "ffmpeg 실행 실패: {}", "Couldn't run ffmpeg: {}", "ffmpegを実行できませんでした: {}"),
    ("ffmpeg_error", "ffmpeg 오류: {}", "ffmpeg error: {}", "ffmpegのエラー: {}"),
    ("video_length", "영상 길이를 읽을 수 없습니다: {}", "Couldn't read the video length: {}", "動画の長さを読み取れません: {}"),
    ("audio_extract_failed", "오디오 추출 실패: {}", "Couldn't extract the audio: {}", "音声を取り出せませんでした: {}"),
    ("audio_cache_failed", "오디오 캐시 열기 실패: {}", "Couldn't open the audio cache: {}", "音声キャッシュを開けませんでした: {}"),
    ("wav_read_failed", "WAV 읽기 실패: {}", "Couldn't read the WAV file: {}", "WAVを読み込めませんでした: {}"),
    ("wav_not_16k", "16kHz mono WAV가 아닙니다 ({}Hz, {}ch)", "Not a 16 kHz mono WAV ({} Hz, {} ch)", "16kHzモノラルWAVではありません ({}Hz, {}ch)"),
    ("source_missing", "원본 영상을 찾을 수 없습니다", "The source video can't be found", "元の動画が見つかりません"),
    ("asset_not_found", "해당 영상을 찾을 수 없습니다", "That video can't be found", "その動画が見つかりません"),
    ("no_audio", "소리가 없는 영상입니다: {}", "This video has no sound: {}", "音声の無い動画です: {}"),
    ("project_json_failed", "project.json 읽기 실패: {}", "Couldn't read project.json: {}", "project.jsonを読み込めませんでした: {}"),
    ("bad_path", "잘못된 파일 경로입니다", "Invalid file path", "無効なファイルパスです"),
    ("cannot_name_file", "파일 이름을 정할 수 없습니다", "Couldn't choose a file name", "ファイル名を決められません"),
    ("file_error", "파일 오류: {}", "File error: {}", "ファイルのエラー: {}"),
    // ui/i18n.js의 "err.cancelled"와 같은 글자여야 취소로 인식함
    ("cancelled", "취소됨", "Cancelled", "キャンセル済み"),
];

pub fn text(key: &str) -> &'static str {
    let lang = LANG.load(Ordering::Relaxed);
    TEXTS
        .iter()
        .find(|t| t.0 == key)
        .map(|t| match lang { 1 => t.2, 2 => t.3, _ => t.1 })
        .unwrap_or_else(|| panic!("번역 없음: {key}"))
}

pub fn fill(template: &str, args: &[&dyn std::fmt::Display]) -> String {
    let mut out = String::new();
    let mut rest = template;
    let mut args = args.iter();
    while let Some(i) = rest.find("{}") {
        out.push_str(&rest[..i]);
        if let Some(a) = args.next() {
            out.push_str(&a.to_string());
        }
        rest = &rest[i + 2..];
    }
    out.push_str(rest);
    out
}

/// `tr!("key")` → &'static str, `tr!("key", a, b)` → `{}`를 채운 String
macro_rules! tr {
    ($key:expr) => {
        $crate::i18n::text($key)
    };
    ($key:expr, $($arg:expr),+ $(,)?) => {
        $crate::i18n::fill($crate::i18n::text($key), &[$(&$arg as &dyn std::fmt::Display),+])
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_key_has_all_languages_and_matching_placeholders() {
        for (key, ko, en, ja) in TEXTS {
            let n = ko.matches("{}").count();
            assert!(!en.is_empty() && !ja.is_empty(), "{key}");
            assert_eq!(en.matches("{}").count(), n, "{key} en");
            assert_eq!(ja.matches("{}").count(), n, "{key} ja");
        }
        assert_eq!(fill("{} (x): {}", &[&"a.mp4", &3]), "a.mp4 (x): 3");
    }
}
