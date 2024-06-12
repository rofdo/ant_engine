#![allow(dead_code)]
mod lobby;
mod game_server;

use std::net::SocketAddr;
use std::sync::mpsc::Sender;
use lobby::lobby_code;

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

#[cfg(test)]
mod server_tests {
    use super::*;
    #[test]
    fn test_distributer() {
        let main = "0.0.0.0:0";
    }
}
