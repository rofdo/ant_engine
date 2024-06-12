use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Serialize, Deserialize, Debug)]
pub enum DistributorClientMessages {
    AskForLobbies,
    OpenLobby,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum DistributorServerMessages {
    Lobbies(Vec<SocketAddr>),
    LobbyOpened(SocketAddr),
}
