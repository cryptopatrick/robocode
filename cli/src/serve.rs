//! Serves the battle viewer and one recording on 127.0.0.1.

use axum::Router;
use axum::extract::Path;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;

use crate::Failure;

mod assets {
    include!(concat!(env!("OUT_DIR"), "/viewer_assets.rs"));
}

fn content_type(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "wasm" => "application/wasm",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        _ => "application/octet-stream",
    }
}

fn asset(path: &str) -> Response {
    match assets::VIEWER_ASSETS.iter().find(|(p, _)| *p == path) {
        Some((p, bytes)) => ([(header::CONTENT_TYPE, content_type(p))], *bytes).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Serves until Ctrl+C. Opens the browser unless `no_open`.
pub fn serve(recording: String, no_open: bool) -> Result<(), Failure> {
    let runtime = tokio::runtime::Runtime::new().map_err(|e| Failure::usage(format!("could not start the server: {e}")))?;
    runtime.block_on(async move {
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .map_err(|e| Failure::usage(format!("could not start the server: {e}")))?;
        let port = listener.local_addr().map_err(|e| Failure::usage(e.to_string()))?.port();
        let app = Router::new()
            .route("/", get(|| async { asset("index.html") }))
            .route(
                "/battle.json",
                get(move || {
                    let body = recording.clone();
                    async move { ([(header::CONTENT_TYPE, "application/json")], body) }
                }),
            )
            .route("/{*path}", get(|Path(path): Path<String>| async move { asset(&path) }));

        let url = format!("http://127.0.0.1:{port}/?recording=/battle.json");
        println!("Watch the battle: {url}   (Ctrl+C to stop)");
        if !no_open && webbrowser::open(&url).is_err() {
            println!("Could not open a browser; open the address above.");
        }
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = tokio::signal::ctrl_c().await;
            })
            .await
            .map_err(|e| Failure::usage(format!("the server stopped: {e}")))
    })
}

/// What the live server knows: every line streamed so far (so late viewers
/// get the whole battle), the viewers listening, and the final recording.
#[derive(Default)]
struct LiveState {
    lines: Vec<String>,
    viewers: Vec<tokio::sync::mpsc::UnboundedSender<String>>,
    recording: Option<String>,
}

type Shared<T> = std::sync::Arc<std::sync::Mutex<T>>;

/// Serves a live battle: the battle program's frames go out as Server-Sent
/// Events on `/live`, commands come in on `POST /control`, and `/battle.json`
/// answers 409 until the battle has finished. Serves until Ctrl+C.
pub fn serve_live(
    compiled: &crate::battle::Compiled,
    tps: Option<u32>,
    output: Option<std::path::PathBuf>,
    no_open: bool,
) -> Result<(), Failure> {
    use std::io::{BufRead, Write};

    let mut child = crate::battle::spawn_live(compiled, tps)?;
    let stdout = child.stdout.take().expect("piped stdout");
    let stdin: Shared<std::process::ChildStdin> = std::sync::Arc::new(std::sync::Mutex::new(child.stdin.take().expect("piped stdin")));
    let state: Shared<LiveState> = std::sync::Arc::default();

    // Battle program → all viewers.
    let reader_state = state.clone();
    let recording_path = compiled.recording_path.clone();
    std::thread::spawn(move || {
        for line in std::io::BufReader::new(stdout).lines() {
            let Ok(line) = line else { break };
            let mut s = reader_state.lock().unwrap();
            if line.contains(r#""done":true"#) && line.len() < 20 {
                if let Ok(json) = std::fs::read_to_string(&recording_path) {
                    if let Some(out) = &output {
                        match std::fs::write(out, &json) {
                            Ok(()) => eprintln!("Saved the recording to {}", out.display()),
                            Err(e) => eprintln!("could not write {}: {e}", out.display()),
                        }
                    }
                    s.recording = Some(json);
                }
            }
            s.viewers.retain(|v| v.send(line.clone()).is_ok());
            s.lines.push(line);
        }
    });

    let runtime = tokio::runtime::Runtime::new().map_err(|e| Failure::usage(format!("could not start the server: {e}")))?;
    let result = runtime.block_on(async move {
        use axum::response::sse::{Event, KeepAlive, Sse};
        use tokio_stream::StreamExt;

        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .map_err(|e| Failure::usage(format!("could not start the server: {e}")))?;
        let port = listener.local_addr().map_err(|e| Failure::usage(e.to_string()))?.port();
        let (live_state, json_state) = (state.clone(), state.clone());
        let app = Router::new()
            .route("/", get(|| async { asset("index.html") }))
            .route(
                "/live",
                get(move || {
                    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
                    {
                        let mut s = live_state.lock().unwrap();
                        for line in &s.lines {
                            let _ = tx.send(line.clone());
                        }
                        s.viewers.push(tx);
                    }
                    let stream = tokio_stream::wrappers::UnboundedReceiverStream::new(rx)
                        .map(|line| Ok::<_, std::convert::Infallible>(Event::default().data(line)));
                    async move { Sse::new(stream).keep_alive(KeepAlive::default()) }
                }),
            )
            .route(
                "/battle.json",
                get(move || {
                    let recording = json_state.lock().unwrap().recording.clone();
                    async move {
                        match recording {
                            Some(json) => ([(header::CONTENT_TYPE, "application/json")], json).into_response(),
                            None => (StatusCode::CONFLICT, "the battle is still running").into_response(),
                        }
                    }
                }),
            )
            .route(
                "/control",
                axum::routing::post(move |body: String| {
                    let command = body.trim().to_string();
                    let valid = matches!(command.as_str(), "pause" | "resume" | "step")
                        || command.strip_prefix("tps ").is_some_and(|t| t == "max" || t.parse::<u32>().is_ok_and(|t| (1..=1000).contains(&t)));
                    let written = valid && writeln!(stdin.lock().unwrap(), "{command}").is_ok();
                    async move {
                        if written { StatusCode::OK } else if valid { StatusCode::GONE } else { StatusCode::BAD_REQUEST }
                    }
                }),
            )
            .route("/{*path}", get(|Path(path): Path<String>| async move { asset(&path) }));

        let url = format!("http://127.0.0.1:{port}/?live=/live");
        println!("Watch the battle live: {url}   (Ctrl+C to stop)");
        if !no_open && webbrowser::open(&url).is_err() {
            println!("Could not open a browser; open the address above.");
        }
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = tokio::signal::ctrl_c().await;
            })
            .await
            .map_err(|e| Failure::usage(format!("the server stopped: {e}")))
    });
    let _ = child.kill();
    let _ = child.wait();
    result
}
