use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use tokio::net::UdpSocket;

struct Server {
    socket: UdpSocket,
    view: Vec<u8>,
}

impl Server {
    pub async fn new() -> Result<Self, tokio::io::Error> {
        let socket = socket2::Socket::new(
            socket2::Domain::IPV4,
            socket2::Type::DGRAM,
            Some(socket2::Protocol::UDP),
        )?;
        socket.set_reuse_address(true)?;
        socket.bind(&socket2::SockAddr::from(SocketAddrV4::new(
            Ipv4Addr::LOCALHOST,
            mosh::SERVER_LISTEN_PORT,
        )))?;
        let std_socket = std::net::UdpSocket::from(socket);
        let socket = UdpSocket::from_std(std_socket)?;
        Ok(Self {
            socket,
            view: Default::default(),
        })
    }

    async fn run(mut self) -> Result<(), tokio::io::Error> {
        println!("Starting server");
        use std::io::Write;

        let mut term = console::Term::stdout();
        term.clear_screen().unwrap();
        let mut buf = [0; 1024];
        loop {
            let (n, from) = self.socket.recv_from(&mut buf).await?;
            if n == 0 {
                return Ok(());
            }
            let message = std::str::from_utf8(&buf[0..n]).expect("invalid UTF-8");
            self.view.extend(message.as_bytes());
            term.write(message.as_bytes()).unwrap();
            self.send_view(&from).await?;
        }
    }

    async fn send_view(&self, to: &SocketAddr) -> Result<(), tokio::io::Error> {
        self.socket.send_to(self.view.as_slice(), to).await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = Server::new().await?;
    server.run().await.unwrap();
    Ok(())
}
