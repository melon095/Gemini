use crate::network::NetworkError;
use rustls::pki_types::ServerName;
use rustls::ClientConnection;
use std::io;
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::Arc;

#[derive(Debug)]
pub struct TlsClient {
    socket: TcpStream,
    client_connection: ClientConnection,
    #[allow(dead_code)]
    sni: ServerName<'static>,
}

impl TlsClient {
    fn new(
        socket: TcpStream,
        server_name: ServerName<'static>,
        tls_config: Arc<rustls::ClientConfig>,
    ) -> Result<Self, NetworkError> {
        Ok(Self {
            socket: socket,
            client_connection: rustls::ClientConnection::new(tls_config, server_name.clone())?,
            sni: server_name,
        })
    }

    pub fn new_from_host(
        addr: (&str, u16),
        tls_config: Arc<rustls::ClientConfig>,
    ) -> Result<Self, NetworkError> {
        // NOTE: Does not accept ToSocketAddrs, as we need to know domain.
        let host = addr.0;
        let port = addr.1;
        let addr = format!("{}:{}", host, port)
            .to_socket_addrs()?
            .next()
            .ok_or(NetworkError::InvalidAddress)?;

        let tcp = TcpStream::connect(addr)?;
        let server_name = ServerName::try_from(host)
            .map_err(|_| NetworkError::InvalidAddress)?
            .to_owned();

        Self::new(tcp, server_name, tls_config)
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
