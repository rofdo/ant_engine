use std::sync::mpsc::Receiver;
use std::net::SocketAddr;

pub fn game_server(udp_addr: SocketAddr, stop: Receiver<()>) {
    let udp_socket = std::net::UdpSocket::bind(udp_addr).unwrap();
    udp_socket.set_nonblocking(true).unwrap();
    let mut buf = [0; 1024];
    while stop.try_recv().is_err() {
        match udp_socket.recv_from(&mut buf) {
            Ok((size, src)) => {
                let message = &buf[..size];
                println!("Received: {:?}", String::from_utf8_lossy(message));
                udp_socket.send_to(message, src).expect("Failed to send a response");
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // No data received, just continue the loop
                continue;
            }
            Err(e) => {
                eprintln!("recv_from error: {}", e);
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;
    use crate::utils::ADDRESSES;

    #[test]
    fn test_game_server() {
        let (tx, rx) = channel();
        let udp_addr: SocketAddr = ADDRESSES[0].parse().unwrap();
        let handle = std::thread::spawn(move || {
            game_server(udp_addr, rx);
        });

        let udp_socket = std::net::UdpSocket::bind(ADDRESSES[1]).unwrap();
        udp_socket.send_to(b"Hello, world!", udp_addr).unwrap();

        std::thread::sleep(std::time::Duration::from_secs(1));
        tx.send(()).unwrap();
        handle.join().unwrap();
    }
}
