use better_mosh::{MULTICAST_IPV4, PORT};
use std::net::{Ipv4Addr, SocketAddrV4};
use tokio::net::UdpSocket;

struct Server {
    socket: UdpSocket,
    buffer: Vec<u8>,
}

impl Server {
    pub async fn new() -> Result<Self, tokio::io::Error> {
        let socket = socket2::Socket::new(
            socket2::Domain::IPV4,
            socket2::Type::DGRAM,
            Some(socket2::Protocol::UDP),
        )?;
        socket.set_reuse_address(true)?;
        socket.set_multicast_if_v4(&Ipv4Addr::UNSPECIFIED)?;
        socket.bind(&socket2::SockAddr::from(SocketAddrV4::new(
            Ipv4Addr::UNSPECIFIED,
            PORT,
        )))?;
        let std_socket = std::net::UdpSocket::from(socket);
        let socket = UdpSocket::from_std(std_socket)?;
        Ok(Self {
            socket,
            buffer: Vec::new(),
        })
    }

    async fn run(self) -> Result<(), tokio::io::Error> {
        println!("Starting server");
        use std::io::Write;

        let mut term = console::Term::stdout();
        term.clear_screen().unwrap();
        let mut buf = [0; 1024];
        loop {
            let n = self.socket.recv(&mut buf).await?;
            if n == 0 {
                return Ok(());
            }
            let message = std::str::from_utf8(&buf[0..n]).expect("invalid UTF-8");
            term.write(message.as_bytes()).unwrap();
            // self.multicast_buffer().await?;
        }
    }

    async fn multicast_buffer(&self) -> Result<(), tokio::io::Error> {
        self.socket
            .send_to(b"hello world", SocketAddrV4::new(MULTICAST_IPV4, PORT))
            .await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = Server::new().await?;
    server.run().await.unwrap();
    Ok(())
}
