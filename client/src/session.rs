use mosh::{ClientFrame, ServerFrame};
use std::{
    io::Write,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::Arc,
};
use tokio::net::UdpSocket;

#[derive(Debug, Clone)]
pub struct Session {
    pub id: mosh::session::ID,
    socket: Arc<UdpSocket>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io: {0}")]
    IO(#[from] std::io::Error),
    #[error("json: {0}")]
    JSON(#[from] serde_json::Error),
}

async fn send(socket: &UdpSocket, frame: ClientFrame) -> Result<(), Error> {
    let bytes = serde_json::to_vec(&frame)?;
    tracing::debug!("Sending frame: {:?}, bytes: {:?}", frame, bytes);
    socket.send(&bytes).await?;
    Ok(())
}

async fn recv(socket: &UdpSocket, buf: &mut [u8]) -> Result<ServerFrame, Error> {
    let n = socket.recv(buf).await?;
    let frame: ServerFrame = serde_json::from_slice(&buf[..n])?;
    tracing::debug!("Received frame: {:?}", frame);
    Ok(frame)
}

impl Session {
    #[tracing::instrument]
    pub async fn create(address: SocketAddr) -> Result<Self, Error> {
        let socket = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)).await?;
        socket.connect(address).await?;
        send(&socket, ClientFrame::NewSession {}).await?;
        let mut buf = vec![0; 1024];
        let session_id = match recv(&socket, &mut buf).await? {
            ServerFrame::NewSessionAck { session_id } => session_id,
            _ => panic!("received unexpected frame"),
        };
        tracing::info!("Connected with Session ID: {}", session_id);

        Ok(Self {
            socket: Arc::new(socket),
            id: session_id,
        })
    }

    pub async fn run(&self) -> Result<(), Error> {
        const BUFFER_SIZE: u16 = 65535;
        let mut buf = vec![0; BUFFER_SIZE as usize];
        let mut term = console::Term::stdout();
        loop {
            let frame = recv(&self.socket, &mut buf).await?;
            match frame {
                ServerFrame::UpdateState { state } => {
                    term.write(&state)?;
                    term.flush().unwrap();
                }
                _ => todo!(),
            };
        }
    }

    pub async fn send(&self, frame: ClientFrame) -> Result<(), Error> {
        send(&self.socket, frame).await
    }
}
