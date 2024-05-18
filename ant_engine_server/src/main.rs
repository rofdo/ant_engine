use ant_engine_shared::game::Game;
use ant_engine_shared::networking::Command;
use std::net::UdpSocket;
use serde::{Deserialize, Serialize};

fn main() {
    let game_handler = Game::start();

    // Define the address to bind to
    let address = "127.0.0.1:12345";
    
    // Create a UDP socket
    let socket = UdpSocket::bind(address).expect("Failed to bind to address");
    println!("Listening on {}", address);

    // Buffer to store received data
    let mut buf = [0; 1024];

    loop {
        // Receive data
        match socket.recv_from(&mut buf) {
            Ok((n, src)) => {
                let command: Command = bincode::deserialize(&buf[..n]).unwrap();
                println!("Received command: {:?}", command);
            }
            Err(e) => {
                eprintln!("Error receiving data: {}", e);
                break;
            }
        }
    }
}

