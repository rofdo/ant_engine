use log::{debug, error, info};

use crate::server::game_server::game_server;
use std::net::SocketAddr;
use std::sync::mpsc::Receiver;

pub fn lobby_code(
    tcp_addr: SocketAddr,
    udp_addr: SocketAddr,
    stop: Receiver<()>,
) -> Result<(), Box<dyn std::error::Error>> {
    // open game server thread
    let (tx, rx) = std::sync::mpsc::channel();
    let game_server = std::thread::spawn(move || {
        game_server(udp_addr, rx);
    });

    // open tcp
    let tcp_listener = std::net::TcpListener::bind(tcp_addr)?;
    while stop.try_recv().is_err() {
        let streams = tcp_listener.incoming().take_while(|v| v.is_ok());
    }
    tx.send(()).unwrap();
    info!("Waiting for game server to finish");
    game_server
        .join()
        .expect("Failed to join game server thread");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::ADDRESSES;
    use std::sync::mpsc::channel;

    #[test]
    fn test_lobby_code() {
        env_logger::init();
        let (tx, rx) = channel();
        let tcp_addr: SocketAddr = ADDRESSES[5].parse().unwrap();
        let udp_addr: SocketAddr = ADDRESSES[3].parse().unwrap();
        let handle = std::thread::spawn(move || {
            lobby_code(tcp_addr, udp_addr, rx).unwrap();
        });
        std::thread::sleep(std::time::Duration::from_secs(1));
        let tcp_stream = std::net::TcpStream::connect(ADDRESSES[5]).unwrap();
        std::thread::sleep(std::time::Duration::from_secs(1));
        tx.send(()).unwrap();
        handle.join().unwrap();
    }
}
