//! ffmpeg 호출. 사용자 PC에 설치된 ffmpeg에 의존하지 않도록 실행파일 옆에 동봉된 바이너리를 우선 사용한다.
//! (개발 중에는 PATH의 ffmpeg로 폴백)

use std::path::{Path, PathBuf};
use std::process::Command;

fn ffmpeg_name() -> &'static str {
    if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    }
}

/// 동봉된 ffmpeg 경로. 없으면 PATH 검색용 이름을 그대로 반환.
pub fn ffmpeg_path() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let bundled = dir.join(ffmpeg_name());
            if bundled.is_file() {
                return bundled;
            }
        }
    }
    PathBuf::from(ffmpeg_name())
}

fn ffmpeg() -> Command {
    let mut cmd = Command::new(ffmpeg_path());
    cmd.arg("-hide_banner").arg("-nostdin");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

pub struct ProbeInfo {
    pub duration: f64,
    pub has_audio: bool,
    pub has_video: bool,
}

/// ffprobe를 따로 동봉하지 않기 위해 `ffmpeg -i` 의 stderr를 파싱한다.
pub fn probe(input: &Path) -> Result<ProbeInfo, String> {
    let out = ffmpeg()
        .arg("-i")
        .arg(input)
        .output()
        .map_err(|e| tr!("ffmpeg_run_failed", e))?;
    let text = String::from_utf8_lossy(&out.stderr);
    let duration = text
        .lines()
        .find_map(|l| l.trim().strip_prefix("Duration: "))
        .and_then(|rest| rest.split(',').next())
        .and_then(parse_timestamp)
        .ok_or_else(|| tr!("video_length", input.display()))?;
    Ok(ProbeInfo {
        duration,
        has_audio: text.contains("Audio: "),
        has_video: text.contains("Video: "),
    })
}

fn parse_timestamp(s: &str) -> Option<f64> {
    let mut parts = s.trim().split(':');
    let h: f64 = parts.next()?.parse().ok()?;
    let m: f64 = parts.next()?.parse().ok()?;
    let sec: f64 = parts.next()?.parse().ok()?;
    Some(h * 3600.0 + m * 60.0 + sec)
}

/// 원본 영상에서 48kHz mono 16bit WAV 추출 (출력 품질용 원본 오디오)
pub fn extract_wav(input: &Path, output: &Path, sample_rate: u32) -> Result<(), String> {
    extract_wav_ch(input, output, sample_rate, 1)
}

/// 말소리용: 스테레오는 가운데(보컬)만 남기고, 말소리 대역만 통과. 신경망 분리 모델은 쓰지 않는다.
pub fn extract_speech_wav(input: &Path, output: &Path, sample_rate: u32) -> Result<(), String> {
    let af = "aformat=channel_layouts=stereo,highpass=f=80,lowpass=f=8000,pan=mono|c0=0.5*c0+0.5*c1";
    let out = ffmpeg()
        .arg("-y")
        .arg("-i")
        .arg(input)
        .args(["-vn", "-af", af, "-ac", "1", "-ar", &sample_rate.to_string(), "-c:a", "pcm_s16le"])
        .arg(output)
        .output()
        .map_err(|e| tr!("ffmpeg_run_failed", e))?;
    if out.status.success() {
        return Ok(());
    }
    extract_wav(input, output, sample_rate)
}

/// 채널 수를 지정해 16bit WAV 추출 (반주는 스테레오 유지)
pub fn extract_wav_ch(input: &Path, output: &Path, sample_rate: u32, channels: u32) -> Result<(), String> {
    let out = ffmpeg()
        .arg("-y")
        .arg("-i")
        .arg(input)
        .args(["-vn", "-ac", &channels.to_string(), "-ar", &sample_rate.to_string(), "-c:a", "pcm_s16le"])
        .arg(output)
        .output()
        .map_err(|e| tr!("ffmpeg_run_failed", e))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        let tail: Vec<&str> = err.lines().rev().take(3).collect();
        return Err(tr!("audio_extract_failed", tail.into_iter().rev().collect::<Vec<_>>().join(" / ")));
    }
    Ok(())
}

/// ffmpeg 실행 후 실패 시 stderr 마지막 줄들로 오류 메시지
pub fn run(args: &[String]) -> Result<(), String> {
    let out = ffmpeg().args(args).output().map_err(|e| tr!("ffmpeg_run_failed", e))?;
    if out.status.success() {
        return Ok(());
    }
    let err = String::from_utf8_lossy(&out.stderr);
    let tail: Vec<&str> = err.lines().rev().take(3).collect();
    Err(tr!("ffmpeg_error", tail.into_iter().rev().collect::<Vec<_>>().join(" / ")))
}

/// H.264 인코더 선택. GPL인 libx264는 배포판에 넣지 않으므로 OS 하드웨어 인코더 → OpenH264 순.
/// (개발용 ffmpeg에만 있는 libx264는 마지막 폴백)
pub fn h264_args() -> &'static [&'static str] {
    static CHOICE: std::sync::OnceLock<&'static [&'static str]> = std::sync::OnceLock::new();
    CHOICE.get_or_init(|| {
        let list = ffmpeg()
            .args(["-encoders"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
            .unwrap_or_default();
        let has = |name: &str| list.split_whitespace().any(|w| w == name);
        if has("h264_videotoolbox") {
            &["-c:v", "h264_videotoolbox", "-b:v", "2500k", "-allow_sw", "1"]
        } else if has("h264_mf") {
            &["-c:v", "h264_mf", "-b:v", "2500k"]
        } else if has("libopenh264") {
            &["-c:v", "libopenh264", "-b:v", "2500k"]
        } else if has("libx264") {
            &["-c:v", "libx264", "-preset", "veryfast", "-crf", "23"]
        } else {
            &["-c:v", "mpeg4", "-q:v", "4"]
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ffmpeg_duration() {
        assert_eq!(parse_timestamp("00:01:02.50"), Some(62.5));
        assert_eq!(parse_timestamp("01:00:00.00"), Some(3600.0));
        assert_eq!(parse_timestamp("N/A"), None);
    }

    #[test]
    fn speech_extract_writes_mono_wav() {
        let dir = std::env::temp_dir().join(format!("dcinvox-speech-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let src = dir.join("in.wav");
        let dst = dir.join("out.wav");
        let ok = ffmpeg()
            .args(["-loglevel", "error", "-y", "-f", "lavfi", "-i", "sine=frequency=440:duration=1", "-ac", "2"])
            .arg(&src)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !ok {
            eprintln!("ffmpeg 없음 — 테스트 건너뜀");
            return;
        }
        extract_speech_wav(&src, &dst, 16000).unwrap();
        let r = hound::WavReader::open(&dst).unwrap();
        assert_eq!(r.spec().channels, 1);
        assert_eq!(r.spec().sample_rate, 16000);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
