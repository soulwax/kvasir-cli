mod config;
mod gui;
mod jobs;
mod style;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use kvasir_core::{acquire, parse_info, Session};
use style::{banner, error, format_seconds, info, note, pending, success, warn};

#[derive(Parser)]
#[command(
    name = env!("CARGO_BIN_NAME"),
    about = "Look up, resolve, and fetch music through kvasir.",
    after_help = "With no command, a window opens. Sign in there once; the session is stored in the user config folder. `reset` deletes that folder. Quality is 128, 320, or flac."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Open the window.
    Gui,
    /// Open Deezer and save the session cookie.
    Login,
    /// Delete the saved login and settings so the next launch starts fresh.
    Reset,
    /// Show the signed-in Deezer account.
    Whoami,
    /// Search the Deezer gateway.
    Search { query: String },
    /// Resolve a Deezer, Spotify, Tidal, or YouTube URL to Deezer tracks.
    Resolve { url: String },
    /// Download, decrypt, inspect, and tag one Deezer track.
    Acquire {
        track_id: String,
        /// 128, 320, flac, or the numeric shorthand 1, 3, 9.
        #[arg(long, default_value = "128")]
        quality: String,
        /// Write the tagged audio here. Omit to print a summary only.
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

fn main() -> ExitCode {
    match Cli::parse().command {
        None | Some(Command::Gui) => match gui::run() {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => fail(&err.to_string()),
        },
        Some(Command::Login) => {
            println!("{}", banner());
            println!("{}", pending("opening Deezer so the session cookie can be read"));
            match gui::login() {
                Ok(()) => ExitCode::SUCCESS,
                Err(err) => fail(&err.to_string()),
            }
        }
        Some(Command::Reset) => match config::reset() {
            Ok(dir) => {
                println!("{}", success(&format!("Removed {}", dir.display())));
                println!("{}", note("The next launch starts signed out."));
                ExitCode::SUCCESS
            }
            Err(err) => fail(&err.to_string()),
        },
        Some(command) => {
            println!("{}", banner());
            jobs::runtime().block_on(async {
                match run(command).await {
                    Ok(()) => ExitCode::SUCCESS,
                    Err(err) => fail(&err.to_string()),
                }
            })
        }
    }
}

fn fail(message: &str) -> ExitCode {
    eprintln!("{}", error(message));
    if message.contains("Sign in") {
        eprintln!(
            "{}",
            note(&format!("Saved settings live in {}.", config::config_dir().display()))
        );
    }
    ExitCode::from(1)
}

async fn run(command: Command) -> Result<(), Box<dyn std::error::Error>> {
    let session = open_session().await?;
    match command {
        Command::Whoami => whoami(&session).await?,
        Command::Search { query } => search(&session, &query).await?,
        Command::Resolve { url } => resolve(&session, &url).await?,
        Command::Acquire { track_id, quality, out } => acquire_track(&session, &track_id, &quality, out).await?,
        Command::Gui | Command::Login | Command::Reset => {}
    }
    Ok(())
}

async fn open_session() -> Result<Session, Box<dyn std::error::Error>> {
    println!("{}", pending("opening a Deezer session"));
    Ok(jobs::open_session().await?)
}

async fn whoami(session: &Session) -> Result<(), Box<dyn std::error::Error>> {
    let user = session.get_user().await?;
    let name = field(&user, "BLOG_NAME");
    let shown = if name.is_empty() { "your account".into() } else { name };
    println!("{}", success(&format!("Logged in as {shown}")));
    println!(
        "{}",
        note(&format!("id {} · {}", field(&user, "USER_ID"), field(&user, "COUNTRY")))
    );
    Ok(())
}

async fn search(session: &Session, query: &str) -> Result<(), Box<dyn std::error::Error>> {
    let result = session.search_music(query, &["TRACK"], 5).await?;
    let tracks = result.pointer("/TRACK/data").and_then(|value| value.as_array()).cloned().unwrap_or_default();
    if tracks.is_empty() {
        println!("{}", warn("Nothing to show."));
        return Ok(());
    }
    println!("{}", success(&format!("{} track(s)", tracks.len())));
    for track in tracks {
        let title = track.get("SNG_TITLE").and_then(|value| value.as_str()).unwrap_or("Unknown");
        let artist = track.get("ART_NAME").and_then(|value| value.as_str()).unwrap_or("Unknown");
        let album = track.get("ALB_TITLE").and_then(|value| value.as_str()).unwrap_or("Unknown");
        let seconds = json_id(track.get("DURATION")).parse::<u64>().unwrap_or(0);
        println!("  {title} — {artist}");
        println!("{}", note(&format!("Album: {album} · {}", format_seconds(seconds))));
    }
    Ok(())
}

async fn resolve(session: &Session, url: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", pending(&format!("resolving {url}")));
    let parsed = parse_info(session, url).await?;
    let count = parsed.tracks.len();
    let label = if count == 1 { "track" } else { "tracks" };
    println!("{}", info(&format!("{} {}", parsed.info.kind, parsed.info.id)));
    println!("{}", success(&format!("{count} {label}")));
    for track in parsed.tracks.iter().take(8) {
        println!("  {} · {} — {}", track.sng_id(), track.artist_name(), track.title());
    }
    if count > 8 {
        println!("{}", note(&format!("{} more", count - 8)));
    }
    Ok(())
}

async fn acquire_track(
    session: &Session,
    track_id: &str,
    quality: &str,
    out: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let quality = jobs::parse_quality(quality)?;
    println!("{}", pending(&format!("fetching {track_id} at {}", quality.format_name())));
    let acquired = acquire(session, track_id, quality).await?;
    let tagged = if acquired.tagged { "tagged" } else { "untagged" };
    println!(
        "{}",
        success(&format!(
            "{} — {} · {} · {tagged}",
            acquired.catalogue.title,
            acquired.analysis.format.id.as_str(),
            bytes_label(acquired.bytes.len())
        ))
    );
    if acquired.issues.is_empty() {
        println!("{}", note("catalogue and bytes agree"));
    } else {
        println!("{}", warn(&format!("{} disagreement(s)", acquired.issues.len())));
        for issue in &acquired.issues {
            println!("{}", note(&format!("{issue:?}")));
        }
    }
    if let Some(path) = out {
        std::fs::write(&path, &acquired.bytes)?;
        println!("{}", success(&format!("wrote {}", path.display())));
    }
    Ok(())
}

fn bytes_label(len: usize) -> String {
    if len >= 1024 * 1024 {
        format!("{:.1} MB", len as f64 / (1024.0 * 1024.0))
    } else if len >= 1024 {
        format!("{} KB", len / 1024)
    } else {
        format!("{len} bytes")
    }
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
