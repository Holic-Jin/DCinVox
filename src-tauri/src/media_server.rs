//! 영상 재생용 초경량 로컬 HTTP 서버 (127.0.0.1 전용, Range 요청 지원).
//! 웹뷰의 커스텀 asset 프로토콜로는 macOS에서 영상 소리가 나지 않는 경우가 있어,
//! 플랫폼 미디어 엔진이 가장 잘 다루는 http + Range 방식으로 현재 프로젝트 폴더의 파일만 제공한다.

use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub struct MediaServer {
    pub port: u16,
    pub token: String,
    root: Arc<Mutex<Option<PathBuf>>>,
}

impl MediaServer {
    pub fn start() -> std::io::Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let server = MediaServer {
            port: listener.local_addr()?.port(),
            token: random_token(),
            root: Arc::new(Mutex::new(None)),
        };
        let s = server.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let s = s.clone();
                std::thread::spawn(move || {
                    let _ = s.handle(stream);
                });
            }
        });
        Ok(server)
    }

    /// 제공할 프로젝트 폴더 지정. 이 폴더 밖의 파일은 절대 제공하지 않는다.
    pub fn set_root(&self, root: &Path) {
        *self.root.lock().unwrap() = Some(root.to_path_buf());
    }

    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}/{}", self.port, self.token)
    }

    fn handle(&self, stream: TcpStream) -> std::io::Result<()> {
        let mut reader = BufReader::new(stream.try_clone()?);
        let mut out = stream;

        let mut request_line = String::new();
        reader.read_line(&mut request_line)?;
        let mut range = None;
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line)? == 0 || line == "\r\n" || line == "\n" {
                break;
            }
            if let Some((k, v)) = line.split_once(':') {
                if k.trim().eq_ignore_ascii_case("range") {
                    range = Some(v.trim().to_string());
                }
            }
        }

        let mut parts = request_line.split_whitespace();
        let method = parts.next().unwrap_or("");
        let target = parts.next().unwrap_or("");
        if method != "GET" && method != "HEAD" {
            return respond_status(&mut out, "405 Method Not Allowed");
        }
        let Some(path) = self.resolve(target) else {
            return respond_status(&mut out, "404 Not Found");
        };
        let Ok(mut file) = File::open(&path) else {
            return respond_status(&mut out, "404 Not Found");
        };
        let len = file.metadata()?.len();

        let (start, end, partial) = match range.as_deref().and_then(|r| parse_range(r, len)) {
            Some((s, e)) => (s, e, true),
            None if range.is_some() => {
                let head = format!("HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{len}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                return out.write_all(head.as_bytes());
            }
            None => (0, len.saturating_sub(1), false),
        };
        let body_len = if len == 0 { 0 } else { end - start + 1 };

        let mut head = String::new();
        head.push_str(if partial { "HTTP/1.1 206 Partial Content\r\n" } else { "HTTP/1.1 200 OK\r\n" });
        head.push_str(&format!("Content-Type: {}\r\n", content_type(&path)));
        head.push_str("Accept-Ranges: bytes\r\n");
        head.push_str(&format!("Content-Length: {body_len}\r\n"));
        if partial {
            head.push_str(&format!("Content-Range: bytes {start}-{end}/{len}\r\n"));
        }
        head.push_str("Access-Control-Allow-Origin: *\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n");
        out.write_all(head.as_bytes())?;

        if method == "GET" && body_len > 0 {
            file.seek(SeekFrom::Start(start))?;
            std::io::copy(&mut file.take(body_len), &mut out)?;
        }
        out.flush()
    }

    /// `/<token>/<프로젝트 기준 상대경로(퍼센트 인코딩)>` → 실제 파일 경로
    fn resolve(&self, target: &str) -> Option<PathBuf> {
        let target = target.split('?').next()?;
        let rest = target.strip_prefix('/')?.strip_prefix(self.token.as_str())?.strip_prefix('/')?;
        let rel = PathBuf::from(percent_decode(rest)?);
        // 상위 폴더 이동·절대경로 차단
        if rel.components().any(|c| !matches!(c, Component::Normal(_))) {
            return None;
        }
        let root = self.root.lock().unwrap().clone()?;
        Some(root.join(rel))
    }
}

fn respond_status(out: &mut TcpStream, status: &str) -> std::io::Result<()> {
    out.write_all(format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").as_bytes())
}

/// `bytes=start-end` / `bytes=start-` / `bytes=-suffix` (단일 구간만)
fn parse_range(header: &str, len: u64) -> Option<(u64, u64)> {
    let spec = header.strip_prefix("bytes=")?.split(',').next()?.trim();
    let (a, b) = spec.split_once('-')?;
    if len == 0 {
        return None;
    }
    let (start, end) = if a.is_empty() {
        let suffix: u64 = b.parse().ok()?;
        (len.saturating_sub(suffix), len - 1)
    } else {
        let start: u64 = a.parse().ok()?;
        let end = if b.is_empty() { len - 1 } else { b.parse::<u64>().ok()?.min(len - 1) };
        (start, end)
    };
    (start <= end && start < len).then_some((start, end))
}

fn percent_decode(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = std::str::from_utf8(bytes.get(i + 1..i + 3)?).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref() {
        Some("mp4" | "m4v") => "video/mp4",
        Some("mov") => "video/quicktime",
        Some("webm") => "video/webm",
        Some("mkv") => "video/x-matroska",
        Some("wav") => "audio/wav",
        Some("mp3") => "audio/mpeg",
        Some("m4a") => "audio/mp4",
        _ => "application/octet-stream",
    }
}

/// 다른 로컬 프로그램이 URL을 추측하지 못하도록 실행마다 다른 토큰
fn random_token() -> String {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let addr = &nanos as *const _ as usize as u128;
    let mut x = nanos ^ (addr << 64) ^ (std::process::id() as u128) << 32;
    let mut s = String::new();
    for _ in 0..24 {
        // xorshift
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        s.push(char::from_digit((x % 36) as u32, 36).unwrap());
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_parsing() {
        assert_eq!(parse_range("bytes=0-1", 10), Some((0, 1)));
        assert_eq!(parse_range("bytes=5-", 10), Some((5, 9)));
        assert_eq!(parse_range("bytes=-3", 10), Some((7, 9)));
        assert_eq!(parse_range("bytes=0-100", 10), Some((0, 9)));
        assert_eq!(parse_range("bytes=20-", 10), None);
    }

    #[test]
    fn serves_only_project_files_with_ranges() {
        let dir = std::env::temp_dir().join(format!("dcinvox-ms-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("Assets")).unwrap();
        std::fs::write(dir.join("Assets").join("한 글.mp4"), b"0123456789").unwrap();

        let server = MediaServer::start().unwrap();
        server.set_root(&dir);
        let get = |path: &str, range: Option<&str>| {
            let mut s = TcpStream::connect(("127.0.0.1", server.port)).unwrap();
            let r = range.map(|r| format!("Range: {r}\r\n")).unwrap_or_default();
            write!(s, "GET {path} HTTP/1.1\r\nHost: x\r\n{r}\r\n").unwrap();
            let mut buf = String::new();
            s.read_to_string(&mut buf).unwrap();
            buf
        };
        let base = format!("/{}", server.token);

        let full = get(&format!("{base}/Assets/%ED%95%9C%20%EA%B8%80.mp4"), None);
        assert!(full.starts_with("HTTP/1.1 200"));
        assert!(full.ends_with("0123456789"));

        let part = get(&format!("{base}/Assets/%ED%95%9C%20%EA%B8%80.mp4"), Some("bytes=2-4"));
        assert!(part.starts_with("HTTP/1.1 206"));
        assert!(part.contains("Content-Range: bytes 2-4/10"));
        assert!(part.ends_with("234"));

        assert!(get(&format!("{base}/../etc/passwd"), None).starts_with("HTTP/1.1 404"));
        assert!(get("/wrongtoken/Assets/x.mp4", None).starts_with("HTTP/1.1 404"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
