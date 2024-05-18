use ant_engine_shared::game::Game;
use ant_engine_shared::networking::Command;
use std::io::Write;

fn main() {
    let game_handler = Game::start();
    // bind udp socket
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").unwrap();
    println!("Socket bound to {}", socket.local_addr().unwrap());
    println!("Enter the address of the server: ");
    let mut address = String::new();
    std::io::stdin().read_line(&mut address).unwrap();
    // lisen for keepresses
    loop {
        println!("Enter a number or a command: ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        let trimmed = input.trim();
        match trimmed {
            "stop" => {
                game_handler.stop().unwrap();
                break;
            }
            "view" => {
                let state = game_handler.last_state().unwrap();
                println!("State: {:?}", state);
            }
            _ => {
                let num: i32 = match trimmed.parse() {
                    Ok(num) => num,
                    Err(_) => {
                        println!("Not a valid number or command");
                        continue;
                    }
                };
                game_handler.send(Command::Add(num)).unwrap();
                // send commands to server
                let command = Command::Add(num);
                let serialized = bincode::serialize(&command).unwrap();
                let sent = socket.send_to(&serialized, address.trim()).unwrap();
                println!("Sent {} bytes to {}", sent, address.trim());

            }
        }
    }
}
