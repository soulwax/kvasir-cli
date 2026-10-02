use std::io::IsTerminal;

pub fn color_enabled() -> bool {
    std::env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal()
}

fn paint(text: &str, code: &str) -> String {
    if color_enabled() {
        format!("\u{1b}[{code}m{text}\u{1b}[0m")
    } else {
        text.to_string()
    }
}

pub fn info(message: &str) -> String {
    format!("{}{message}", paint("ℹ info ", "94"))
}

pub fn warn(message: &str) -> String {
    format!("{}{message}", paint("⚠ warn ", "93"))
}

pub fn pending(message: &str) -> String {
    format!("{}{message}", paint("● pending ", "94"))
}

pub fn success(message: &str) -> String {
    format!("{}{message}", paint("✔ success ", "92"))
}

pub fn error(message: &str) -> String {
    format!("{}{message}", paint("✖ error ", "91"))
}

pub fn note(message: &str) -> String {
    paint(&format!("  → {message}"), "90")
}

pub fn banner() -> String {
    let lines = [
        "             ♥ kvasir ♥             ",
        " ────────────────────────────────── ",
        " │ github.com/soulwax/kvasir-cli │ ",
        " ────────────────────────────────── ",
    ];
    let codes = ["91", "93", "38;5;208"];
    lines
        .iter()
        .enumerate()
        .map(|(index, line)| {
            if color_enabled() {
                paint(line, codes[index % codes.len()])
            } else {
                (*line).to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn format_seconds(seconds: u64) -> String {
    if seconds < 60 {
        return format!("{seconds}s");
    }
    let minutes = seconds / 60;
    let rest = seconds % 60;
    format!("{minutes:02}m {rest:02}s")
}
