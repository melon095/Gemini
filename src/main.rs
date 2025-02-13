use std::io;
use std::io::{Read, Write};
use std::net::{TcpStream};
use std::sync::Arc;
use rustls::crypto::{aws_lc_rs, CryptoProvider};
use rustls::pki_types::ServerName;
use rustls::{ClientConnection, RootCertStore};

#[derive(Debug)]
struct TlsClient
{
    socket: TcpStream,
    client_connection: ClientConnection,
    closing: bool,
    clean_closure: bool,
}

impl TlsClient
{
    fn new(socket: TcpStream, server_name: ServerName<'static>, tls_config: Arc<rustls::ClientConfig>) -> Result<Self, rustls::Error> {
        let mut cl = rustls::ClientConnection::new(tls_config, server_name)?;
        let mut socket = socket;

        Ok(Self {
            socket: socket,
            client_connection: cl,
            closing: false,
            clean_closure: false,
        })
    }

    fn complete_prior_io(&mut self) -> Result<(), std::io::Error> {
        if self.client_connection.is_handshaking() {
            self.client_connection.complete_io(&mut self.socket)?;
        }

        if self.client_connection.wants_write() {
            self.client_connection.complete_io(&mut self.socket)?;
        }

        Ok(())
    }

    fn prepare_read(&mut self) -> Result<(), std::io::Error> {
        self.complete_prior_io()?;

        while self.client_connection.wants_read() {
            if self.client_connection.complete_io(&mut self.socket)?.0 == 0 {
                break;
            }
        }

        Ok(())
    }
}

impl io::Write for TlsClient {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.complete_prior_io()?;

        let len = self.client_connection.writer().write(buf)?;

        let _ = self.client_connection.complete_io(&mut self.socket)?;

        Ok(len)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.complete_prior_io()?;

        self.client_connection.writer().flush()?;
        if self.client_connection.wants_write() {
            self.client_connection.complete_io(&mut self.socket)?;
        }

        Ok(())
    }
}

impl io::Read for TlsClient {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // self.stream.read(buf)
        self.prepare_read()?;
        self.client_connection.reader().read(buf)
    }
}

fn make_tls_config() -> Result<Arc<rustls::ClientConfig>, rustls::Error>  {
    let mut root_store = RootCertStore::empty();

    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    let suites = aws_lc_rs::ALL_CIPHER_SUITES.to_vec();
    let versions = rustls::DEFAULT_VERSIONS.to_vec();
    let mut config = rustls::ClientConfig::builder_with_provider(
        CryptoProvider {
            cipher_suites: suites,
            ..aws_lc_rs::default_provider()
        }
        .into()
    )
        .with_protocol_versions(&versions)?
        .with_root_certificates(root_store)
        .with_no_client_auth();

    config.key_log = Arc::new(rustls::KeyLogFile::new());

    Ok(Arc::new(config))
}

fn main() {
    const HOST: &'static str = "howsmyssl.com";
    // const PORT: u16 = 1965;
    const PORT: u16 = 443;

    let addr = format!("{}:{}", HOST, PORT);

    let tls_config = make_tls_config().unwrap();

    let tcp_conn = TcpStream::connect(&addr).unwrap();
    println!("Connected to {}", addr);

    let server_name = ServerName::try_from(HOST).expect("server name conversion failed").to_owned();

    let mut conn = TlsClient::new(tcp_conn, server_name, tls_config).unwrap();

    write!(conn, "GET /a/check HTTP/1.1\r\nHost: www.howsmyssl.com\r\n\r\n").unwrap();

    let mut pt = vec![];
    conn.read_to_end(&mut pt).unwrap();
    println!("{}", String::from_utf8(pt).unwrap());
    
}
