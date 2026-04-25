use std::fs;
use std::path::{Path, PathBuf};
use tiny_http::{Header, Response, Server};

use crate::engine::{RegistryEngine, TaskEngine};

pub fn run(host: &str, port: u16, static_dir: PathBuf) -> crate::Result<()> {
    let addr = format!("{}:{}", host, port);
    let server = Server::http(&addr)
        .map_err(|e| crate::Error::Db(format!("cannot bind to {addr}: {e}")))?;

    eprintln!("Kerrigan viz listening on http://{}:{}", host, port);
    eprintln!("Serving static files from: {}", static_dir.display());

    for request in server.incoming_requests() {
        let url = request.url().to_string();
        let path = url.split('?').next().unwrap_or("/").to_string();

        let response = if path.starts_with("/api/") {
            handle_api(&path)
        } else {
            serve_static(&path, &static_dir)
        };

        let _ = request.respond(response);
    }

    Ok(())
}

fn json_response(body: String, status: u16) -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_data(body.into_bytes())
        .with_status_code(status)
        .with_header(json_header())
        .with_header(cors_header())
}

fn handle_api(path: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    match api_handler(path) {
        Ok(json) => json_response(json.to_string(), 200),
        Err(e) => json_response(
            serde_json::json!({"error": e}).to_string(),
            if e.contains("not found") { 404 } else { 500 },
        ),
    }
}

fn api_handler(path: &str) -> Result<serde_json::Value, String> {
    let reg = RegistryEngine::default_root().map_err(|e| e.to_string())?;

    if path == "/api/tasks" {
        let mut tasks = reg.task_list().map_err(|e| e.to_string())?;
        tasks.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(serde_json::json!(tasks))
    } else if let Some(id) = path.strip_prefix("/api/tasks/") {
        let task = reg
            .task_get(id)
            .map_err(|_| format!("task {id} not found"))?;
        let task_dir = reg.task_dir(id);
        let te = TaskEngine::open(&task_dir).map_err(|e| e.to_string())?;
        let (nodes, edges) = te.viz_data().map_err(|e| e.to_string())?;
        Ok(serde_json::json!({
            "task": task,
            "nodes": nodes,
            "edges": edges,
        }))
    } else {
        Err(format!("not found: {path}"))
    }
}

fn serve_static(path: &str, static_dir: &Path) -> Response<std::io::Cursor<Vec<u8>>> {
    let file_path = resolve_static_path(path, static_dir);

    let (bytes, resolved_path) = match fs::read(&file_path) {
        Ok(b) => (b, file_path.clone()),
        Err(_) => {
            // SPA fallback: serve index.html for unknown paths
            let index = static_dir.join("index.html");
            match fs::read(&index) {
                Ok(b) => (b, index),
                Err(_) => {
                    let msg = format!(
                        "Static files not found at '{}'. Run 'make viz-build' first.",
                        static_dir.display()
                    );
                    return Response::from_data(msg.into_bytes())
                        .with_status_code(503)
                        .with_header(text_header());
                }
            }
        }
    };

    let mime = mime_type(&resolved_path);
    Response::from_data(bytes)
        .with_header(Header::from_bytes("Content-Type", mime).unwrap())
}

fn resolve_static_path(path: &str, static_dir: &Path) -> PathBuf {
    let clean = path.trim_start_matches('/');
    if clean.is_empty() {
        static_dir.join("index.html")
    } else {
        let p = static_dir.join(clean);
        // If path points to a directory, try index.html inside it
        if p.is_dir() {
            p.join("index.html")
        } else {
            p
        }
    }
}

fn mime_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") | Some("mjs") => "application/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json") => "application/json",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("ico") => "image/x-icon",
        Some("woff2") => "font/woff2",
        Some("woff") => "font/woff",
        Some("txt") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

fn json_header() -> Header {
    Header::from_bytes("Content-Type", "application/json").unwrap()
}

fn cors_header() -> Header {
    Header::from_bytes("Access-Control-Allow-Origin", "*").unwrap()
}

fn text_header() -> Header {
    Header::from_bytes("Content-Type", "text/plain; charset=utf-8").unwrap()
}
