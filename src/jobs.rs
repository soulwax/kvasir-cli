use std::path::{Path, PathBuf};

use kvasir_core::{acquire, create_session, parse_info, Quality, Session};

use crate::config::Config;

pub fn runtime() -> &'static tokio::runtime::Runtime {
    static RUNTIME: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();
    RUNTIME.get_or_init(|| tokio::runtime::Runtime::new().expect("tokio runtime"))
}

pub fn resolve_arl() -> Result<String, String> {
    if let Ok(value) = std::env::var("DEEZER_ARL") {
        let value = value.trim().to_string();
        if !value.is_empty() {
            return Ok(value);
        }
    }
    let config = Config::load();
    if config.signed_in() {
        return Ok(config.arl.trim().to_string());
    }
    Err(format!(
        "Sign in from the window, or run `{} login`. Settings are kept in {}.",
        env!("CARGO_BIN_NAME"),
        crate::config::config_dir().display()
    ))
}

pub async fn open_session() -> Result<Session, String> {
    let arl = resolve_arl()?;
    create_session(Some(arl.trim()))
        .await
        .map_err(|err| err.to_string())
}

pub fn account_name(user: &serde_json::Value) -> String {
    match user.get("BLOG_NAME").and_then(|value| value.as_str()) {
        Some(name) if !name.is_empty() => name.to_string(),
        _ => "your Deezer account".into(),
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct Hit {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: String,
}

pub async fn search(session: &Session, query: &str) -> Result<Vec<Hit>, String> {
    let result = session
        .search_music(query, &["TRACK"], 12)
        .await
        .map_err(|err| err.to_string())?;
    let tracks = result
        .pointer("/TRACK/data")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    Ok(tracks
        .into_iter()
        .map(|track| {
            let seconds = json_id(track.get("DURATION")).parse::<u64>().unwrap_or(0);
            Hit {
                id: json_id(track.get("SNG_ID")),
                title: text(&track, "SNG_TITLE"),
                artist: text(&track, "ART_NAME"),
                album: text(&track, "ALB_TITLE"),
                duration: crate::style::format_seconds(seconds),
            }
        })
        .filter(|hit| !hit.id.is_empty())
        .collect())
}

pub fn looks_like_target(value: &str) -> bool {
    let value = value.trim();
    if value.is_empty() {
        return false;
    }
    value.chars().all(|ch| ch.is_ascii_digit())
        || value.contains("://")
        || value.contains("deezer.")
        || value.contains("spotify")
        || value.contains("tidal.")
        || value.contains("youtu")
}

pub async fn download(session: &Session, target: &str, quality: &str, directory: &Path) -> Result<Vec<PathBuf>, String> {
    std::fs::create_dir_all(directory).map_err(|err| err.to_string())?;
    let quality = parse_quality(quality)?;
    let ids = if target.trim().chars().all(|ch| ch.is_ascii_digit()) {
        vec![(target.trim().to_string(), String::new(), String::new())]
    } else {
        let parsed = parse_info(session, target.trim()).await.map_err(|err| err.to_string())?;
        if parsed.tracks.is_empty() {
            return Err("That link did not resolve to any tracks.".into());
        }
        parsed
            .tracks
            .iter()
            .take(100)
            .map(|track| (track.sng_id(), track.artist_name(), track.title()))
            .collect()
    };
    let mut written = Vec::new();
    for (id, artist, title) in ids {
        let acquired = acquire(session, &id, quality.clone()).await.map_err(|err| err.to_string())?;
        let artist = if artist.is_empty() {
            acquired.catalogue.artists.join(", ")
        } else {
            artist
        };
        let title = if title.is_empty() { acquired.catalogue.title.clone() } else { title };
        let ext = match acquired.analysis.format.id.as_str() {
            "flac" => "flac",
            _ => "mp3",
        };
        let path = directory.join(format!("{} [{id}].{ext}", file_stem(&artist, &title)));
        std::fs::write(&path, &acquired.bytes).map_err(|err| err.to_string())?;
        written.push(path);
    }
    Ok(written)
}

pub fn parse_quality(value: &str) -> Result<Quality, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "128" | "mp3" | "mp3_128" => return Ok(Quality::Mp3_128),
        "320" | "mp3_320" => return Ok(Quality::Mp3_320),
        "flac" | "lossless" => return Ok(Quality::Flac),
        _ => {}
    }
    if let Ok(code) = value.parse::<i64>() {
        return Quality::from_code(code).map_err(|err| err.to_string());
    }
    Ok(Quality::parse(value))
}

fn file_stem(artist: &str, title: &str) -> String {
    let raw = if artist.is_empty() {
        title.to_string()
    } else {
        format!("{artist} - {title}")
    };
    let mut cleaned = String::new();
    for ch in raw.chars() {
        if ch.is_control() || matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') {
            cleaned.push(' ');
        } else {
            cleaned.push(ch);
        }
    }
    let collapsed = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let collapsed = collapsed.trim().trim_matches('.').trim();
    let stem: String = if collapsed.is_empty() { "track".into() } else { collapsed.chars().take(80).collect() };
    stem
}

fn text(value: &serde_json::Value, key: &str) -> String {
    value.get(key).and_then(|item| item.as_str()).unwrap_or("Unknown").to_string()
}

fn json_id(value: Option<&serde_json::Value>) -> String {
    match value {
        Some(serde_json::Value::String(text)) => text.clone(),
        Some(serde_json::Value::Number(number)) => number.to_string(),
        _ => String::new(),
    }
}
