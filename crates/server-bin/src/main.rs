pub mod config;

use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::{
    io::BufReader,
    net::{TcpListener, TcpStream},
};
use tokio_rustls::TlsAcceptor;

#[cfg(target_os = "xd")]
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());

    let module = Module::from_file(&engine, "test.wasm").unwrap();
    let memory = Memory::new(&mut store, MemoryType::new(2, None)).unwrap();

    let mut linker = Linker::new(&engine);
    linker.define(&store, "env", "memory", memory).unwrap();
    linker
        .func_wrap("env", "sleep", |duration: i32| {
            println!("Sleeping for {} seconds", duration);

            std::thread::sleep(Duration::from_secs(duration as u64));

            println!("Done sleeping");
        })
        .unwrap();

    let instance = linker.instantiate(&mut store, &module).unwrap();

    println!("Wasm module executed successfully!");

    // run main in the wasm module
    let main = instance
        .get_typed_func::<(), i32>(&mut store, "_start")
        .unwrap();

    main.call(&mut store, ()).unwrap();
}

// https://github.com/rustls/tokio-rustls/blob/main/tests/certs/main.rs
use crate::config::{read_and_parse_config, GetProperty};
use rcgen::{
    BasicConstraints, CertificateParams, DistinguishedName, DnType, ExtendedKeyUsagePurpose, IsCa,
    KeyPair, KeyUsagePurpose,
};
use rustls::internal::msgs::handshake::ServerExtension;
use rustls::Side::Server;
use std::fs::File;
use std::io::Write;
use std::str::FromStr;

// TODO: Remove :)
fn regenerate_certs(domain: String) {
    let root_key = KeyPair::generate().unwrap();
    let root_ca = issuer_params("asdasd").self_signed(&root_key).unwrap();

    let mut root_file = File::create("root.pem").unwrap();
    root_file.write_all(root_ca.pem().as_bytes()).unwrap();

    let intermediate_key = KeyPair::generate().unwrap();
    let intermediate_ca = issuer_params("asdasd - 2")
        .signed_by(&intermediate_key, &root_ca, &root_key)
        .unwrap();

    let end_entity_key = KeyPair::generate().unwrap();
    let mut end_entity_params = CertificateParams::new(vec![domain]).unwrap();
    end_entity_params.is_ca = IsCa::ExplicitNoCa;
    end_entity_params.extended_key_usages = vec![
        ExtendedKeyUsagePurpose::ServerAuth,
        ExtendedKeyUsagePurpose::ClientAuth,
    ];
    let end_entity = end_entity_params
        .signed_by(&end_entity_key, &intermediate_ca, &intermediate_key)
        .unwrap();

    let mut chain_file = File::create("cert.pem").unwrap();
    chain_file.write_all(end_entity.pem().as_bytes()).unwrap();
    chain_file
        .write_all(intermediate_ca.pem().as_bytes())
        .unwrap();

    let mut key_file = File::create("key.key").unwrap();
    key_file
        .write_all(end_entity_key.serialize_pem().as_bytes())
        .unwrap();
}

fn issuer_params(common_name: &str) -> CertificateParams {
    let mut issuer_name = DistinguishedName::new();
    issuer_name.push(DnType::CommonName, common_name);
    let mut issuer_params = CertificateParams::default();
    issuer_params.distinguished_name = issuer_name;
    issuer_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    issuer_params.key_usages = vec![
        KeyUsagePurpose::KeyCertSign,
        KeyUsagePurpose::DigitalSignature,
    ];
    issuer_params
}

#[derive(Debug, Clone)]
struct GlobalState {
    tls_config: Arc<rustls::ServerConfig>,
}

type GlobalStateArc = Arc<GlobalState>;

const MAX_REQUEST_SIZE: usize = 1024;

async fn handle_client_request(
    conn: TlsConnection,
    global_state: GlobalStateArc,
) -> anyhow::Result<()> {
    log::info!("Accepted connection from {:?}", conn.addr);

    let mut stream = conn.acceptor.accept(conn.socket).await?;

    let mut line_reader = BufReader::new(stream);

    loop {
        let mut req = String::new();

        match line_reader.read_line(&mut req).await {
            Ok(0) => {
                log::info!("Connection closed by client");
                break;
            }
            Ok(_) => {}
            Err(e) => {
                log::error!("Failed to read from socket; error = {:?}", e);
                break;
            }
        }

        if req.is_empty() {
            log::debug!("Empty request; closing connection");

            break;
        }
        if req.len() > MAX_REQUEST_SIZE {
            log::warn!("Request too large: {:?}", req);
            todo!("Handle this error");
            break;
        }

        log::debug!("Received request: {:?}", req);

        let resp = "20 text/gemini\r\n# Hi :)\r\n=> /index.gmi\r\n".to_string();

        let stream = line_reader.get_mut();
        stream.write_all(&resp.as_bytes()).await?;
        stream.shutdown().await?;
    }

    Ok(())
}

struct TlsConnection {
    socket: TcpStream,
    addr: SocketAddr,
    acceptor: TlsAcceptor,
}

fn make_tls_config(cert: PathBuf, key: PathBuf) -> anyhow::Result<Arc<rustls::ServerConfig>> {
    // TODO: TLS Certificates should be runtime configurable

    let certs = CertificateDer::pem_file_iter(cert)
        .expect("Failed to read certificate")
        .collect::<Result<Vec<_>, _>>()?;
    let key = PrivateKeyDer::from_pem_file(key).expect("Failed to read private key");

    let config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .expect("Failed to create TLS config");

    Ok(Arc::new(config))
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();

    let config = {
        let path = if std::env::args().len() > 1 {
            PathBuf::from_str(&std::env::args().nth(1).unwrap())
        } else {
            PathBuf::from_str("config.cfg")
        }
        .expect("Failed to parse config file path");

        let contents = std::fs::read_to_string(path).expect("Failed to read config file");

        contents
    };
    // config.get_property_of_string("port");
    // config.get_block("vhost")
    //     .get_property_of_string("tls_cert");
    //
    // config.get_block_where("vhost", |is| is.arg_is("host", "localhost"));
    // config.get_block_where("vhost", |is| is.arg_is("host", "localhost"))
    //     .get_property_of_string("tls_cert");

    let config = read_and_parse_config(&config).unwrap();
    let port = config.get_property_of_number("port").unwrap();

    let domain = "localhost".to_string();
    regenerate_certs(domain);

    let cert = PathBuf::from("cert.pem");
    let key = PathBuf::from("key.key");

    let tls_config = make_tls_config(cert, key).expect("Failed to create TLS config");
    let global_state = Arc::new(GlobalState { tls_config });

    let tcp_listener = TcpListener::bind(("[::]", port as u16))
        .await
        .expect("Failed to bind to port");

    log::info!(
        "Listening on: {}",
        tcp_listener.local_addr().expect("Failed to get local addr")
    );

    loop {
        let (socket, addr) = match tcp_listener.accept().await {
            Ok((socket, addr)) => (socket, addr),
            Err(e) => {
                log::error!("Failed to accept connection; error = {:?}", e);
                continue;
            }
        };

        let global_state = global_state.clone();

        tokio::spawn(async move {
            let socket = TlsConnection {
                socket,
                addr,
                acceptor: TlsAcceptor::from(global_state.tls_config.clone()),
            };

            if let Err(e) = handle_client_request(socket, global_state).await {
                log::error!("failed to handle client request; error = {:?}", e);
            }
        });
    }
}
