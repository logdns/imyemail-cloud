use crate::connect::authenticate;
use crate::session::{ImapSession, capability_has, map_io, parse_list_name};
use chck_providers::{Provider, QUIRK_IMAP_ID};
use chck_types::{AddAccountRequest, ConnectProbe, EngineError, ProbeKind};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(8);

#[derive(Debug, Clone)]
pub struct Endpoint {
    pub host: String,
    pub port: u16,
    pub starttls: bool,
}

pub fn resolved_endpoint(
    req: &AddAccountRequest,
    provider: Option<&Provider>,
) -> Result<Endpoint, String> {
    if let Some(host) = req.imap_host.as_ref().filter(|h| !h.is_empty()) {
        let port = req.imap_port.unwrap_or(if req.imap_starttls { 143 } else { 993 });
        return Ok(Endpoint { host: host.clone(), port, starttls: req.imap_starttls });
    }
    if let Some(p) = provider {
        let starttls = req.imap_starttls || matches!(p.imap.tls, chck_providers::TlsMode::Starttls);
        let port = req.imap_port.unwrap_or(p.imap.port);
        return Ok(Endpoint { host: p.imap.host.clone(), port, starttls });
    }
    Err("no IMAP host; choose a provider or enter server".into())
}

pub fn probe(
    req: &AddAccountRequest,
    provider: Option<&Provider>,
) -> Result<ConnectProbe, EngineError> {
    let mut probe = ConnectProbe::default();
    if !req.email.contains('@') {
        return Err(EngineError::Invalid("email".into()));
    }

    let endpoint = match resolved_endpoint(req, provider) {
        Ok(ep) => ep,
        Err(msg) => {
            probe.push_fail(ProbeKind::Dns, msg);
            probe.skip_rest(ProbeKind::Tcp);
            return Ok(probe);
        }
    };

    let addrs = match (endpoint.host.as_str(), endpoint.port).to_socket_addrs() {
        Ok(iter) => {
            let v: Vec<SocketAddr> = iter.collect();
            if v.is_empty() {
                probe.push_fail(ProbeKind::Dns, "no addresses");
                probe.skip_rest(ProbeKind::Tcp);
                return Ok(probe);
            }
            probe.push_ok(ProbeKind::Dns, format!("{}:{}", endpoint.host, endpoint.port));
            v
        }
        Err(_) => {
            probe.push_fail(ProbeKind::Dns, "resolution failed");
            probe.skip_rest(ProbeKind::Tcp);
            return Ok(probe);
        }
    };

    let tcp = match connect_any(&addrs) {
        Ok(s) => {
            let peer = s.peer_addr().map(|a| a.to_string()).unwrap_or_default();
            probe.push_ok(ProbeKind::Tcp, format!("connected {peer}"));
            s
        }
        Err(err) => {
            probe.push_fail(ProbeKind::Tcp, err.to_string());
            probe.skip_rest(ProbeKind::Tls);
            return Ok(probe);
        }
    };

    let force_id = provider.is_some_and(|p| p.quirks.iter().any(|q| q == QUIRK_IMAP_ID));
    if let Err(err) = run_imap(tcp, &endpoint, req, force_id, &mut probe) {
        if probe.step(ProbeKind::Tls).is_none() {
            probe.push_fail(ProbeKind::Tls, err.to_string());
            probe.skip_rest(ProbeKind::Certificate);
        } else if probe.step(ProbeKind::Auth).is_none() {
            probe.push_fail(ProbeKind::Auth, err.to_string());
            probe.skip_rest(ProbeKind::Folders);
        } else if probe.step(ProbeKind::Folders).is_none() {
            probe.push_fail(ProbeKind::Folders, err.to_string());
        }
    }
    Ok(probe)
}

fn connect_any(addrs: &[SocketAddr]) -> Result<TcpStream, EngineError> {
    let mut last = EngineError::Network("refused");
    for addr in addrs {
        match TcpStream::connect_timeout(addr, CONNECT_TIMEOUT) {
            Ok(s) => return Ok(s),
            Err(e) => last = map_io(e),
        }
    }
    Err(last)
}

fn run_imap(
    tcp: TcpStream,
    endpoint: &Endpoint,
    req: &AddAccountRequest,
    force_id: bool,
    probe: &mut ConnectProbe,
) -> Result<(), EngineError> {
    let mut session = if endpoint.starttls {
        ImapSession::plain(tcp)?
    } else {
        match ImapSession::implicit_tls(tcp, &endpoint.host, req.accept_invalid_certs) {
            Ok(s) => s,
            Err(err) => {
                probe.push_fail(ProbeKind::Tls, err.to_string());
                probe.push_fail(ProbeKind::Certificate, "handshake failed");
                probe.skip_rest(ProbeKind::Auth);
                return Ok(());
            }
        }
    };

    if !endpoint.starttls {
        mark_tls_ok(probe, &session, req.accept_invalid_certs);
    }

    let greeting = session.greet()?;
    let g = greeting.to_ascii_uppercase();
    if !g.contains("OK") && !greeting.starts_with('*') {
        probe.push_fail(ProbeKind::Auth, "bad greeting");
        probe.skip_rest(ProbeKind::Folders);
        return Ok(());
    }

    let caps = session.command("CAPABILITY")?;
    if endpoint.starttls {
        if !capability_has(&caps.untagged, "STARTTLS") {
            probe.push_fail(ProbeKind::Tls, "STARTTLS not offered; plaintext IMAP is forbidden");
            probe.skip_rest(ProbeKind::Certificate);
            return Ok(());
        }
        let st = session.command("STARTTLS")?;
        if !st.ok {
            probe.push_fail(ProbeKind::Tls, st.tagged);
            probe.skip_rest(ProbeKind::Certificate);
            return Ok(());
        }
        if let Err(err) = session.upgrade_starttls(&endpoint.host, req.accept_invalid_certs) {
            probe.push_fail(ProbeKind::Tls, err.to_string());
            probe.push_fail(ProbeKind::Certificate, "handshake failed");
            probe.skip_rest(ProbeKind::Auth);
            return Ok(());
        }
        mark_tls_ok(probe, &session, req.accept_invalid_certs);
        let _ = session.command("CAPABILITY")?;
    }

    if !session.encrypted {
        probe.push_fail(ProbeKind::Tls, "plaintext IMAP is forbidden");
        probe.skip_rest(ProbeKind::Certificate);
        return Ok(());
    }

    if !req.has_mail_secret() {
        probe.push_fail(ProbeKind::Auth, "password or access token required");
        probe.skip_rest(ProbeKind::Folders);
        return Ok(());
    }

    let send_id = force_id || capability_has(&caps.untagged, "ID");
    if send_id {
        let id = format!(
            "ID (\"name\" \"imyemail-cloud\" \"version\" \"{}\" \"vendor\" \"imy.email\")",
            env!("CARGO_PKG_VERSION")
        );
        let id_res = session.command(&id)?;
        if force_id && !id_res.ok {
            probe.push_fail(ProbeKind::Auth, "IMAP ID rejected");
            probe.skip_rest(ProbeKind::Folders);
            return Ok(());
        }
    }

    match authenticate(&mut session, req) {
        Ok(()) => probe
            .push_ok(ProbeKind::Auth, if req.uses_xoauth2() { "xoauth2 ok" } else { "login ok" }),
        Err(_) => {
            probe.push_fail(ProbeKind::Auth, "authentication failed");
            probe.skip_rest(ProbeKind::Folders);
            return Ok(());
        }
    }

    let list = session.command("LIST \"\" \"*\"")?;
    if !list.ok {
        probe.push_fail(ProbeKind::Folders, list.tagged);
        return Ok(());
    }
    let names: Vec<String> = list.untagged.iter().filter_map(|l| parse_list_name(l)).collect();
    if names.is_empty() {
        probe.push_fail(ProbeKind::Folders, "no folders");
        return Ok(());
    }
    probe.push_ok(ProbeKind::Folders, format!("{} folders", names.len()));
    let _ = session.command("LOGOUT");
    Ok(())
}

fn mark_tls_ok(probe: &mut ConnectProbe, session: &ImapSession, accept_invalid: bool) {
    let fp = session.fingerprint.clone().unwrap_or_default();
    probe.push_ok(ProbeKind::Tls, "tls 1.2+");
    let detail = if accept_invalid {
        format!("untrusted fingerprint={fp} (user accepted)")
    } else {
        format!("fingerprint={fp}")
    };
    probe.push_ok(ProbeKind::Certificate, detail);
}

#[cfg(test)]
mod endpoint_tests {
    use super::*;
    use chck_providers::ProviderCatalog;

    #[test]
    fn manual_host_wins() {
        let mut req = AddAccountRequest::new("dev@imyemail.test");
        req.imap_host = Some("127.0.0.1".into());
        req.imap_port = Some(3993);
        let ep = resolved_endpoint(&req, None).unwrap();
        assert_eq!(ep.host, "127.0.0.1");
        assert_eq!(ep.port, 3993);
        assert!(!ep.starttls);
    }

    #[test]
    fn provider_fills_qq() {
        let cat = ProviderCatalog::builtin();
        let qq = cat.by_email("a@qq.com").unwrap();
        let req = AddAccountRequest::new("a@qq.com");
        let ep = resolved_endpoint(&req, Some(qq)).unwrap();
        assert_eq!(ep.host, "imap.qq.com");
        assert_eq!(ep.port, 993);
        assert!(!ep.starttls);
    }

    #[test]
    fn custom_without_host_fails() {
        let req = AddAccountRequest::new("me@example.com");
        assert!(resolved_endpoint(&req, None).is_err());
    }
}
