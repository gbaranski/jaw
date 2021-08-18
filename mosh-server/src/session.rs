use crate::pty;
use dashmap::DashMap;
use mosh::{session::ID, ServerFrame};
use std::{net::SocketAddr, sync::Arc};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter},
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

    pub async fn run(mut self) -> Result<(), Error> {
        let rx = self.rx;
        dbg!(&self.pty.child);
        let stdout = BufReader::new(self.pty.child.stdout.take().expect("open stdout fail"));
        let stderr = BufReader::new(self.pty.child.stderr.take().expect("open stderr fail"));
        let stdin = BufWriter::new(self.pty.child.stdin.take().expect("open stdin fail"));
        let stdin_task =
            tokio::spawn(async move { Self::write_stdin(stdin, rx).await.expect("stdin error") });
        let stdout_task = {
            let socket = self.socket.clone();
            let address = self.address.clone();
            tokio::spawn(async move {
                Self::read_stdout(stdout, socket, address)
                    .await
                    .expect("stdout error")
            })
        };
        let stderr_task = {
            let socket = self.socket.clone();
            let address = self.address.clone();
            tokio::spawn(async move {
                Self::read_stderr(stderr, socket, address)
                    .await
                    .expect("stderr error")
            })
        };
        tokio::select! {
            value = stdout_task => value.unwrap(),
            value = stderr_task => value.unwrap(),
            value = stdin_task => value.unwrap(),
        };
        Ok(())
    }

    #[tracing::instrument(err, skip(channel))]
    async fn write_stdin(
        mut stdin: BufWriter<ChildStdin>,
        mut channel: mpsc::Receiver<Vec<u8>>,
    ) -> Result<(), Error> {
        loop {
            let bytes = channel.recv().await.ok_or(Error::Closed)?;
            stdin.write(&bytes).await.unwrap();
            stdin.flush().await.unwrap();
            tracing::info!("Wrote {} bytes to stdin", bytes.len());
        }
    }

    #[tracing::instrument(err, skip(stdout))]
    async fn read_stdout(
        mut stdout: BufReader<ChildStdout>,
        socket: Arc<UdpSocket>,
        address: SocketAddr,
    ) -> Result<(), Error> {
        let mut buf = String::new();
        loop {
            let n = stdout.read_line(&mut buf).await.unwrap();
            if n == 0 {
                return Err(Error::Closed);
            }
            let line = &buf[..n];
            socket
                .send_to(
                    &serde_json::to_vec(&ServerFrame::UpdateState {
                        state: line.as_bytes().to_vec(),
                    })
                    .unwrap(),
                    address,
                )
                .await?;
            tracing::info!("stdout: {:?}", line);
        }
    }

    #[tracing::instrument(err, skip(stderr))]
    async fn read_stderr(
        mut stderr: BufReader<ChildStderr>,
        socket: Arc<UdpSocket>,
        address: SocketAddr,
    ) -> Result<(), Error> {
        let mut buf = String::new();
        loop {
            let n = stderr.read_line(&mut buf).await.unwrap();
            if n == 0 {
                return Err(Error::Closed);
            }
            let line = &buf[..n];
            socket
                .send_to(
                    &serde_json::to_vec(&ServerFrame::UpdateState {
                        state: line.as_bytes().to_vec(),
                    })
                    .unwrap(),
                    address,
                )
                .await?;
            tracing::error!("stderr: {:?}", line);
        }
    }
}
