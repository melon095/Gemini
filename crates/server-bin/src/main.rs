pub mod config;
mod tls_store;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
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
use crate::config::{read_and_parse_config, Config, GetProperty};
use crate::tls_store::make_tls_config;
use rcgen::{
    BasicConstraints, CertificateParams, DistinguishedName, DnType, ExtendedKeyUsagePurpose, IsCa,
    KeyPair, KeyUsagePurpose,
};
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

struct GlobalState<'a> {
    tls_config: Arc<rustls::ServerConfig>,
    config: Arc<Config<'a>>,
}

type GlobalStateArc<'a> = Arc<GlobalState<'a>>;

const MAX_REQUEST_SIZE: usize = 1024;

async fn handle_client_request<'a>(
    conn: TlsConnection,
    global_state: GlobalStateArc<'a>,
) -> anyhow::Result<()> {
    log::info!("Accepted connection from {:?}", conn.addr);

    // let (sni, valid, mut stream) = {
    //     let mut sni = None;
    //     let mut valid = false;
    let stream = conn
        .acceptor
        // .accept_with(conn.socket, |sc| {
        //     if let Some(server_name) = sc.server_name() {
        //         sni = Some(server_name.to_string());
        //         valid = global_state
        //             .config
        //             .get_blocks("vhost")
        //             .iter()
        //             .find(|block| {
        //                 block
        //                     .get_property_string("for")
        //                     .map_or(false, |s| s == server_name)
        //             })
        //             .is_some();
        //     }
        // })
        .accept(conn.socket)
        .await?;

    //     (sni.unwrap_or_default(), valid, stream)
    // };
    //
    // if !valid {
    //     log::warn!(
    //         "Invalid domain name: {:?}",
    //         stream.into_inner().1.server_name()
    //     );
    //     return Ok(());
    // }
    //
    // // TODO: Merge them
    // let vhost = global_state
    //     .config
    //     .get_blocks("vhost")
    //     .iter()
    //     .find(|block| block.get_property_string("for").map_or(false, |s| s == sni))
    //     .unwrap();

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

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();

    let config_str: &'static str = {
        let path = if std::env::args().len() > 1 {
            PathBuf::from_str(&std::env::args().nth(1).unwrap())
        } else {
            PathBuf::from_str("config.cfg")
        }
        .expect("Failed to parse config file path");

        let data = std::fs::read_to_string(path).expect("Failed to read config file");

        data.leak()
    };

    let config = Arc::new(read_and_parse_config(&config_str).unwrap());

    println!("{:#?}", &config);
    let port = config.get_property_number("port").unwrap();

    regenerate_certs("localhost".into());

    let tls_config = make_tls_config(&config)?;
    let global_state = Arc::new(GlobalState { config, tls_config });

    let tcp_listener = TcpListener::bind(format!("[::]:{port}"))
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
