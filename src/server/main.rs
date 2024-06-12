use std::net::SocketAddr;

use ant_engine::server::Distributer;
use log::info;

fn main() {
    env_logger::init();
    info!("Starting the game server");

    let ports: [u16; 10] = [
        3000, 3001, 3002, 3003, 3004, 3005, 3006, 3007, 3008, 3009,
    ];
    let addresses: Vec<SocketAddr> = ports.iter().map(|port| format!("0.0.0.0:{}", port).parse().unwrap()).collect();

    let mut distributer = Distributer::new(addresses);
    distributer.run();
}
