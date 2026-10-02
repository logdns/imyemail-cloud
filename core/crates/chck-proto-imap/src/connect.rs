use crate::probe::Endpoint;
use crate::session::{ImapSession, capability_has, imap_quote, map_io};
use chck_providers::{Provider, QUIRK_IMAP_ID};
use chck_types::{AddAccountRequest, EngineError, MailAuth};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(8);

pub fn connect_and_login(
    req: &AddAccountRequest,
    provider: Option<&Provider>,
) -> Result<ImapSession, EngineError> {
    let endpoint = crate::probe::resolved_endpoint(req, provider)
        .map_err(|_| EngineError::Invalid("imap host".into()))?;
    connect_endpoint(&endpoint, req, provider)
}

pub fn connect_endpoint(
    endpoint: &Endpoint,
    req: &AddAccountRequest,
    provider: Option<&Provider>,
) -> Result<ImapSession, EngineError> {
    let addrs: Vec<_> =
        (endpoint.host.as_str(), endpoint.port).to_socket_addrs().map_err(map_io)?.collect();
    if addrs.is_empty() {
        return Err(EngineError::Network("dns"));
    }
    let mut last = EngineError::Network("refused");
    let mut tcp = None;
    for addr in addrs {
        match TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT) {
            Ok(s) => {
                tcp = Some(s);
                break;
            }
            Err(e) => last = map_io(e),
        }
    }
    let tcp = tcp.ok_or(last)?;
    let mut session = if endpoint.starttls {
        ImapSession::plain(tcp)?
    } else {
        ImapSession::implicit_tls(tcp, &endpoint.host, req.accept_invalid_certs)?
    };
    let _ = session.greet()?;
    let caps = session.command("CAPABILITY")?;
    if endpoint.starttls {
        if !capability_has(&caps.untagged, "STARTTLS") {
            return Err(EngineError::Tls);
        }
        let st = session.command("STARTTLS")?;
        if !st.ok {
            return Err(EngineError::Tls);
        }
        session.upgrade_starttls(&endpoint.host, req.accept_invalid_certs)?;
        let _ = session.command("CAPABILITY")?;
    }
    if !session.encrypted {
        return Err(EngineError::Tls);
    }
    let force_id = provider.is_some_and(|p| p.quirks.iter().any(|q| q == QUIRK_IMAP_ID));
    let send_id = force_id || capability_has(&caps.untagged, "ID");
    if send_id {
        let id = format!(
            "ID (\"name\" \"imyemail-cloud\" \"version\" \"{}\" \"vendor\" \"imy.email\")",
            env!("CARGO_PKG_VERSION")
        );
        let id_res = session.command(&id)?;
        if force_id && !id_res.ok {
            return Err(EngineError::AuthFailed);
        }
    }
    authenticate(&mut session, req)?;
    Ok(session)
}

pub fn authenticate(session: &mut ImapSession, req: &AddAccountRequest) -> Result<(), EngineError> {
    match req.mail_auth()? {
        MailAuth::Xoauth2 { ir, .. } => {
            let res = session.authenticate_xoauth2(&ir)?;
            if res.ok { Ok(()) } else { Err(EngineError::AuthFailed) }
        }
        MailAuth::Login { user, password } => {
            let login = session.command(&format!(
                "LOGIN {} {}",
                imap_quote(&user),
                imap_quote(&password)
            ))?;
            if login.ok { Ok(()) } else { Err(EngineError::AuthFailed) }
        }
    }
}
