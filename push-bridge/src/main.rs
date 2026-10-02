use push_bridge::{assemble_push, parse_event, verify_signature};
use std::io::{Read, Write};
use std::net::TcpListener;

fn main() {
    let secret =
        std::env::var("IMYEMAIL_CLOUD_WEBHOOK_SECRET").unwrap_or_else(|_| "dev-secret".into());
    let bind =
        std::env::var("IMYEMAIL_CLOUD_BRIDGE_BIND").unwrap_or_else(|_| "127.0.0.1:8787".into());
    let listener = TcpListener::bind(&bind).expect("bind");
    eprintln!("push-bridge listening on {bind}");
    for stream in listener.incoming().flatten() {
        let _ = handle(stream, secret.as_bytes());
    }
}

fn handle(mut stream: std::net::TcpStream, secret: &[u8]) -> std::io::Result<()> {
    let mut buf = [0u8; 8192];
    let n = stream.read(&mut buf)?;
    let req = String::from_utf8_lossy(&buf[..n]);
    let mut lines = req.split("\r\n");
    let start = lines.next().unwrap_or("");
    if !start.starts_with("POST ") {
        return write_http(&mut stream, 405, "method");
    }
    let mut sig = String::new();
    let mut content_len = 0usize;
    for line in lines.by_ref() {
        if line.is_empty() {
            break;
        }
        if let Some(v) = line
            .strip_prefix("X-Imyemail-Cloud-Signature: ")
            .or_else(|| line.strip_prefix("X-Hub-Signature-256: "))
        {
            sig = v.trim().trim_start_matches("sha256=").to_string();
        }
        if let Some(v) = line.strip_prefix("Content-Length: ") {
            content_len = v.trim().parse().unwrap_or(0);
        }
    }
    let body_start = req.find("\r\n\r\n").map(|i| i + 4).unwrap_or(n);
    let mut body = req[body_start.min(req.len())..].as_bytes().to_vec();
    while body.len() < content_len {
        let k = stream.read(&mut buf)?;
        if k == 0 {
            break;
        }
        body.extend_from_slice(&buf[..k]);
    }
    body.truncate(content_len);
    if !verify_signature(secret, &body, &sig) {
        return write_http(&mut stream, 401, "bad signature");
    }
    let Ok(event) = parse_event(&String::from_utf8_lossy(&body)) else {
        return write_http(&mut stream, 400, "bad json");
    };
    let push = assemble_push(&event);
    let payload = serde_json::to_string(&push).unwrap_or_else(|_| "{}".into());
    write_http(&mut stream, 200, &payload)
}

fn write_http(stream: &mut std::net::TcpStream, code: u16, body: &str) -> std::io::Result<()> {
    let reason = match code {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        _ => "Error",
    };
    let resp = format!(
        "HTTP/1.1 {code} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(resp.as_bytes())
}
