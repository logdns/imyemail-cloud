//! imyemail HTTPS API。缺失字段降级；未知字段忽略。

use chck_types::{EngineError, MessageId, OAuthTokenSet, SearchHit, SearchSource};
use rustls::pki_types::ServerName;
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::{Arc, Once};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Capabilities {
    #[serde(default)]
    pub product: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub features: Features,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Features {
    #[serde(default)]
    pub server_search: bool,
    #[serde(default)]
    pub rules: bool,
    #[serde(default)]
    pub scheduled_send: bool,
    #[serde(default)]
    pub snooze: bool,
    #[serde(default)]
    pub labels: bool,
    #[serde(default)]
    pub gallery: bool,
    #[serde(default)]
    pub quota: bool,
    #[serde(default)]
    pub app_passwords: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WellKnown {
    #[serde(default)]
    pub provider: String,
    pub api_base: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerSearchHit {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub message_id: String,
}

pub trait HttpGet: Send + Sync {
    fn get_json(&self, url: &str, token: Option<&str>) -> Result<String, EngineError>;
}

pub trait HttpPost: Send + Sync {
    fn post_form(&self, url: &str, body: &str) -> Result<String, EngineError>;
}

pub struct TlsHttp;

impl HttpGet for TlsHttp {
    fn get_json(&self, url: &str, token: Option<&str>) -> Result<String, EngineError> {
        https_request("GET", url, token, None)
    }
}

impl HttpPost for TlsHttp {
    fn post_form(&self, url: &str, body: &str) -> Result<String, EngineError> {
        https_request("POST", url, None, Some(body))
    }
}

pub fn probe(http: &dyn HttpGet, api_base: &str, token: &str) -> Result<Capabilities, EngineError> {
    let url = format!("{}/api/v1/capabilities", api_base.trim_end_matches('/'));
    let body = http.get_json(&url, Some(token))?;
    let cap: Capabilities = serde_json::from_str(&body).unwrap_or_default();
    if cap.product.is_empty() {
        return Err(EngineError::Server("capability"));
    }
    Ok(cap)
}

pub fn well_known(http: &dyn HttpGet, origin: &str) -> Result<WellKnown, EngineError> {
    let url = format!("{}/.well-known/imyemailcloud.json", origin.trim_end_matches('/'));
    let body = http.get_json(&url, None)?;
    serde_json::from_str(&body).map_err(|_| EngineError::Server("well-known"))
}

pub fn server_search(
    http: &dyn HttpGet,
    api_base: &str,
    token: &str,
    q: &str,
) -> Result<Vec<SearchHit>, EngineError> {
    let url = format!("{}/api/v1/search?q={}", api_base.trim_end_matches('/'), urlencoding_lite(q));
    let body = http.get_json(&url, Some(token))?;
    let parsed: Vec<ServerSearchHit> = serde_json::from_str(&body).unwrap_or_default();
    Ok(parsed
        .into_iter()
        .map(|h| {
            let id = if h.message_id.is_empty() { h.id } else { h.message_id };
            SearchHit { message_id: MessageId::from(id), source: SearchSource::Server, rank: 1 }
        })
        .collect())
}

pub fn merge_hits(local: Vec<SearchHit>, server: Vec<SearchHit>) -> Vec<SearchHit> {
    let mut out = local;
    for hit in server {
        if !out.iter().any(|h| h.message_id == hit.message_id) {
            out.push(hit);
        }
    }
    out
}

pub fn exchange_code(
    http: &dyn HttpPost,
    token_url: &str,
    client_id: &str,
    code: &str,
    redirect_uri: &str,
    verifier: &str,
) -> Result<OAuthTokenSet, EngineError> {
    let form = format!(
        "grant_type=authorization_code&code={}&redirect_uri={}&client_id={}&code_verifier={}",
        urlencoding_lite(code),
        urlencoding_lite(redirect_uri),
        urlencoding_lite(client_id),
        urlencoding_lite(verifier),
    );
    parse_token_set(&http.post_form(token_url, &form)?)
}

pub fn refresh_access_token(
    http: &dyn HttpPost,
    token_url: &str,
    client_id: &str,
    refresh_token: &str,
) -> Result<OAuthTokenSet, EngineError> {
    let form = format!(
        "grant_type=refresh_token&refresh_token={}&client_id={}",
        urlencoding_lite(refresh_token),
        urlencoding_lite(client_id),
    );
    let mut set = parse_token_set(&http.post_form(token_url, &form)?)?;
    if set.refresh_token.is_none() {
        set.refresh_token = Some(refresh_token.to_string());
    }
    Ok(set)
}

fn parse_token_set(body: &str) -> Result<OAuthTokenSet, EngineError> {
    let set: OAuthTokenSet =
        serde_json::from_str(body).map_err(|_| EngineError::Server("oauth"))?;
    if set.access_token.is_empty() {
        return Err(EngineError::AuthExpired("OAuth"));
    }
    Ok(set)
}

fn urlencoding_lite(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn https_request(
    method: &str,
    url: &str,
    token: Option<&str>,
    body: Option<&str>,
) -> Result<String, EngineError> {
    let rest = url.strip_prefix("https://").ok_or(EngineError::Tls)?;
    let (hostport, path) = match rest.split_once('/') {
        Some((h, p)) => (h, format!("/{p}")),
        None => (rest, "/".into()),
    };
    let host = hostport.split(':').next().unwrap_or(hostport);
    let port: u16 = hostport.split_once(':').and_then(|(_, p)| p.parse().ok()).unwrap_or(443);
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
    let mut roots = RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let cfg = ClientConfig::builder().with_root_certificates(roots).with_no_client_auth();
    let addr = (host, port)
        .to_socket_addrs()
        .map_err(|_| EngineError::Network("dns"))?
        .next()
        .ok_or(EngineError::Network("dns"))?;
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(8))
        .map_err(|_| EngineError::Network("tcp"))?;
    stream.set_read_timeout(Some(Duration::from_secs(8))).ok();
    let name = ServerName::try_from(host.to_string()).map_err(|_| EngineError::Tls)?;
    let mut conn = ClientConnection::new(Arc::new(cfg), name).map_err(|_| EngineError::Tls)?;
    while conn.is_handshaking() {
        conn.complete_io(&mut stream).map_err(|_| EngineError::Tls)?;
    }
    let mut tls = StreamOwned::new(conn, stream);
    let mut req = format!(
        "{method} {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\nAccept: application/json\r\n"
    );
    if let Some(t) = token.filter(|s| !s.is_empty()) {
        req.push_str(&format!("Authorization: Bearer {t}\r\n"));
    }
    if let Some(body) = body {
        req.push_str("Content-Type: application/x-www-form-urlencoded\r\n");
        req.push_str(&format!("Content-Length: {}\r\n\r\n{body}", body.len()));
    } else {
        req.push_str("\r\n");
    }
    tls.write_all(req.as_bytes()).map_err(|_| EngineError::Network("io"))?;
    tls.flush().ok();
    let mut raw = Vec::new();
    tls.read_to_end(&mut raw).ok();
    let text = String::from_utf8_lossy(&raw);
    let (head, resp_body) =
        text.split_once("\r\n\r\n").or_else(|| text.split_once("\n\n")).unwrap_or(("", &text));
    if head.contains(" 401 ") || head.contains(" 403 ") {
        return Err(EngineError::AuthFailed);
    }
    if !head.contains(" 200 ") && !head.contains("HTTP/1.1 200") && !head.contains("HTTP/1.0 200") {
        return Err(EngineError::Server("http"));
    }
    Ok(resp_body.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MapHttp(&'static str);

    impl HttpGet for MapHttp {
        fn get_json(&self, _url: &str, _token: Option<&str>) -> Result<String, EngineError> {
            Ok(self.0.into())
        }
    }

    #[test]
    fn capabilities_json_roundtrip() {
        let json = r#"{"product":"imyemail","version":"1.3.25","features":{"serverSearch":true,"rules":true,"scheduledSend":true,"snooze":true,"labels":true,"gallery":true,"quota":true,"appPasswords":true}}"#;
        let cap: Capabilities = serde_json::from_str(json).unwrap();
        assert_eq!(cap.product, "imyemail");
        assert!(cap.features.server_search);
    }

    #[test]
    fn missing_fields_degrade() {
        let cap: Capabilities = serde_json::from_str(r#"{"product":"imyemail"}"#).unwrap();
        assert!(!cap.features.server_search);
    }

    #[test]
    fn probe_parses() {
        let http = MapHttp(
            r#"{"product":"imyemail","version":"1.3.25","features":{"serverSearch":true}}"#,
        );
        let cap = probe(&http, "https://mail.example.com", "tok").unwrap();
        assert!(cap.features.server_search);
    }

    #[test]
    fn merge_dedupes() {
        let a =
            SearchHit { message_id: MessageId::from("m1"), source: SearchSource::Local, rank: 0 };
        let b =
            SearchHit { message_id: MessageId::from("m1"), source: SearchSource::Server, rank: 1 };
        let c =
            SearchHit { message_id: MessageId::from("m2"), source: SearchSource::Server, rank: 1 };
        let merged = merge_hits(vec![a], vec![b, c]);
        assert_eq!(merged.len(), 2);
    }

    struct MapPost(&'static str);

    impl HttpPost for MapPost {
        fn post_form(&self, _url: &str, _body: &str) -> Result<String, EngineError> {
            Ok(self.0.into())
        }
    }

    #[test]
    fn exchange_code_parses_tokens() {
        let http = MapPost(r#"{"access_token":"ya29.a","refresh_token":"1//r","expires_in":3600}"#);
        let set = exchange_code(
            &http,
            "https://oauth2.googleapis.com/token",
            "cid",
            "code",
            "http://127.0.0.1:8743/oauth",
            "ver",
        )
        .unwrap();
        assert_eq!(set.access_token, "ya29.a");
        assert_eq!(set.refresh_token.as_deref(), Some("1//r"));
    }

    #[test]
    fn refresh_keeps_old_refresh_when_omitted() {
        let http = MapPost(r#"{"access_token":"ya29.b"}"#);
        let set = refresh_access_token(&http, "https://example/token", "cid", "1//old").unwrap();
        assert_eq!(set.access_token, "ya29.b");
        assert_eq!(set.refresh_token.as_deref(), Some("1//old"));
    }
}
