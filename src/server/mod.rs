#![allow(dead_code)]
pub mod game_server;
mod lobby;

use crate::shared::protocols::{DistributorClientMessages, DistributorServerMessages};
use lobby::lobby_code;
use std::net::SocketAddr;
use std::sync::mpsc::Sender;
use std::io::Write;

pub struct Distributer {
    free: Vec<SocketAddr>,
    main: SocketAddr,
    lobbies: Vec<(std::thread::JoinHandle<()>, Sender<()>, SocketAddr)>,
}

impl Distributer {
    pub fn new(addresses: Vec<SocketAddr>) -> Distributer {
        let main = addresses[0];
        let free = addresses[1..].to_vec();
        Distributer {
            free,
            main,
            lobbies: Vec::new(),
        }
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

        self.lobbies.push((handle, tx, tcp_socket));

        Ok(self.lobbies.len() - 1)
    }

    pub fn run(self) {
        // listen on the main socket for new connections over tcp
        let listener = std::net::TcpListener::bind(self.main).unwrap();
        let thread_safe_self = std::sync::Arc::new(std::sync::Mutex::new(self));
        while let Ok((mut stream, _)) = listener.accept() {
            let thread_safe_clone = thread_safe_self.clone();
            let _ = std::thread::spawn(move || {
                let distributer = thread_safe_clone;
                while let Ok(message) = bincode::deserialize_from(stream.try_clone().unwrap()) {
                    match message {
                        DistributorClientMessages::AskForLobbies => {
                            let lock = distributer.lock().unwrap();
                            let lobbies = lock.lobbies.iter().map(|(_, _, addr)| *addr).collect();
                            drop(lock);

                            // send the lobbies to the client
                            let message = DistributorServerMessages::Lobbies(lobbies);
                            stream.write_all(&bincode::serialize(&message).unwrap()).unwrap();
                        }
                        DistributorClientMessages::OpenLobby => {
                            let mut lock = distributer.lock().unwrap();
                            let lobby = match lock.try_open_lobby() {
                                Ok(lobby) => lobby,
                                Err(e) => {
                                    eprintln!("Error opening lobby: {}", e);
                                    continue;
                                }
                            };
                            let message = DistributorServerMessages::LobbyOpened(lock.lobbies[lobby].2);
                            drop(lock);
                            stream.write_all(&bincode::serialize(&message).unwrap()).unwrap();
                        }
                    }
                }
            });
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
