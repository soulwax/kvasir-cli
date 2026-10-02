use std::io::{self, IsTerminal, Write};
use std::thread;
use std::time::{Duration, Instant};

use crate::config::Config;
use crate::style::{note, pending, success};

const WAIT: Duration = Duration::from_secs(180);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Service {
    Deezer,
    Tidal,
}

pub fn parse_service(value: &str) -> Option<Service> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "deezer" | "dz" => Some(Service::Deezer),
        "2" | "tidal" => Some(Service::Tidal),
        _ => None,
    }
}

pub fn choose_service(given: Option<&str>) -> Result<Service, String> {
    if let Some(value) = given {
        return parse_service(value).ok_or_else(|| "Choose deezer or tidal.".into());
    }
    if !io::stdin().is_terminal() {
        return Err("Pass deezer or tidal, for example `kvasir login deezer`.".into());
    }
    println!("Sign in with:");
    println!("  1  Deezer");
    println!("  2  Tidal");
    print!("Choose 1 or 2: ");
    io::stdout().flush().map_err(|err| err.to_string())?;
    let mut line = String::new();
    io::stdin().read_line(&mut line).map_err(|err| err.to_string())?;
    parse_service(&line).ok_or_else(|| "Choose deezer or tidal.".into())
}

pub fn login(service: Service) -> Result<(), String> {
    let (url, label) = match service {
        Service::Deezer => ("https://www.deezer.com/login", "Deezer"),
        Service::Tidal => ("https://login.tidal.com/", "Tidal"),
    };
    println!("{}", pending(&format!("opening {label} in your browser")));
    open::that(url).map_err(|err| format!("Could not open the browser: {err}"))?;
    println!(
        "{}",
        note("Finish signing in on that tab. This terminal continues when the session shows up.")
    );
    let started = Instant::now();
    loop {
        if let Some(found) = find_session(service) {
            let mut config = Config::load();
            let path = match service {
                Service::Deezer => {
                    config.arl = found;
                    config.save().map_err(|err| err.to_string())?
                }
                Service::Tidal => {
                    config.tidal_access_token = found;
                    config.save().map_err(|err| err.to_string())?
                }
            };
            println!("{}", success(&format!("Saved the {label} session to {}", path.display())));
            return Ok(());
        }
        if started.elapsed() > WAIT {
            return Err(format!(
                "No {label} session appeared in Chrome, Edge, Brave, or Firefox. Sign in on the tab that opened, then run login again."
            ));
        }
        thread::sleep(Duration::from_secs(1));
    }
}

fn find_session(service: Service) -> Option<String> {
    let (domain, accept) = match service {
        Service::Deezer => ("deezer.com", deezer_arl as fn(&str, &str) -> Option<String>),
        Service::Tidal => ("tidal.com", tidal_token as fn(&str, &str) -> Option<String>),
    };
    for (name, value) in browser_cookies(domain) {
        if let Some(secret) = accept(&name, &value) {
            return Some(secret);
        }
    }
    None
}

fn deezer_arl(name: &str, value: &str) -> Option<String> {
    (name == "arl" && value.len() == 192).then(|| value.to_string())
}

fn tidal_token(name: &str, value: &str) -> Option<String> {
    let name = name.to_ascii_lowercase();
    let useful = name.contains("token") || name.contains("session") || name == "st";
    (useful && value.len() >= 20 && !value.contains(' ')).then(|| value.to_string())
}

fn browser_cookies(domain: &str) -> Vec<(String, String)> {
    let domains = Some(vec![domain.to_string()]);
    let mut found = Vec::new();
    for cookies in [
        rookie::chrome(domains.clone()),
        rookie::edge(domains.clone()),
        rookie::brave(domains.clone()),
        rookie::chromium(domains.clone()),
        rookie::firefox(domains.clone()),
    ] {
        let Ok(cookies) = cookies else { continue };
        for cookie in cookies {
            found.push((cookie.name, cookie.value));
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_names() {
        assert_eq!(parse_service("1"), Some(Service::Deezer));
        assert_eq!(parse_service("Deezer"), Some(Service::Deezer));
        assert_eq!(parse_service("2"), Some(Service::Tidal));
        assert_eq!(parse_service("tidal"), Some(Service::Tidal));
        assert_eq!(parse_service("spotify"), None);
    }

    #[test]
    fn cookie_filters() {
        assert!(deezer_arl("arl", &"a".repeat(192)).is_some());
        assert!(deezer_arl("arl", "short").is_none());
        assert!(tidal_token("access_token", &"x".repeat(20)).is_some());
        assert!(tidal_token("locale", "en").is_none());
    }
}
