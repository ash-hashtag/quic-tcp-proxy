use std::net::SocketAddr;

use quinn::{
    Endpoint, Incoming, ServerConfig,
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
            Ok((mut send, mut recv)) => {
                tokio::spawn(async move {
                    println!("QUIC: {} -> TCP: {}", conn.remote_address(), forward_to);
                    match TcpStream::connect(forward_to).await {
                        Ok(tcp) => {
                            let (mut tcp_read, mut tcp_write) = tcp.into_split();

                            let f1 = tokio::io::copy(&mut tcp_read, &mut send);
                            let f2 = tokio::io::copy(&mut recv, &mut tcp_write);

                            let (bytes_sent, bytes_received) = futures::future::join(f1, f2).await;

                            println!(
                                "QUIC: {} -> TCP: {}, bytes sent: {:?}, bytes received: {:?}",
                                conn.remote_address(),
                                forward_to,
                                bytes_sent,
                                bytes_received,
                            );
                        }
                        Err(err) => {
                            eprintln!("{}", err)
                        }
                    }
                });
            }
            Err(err) => {
                eprintln!("{}", err);
                break;
            }
        }
    }
    Ok(())
}
