mod network;

use std::io::{Read, Write};
use crate::network::tls_config::make_tls_config;
use crate::network::tls_client::TlsClient;

fn main() {
    const HOST: &'static str = "geminiprotocol.net";
    const PORT: u16 = 1965;
    // const PORT: u16 = 443;

    let tls_config = make_tls_config().unwrap();

    let mut conn = TlsClient::new_from_host((HOST, PORT), tls_config).unwrap();

    write!(conn, "gemini://geminiprotocol.net/\r\n").unwrap();

    let mut pt = vec![];
    conn.read_to_end(&mut pt).unwrap();
    println!("{}", String::from_utf8(pt).unwrap());
}
