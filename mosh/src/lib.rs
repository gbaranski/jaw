pub mod session;

pub const PORT: u16 = 7070;

use serde::{Serialize, Deserialize};

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerFrame {
    NewSessionAck {
        session_id: session::ID,
    },
    UpdateState {
        state: Vec<u8>,
    },
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientFrame {
    NewSession {},
    UpdateState {
        session_id: session::ID,
        state: Vec<u8>,
    },
}
