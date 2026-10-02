use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use kvasir_core::{acquire, create_session, parse_info, Quality, Session};

#[derive(Parser)]
#[command(name = "kvasir-cli", about = "Exercise kvasir: account, search, resolve, and acquire.")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Show the Deezer account behind DEEZER_ARL.
    Whoami,
    /// Search the Deezer gateway.
    Search { query: String },
    /// Resolve a Deezer, Spotify, Tidal, or YouTube URL to Deezer tracks.
    Resolve { url: String },
    /// Download, decrypt, inspect, and tag one Deezer track.
    Acquire {
        track_id: String,
        /// 1, 3, 9, MP3_128, MP3_320, or FLAC.
        #[arg(long, default_value = "1")]
        quality: String,
        /// Write the tagged audio here. Omit to print a summary only.
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    match run(Cli::parse()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("kvasir-cli: {err}");
            ExitCode::from(1)
        }
    }
}

async fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    let session = open_session().await?;
    match cli.command {
        Command::Whoami => whoami(&session).await?,
        Command::Search { query } => search(&session, &query).await?,
        Command::Resolve { url } => resolve(&session, &url).await?,
        Command::Acquire { track_id, quality, out } => acquire_track(&session, &track_id, &quality, out).await?,
    }
    Ok(())
}

async fn open_session() -> Result<Session, Box<dyn std::error::Error>> {
    let arl = std::env::var("DEEZER_ARL").map_err(|_| "set DEEZER_ARL to a 192-character Deezer arl cookie")?;
    Ok(create_session(Some(arl.trim())).await?)
}

async fn whoami(session: &Session) -> Result<(), Box<dyn std::error::Error>> {
    let user = get_user_on(session).await?;
    println!(
        "user {} · {} · {}",
        field(&user, "USER_ID"),
        field(&user, "BLOG_NAME"),
        field(&user, "COUNTRY")
    );
    Ok(())
}

async fn search(session: &Session, query: &str) -> Result<(), Box<dyn std::error::Error>> {
    let result = session.search_music(query, &["TRACK"], 5).await?;
    let tracks = result.pointer("/TRACK/data").and_then(|value| value.as_array());
    let Some(tracks) = tracks else {
        println!("no tracks");
        return Ok(());
    };
    for track in tracks {
        let title = track.get("SNG_TITLE").and_then(|value| value.as_str()).unwrap_or("");
        let artist = track.get("ART_NAME").and_then(|value| value.as_str()).unwrap_or("");
        let id = json_id(track.get("SNG_ID"));
        println!("{id}\t{artist} — {title}");
    }
    Ok(())
}

async fn resolve(session: &Session, url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let parsed = parse_info(session, url).await?;
    println!("{} {} · {} tracks", parsed.info.kind, parsed.info.id, parsed.tracks.len());
    if let Some(track) = parsed.tracks.first() {
        println!("{} · {} · {}", track.sng_id(), track.artist_name(), track.title());
    }
    Ok(())
}

async fn acquire_track(
    session: &Session,
    track_id: &str,
    quality: &str,
    out: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let quality = parse_quality(quality)?;
    let acquired = acquire(session, track_id, quality).await?;
    println!(
        "{} · {} · {} bytes · tagged={}",
        acquired.catalogue.title,
        acquired.analysis.format.id.as_str(),
        acquired.bytes.len(),
        acquired.tagged
    );
    if !acquired.issues.is_empty() {
        println!("{} issue(s)", acquired.issues.len());
        for issue in &acquired.issues {
            println!("- {issue:?}");
        }
    }
    if let Some(path) = out {
        std::fs::write(&path, &acquired.bytes)?;
        println!("wrote {}", path.display());
    }
    Ok(())
}

fn parse_quality(value: &str) -> Result<Quality, Box<dyn std::error::Error>> {
    if let Ok(code) = value.parse::<i64>() {
        return Ok(Quality::from_code(code)?);
    }
    Ok(Quality::parse(value))
}

async fn get_user_on(session: &Session) -> Result<serde_json::Value, kvasir_core::DeezerError> {
    session.get_user().await
}

fn field(value: &serde_json::Value, key: &str) -> String {
    match value.get(key) {
        Some(serde_json::Value::String(text)) => text.clone(),
        Some(serde_json::Value::Number(number)) => number.to_string(),
        _ => String::new(),
    }
}

fn json_id(value: Option<&serde_json::Value>) -> String {
    match value {
        Some(serde_json::Value::String(text)) => text.clone(),
        Some(serde_json::Value::Number(number)) => number.to_string(),
        _ => String::new(),
    }
}
