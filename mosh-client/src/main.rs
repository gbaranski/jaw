mod session;

use mosh::ClientFrame;
use session::Session;
use std::net::{Ipv4Addr, SocketAddrV4};

fn init_logging() {
    const LOG_ENV: &str = "RUST_LOG";
    use std::str::FromStr;
    use tracing::Level;

    let level = std::env::var(LOG_ENV)
        .map(|env| {
            Level::from_str(env.to_uppercase().as_str())
                .unwrap_or_else(|err| panic!("invalid `{}` environment variable {}", LOG_ENV, err))
        })
        .unwrap_or(Level::INFO);

    tracing_subscriber::fmt()
        .with_writer(|| {
            let log_file_path = xdg::BaseDirectories::with_prefix("mosh-rust")
                .unwrap()
                .get_cache_home()
                .join("mosh.log");
            if !log_file_path.exists() {
                std::fs::create_dir_all(&log_file_path.parent().unwrap()).unwrap();
            }
            let log_file = std::fs::OpenOptions::new()
                .read(true)
                .append(true)
                .create(true)
                .open(log_file_path)
                .unwrap();
            log_file
        })
        .with_max_level(level)
        .init();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();
    let session =
        Session::create(SocketAddrV4::new(Ipv4Addr::LOCALHOST, mosh::PORT).into()).await?;

    let run_session_task = {
        let session = session.clone();
        tokio::spawn(async move {
            session.run().await.unwrap();
        })
    };

    let read_input_task = {
        tokio::spawn(async move {
            read_input(session).await.unwrap();
        })
    };

    tokio::select! {
        value = run_session_task => value.unwrap(),
        value = read_input_task => value.unwrap(),
    };
    Ok(())
}

async fn read_input(session: Session) -> Result<(), Box<dyn std::error::Error>> {
    let mut characters = [0; 1];
    let term = console::Term::stdout();
    loop {
        let character = term.read_char().unwrap();
        character.encode_utf8(&mut characters);
        session
            .send(ClientFrame::Write {
                session_id: session.id.clone(),
                bytes: characters.to_vec(),
            })
            .await?;
    }
}
