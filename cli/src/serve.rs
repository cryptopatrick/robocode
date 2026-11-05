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
