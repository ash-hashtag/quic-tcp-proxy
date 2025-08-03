use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::Arc,
};

use quinn::{ClientConfig, Endpoint};
use rustls::{
    RootCertStore,
    pki_types::{CertificateDer, pem::PemObject},
};
use tokio::net::TcpListener;

pub async fn start_client(serve_at: SocketAddr, forward_to: SocketAddr) -> anyhow::Result<()> {
    let tcp_listener = TcpListener::bind(serve_at).await?;

    let cert = CertificateDer::from_pem_file("./certs/cert.pem")?;
    let mut roots = RootCertStore::empty();
    roots.add(cert)?;
    let client_cfg = ClientConfig::with_root_certificates(Arc::new(roots))?;

    let quic_endpoint =
        Endpoint::client(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)))?;
    let mut quic_conn = quic_endpoint
        .connect_with(client_cfg.clone(), forward_to, "localhost")?
        .await?;

    loop {
        match tcp_listener.accept().await {
            Ok((tcp_stream, addr)) => {
                println!("TCP: {} -> QUIC: {}", addr, quic_conn.remote_address());

                if quic_conn.close_reason().is_some() {
                    quic_conn = quic_endpoint
                        .connect_with(client_cfg.clone(), forward_to, "localhost")?
                        .await?;
                }

                let quicc_conn = quic_conn.clone();

                tokio::spawn(async move {
                    if let Ok((mut send, mut recv)) = quicc_conn.open_bi().await {
                        let (mut tcp_read, mut tcp_write) = tcp_stream.into_split();

                        let f1 = tokio::io::copy(&mut tcp_read, &mut send);
                        let f2 = tokio::io::copy(&mut recv, &mut tcp_write);

                        let (bytes_sent, bytes_received) = futures::future::join(f1, f2).await;
                        println!(
                            "TCP: {} -> QUIC: {}, bytes sent: {:?}, bytes recevied: {:?}",
                            addr,
                            quicc_conn.remote_address(),
                            bytes_sent,
                            bytes_received
                        );
                    }
                });
            }
            Err(err) => {
                eprintln!("{}", err);
                break;
            }
        }
    }

    // let (mut send, mut recv) = quic_conn.open_bi().await?;
    // for i in 0..10 {
    //     let n = send
    //         .write(format!("I am the client {i}").as_bytes())
    //         .await?;
    //     println!("Bytes written {n}");
    //     let mut buf = vec![0u8; 1024];
    //     let n = recv.read(&mut buf).await?.unwrap_or(buf.len());
    //     println!("Received {}", String::from_utf8(buf[0..n].to_vec())?);
    // }

    // let payload = recv.read_to_end(1024).await.unwrap();
    // println!("Received {}", String::from_utf8(payload).unwrap());

    Ok(())
}
