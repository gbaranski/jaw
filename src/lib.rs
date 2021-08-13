use std::net::Ipv4Addr;

pub const MULTICAST_IPV4: Ipv4Addr = Ipv4Addr::new(239, 255, 42, 98);
pub const PORT: u16 = 8080;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
