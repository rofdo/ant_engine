use crate::game::Command;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Message {
    command: Command,
    step: u32,
}

// spawns a new thread that writes all received commands into the given channel
fn subscribe_to_socket(socket: std::net::UdpSocket, command_tx: std::sync::mpsc::Sender<Message>) {
    std::thread::spawn(move || {
        let mut buf = [0; 1024];
        loop {
            match socket.recv_from(&mut buf) {
                Ok((n, _)) => {
                    let command = std::str::from_utf8(&buf[..n]).unwrap();
                    let message = Message {
                        command: Command::Add(command.parse().unwrap()),
                        step: 0,
                    };
                    command_tx.send(message).unwrap();
                }
                Err(e) => {
                    eprintln!("Error receiving data: {}", e);
                    break;
                }
            }
        }
    });
}
