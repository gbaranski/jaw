use dashmap::DashMap;
use mosh::session::ID;
use parking_lot::Mutex;
use std::sync::Arc;

pub type Store = DashMap<ID, Session>;

#[derive(Debug, Clone)]
pub struct Session {
    pub state: Arc<Mutex<Vec<u8>>>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {}

impl Session {
    pub async fn new() -> Result<Self, Error> {
        Ok(Self {
            state: Default::default(),
        })
    }
}
