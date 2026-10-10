//! 프로젝트 폴더 구조
//! ```text
//! MyProject/
//!   project.json
//!   Assets/   사용자가 불러온 영상 (프로그램에는 영상이 포함되지 않음)
//!   cache/    추출한 오디오 등 재생성 가능한 파일
//! ```

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::media;

pub const PROJECT_FILE: &str = "project.json";
pub const ASSETS_DIR: &str = "Assets";
pub const CACHE_DIR: &str = "cache";
pub const OUTPUT_SAMPLE_RATE: u32 = 48_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StepStatus {
    Idle,
    Done,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub id: String,
    pub file_name: String,
    pub duration: f64,
    pub has_video: bool,
    pub analysis: StepStatus,
    /// 음소 정렬까지 끝났는지 (음절 단위 정밀 자르기)
    #[serde(default)]
    pub aligned: bool,
    /// 조각 재검증 점수까지 미리 계산했는지 (생성 속도)
    #[serde(default)]
    pub checked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub version: u32,
    pub assets: Vec<Asset>,
    #[serde(default)]
    pub lyrics: String,
    /// 마지막 생성 결과 (클립 + 미리보기 위치)
    #[serde(default)]
    pub generation: Option<Generation>,
    /// 음계 블록 (노래 설정)
    #[serde(default)]
    pub song: Option<crate::song::Song>,
    /// 마지막 변환 결과 위치 (재생 커서용). 노래를 고치면 지워짐
    #[serde(default)]
    pub conversion: Option<Vec<crate::convert::NoteSpan>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Generation {
    /// 생성에 쓴 가사 (현재 가사와 다르면 다시 생성 필요)
    pub lyrics: String,
    pub clips: Vec<crate::compose::Clip>,
    pub timeline: Vec<crate::render::TimelineItem>,
}

impl Default for Project {
    fn default() -> Self {
        Self { version: 1, assets: Vec::new(), lyrics: String::new(), generation: None, song: None, conversion: None }
    }
}

/// UI로 보내는 프로젝트 상태 (경로 포함)
#[derive(Debug, Clone, Serialize)]
pub struct ProjectView {
    pub root: String,
    pub name: String,
    pub assets: Vec<AssetView>,
    pub lyrics: String,
    pub generation: Option<Generation>,
    pub song: Option<crate::song::Song>,
    pub conversion: Option<Vec<crate::convert::NoteSpan>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AssetView {
    #[serde(flatten)]
    pub asset: Asset,
    pub path: String,
}

pub fn open_or_create(root: &Path) -> Result<Project, String> {
    fs::create_dir_all(root.join(ASSETS_DIR)).map_err(io_err)?;
    fs::create_dir_all(root.join(CACHE_DIR)).map_err(io_err)?;
    let file = root.join(PROJECT_FILE);
    if file.is_file() {
        let text = fs::read_to_string(&file).map_err(io_err)?;
        serde_json::from_str(&text).map_err(|e| tr!("project_json_failed", e))
    } else {
        let project = Project::default();
        save(root, &project)?;
        Ok(project)
    }
}

pub fn save(root: &Path, project: &Project) -> Result<(), String> {
    let text = serde_json::to_string_pretty(project).map_err(|e| e.to_string())?;
    // 저장 중 강제 종료돼도 project.json이 깨지지 않도록 임시파일 후 교체
    let tmp = root.join(format!("{PROJECT_FILE}.tmp"));
    fs::write(&tmp, text).map_err(io_err)?;
    fs::rename(&tmp, root.join(PROJECT_FILE)).map_err(io_err)
}

pub fn view(root: &Path, project: &Project) -> ProjectView {
    ProjectView {
        root: root.display().to_string(),
        name: root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
        assets: project
            .assets
            .iter()
            .map(|a| AssetView {
                asset: a.clone(),
                path: root.join(ASSETS_DIR).join(&a.file_name).display().to_string(),
            })
            .collect(),
        lyrics: project.lyrics.clone(),
        generation: project.generation.clone(),
        song: project.song.clone(),
        conversion: project.conversion.clone(),
    }
}

pub fn cache_wav(root: &Path, asset_id: &str) -> PathBuf {
    root.join(CACHE_DIR).join(format!("{asset_id}.wav"))
}

/// 영상을 Assets/로 복사하고 원본 오디오를 cache/에 추출한다.
pub fn import_video(root: &Path, project: &mut Project, source: &Path) -> Result<(), String> {
    let info = media::probe(source)?;
    if !info.has_audio {
        return Err(tr!("no_audio", display_name(source)));
    }

    let file_name = unique_file_name(&root.join(ASSETS_DIR), source)?;
    let dest = root.join(ASSETS_DIR).join(&file_name);
    let already_inside = source.canonicalize().ok() == dest.canonicalize().ok();
    if !already_inside {
        fs::copy(source, &dest).map_err(io_err)?;
    }

    let id = next_id(project);
    if let Err(e) = media::extract_wav(&dest, &cache_wav(root, &id), OUTPUT_SAMPLE_RATE) {
        if !already_inside {
            let _ = fs::remove_file(&dest);
        }
        return Err(e);
    }

    project.assets.push(Asset {
        id,
        file_name,
        duration: info.duration,
        has_video: info.has_video,
        analysis: StepStatus::Idle,
        aligned: false,
        checked: false,
    });
    Ok(())
}

pub fn remove_asset(root: &Path, project: &mut Project, asset_id: &str) -> Result<(), String> {
    let pos = project
        .assets
        .iter()
        .position(|a| a.id == asset_id)
        .ok_or(tr!("asset_not_found"))?;
    let asset = project.assets.remove(pos);
    let _ = fs::remove_file(cache_wav(root, &asset.id));
    // Assets/ 의 영상 파일 자체는 사용자 소유물이므로 지우지 않는다.
    Ok(())
}

fn next_id(project: &Project) -> String {
    let max = project
        .assets
        .iter()
        .filter_map(|a| a.id.strip_prefix('a').and_then(|n| n.parse::<u32>().ok()))
        .max()
        .unwrap_or(0);
    format!("a{:04}", max + 1)
}

fn display_name(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

pub fn unique_file_name(dir: &Path, source: &Path) -> Result<String, String> {
    let name = display_name(source);
    if name.is_empty() {
        return Err(tr!("bad_path").into());
    }
    let candidate = dir.join(&name);
    if !candidate.exists() || source.canonicalize().ok() == candidate.canonicalize().ok() {
        return Ok(name);
    }
    let stem = source.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let ext = source.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    (2..10_000)
        .map(|n| format!("{stem} ({n}){ext}"))
        .find(|n| !dir.join(n).exists())
        .ok_or_else(|| tr!("cannot_name_file").into())
}

fn io_err(e: std::io::Error) -> String {
    tr!("file_error", e)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_increment_past_removed_assets() {
        let mut p = Project::default();
        for id in ["a0001", "a0003"] {
            p.assets.push(Asset {
                id: id.into(),
                file_name: String::new(),
                duration: 0.0,
                has_video: true,
                analysis: StepStatus::Idle,
                aligned: false,
                checked: false,
            });
        }
        assert_eq!(next_id(&p), "a0004");
    }

    /// ffmpeg로 2초짜리 테스트 영상을 만들어 불러오기 전체 흐름을 확인
    #[test]
    fn imports_video_into_assets_and_cache() {
        let tmp = std::env::temp_dir().join(format!("dcinvox-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let src = tmp.join("clip.mp4");
        let ok = std::process::Command::new(media::ffmpeg_path())
            .args(["-loglevel", "error", "-f", "lavfi", "-i", "testsrc=size=160x120:rate=10:duration=2",
                   "-f", "lavfi", "-i", "sine=frequency=220:duration=2", "-shortest", "-pix_fmt", "yuv420p"])
            .arg(&src)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !ok {
            eprintln!("ffmpeg 없음 — 테스트 건너뜀");
            return;
        }

        let root = tmp.join("Proj");
        let mut p = open_or_create(&root).unwrap();
        import_video(&root, &mut p, &src).unwrap();
        import_video(&root, &mut p, &src).unwrap(); // 같은 이름 두 번 → 이름 중복 회피

        assert_eq!(p.assets.len(), 2);
        assert_eq!(p.assets[1].file_name, "clip (2).mp4");
        assert!((p.assets[0].duration - 2.0).abs() < 0.2);
        assert!(p.assets[0].has_video);
        assert!(cache_wav(&root, &p.assets[0].id).is_file());

        save(&root, &p).unwrap();
        assert_eq!(open_or_create(&root).unwrap().assets.len(), 2);
        let _ = fs::remove_dir_all(&tmp);
    }
}
