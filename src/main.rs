use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::thread;
use std::time::Duration;

const APP_HTML: &str = include_str!("../web/index.html");
const MAX_BODY_BYTES: usize = 2 * 1024 * 1024;

fn main() -> std::io::Result<()> {
    let port = env::args()
        .nth(1)
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(3000);
    let listener = TcpListener::bind(("127.0.0.1", port))?;
    println!("Room Planner is ready at http://127.0.0.1:{port}");

    for incoming in listener.incoming() {
        match incoming {
            Ok(stream) => {
                thread::spawn(|| {
                    if let Err(error) = handle_connection(stream) {
                        eprintln!("Request error: {error}");
                    }
                });
            }
            Err(error) => eprintln!("Connection error: {error}"),
        }
    }
    Ok(())
}

fn handle_connection(mut stream: TcpStream) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut request = Vec::with_capacity(4096);
    let mut chunk = [0_u8; 4096];
    let header_end = loop {
        if let Some(index) = request.windows(4).position(|window| window == b"\r\n\r\n") {
            break index + 4;
        }
        if request.len() > 16 * 1024 {
            return respond(&mut stream, 431, "text/plain; charset=utf-8", "Headers too large");
        }
        let count = stream.read(&mut chunk)?;
        if count == 0 {
            return Ok(());
        }
        request.extend_from_slice(&chunk[..count]);
    };

    let header_text = String::from_utf8_lossy(&request[..header_end]).into_owned();
    let mut lines = header_text.split("\r\n");
    let first_line = lines.next().unwrap_or_default();
    let mut request_parts = first_line.split_whitespace();
    let method = request_parts.next().unwrap_or_default();
    let path = request_parts.next().unwrap_or("/").split('?').next().unwrap_or("/");
    let content_length = lines
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.trim().parse::<usize>().ok())
        .unwrap_or(0);

    if content_length > MAX_BODY_BYTES {
        return respond(&mut stream, 413, "text/plain; charset=utf-8", "Plan is too large");
    }
    let total_length = header_end + content_length;
    while request.len() < total_length {
        let count = stream.read(&mut chunk)?;
        if count == 0 {
            return respond(&mut stream, 400, "text/plain; charset=utf-8", "Incomplete request body");
        }
        request.extend_from_slice(&chunk[..count]);
    }
    let body = &request[header_end..total_length];

    match (method, path) {
        ("GET", "/") | ("GET", "/index.html") => {
            respond(&mut stream, 200, "text/html; charset=utf-8", APP_HTML)
        }
        ("GET", "/api/plans") => list_plans(&mut stream),
        ("GET", route) if route.starts_with("/api/plans/") => {
            let id = &route["/api/plans/".len()..];
            read_plan(&mut stream, id)
        }
        ("PUT", route) if route.starts_with("/api/plans/") => {
            let id = &route["/api/plans/".len()..];
            write_plan(&mut stream, id, body)
        }
        ("DELETE", route) if route.starts_with("/api/plans/") => {
            let id = &route["/api/plans/".len()..];
            delete_plan(&mut stream, id)
        }
        _ => respond(&mut stream, 404, "text/plain; charset=utf-8", "Not found"),
    }
}

fn plans_dir() -> &'static Path {
    Path::new("plans")
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 80
        && id.bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

fn list_plans(stream: &mut TcpStream) -> std::io::Result<()> {
    let mut ids = Vec::new();
    if let Ok(entries) = fs::read_dir(plans_dir()) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|extension| extension == "json") {
                if let Some(id) = path.file_stem().and_then(|stem| stem.to_str()) {
                    if valid_id(id) {
                        ids.push(format!("\"{id}\""));
                    }
                }
            }
        }
    }
    ids.sort();
    let body = format!("[{}]", ids.join(","));
    respond(stream, 200, "application/json; charset=utf-8", &body)
}

fn read_plan(stream: &mut TcpStream, id: &str) -> std::io::Result<()> {
    if !valid_id(id) {
        return respond(stream, 400, "text/plain; charset=utf-8", "Invalid plan id");
    }
    match fs::read_to_string(plans_dir().join(format!("{id}.json"))) {
        Ok(contents) => respond(stream, 200, "application/json; charset=utf-8", &contents),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            respond(stream, 404, "text/plain; charset=utf-8", "Plan not found")
        }
        Err(error) => Err(error),
    }
}

fn write_plan(stream: &mut TcpStream, id: &str, body: &[u8]) -> std::io::Result<()> {
    if !valid_id(id) {
        return respond(stream, 400, "text/plain; charset=utf-8", "Invalid plan id");
    }
    let text = String::from_utf8_lossy(body);
    if !text.trim_start().starts_with('{') {
        return respond(stream, 400, "text/plain; charset=utf-8", "Plan must be a JSON object");
    }
    fs::create_dir_all(plans_dir())?;
    let final_path = plans_dir().join(format!("{id}.json"));
    let temp_path = plans_dir().join(format!("{id}.json.tmp"));
    fs::write(&temp_path, body)?;
    fs::rename(temp_path, final_path)?;
    respond(stream, 200, "application/json; charset=utf-8", "{\"saved\":true}")
}

fn delete_plan(stream: &mut TcpStream, id: &str) -> std::io::Result<()> {
    if !valid_id(id) {
        return respond(stream, 400, "text/plain; charset=utf-8", "Invalid plan id");
    }
    match fs::remove_file(plans_dir().join(format!("{id}.json"))) {
        Ok(()) => respond(stream, 200, "application/json; charset=utf-8", "{\"deleted\":true}"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            respond(stream, 404, "text/plain; charset=utf-8", "Plan not found")
        }
        Err(error) => Err(error),
    }
}

fn respond(stream: &mut TcpStream, status: u16, content_type: &str, body: &str) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        413 => "Payload Too Large",
        431 => "Request Header Fields Too Large",
        _ => "Error",
    };
    write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\nX-Content-Type-Options: nosniff\r\n\r\n",
        body.as_bytes().len()
    )?;
    stream.write_all(body.as_bytes())?;
    stream.flush()
}
