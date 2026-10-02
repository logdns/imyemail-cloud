use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
#[cfg(unix)]
use std::io::Read;

const B64STD: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const B64URL: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pkce {
    pub verifier: String,
    pub challenge: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OAuthStart {
    pub authorize_url: String,
    pub verifier: String,
    pub state: String,
    pub redirect_uri: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MailAuth {
    Login { user: String, password: String },
    Xoauth2 { user: String, ir: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct OAuthTokenSet {
    #[serde(default)]
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub expires_in: Option<u64>,
    #[serde(default)]
    pub token_type: Option<String>,
}

/// RFC 7628 SASL XOAUTH2 initial response: `user=…\x01auth=Bearer …\x01\x01`.
#[must_use]
pub fn xoauth2_ir(user: &str, access_token: &str) -> String {
    b64_encode(
        format!("user={user}\x01auth=Bearer {access_token}\x01\x01").as_bytes(),
        B64STD,
        true,
    )
}

#[must_use]
pub fn pkce_verifier() -> String {
    let mut raw = [0u8; 32];
    fill_random(&mut raw);
    b64_encode(&raw, B64URL, false)
}

#[must_use]
pub fn pkce_s256(verifier: &str) -> Pkce {
    let digest = Sha256::digest(verifier.as_bytes());
    Pkce { verifier: verifier.to_string(), challenge: b64_encode(&digest, B64URL, false) }
}

#[must_use]
pub fn authorize_url(
    auth_url: &str,
    client_id: &str,
    redirect_uri: &str,
    scopes: &[String],
    state: &str,
    challenge: &str,
) -> String {
    let scope = urlencode(&scopes.join(" "));
    format!(
        "{auth_url}?response_type=code&client_id={}&redirect_uri={}&scope={scope}&state={}&code_challenge={}&code_challenge_method=S256",
        urlencode(client_id),
        urlencode(redirect_uri),
        urlencode(state),
        urlencode(challenge),
    )
}

fn fill_random(buf: &mut [u8]) {
    #[cfg(unix)]
    if let Ok(mut f) = std::fs::File::open("/dev/urandom")
        && f.read_exact(buf).is_ok()
    {
        return;
    }
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut state = nanos ^ 0x9E37_79B9_7F4A_7C15 ^ u128::from(std::process::id());
    for byte in buf {
        state = state.wrapping_mul(0x5851_F42D_4C95_7F2D).wrapping_add(1);
        *byte = (state >> 33) as u8;
    }
}

fn b64_encode(input: &[u8], table: &[u8], padded: bool) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i < input.len() {
        let remain = input.len() - i;
        let b0 = u32::from(input[i]);
        let b1 = if remain > 1 { u32::from(input[i + 1]) } else { 0 };
        let b2 = if remain > 2 { u32::from(input[i + 2]) } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(table[((n >> 18) & 63) as usize] as char);
        out.push(table[((n >> 12) & 63) as usize] as char);
        if remain > 1 {
            out.push(table[((n >> 6) & 63) as usize] as char);
        } else if padded {
            out.push('=');
        }
        if remain > 2 {
            out.push(table[(n & 63) as usize] as char);
        } else if padded {
            out.push('=');
        }
        i += 3;
    }
    out
}

fn urlencode(s: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xoauth2_ir_is_standard_base64() {
        let ir = xoauth2_ir("user@gmail.com", "ya29.token");
        let expected = "dXNlcj11c2VyQGdtYWlsLmNvbQFhdXRoPUJlYXJlciB5YTI5LnRva2VuAQE=";
        assert_eq!(ir, expected);
    }

    #[test]
    fn pkce_s256_rfc7636_appendix_b() {
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let pkce = pkce_s256(verifier);
        assert_eq!(pkce.challenge, "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    }

    #[test]
    fn authorize_url_includes_pkce() {
        let url = authorize_url(
            "https://accounts.google.com/o/oauth2/v2/auth",
            "cid",
            "http://127.0.0.1:9/cb",
            &["https://mail.google.com/".into()],
            "st",
            "chal",
        );
        assert!(url.contains("code_challenge=chal"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("response_type=code"));
    }
}
