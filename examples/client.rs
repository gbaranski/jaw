use better_mosh::{MULTICAST_IPV4, PORT};
use std::io::Write;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::Arc;
use tokio::net::UdpSocket;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let socket = socket2::Socket::new(
        socket2::Domain::IPV4,
        socket2::Type::DGRAM,
        Some(socket2::Protocol::UDP),
    )?;
    socket.join_multicast_v4(&MULTICAST_IPV4, &Ipv4Addr::UNSPECIFIED)?;
    socket.set_reuse_address(true)?;
    socket.bind(&SocketAddrV4::new(MULTICAST_IPV4, PORT).into())?;
    let std_socket = std::net::UdpSocket::from(socket);
    let socket = UdpSocket::from_std(std_socket)?;
    println!("Starting client");

    let term = console::Term::stdout();
    let socket = Arc::new(socket);
    tokio::select! {
        value = read_socket(socket.clone(),term.clone()) => value?,
        value = read_terminal(socket.clone(), term.clone()) => value?,
    };
    Ok(())
}

async fn read_socket(
    socket: Arc<UdpSocket>,
    mut term: console::Term,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = [0; 1024];
    loop {
        let n = socket.recv(&mut buffer).await?;
        let str = std::str::from_utf8(&buffer).unwrap();
        term.write(str[0..n].as_bytes())?;
    }
}

async fn read_terminal(
    socket: Arc<UdpSocket>,
    term: console::Term,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut characters = [0; 1];
    loop {
        let character = term.read_char()?;
        character.encode_utf8(&mut characters);
        socket.send_to(&characters, SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, PORT)).await?;
    }
}
