use std::net::SocketAddr;
use std::sync::mpsc::{Receiver, Sender};

struct Distributer {
    free: Vec<SocketAddr>,
    main: SocketAddr,
    lobbies: Vec<(std::thread::JoinHandle<()>, Sender<()>)>,
}

impl Distributer {
    fn new(main: SocketAddr) -> Distributer {
        Distributer {
            free: Vec::new(),
            main,
            lobbies: Vec::new(),
        }
    }

    fn add_free_socket(&mut self, addr: SocketAddr) {
        self.free.push(addr);
    }

    fn try_open_lobby(&mut self) -> Result<usize, Box<dyn std::error::Error>> {
        if self.free.len() < 2 {
            return Err("Not enough free sockets".into());
        }
        let tcp_socket = self.free.pop().unwrap();
        let udp_socket = self.free.pop().unwrap();

        let (tx, rx) = std::sync::mpsc::channel();
        let handle = std::thread::spawn(move || {
            lobby_code(tcp_socket, udp_socket, rx);
        });

        self.lobbies.push((handle, tx));

        Ok(self.lobbies.len() - 1)
    }
}

fn lobby_code(tcp_addr: SocketAddr, udp_addr: SocketAddr, receiver: Receiver<()>) {
    while receiver.try_recv().is_err() {
        // read udp
        let udp_socket = std::net::UdpSocket::bind(udp_addr).unwrap();
        let mut buf = [0; 1024];
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
mod server_tests {
    use super::*;
    #[test]
    fn test_distributer() {
        let main = "0.0.0.0:0";
    }
}
