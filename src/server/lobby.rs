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
        let (stream, _) = match tcp_listener.accept() {
            Ok((stream, addr)) => {
                println!("New TCP connection: {:?}", addr);
                (stream, addr)
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // No new connections, just continue the loop
                continue;
            }
            Err(e) => {
                println!("TCP accept error: {}", e);
                continue;
            }
        };

        // handle the stream
        let mut buf = [0; 1024];
    }
    panic!("Not implemented");
    tx.send(())?;
    return Ok(());
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
        std::thread::sleep(std::time::Duration::from_secs(1));
        tx.send(()).unwrap();
        tx.send(()).unwrap();
        tx.send(()).unwrap();
        tx.send(()).unwrap();
        tx.send(()).unwrap();
        tx.send(()).unwrap();
        handle.join().unwrap();
    }
}
