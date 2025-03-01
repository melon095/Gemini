use crate::config::{Config, GetBlocks, GetProperty};
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

    for block in config.get_blocks("vhost") {
        let domain = block
            .get_property_string("for")
            .context("vhost block is missing the 'for' property")?;

        let cert = block
            .get_property_string("tls_cert")
            .context(format!(
                "The vhost '{}' is missing the 'tls_cert' property",
                domain
            ))?
            .into();

        let key = block
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

        resolver.add(domain, CertifiedKey::from_der(certs, key, &provider)?)?
    }

    let config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_cert_resolver(Arc::new(resolver));

    Ok(Arc::new(config))
}
