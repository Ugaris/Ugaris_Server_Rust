use std::net::{IpAddr, Ipv4Addr, SocketAddr};

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub bind_addr: SocketAddr,
    pub area_id: u16,
    pub mirror_id: u16,
    pub max_players: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 5556),
            area_id: 1,
            mirror_id: 1,
            max_players: 512,
        }
    }
}
