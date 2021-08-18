use crate::pty;
use dashmap::DashMap;
use mosh::{session::ID, ServerFrame};
use std::{net::SocketAddr, sync::Arc};
use tokio::{
    io::{
        AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader, BufWriter,
    },
    net::UdpSocket,
    process::{ChildStderr, ChildStdin, ChildStdout, Command},
    sync::mpsc,
};

pub type Store = DashMap<ID, mpsc::Sender<Vec<u8>>>;

// #[derive(Debug)]
pub struct Session {
    pub tx: mpsc::Sender<Vec<u8>>,
    rx: mpsc::Receiver<Vec<u8>>,
    pty: pty::System,
    socket: Arc<UdpSocket>,
    address: SocketAddr,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io: {0}")]
    IO(#[from] std::io::Error),
    #[error("IO has been closed")]
    Closed,
    #[error("pty: {0}")]
    Pty(String),
}

impl Session {
    pub async fn new(socket: Arc<UdpSocket>, address: SocketAddr) -> Result<Self, Error> {
        let pty = pty::System::new(Command::new("bash"))?;
        let size = pty::Size { col: 80, row: 24 };
        pty.resize(size).unwrap();
        dbg!(&pty);

        let (tx, rx) = mpsc::channel(32);
        Ok(Self {
            address,
            pty,
            socket,
            rx,
            tx,
        })
    }

    pub async fn run(self) -> Result<(), Error> {
        let Self {
            rx,
            pty,
            socket,
            address,
            ..
        } = self;
        let (child_rx, child_tx) = tokio::io::split(pty);
        let write_task =
            tokio::spawn(async move { Self::write(child_tx, rx).await.expect("stdin error") });
        let read_task = tokio::spawn(async move {
            Self::read(child_rx, socket, address)
                .await
                .expect("stderr error")
        });
        tokio::select! {
            value = read_task => value.unwrap(),
            value = write_task => value.unwrap(),
        };
        Ok(())
    }

    #[tracing::instrument(err, skip(tx, rx))]
    async fn write(
        mut tx: impl AsyncWrite + Unpin,
        mut rx: mpsc::Receiver<Vec<u8>>,
    ) -> Result<(), Error> {
        loop {
            let bytes = rx.recv().await.ok_or(Error::Closed)?;
            tx.write(&bytes).await.unwrap();
            tx.flush().await.unwrap();
            tracing::info!("Wrote {} bytes to stdin", bytes.len());
        }
    }

    #[tracing::instrument(err, skip(rx))]
    async fn read(
        mut rx: impl AsyncRead + Unpin,
        socket: Arc<UdpSocket>,
        address: SocketAddr,
    ) -> Result<(), Error> {
        let mut buf = vec![0; 1024];
        loop {
            let n = rx.read(&mut buf).await.unwrap();
            if n == 0 {
                return Err(Error::Closed);
            }
            let line = &buf[..n];
            socket
                .send_to(
                    &serde_json::to_vec(&ServerFrame::UpdateState {
                        state: line.to_vec(),
                    })
                    .unwrap(),
                    address,
                )
                .await?;
            tracing::info!("stdout: {:?}", line);
        }
    }
}
