mod pty;
mod session;

use jaw::{ClientFrame, ServerFrame, PORT};
use session::Session;
use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::Arc,
};
use tokio::net::UdpSocket;

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("session: {0}")]
    IO(#[from] std::io::Error),
    #[error("session: {0}")]
    Session(#[from] session::Error),
    #[error("json: {0}")]
    JSON(#[from] serde_json::Error),
}

struct Server {
    sessions: session::Store,
    socket: Arc<UdpSocket>,
}

impl Server {
    pub async fn new() -> Result<Self, Error> {
        let socket = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, PORT)).await?;
        Ok(Self {
            sessions: session::Store::default(),
            socket: Arc::new(socket),
        })
    }

    pub async fn run(self) -> Result<(), Error> {
        let mut buf = vec![0; 1024];
        loop {
            let (n, from) = self.socket.recv_from(&mut buf).await?;
            if n == 0 {
                tracing::debug!("received empty packet, skipping");
                continue;
            }
            let bytes = &buf[..n];
            let frame = serde_json::from_slice(bytes)?;
            tracing::debug!("Received: {:?}", frame);
            if let Some(frame) = self.handle(frame, from).await? {
                tracing::debug!("Sending response: {:?}", frame);
                self.socket
                    .send_to(&serde_json::to_vec(&frame)?, from)
                    .await?;
            }
        }
    }

    #[tracing::instrument(skip(self, frame))]
    async fn handle(
        &self,
        frame: ClientFrame,
        from: SocketAddr,
    ) -> Result<Option<ServerFrame>, Error> {
        let frame = match frame {
            ClientFrame::NewSession {} => {
                let session_id = jaw::session::ID::new_v4();
                let session = Session::new(self.socket.clone(), from).await?;
                self.sessions.insert(session_id, session.tx.clone());
                tracing::info!(id = %session_id, "Created new session");
                tokio::spawn(async move { session.run().await.expect("session error") }); // TODO: remove session there
                Some(ServerFrame::NewSessionAck { session_id })
            }
            ClientFrame::Write { session_id, bytes } => {
                let session = self.sessions.get(&session_id).unwrap();
                session.send(bytes).await.unwrap();
                None
            }
        };
        Ok(frame)
    }
}

fn init_logging() {
    const LOG_ENV: &str = "RUST_LOG";
    use std::str::FromStr;
    use tracing::Level;
    use tracing_subscriber::EnvFilter;

    let filter = std::env::var(LOG_ENV)
        .map(|env| {
            EnvFilter::from_str(env.to_uppercase().as_str())
                .unwrap_or_else(|err| panic!("invalid `{}` environment variable {}", LOG_ENV, err))
        })
        .unwrap_or(EnvFilter::default().add_directive(Level::INFO.into()));

    tracing_subscriber::fmt().with_env_filter(filter).init();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();
    let server = Server::new().await?;
    tracing::info!("Starting server");
    server.run().await.unwrap();
    Ok(())
}
