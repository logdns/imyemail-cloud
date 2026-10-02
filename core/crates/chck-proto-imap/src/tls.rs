use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{ClientConfig, DigitallySignedStruct, RootCertStore, SignatureScheme};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex, Once};

pub fn ensure_crypto() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

#[must_use]
pub fn hex_sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

pub fn client_config(
    accept_invalid_certs: bool,
    fingerprint: Arc<Mutex<Option<String>>>,
) -> ClientConfig {
    ensure_crypto();
    if accept_invalid_certs {
        ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(AcceptAny { fingerprint }))
            .with_no_client_auth()
    } else {
        let mut roots = RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let mut cfg = ClientConfig::builder().with_root_certificates(roots).with_no_client_auth();
        cfg.alpn_protocols.clear();
        let _ = fingerprint;
        cfg
    }
}

#[derive(Debug)]
struct AcceptAny {
    fingerprint: Arc<Mutex<Option<String>>>,
}

impl ServerCertVerifier for AcceptAny {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        if let Ok(mut slot) = self.fingerprint.lock() {
            *slot = Some(hex_sha256(end_entity.as_ref()));
        }
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}
