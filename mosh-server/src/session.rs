use dashmap::DashMap;
use mosh::session::ID;
use std::process::Stdio;
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, BufReader, BufWriter},
    process::{Child, ChildStderr, ChildStdin, ChildStdout, Command},
};

pub type Store = DashMap<ID, ()>;

#[derive(Debug)]
pub struct Session {
    process: Child,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io: {0}")]
    IO(std::io::Error),
    #[error("IO has been closed")]
    Closed,
}

impl Session {
    pub async fn new() -> Result<Self, Error> {
        let process = Command::new("sh")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .expect("failed to spawn child command");

        Ok(Self { process })
    }

    pub async fn run(mut self) -> Result<(), Error> {
        // let stdin = BufWriter::new(self.process.stdin.take().expect("open stdin fail"));
        let stdout = BufReader::new(self.process.stdout.take().expect("open stdout fail"));
        let stderr = BufReader::new(self.process.stderr.take().expect("open stderr fail"));
        let stdout_task =
            tokio::spawn(async move { Self::read_stdout(stdout).await.expect("stdout error") });
        let stderr_task =
            tokio::spawn(async move { Self::read_stderr(stderr).await.expect("stderr error") });
        tokio::select! {
            value = stdout_task => value.unwrap(),
            value = stderr_task => value.unwrap(),
        };
        Ok(())
    }

    #[tracing::instrument(err, skip(stdout))]
    async fn read_stdout(mut stdout: BufReader<ChildStdout>) -> Result<(), Error> {
        let mut buf = String::new();
        loop {
            let n = stdout.read_to_string(&mut buf).await.unwrap();
            if n == 0 {
                return Err(Error::Closed);
            }
            let line = &buf[..n];
            tracing::info!("stdout: {:?}", line);
        }
    }

    #[tracing::instrument(err, skip(stderr))]
    async fn read_stderr(mut stderr: BufReader<ChildStderr>) -> Result<(), Error> {
        let mut buf = String::new();
        loop {
            let n = stderr.read_to_string(&mut buf).await.unwrap();
            if n == 0 {
                return Err(Error::Closed);
            }
            let line = &buf[..n];
            tracing::error!("stderr: {:?}", line);
        }
    }
}
