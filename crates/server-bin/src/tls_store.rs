use crate::config::{Config, GetProperty};
use anyhow::Context;
use rustls::crypto::aws_lc_rs;
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::server::ResolvesServerCertUsingSni;
use rustls::sign::CertifiedKey;
use std::path::PathBuf;
use std::sync::Arc;

fn load_tls_files(
    cert: PathBuf,
    key: PathBuf,
) -> anyhow::Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)> {
    if !cert.exists() {
        return Err(anyhow::anyhow!(
            "Certificate file does not exist: {:?}",
            cert
        ));
    }

    if !key.exists() {
        return Err(anyhow::anyhow!(
            "Private key file does not exist: {:?}",
            key
        ));
    }

    let certs = CertificateDer::pem_file_iter(cert)
        .expect("Failed to read certificate")
        .collect::<Result<Vec<_>, _>>()?;
    let key = PrivateKeyDer::from_pem_file(key).expect("Failed to read private key");

    Ok((certs, key))
}

pub fn make_tls_config(config: &Config) -> anyhow::Result<Arc<rustls::ServerConfig>> {
    let provider = aws_lc_rs::default_provider();
    let mut resolver = ResolvesServerCertUsingSni::new();

    for vhost in &config.server.vhosts {
        let domain = &vhost.vhost;

        let cert = vhost
            .get_property_string("tls_cert")
            .context(format!(
                "The vhost '{}' is missing the 'tls_cert' property",
                domain
            ))?
            .into();

        let key = vhost
            .get_property_string("tls_key")
            .context(format!(
                "The vhost '{}' is missing the 'tls_key' property",
                domain
            ))?
            .into();

        let (certs, key) = load_tls_files(cert, key).context(format!(
            "Failed to create TLS config for vhost '{}'",
            domain
        ))?;

        resolver.add(domain.0, CertifiedKey::from_der(certs, key, &provider)?)?
    }

    let mut config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_cert_resolver(Arc::new(resolver));

    config.key_log = Arc::new(rustls::KeyLogFile::new());

    Ok(Arc::new(config))
}
