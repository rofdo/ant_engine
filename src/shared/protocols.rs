use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub enum DistributorClientMessages {
    AskForLobbies,
    OpenLobby,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub enum DistributorServerMessages {
    Lobbies(Vec<SocketAddr>),
    LobbyOpened(SocketAddr),
}
