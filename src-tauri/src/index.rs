//! 단어/음절 조각 색인 (프로젝트 폴더의 index.db, SQLite)

use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::path::Path;

use crate::g2p;
use crate::stt::{Segment, Transcript};

pub const INDEX_FILE: &str = "index.db";

pub fn open(root: &Path) -> Result<Connection, String> {
    let conn = Connection::open(root.join(INDEX_FILE)).map_err(db_err)?;
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         CREATE TABLE IF NOT EXISTS segments (
           id INTEGER PRIMARY KEY,
           asset_id TEXT NOT NULL,
           start REAL NOT NULL,
           end REAL NOT NULL,
           text TEXT NOT NULL
         );
         CREATE TABLE IF NOT EXISTS tokens (
           id INTEGER PRIMARY KEY,
           asset_id TEXT NOT NULL,
           segment_id INTEGER NOT NULL,
           kind TEXT NOT NULL,          -- word | syllable (음절은 마일스톤 4)
           text TEXT NOT NULL,          -- 화면 표시용 원문
           norm TEXT NOT NULL,          -- 매칭용 정규화 문자열 (마일스톤 3에서 발음형으로 교체)
           lang TEXT NOT NULL,
           start REAL NOT NULL,
           end REAL NOT NULL,
           prob REAL NOT NULL
         );
         CREATE INDEX IF NOT EXISTS tokens_norm ON tokens(norm);
         CREATE INDEX IF NOT EXISTS tokens_asset ON tokens(asset_id);
         CREATE INDEX IF NOT EXISTS segments_asset ON segments(asset_id);",
    )
    .map_err(db_err)?;
    migrate(&conn)?;
    Ok(conn)
}

/// 이전 버전 색인에 발음형(pron) 열 추가 후 채우기 — 재분석 없이 업그레이드
fn migrate(conn: &Connection) -> Result<(), String> {
    let has_pron = conn
        .prepare("SELECT 1 FROM pragma_table_info('tokens') WHERE name = 'pron'")
        .and_then(|mut st| st.exists([]))
        .map_err(db_err)?;
    if !has_pron {
        conn.execute("ALTER TABLE tokens ADD COLUMN pron TEXT", []).map_err(db_err)?;
    }
    let missing: Vec<(i64, String)> = conn
        .prepare("SELECT id, text FROM tokens WHERE pron IS NULL")
        .map_err(db_err)?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .map_err(db_err)?
        .collect::<Result<_, _>>()
        .map_err(db_err)?;
    if !missing.is_empty() {
        let tx = conn.unchecked_transaction().map_err(db_err)?;
        {
            let mut up = tx.prepare("UPDATE tokens SET pron = ?1 WHERE id = ?2").map_err(db_err)?;
            for (id, text) in missing {
                up.execute(params![g2p::pron(&text), id]).map_err(db_err)?;
            }
        }
        tx.commit().map_err(db_err)?;
    }
    conn.execute("CREATE INDEX IF NOT EXISTS tokens_pron ON tokens(pron)", []).map_err(db_err)?;

    // 음소 정렬 결과 (마일스톤 4)
    let has_aligned = conn
        .prepare("SELECT 1 FROM pragma_table_info('tokens') WHERE name = 'aligned'")
        .and_then(|mut st| st.exists([]))
        .map_err(db_err)?;
    if !has_aligned {
        conn.execute("ALTER TABLE tokens ADD COLUMN aligned INTEGER NOT NULL DEFAULT 0", []).map_err(db_err)?;
    }
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS syllables (
           token_id INTEGER NOT NULL,
           idx INTEGER NOT NULL,
           unit TEXT NOT NULL,
           start REAL NOT NULL,
           end REAL NOT NULL,
           v_start REAL NOT NULL,   -- 모음 구간 (노래로 늘일 부분)
           v_end REAL NOT NULL
         );
         CREATE INDEX IF NOT EXISTS syllables_token ON syllables(token_id);",
    )
    .map_err(db_err)?;

    // 재검증 점수 미리 계산 (잘라 놓고 들었을 때 자기 소리로 들리는 정도)
    for (table, col) in [("syllables", "check_score"), ("tokens", "check_score")] {
        let has = conn
            .prepare(&format!("SELECT 1 FROM pragma_table_info('{table}') WHERE name = '{col}'"))
            .and_then(|mut st| st.exists([]))
            .map_err(db_err)?;
        if !has {
            conn.execute(&format!("ALTER TABLE {table} ADD COLUMN {col} REAL"), []).map_err(db_err)?;
        }
    }
    Ok(())
}

/// 미리 재검증할 조각: (테이블, rowid, 발음형, 시작, 끝)
pub fn pieces_to_check(conn: &Connection, asset_id: &str) -> Result<Vec<(&'static str, i64, String, f64, f64)>, String> {
    let mut out = Vec::new();
    let mut st = conn
        .prepare("SELECT s.rowid, s.unit, s.start, s.end FROM syllables s JOIN tokens t ON t.id = s.token_id
                  WHERE t.asset_id = ?1 AND s.check_score IS NULL")
        .map_err(db_err)?;
    for r in st.query_map([asset_id], |r| Ok(("syllables", r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))).map_err(db_err)? {
        out.push(r.map_err(db_err)?);
    }
    let mut st = conn
        .prepare("SELECT id, pron, start, end FROM tokens WHERE asset_id = ?1 AND aligned = 1 AND check_score IS NULL")
        .map_err(db_err)?;
    for r in st.query_map([asset_id], |r| Ok(("tokens", r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))).map_err(db_err)? {
        out.push(r.map_err(db_err)?);
    }
    Ok(out)
}

pub fn store_checks(conn: &mut Connection, scores: &[(&'static str, i64, f32)]) -> Result<(), String> {
    let tx = conn.transaction().map_err(db_err)?;
    {
        let mut syl = tx.prepare("UPDATE syllables SET check_score = ?1 WHERE rowid = ?2").map_err(db_err)?;
        let mut tok = tx.prepare("UPDATE tokens SET check_score = ?1 WHERE id = ?2").map_err(db_err)?;
        for &(table, id, score) in scores {
            if table == "syllables" { &mut syl } else { &mut tok }.execute(params![score, id]).map_err(db_err)?;
        }
    }
    tx.commit().map_err(db_err)
}

/// 정렬할 문장 단위 단어 목록: (단어 id, 발음형, 시작, 끝)
pub fn segments_for_align(conn: &Connection, asset_id: &str) -> Result<Vec<Vec<(i64, String, f64, f64)>>, String> {
    let rows: Vec<(i64, i64, String, f64, f64)> = conn
        .prepare("SELECT segment_id, id, pron, start, end FROM tokens WHERE asset_id = ?1 AND kind = 'word' ORDER BY segment_id, start")
        .map_err(db_err)?
        .query_map([asset_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)))
        .map_err(db_err)?
        .collect::<Result<_, _>>()
        .map_err(db_err)?;
    let mut out: Vec<Vec<(i64, String, f64, f64)>> = Vec::new();
    let mut cur_seg = None;
    for (seg, id, pron, start, end) in rows {
        if cur_seg != Some(seg) {
            out.push(Vec::new());
            cur_seg = Some(seg);
        }
        out.last_mut().unwrap().push((id, pron, start, end));
    }
    Ok(out)
}

/// 정렬 결과 저장: 단어 시간 갱신 + 음절 구간 기록
pub fn store_alignment(conn: &mut Connection, words: &[crate::align::WordOut]) -> Result<(), String> {
    let tx = conn.transaction().map_err(db_err)?;
    {
        let mut del = tx.prepare("DELETE FROM syllables WHERE token_id = ?1").map_err(db_err)?;
        let mut ins = tx
            .prepare("INSERT INTO syllables (token_id, idx, unit, start, end, v_start, v_end) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)")
            .map_err(db_err)?;
        let mut up = tx.prepare("UPDATE tokens SET start = ?1, end = ?2, aligned = 1 WHERE id = ?3").map_err(db_err)?;
        for w in words {
            if w.syllables.is_empty() {
                continue;
            }
            del.execute([w.token_id]).map_err(db_err)?;
            for (i, s) in w.syllables.iter().enumerate() {
                ins.execute(params![w.token_id, i as i64, s.unit, s.start, s.end, s.v_start, s.v_end]).map_err(db_err)?;
            }
            up.execute(params![w.start, w.end, w.token_id]).map_err(db_err)?;
        }
    }
    tx.commit().map_err(db_err)
}

/// 한 영상의 인식 결과를 통째로 교체 (재분석 시 이전 결과 삭제)
pub fn replace_asset(conn: &mut Connection, asset_id: &str, t: &Transcript) -> Result<usize, String> {
    let tx = conn.transaction().map_err(db_err)?;
    tx.execute("DELETE FROM syllables WHERE token_id IN (SELECT id FROM tokens WHERE asset_id = ?1)", [asset_id])
        .map_err(db_err)?;
    tx.execute("DELETE FROM tokens WHERE asset_id = ?1", [asset_id]).map_err(db_err)?;
    tx.execute("DELETE FROM segments WHERE asset_id = ?1", [asset_id]).map_err(db_err)?;
    let mut count = 0;
    {
        let mut ins_seg = tx
            .prepare("INSERT INTO segments (asset_id, start, end, text) VALUES (?1, ?2, ?3, ?4)")
            .map_err(db_err)?;
        let mut ins_tok = tx
            .prepare(
                "INSERT INTO tokens (asset_id, segment_id, kind, text, norm, lang, start, end, prob, pron)
                 VALUES (?1, ?2, 'word', ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            )
            .map_err(db_err)?;
        for seg in &t.segments {
            ins_seg.execute(params![asset_id, seg.start, seg.end, seg.text]).map_err(db_err)?;
            let seg_id = tx.last_insert_rowid();
            for w in &seg.words {
                let norm = normalize(&w.text);
                if norm.is_empty() {
                    continue;
                }
                ins_tok
                    .execute(params![asset_id, seg_id, w.text, norm, t.lang, w.start, w.end, w.prob, g2p::pron(&w.text)])
                    .map_err(db_err)?;
                count += 1;
            }
        }
    }
    tx.commit().map_err(db_err)?;
    Ok(count)
}

pub fn remove_asset(conn: &Connection, asset_id: &str) -> Result<(), String> {
    conn.execute("DELETE FROM syllables WHERE token_id IN (SELECT id FROM tokens WHERE asset_id = ?1)", [asset_id])
        .map_err(db_err)?;
    conn.execute("DELETE FROM tokens WHERE asset_id = ?1", [asset_id]).map_err(db_err)?;
    conn.execute("DELETE FROM segments WHERE asset_id = ?1", [asset_id]).map_err(db_err)?;
    Ok(())
}

#[derive(serde::Serialize)]
pub struct TranscriptView {
    pub segments: Vec<Segment>,
}

/// UI 표시용: 한 영상의 문장·단어 목록
pub fn transcript(conn: &Connection, asset_id: &str) -> Result<TranscriptView, String> {
    let mut seg_stmt = conn
        .prepare("SELECT id, start, end, text FROM segments WHERE asset_id = ?1 ORDER BY start")
        .map_err(db_err)?;
    let mut word_stmt = conn
        .prepare("SELECT text, start, end, prob FROM tokens WHERE segment_id = ?1 AND kind = 'word' ORDER BY start")
        .map_err(db_err)?;
    let rows: Vec<(i64, f64, f64, String)> = seg_stmt
        .query_map([asset_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
        .map_err(db_err)?
        .collect::<Result<_, _>>()
        .map_err(db_err)?;
    let mut segments = Vec::with_capacity(rows.len());
    for (id, start, end, text) in rows {
        let words = word_stmt
            .query_map([id], |r| {
                Ok(crate::stt::Word { text: r.get(0)?, start: r.get(1)?, end: r.get(2)?, prob: r.get(3)? })
            })
            .map_err(db_err)?
            .collect::<Result<_, _>>()
            .map_err(db_err)?;
        segments.push(Segment { start, end, text, words });
    }
    Ok(TranscriptView { segments })
}

/// 생성(매칭)에 쓰는 색인 단어
#[derive(Debug, Clone)]
pub struct IndexedWord {
    pub id: i64,
    pub asset_id: String,
    pub segment_id: i64,
    pub text: String,
    pub pron: String,
    pub start: f64,
    pub end: f64,
    pub prob: f32,
    /// 음소 정렬된 음절 구간 (정렬 안 된 단어는 비어 있음)
    pub syls: Vec<SylSpan>,
    /// 미리 계산한 단어 전체 재검증 점수
    pub check: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SylSpan {
    pub start: f64,
    pub end: f64,
    pub v_start: f64,
    pub v_end: f64,
    /// 미리 계산한 재검증 점수 (이 음절 하나만 잘라 들었을 때)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub check: Option<f32>,
}

/// 전체 단어를 영상·문장·시간 순으로 (같은 문장 안 연속 단어 = 구 매칭에 사용)
pub fn load_words(conn: &Connection) -> Result<Vec<IndexedWord>, String> {
    let mut syls: HashMap<i64, Vec<SylSpan>> = HashMap::new();
    let mut st = conn
        .prepare("SELECT token_id, start, end, v_start, v_end, check_score FROM syllables ORDER BY token_id, idx")
        .map_err(db_err)?;
    let rows = st
        .query_map([], |r| {
            Ok((r.get::<_, i64>(0)?, SylSpan { start: r.get(1)?, end: r.get(2)?, v_start: r.get(3)?, v_end: r.get(4)?, check: r.get(5)? }))
        })
        .map_err(db_err)?;
    for row in rows {
        let (id, span) = row.map_err(db_err)?;
        syls.entry(id).or_default().push(span);
    }

    conn.prepare(
        "SELECT id, asset_id, segment_id, text, pron, start, end, prob, check_score FROM tokens
         WHERE kind = 'word' AND pron <> '' ORDER BY asset_id, segment_id, start",
    )
    .map_err(db_err)?
    .query_map([], |r| {
        Ok(IndexedWord {
            id: r.get(0)?,
            asset_id: r.get(1)?,
            segment_id: r.get(2)?,
            text: r.get(3)?,
            pron: r.get(4)?,
            start: r.get(5)?,
            end: r.get(6)?,
            prob: r.get(7)?,
            check: r.get(8)?,
            syls: Vec::new(),
        })
    })
    .map_err(db_err)?
    .map(|w| {
        w.map(|mut w| {
            w.syls = syls.remove(&w.id).unwrap_or_default();
            w
        })
    })
    .collect::<Result<_, _>>()
    .map_err(db_err)
}

/// 매칭용 정규화: 소문자, 문장부호·공백 제거
pub fn normalize(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric() || *c == 'ー')
        .flat_map(|c| c.to_lowercase())
        .collect()
}

fn db_err(e: rusqlite::Error) -> String {
    tr!("db_error", e)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stt::Word;

    #[test]
    fn normalizes_for_matching() {
        assert_eq!(normalize("Hello,"), "hello");
        assert_eq!(normalize("안녕하세요!"), "안녕하세요");
        assert_eq!(normalize("ラーメン。"), "ラーメン");
    }

    #[test]
    fn stores_and_replaces_transcript() {
        let dir = std::env::temp_dir().join(format!("dcinvox-idx-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut conn = open(&dir).unwrap();
        let w = |t: &str, s: f64| Word { text: t.into(), start: s, end: s + 0.3, prob: 0.9 };
        let t = Transcript {
            lang: "ko".into(),
            segments: vec![Segment { start: 0.0, end: 1.0, text: "안녕 여러분".into(), words: vec![w("안녕", 0.0), w("여러분", 0.4)] }],
        };
        assert_eq!(replace_asset(&mut conn, "a0001", &t).unwrap(), 2);
        assert_eq!(replace_asset(&mut conn, "a0001", &t).unwrap(), 2); // 재분석해도 중복 없음
        let view = transcript(&conn, "a0001").unwrap();
        assert_eq!(view.segments.len(), 1);
        assert_eq!(view.segments[0].words.len(), 2);
        drop(conn);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
