//! 앱과 함께 설치되는 AI 모델 파일 찾기. 네트워크 다운로드는 하지 않는다(의존성 없는 배포).

use std::path::{Path, PathBuf};

pub const WHISPER_MODEL: &str = "ggml-large-v3-q5_0.bin";
pub const ALIGN_MODEL: &str = "phoneme-aligner-int8.onnx";
pub const ALIGN_VOCAB: &str = "phoneme-aligner-vocab.json";
/// Hugging Face 페이지 HTML이 저장된 경우를 걸러 냄 (~1GB 모델)
const WHISPER_MIN_BYTES: u64 = 50_000_000;

pub(crate) fn search_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let mut add = |p: PathBuf| {
        if !dirs.iter().any(|d| d == &p) {
            dirs.push(p);
        }
    };
    if let Ok(raw) = std::env::var("DCINVOX_MODELS") {
        add(PathBuf::from(raw));
    }
    if let Ok(exe) = std::env::current_exe() {
        let exe = std::fs::canonicalize(&exe).unwrap_or(exe);
        if let Some(dir) = exe.parent() {
            let mut dir = dir.to_path_buf();
            for _ in 0..8 {
                add(dir.join("models"));
                add(dir.join("Resources/models"));
                if !dir.pop() {
                    break;
                }
            }
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        add(cwd.join("models"));
    }
    if cfg!(debug_assertions) || cfg!(test) {
        add(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../models"));
    }
    dirs
}

fn translocated() -> bool {
    std::env::current_exe()
        .ok()
        .map(|p| p.to_string_lossy().contains("AppTranslocation"))
        .unwrap_or(false)
}

fn lookup(name: &str) -> Option<PathBuf> {
    search_dirs().into_iter().map(|d| d.join(name)).find(|p| p.is_file())
}

fn whisper_too_small(path: &Path) -> Option<u64> {
    let n = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    (n < WHISPER_MIN_BYTES).then_some(n)
}

pub fn find(name: &str) -> Result<PathBuf, String> {
    let path = lookup(name).ok_or_else(|| explain_missing(&[name]))?;
    if name == WHISPER_MODEL {
        if let Some(n) = whisper_too_small(&path) {
            return Err(tr!("model_too_small", name, n));
        }
    }
    Ok(path)
}

/// 분석·생성 전에 필요한 파일을 한꺼번에 확인. 있는 파일 이름을 없는 파일로 바꾸지 않는다.
pub fn require(names: &[&str]) -> Result<(), String> {
    let mut missing = Vec::new();
    for name in names {
        match lookup(name) {
            None => missing.push(*name),
            Some(p) if *name == WHISPER_MODEL => {
                if let Some(n) = whisper_too_small(&p) {
                    return Err(tr!("model_too_small", name, n));
                }
            }
            Some(_) => {}
        }
    }
    if missing.is_empty() {
        Ok(())
    } else {
        Err(explain_missing(&missing))
    }
}

fn explain_missing(missing: &[&str]) -> String {
    let mut lines = Vec::new();
    for name in missing {
        lines.push(tr!("model_missing", name));
        if *name == ALIGN_MODEL {
            lines.push(tr!("model_onnx_hint").to_string());
        }
        if *name == WHISPER_MODEL {
            lines.push(tr!("model_whisper_hint").to_string());
        }
    }
    lines.push(tr!("model_looked_in").to_string());
    for d in search_dirs() {
        let exists = d.is_dir();
        lines.push(format!("  {} {}", if exists { "+" } else { "-" }, d.display()));
        if exists {
            if let Ok(rd) = std::fs::read_dir(&d) {
                for ent in rd.flatten() {
                    let n = ent.file_name();
                    let n = n.to_string_lossy();
                    if n.ends_with(".bin") || n.ends_with(".onnx") || n.ends_with(".json") {
                        lines.push(format!("      {n}"));
                    }
                }
            }
        }
    }
    if translocated() {
        lines.push(tr!("model_translocated").to_string());
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_vocab_from_repo_models() {
        let path = find("phoneme-aligner-vocab.json").expect("vocab is in repo models/");
        assert!(path.ends_with("phoneme-aligner-vocab.json"));
    }

    #[test]
    fn missing_model_mentions_install_docs() {
        let err = find("definitely-not-a-model.bin").unwrap_err();
        assert!(err.contains("definitely-not-a-model.bin"));
        assert!(err.contains("INSTALL.md"), "{err}");
    }

    #[test]
    fn onnx_missing_does_not_blame_whisper() {
        let err = explain_missing(&[ALIGN_MODEL]);
        assert!(err.contains(ALIGN_MODEL), "{err}");
        assert!(err.contains("export_aligner"), "{err}");
        assert!(!err.contains(WHISPER_MODEL), "{err}");
    }

    #[test]
    fn require_reports_only_absent_files() {
        let err = require(&[ALIGN_VOCAB, ALIGN_MODEL, "no-such-model.bin"]).unwrap_err();
        assert!(err.contains("no-such-model.bin"), "{err}");
        assert!(err.contains(ALIGN_MODEL), "{err}");
        assert!(!err.contains(&format!("모델 파일({ALIGN_VOCAB})")), "{err}");
    }
}
