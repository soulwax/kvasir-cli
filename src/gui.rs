use std::time::{Duration, Instant};

use tao::event::{Event, StartCause, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoop};
use tao::platform::run_return::EventLoopExtRunReturn;
use tao::window::WindowBuilder;
use wry::http::Request;
use wry::{WebContext, WebViewBuilder};

use crate::config::{self, Config};
use crate::jobs;

const APP_HTML: &str = include_str!("gui.html");
const BOOT: &str = r#"
window.kvasir = window.kvasir || {};
window.kvasir.send = (msg) => window.ipc.postMessage(JSON.stringify(msg));
"#;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Kvasir")
        .with_inner_size(tao::dpi::LogicalSize::new(980.0, 760.0))
        .build(&event_loop)?;
    let login_window = WindowBuilder::new()
        .with_title("Sign in to Deezer")
        .with_inner_size(tao::dpi::LogicalSize::new(1100.0, 820.0))
        .with_visible(false)
        .build(&event_loop)?;

    let (tx, rx) = std::sync::mpsc::channel::<String>();
    let ipc_tx = tx.clone();
    let mut context = web_context();
    let webview = WebViewBuilder::new_with_web_context(&mut context)
        .with_html(APP_HTML)
        .with_initialization_script(BOOT)
        .with_ipc_handler(move |request: Request<String>| {
            let _ = ipc_tx.send(request.into_body());
        })
        .build(&window)?;
    let login_view = WebViewBuilder::new_with_web_context(&mut context)
        .with_url("https://www.deezer.com/login")
        .build(&login_window)?;

    let main_id = window.id();
    let login_id = login_window.id();
    let mut signing_in = false;
    let mut config = Config::load();

    event_loop.run_return(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(400));
        if matches!(event, Event::NewEvents(StartCause::Init)) {
            emit(&webview, &state_message(&config));
        }
        while let Ok(body) = rx.try_recv() {
            dispatch(&webview, &login_window, &login_view, &tx, &mut config, &mut signing_in, &body);
        }
        if signing_in {
            if let Some(arl) = read_arl(&login_view) {
                config.arl = arl;
                let _ = config.save();
                signing_in = false;
                login_window.set_visible(false);
                emit(&webview, &state_message(&config));
                emit(
                    &webview,
                    &serde_json::json!({"op":"status","level":"ok","text":"Signed in. You can search or paste a link."}),
                );
            }
        }
        if let Event::WindowEvent { window_id, event, .. } = event {
            if let WindowEvent::CloseRequested = event {
                if window_id == login_id {
                    signing_in = false;
                    login_window.set_visible(false);
                } else if window_id == main_id {
                    *control_flow = ControlFlow::Exit;
                }
            }
        }
    });
    Ok(())
}

pub fn login() -> Result<(), Box<dyn std::error::Error>> {
    let mut event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Sign in to Deezer")
        .with_inner_size(tao::dpi::LogicalSize::new(1100.0, 820.0))
        .build(&event_loop)?;
    let mut context = web_context();
    let webview = WebViewBuilder::new_with_web_context(&mut context)
        .with_url("https://www.deezer.com/login")
        .build(&window)?;
    let window_id = window.id();
    let saved = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let saved_flag = saved.clone();
    event_loop.run_return(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(400));
        if let Some(arl) = read_arl(&webview) {
            let mut config = Config::load();
            config.arl = arl;
            match config.save() {
                Ok(path) => {
                    println!("Saved Deezer login to {}", path.display());
                    saved_flag.store(true, std::sync::atomic::Ordering::Relaxed);
                    *control_flow = ControlFlow::Exit;
                }
                Err(err) => eprintln!("Could not save login: {err}"),
            }
        }
        if let Event::WindowEvent { window_id: id, event: WindowEvent::CloseRequested, .. } = event {
            if id == window_id {
                *control_flow = ControlFlow::Exit;
            }
        }
    });
    if saved.load(std::sync::atomic::Ordering::Relaxed) {
        Ok(())
    } else {
        Err("The sign-in window closed before Deezer stored a session.".into())
    }
}

fn web_context() -> WebContext {
    let dir = config::config_dir().join("webview");
    let _ = std::fs::create_dir_all(&dir);
    WebContext::new(Some(dir))
}

fn dispatch(
    webview: &wry::WebView,
    login_window: &tao::window::Window,
    login_view: &wry::WebView,
    tx: &std::sync::mpsc::Sender<String>,
    config: &mut Config,
    signing_in: &mut bool,
    body: &str,
) {
    let op = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| value.get("op").and_then(|value| value.as_str()).map(str::to_string));
    if matches!(op.as_deref(), Some("results" | "downloaded" | "status" | "busy")) {
        if let Ok(message) = serde_json::from_str::<serde_json::Value>(body) {
            emit(webview, &message);
        }
        return;
    }
    handle_ipc(webview, login_window, login_view, tx, config, signing_in, body);
}

fn handle_ipc(
    webview: &wry::WebView,
    login_window: &tao::window::Window,
    login_view: &wry::WebView,
    tx: &std::sync::mpsc::Sender<String>,
    config: &mut Config,
    signing_in: &mut bool,
    body: &str,
) {
    let Ok(message) = serde_json::from_str::<serde_json::Value>(body) else {
        return;
    };
    match message.get("op").and_then(|value| value.as_str()).unwrap_or("") {
        "ready" | "refresh" => emit(webview, &state_message(config)),
        "login" => {
            *signing_in = true;
            let _ = login_view.load_url("https://www.deezer.com/login");
            login_window.set_visible(true);
            login_window.set_focus();
            emit(
                webview,
                &serde_json::json!({"op":"status","level":"info","text":"Sign in in the Deezer window. Kvasir reads the session cookie itself."}),
            );
        }
        "logout" => {
            config.arl.clear();
            let _ = config.save();
            emit(webview, &state_message(config));
        }
        "reset" => match config::reset() {
            Ok(dir) => {
                *config = Config::default();
                emit(webview, &state_message(config));
                emit(
                    webview,
                    &serde_json::json!({"op":"status","level":"ok","text":format!("Removed {}", dir.display())}),
                );
            }
            Err(err) => emit(
                webview,
                &serde_json::json!({"op":"status","level":"err","text":err.to_string()}),
            ),
        },
        "browse" => {
            let mut dialog = rfd::FileDialog::new();
            if config.download_dir.is_dir() {
                dialog = dialog.set_directory(&config.download_dir);
            }
            if let Some(path) = dialog.pick_folder() {
                config.download_dir = path;
                let _ = config.save();
                emit(webview, &state_message(config));
            }
        }
        "settings" => {
            if let Some(quality) = message.get("quality").and_then(|value| value.as_str()) {
                config.quality = quality.to_string();
            }
            if let Some(dir) = message.get("download_dir").and_then(|value| value.as_str()) {
                if !dir.trim().is_empty() {
                    config.download_dir = dir.trim().into();
                }
            }
            let _ = config.save();
            emit(webview, &state_message(config));
        }
        "go" => {
            let query = message.get("query").and_then(|value| value.as_str()).unwrap_or("").trim().to_string();
            if query.is_empty() {
                return;
            }
            let quality = config.quality.clone();
            let directory = config.download_dir.clone();
            let direct = jobs::looks_like_target(&query);
            emit(webview, &serde_json::json!({"op":"busy","on":true}));
            let tx = tx.clone();
            std::thread::spawn(move || {
                let outcome = jobs::runtime().block_on(async {
                    let session = jobs::open_session().await?;
                    if direct {
                        let paths = jobs::download(&session, &query, &quality, &directory).await?;
                        Ok::<_, String>(serde_json::json!({
                            "op": "downloaded",
                            "paths": paths.iter().map(|path| path.display().to_string()).collect::<Vec<_>>()
                        }))
                    } else {
                        let hits = jobs::search(&session, &query).await?;
                        Ok(serde_json::json!({"op":"results","tracks":hits}))
                    }
                });
                report(&tx, outcome);
            });
        }
        "download" => {
            let id = message.get("id").and_then(|value| value.as_str()).unwrap_or("").to_string();
            if id.is_empty() {
                return;
            }
            let quality = config.quality.clone();
            let directory = config.download_dir.clone();
            emit(webview, &serde_json::json!({"op":"busy","on":true}));
            let tx = tx.clone();
            std::thread::spawn(move || {
                let outcome = jobs::runtime().block_on(async {
                    let session = jobs::open_session().await?;
                    let paths = jobs::download(&session, &id, &quality, &directory).await?;
                    Ok(serde_json::json!({
                        "op": "downloaded",
                        "paths": paths.iter().map(|path| path.display().to_string()).collect::<Vec<_>>()
                    }))
                });
                report(&tx, outcome);
            });
        }
        _ => {}
    }
}

fn report(tx: &std::sync::mpsc::Sender<String>, outcome: Result<serde_json::Value, String>) {
    let message = match outcome {
        Ok(message) => message,
        Err(err) => serde_json::json!({"op":"status","level":"err","text":err}),
    };
    if let Ok(text) = serde_json::to_string(&message) {
        let _ = tx.send(text);
    }
    let _ = tx.send(r#"{"op":"busy","on":false}"#.to_string());
}

fn state_message(config: &Config) -> serde_json::Value {
    let name = if config.signed_in() {
        jobs::runtime().block_on(async {
            let session = jobs::open_session().await.ok()?;
            let user = session.get_user().await.ok()?;
            Some(jobs::account_name(&user))
        })
    } else {
        None
    };
    serde_json::json!({
        "op": "state",
        "signed_in": config.signed_in(),
        "name": name,
        "quality": config.quality,
        "download_dir": config.download_dir.display().to_string(),
        "config_dir": config::config_dir().display().to_string(),
    })
}

fn read_arl(webview: &wry::WebView) -> Option<String> {
    let cookies = webview.cookies_for_url("https://www.deezer.com").ok()?;
    cookies.into_iter().find_map(|cookie| {
        if cookie.name() == "arl" && cookie.value().len() == 192 {
            Some(cookie.value().to_string())
        } else {
            None
        }
    })
}

fn emit(webview: &wry::WebView, message: &serde_json::Value) {
    let payload = serde_json::to_string(message).unwrap_or_else(|_| "{}".into());
    let _ = webview.evaluate_script(&format!("window.kvasir && window.kvasir.recv && window.kvasir.recv({payload})"));
}
