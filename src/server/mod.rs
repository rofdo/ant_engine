use crate::shared::game::GameCommand;
use bincode;
use serde::{Deserialize, Serialize};
use std::net::UdpSocket;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use log::warn;

#[derive(Debug, Serialize, Deserialize, Clone)]
enum Message {
    NewConnection,
    Alive,
    Terminate,
    Command(GameCommand),
}

struct Networker {
    address: String,
    // Sends all commands received from clients
    command_sender: Sender<GameCommand>,
    // Distributes all received commands to clients
    command_receiver: Receiver<GameCommand>,
}

pub struct NetworkerHandle {
    thread: thread::JoinHandle<()>,
    command_receiver: Receiver<GameCommand>,
    commnad_sender: Sender<GameCommand>,
}

impl NetworkerHandle {
    pub fn new(address: String) -> NetworkerHandle {
        let (client_command_sender, server_command_receiver) = mpsc::channel();
        let (server_command_sender, client_command_receiver) = mpsc::channel();
        let networker = Networker {
            address,
            command_sender: client_command_sender,
            command_receiver: client_command_receiver,
        };
        let thread = thread::spawn(move || {
            networker.run();
        });
        NetworkerHandle {
            thread,
            command_receiver: server_command_receiver,
            commnad_sender: server_command_sender,
        }
    }

    pub fn receive_commands(&self) -> Vec<GameCommand> {
        let mut commands = Vec::new();
        while let Ok(command) = self.command_receiver.try_recv() {
            commands.push(command);
        }
        commands
    }
}

impl Networker {
    fn run(&self) {
        let socket = UdpSocket::bind(&self.address).expect("Failed to bind socket");
        socket
            .set_nonblocking(true)
            .expect("Failed to set non-blocking");
        let mut buffer = [0; 1024];
        loop {
            match socket.recv_from(&mut buffer) {
                Ok((size, _)) => {
                    let message: Message = bincode::deserialize(&buffer[..size]).unwrap();
                    self.handle_message(message);
                }
                Err(_) => {}
            }
        }
    }

    fn handle_message(&self, message: Message) {
        match message {
            Message::NewConnection => {}
            Message::Alive => {}
            Message::Terminate => {}
            Message::Command(command) => {
                self.command_sender.send(command).unwrap();
            }
        }
    }
}

pub fn start_server(adress: String) {
    let networker = NetworkerHandle::new(adress);
    let game = crate::shared::game::GameHandle::new();
    let mut running = true;
    while running {
        let commands = networker.receive_commands();
        for command in commands {
            if command == GameCommand::Quit {
                running = false;
            }
            match game.send_command(command) {
                Ok(_) => (),
                Err(e) => warn!("Error sending command: {}", e),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::exit;

    #[test]
    fn test_networker() {
        let address = "127.0.0.1:12345".to_string();
        let networker = NetworkerHandle::new(address);
    }

    #[test]
    fn test_start_server() {
        let address = "127.0.0.1.12345".to_string();
        start_server(address);
        // send quit message
        let socket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind socket");
    }
}
