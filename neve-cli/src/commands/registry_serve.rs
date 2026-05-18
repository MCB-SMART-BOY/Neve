//! Registry server for Neve packages.
//! Neve 软件包注册服务器。

use crate::output;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub fn run(dir: &str, port: u16) -> Result<(), String> {
    let data_dir = Arc::new(PathBuf::from(dir));

    // Ensure the directory exists
    fs::create_dir_all(&*data_dir)
        .map_err(|e| format!("failed to create registry directory: {e}"))?;

    let addr = format!("0.0.0.0:{port}");
    let listener =
        TcpListener::bind(&addr).map_err(|e| format!("failed to bind to {addr}: {e}"))?;

    output::info(&format!("Registry server listening on http://{addr}"));
    output::info(&format!("Serving packages from {}", data_dir.display()));

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let data_dir = data_dir.clone();
                std::thread::spawn(move || {
                    let mut reader = BufReader::new(&mut stream);
                    let mut request_line = String::new();
                    if reader.read_line(&mut request_line).is_err() {
                        return;
                    }

                    let parts: Vec<&str> = request_line.split_whitespace().collect();
                    if parts.len() < 2 {
                        return;
                    }

                    let method = parts[0];
                    let path = parts[1];

                    let response = match (method, path) {
                        ("GET", "/") => serve_index(&data_dir),
                        ("GET", "/packages.json") => serve_packages_json(&data_dir),
                        ("GET", p) if p.starts_with("/packages/") => {
                            let name = p.trim_start_matches("/packages/");
                            serve_package(&data_dir, name)
                        }
                        ("GET", "/health") => {
                            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nOK".to_string()
                        }
                        _ => "HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\n\r\nNot Found"
                            .to_string(),
                    };

                    let _ = stream.write_all(response.as_bytes());
                });
            }
            Err(e) => {
                output::warning(&format!("connection error: {e}"));
            }
        }
    }

    Ok(())
}

fn serve_index(_data_dir: &Path) -> String {
    let body = r#"{"registry":"neve","version":"1.0","endpoints":["/packages.json","/packages/{name}.json","/health"]}"#.to_string();
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

fn serve_packages_json(data_dir: &Path) -> String {
    let index_path = data_dir.join("packages.json");
    match fs::read_to_string(&index_path) {
        Ok(content) => {
            let len = content.len();
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {len}\r\n\r\n{content}"
            )
        }
        Err(_) => {
            let body = "[]";
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n{body}"
            )
        }
    }
}

fn serve_package(data_dir: &Path, name: &str) -> String {
    // Sanitize name to prevent path traversal
    let safe_name: String = name
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == '.')
        .collect();

    let pkg_path = data_dir.join(format!("packages/{}.json", safe_name));
    match fs::read_to_string(&pkg_path) {
        Ok(content) => {
            let len = content.len();
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {len}\r\n\r\n{content}"
            )
        }
        Err(_) => {
            let body = format!(r#"{{"error":"package '{}' not found"}}"#, name);
            format!(
                "HTTP/1.1 404 Not Found\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            )
        }
    }
}
