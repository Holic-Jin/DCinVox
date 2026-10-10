//! 앱과 함께 설치되는 AI 모델 파일 찾기. 네트워크 다운로드는 하지 않는다(의존성 없는 배포).

use std::path::PathBuf;

pub const WHISPER_MODEL: &str = "ggml-large-v3-q5_0.bin";

fn search_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            // Windows·Linux: 실행파일 옆 models/
            dirs.push(dir.join("models"));
            // macOS .app 안: Contents/MacOS/../Resources/models
            dirs.push(dir.join("../Resources/models"));
            // Steam 배포(macOS): DCinVox.app 옆 models/ — 서명된 .app을 건드리지 않고 모델을 따로 둠
            dirs.push(dir.join("../../../models"));
        }
    }
    // 개발 중: 저장소의 models/ 폴더
    if cfg!(debug_assertions) || cfg!(test) {
        dirs.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../models"));
    }
    dirs
}

pub fn find(name: &str) -> Result<PathBuf, String> {
    search_dirs()
        .into_iter()
        .map(|d| d.join(name))
        .find(|p| p.is_file())
        .ok_or_else(|| tr!("model_missing", name))
}
