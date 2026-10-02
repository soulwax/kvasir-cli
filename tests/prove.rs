use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

fn env_file() -> HashMap<String, String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".env");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (key, value) = line.split_once('=')?;
            Some((key.trim().to_string(), value.trim().to_string()))
        })
        .collect()
}

fn run(args: &[&str]) -> std::process::Output {
    let env = env_file();
    assert!(env.contains_key("DEEZER_ARL"), "DEEZER_ARL is missing from .env");
    let bin = env!("CARGO_BIN_EXE_kvasir-cli");
    let output = Command::new(bin)
        .args(args)
        .envs(&env)
        .output()
        .unwrap_or_else(|err| panic!("spawn {bin}: {err}"));
    if !output.status.success() {
        panic!(
            "kvasir-cli {} failed ({})\n{}",
            args.join(" "),
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    output
}

#[test]
fn whoami_search_resolve_and_acquire() {
    let who = run(&["whoami"]);
    let who_text = String::from_utf8_lossy(&who.stdout);
    assert!(who_text.starts_with("user "), "{who_text}");

    let found = run(&["search", "daft punk"]);
    let found_text = String::from_utf8_lossy(&found.stdout);
    assert!(found_text.lines().any(|line| line.contains('\t')), "{found_text}");

    let resolved = run(&["resolve", "https://www.deezer.com/track/3135556"]);
    let resolved_text = String::from_utf8_lossy(&resolved.stdout);
    assert!(resolved_text.contains("3135556"), "{resolved_text}");

    let tidal = run(&["resolve", "https://tidal.com/browse/track/64975224"]);
    let tidal_text = String::from_utf8_lossy(&tidal.stdout);
    assert!(tidal_text.contains("tidal-track"), "{tidal_text}");

    let out = std::env::temp_dir().join("kvasir-cli-3135556.mp3");
    let acquired = run(&[
        "acquire",
        "3135556",
        "--quality",
        "1",
        "--out",
        out.to_str().unwrap(),
    ]);
    let acquired_text = String::from_utf8_lossy(&acquired.stdout);
    assert!(acquired_text.contains("mp3"), "{acquired_text}");
    assert!(acquired_text.contains("tagged=true"), "{acquired_text}");
    let bytes = std::fs::read(&out).expect("acquired file");
    assert!(bytes.starts_with(b"ID3") || bytes.starts_with(b"fLaC"));
    let _ = std::fs::remove_file(&out);
}
