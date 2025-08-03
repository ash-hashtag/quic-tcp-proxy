use std::{net::SocketAddr, time::Duration};

use quinn::{
    Endpoint, Incoming, RecvStream, SendStream, ServerConfig,
    rustls::pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject},
};
use tokio::net::TcpStream;

pub async fn start_server(serve_at: SocketAddr, forward_to: SocketAddr) -> anyhow::Result<()> {
    let private_key = PrivateKeyDer::from_pem_file("./certs/key.pem")?;
    let certificate = CertificateDer::from_pem_file("./certs/cert.pem")?;

    let server_config = ServerConfig::with_single_cert(vec![certificate], private_key)?;

    let endpoint = Endpoint::server(server_config, serve_at)?;

    println!("Server listening on {}", serve_at);

    while let Some(conn) = endpoint.accept().await {
        tokio::spawn(async move {
            let addr = conn.remote_address();
            println!("New Connection {}", addr);
            if let Err(err) = handle_incoming(conn, forward_to).await {
                println!("Connection Ended {} with {}", addr, err);
            } else {
                println!("Connection Ended {}", addr);
            }
        });
    }

    Ok(())
}

pub async fn handle_incoming(conn: Incoming, forward_to: SocketAddr) -> anyhow::Result<()> {
    let conn = conn.await?;

    loop {
        let conn = conn.clone();
        match conn.accept_bi().await {
            Ok((send, recv)) => {
                tokio::spawn(new_tcp_conn(forward_to, send, recv));
            }
            Err(err) => {
                eprintln!("{}", err);
                break;
            }
        }
    }
    Ok(())
}

async fn new_tcp_conn(
    forward_to: SocketAddr,
    mut send: SendStream,
    mut recv: RecvStream,
) -> anyhow::Result<()> {
    let tcp = TcpStream::connect(forward_to).await?;
    let sock_ref = socket2::SockRef::from(&tcp);
    let mut ka = socket2::TcpKeepalive::new();
    ka = ka.with_time(Duration::from_secs(30));
    ka = ka.with_interval(Duration::from_secs(30));
    sock_ref.set_tcp_keepalive(&ka)?;

    let (mut tcp_read, mut tcp_write) = tcp.into_split();

    let f1 = tokio::io::copy(&mut tcp_read, &mut send);
    let f2 = tokio::io::copy(&mut recv, &mut tcp_write);

    let _ = futures::future::join(f1, f2).await;
    Ok(())
}
