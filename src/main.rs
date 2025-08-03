use std::net::SocketAddr;

use clap::{Parser, Subcommand};
use client::start_client;
use server::start_server;

mod client;
mod server;

#[derive(Parser)]
#[command(version, about, long_about=None)]
struct Cli {
    #[command(subcommand)]
    mode: Mode,
}

#[derive(Subcommand, Debug)]
enum Mode {
    Client { from: SocketAddr, to: SocketAddr },
    Server { from: SocketAddr, to: SocketAddr },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match cli.mode {
        Mode::Server { from, to } => start_server(from, to).await.unwrap(),
        Mode::Client { from, to } => start_client(from, to).await.unwrap(),
    };
}
