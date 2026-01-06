use std::net::{IpAddr, Ipv4Addr, SocketAddr};

const SERVER_IP_ADDR: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);
pub const SERVER_TCP_ADDR: SocketAddr = SocketAddr::new(SERVER_IP_ADDR, 8000);
pub const SERVER_UPD_ADDR: SocketAddr = SocketAddr::new(SERVER_IP_ADDR, 8001);
pub const HEALTHCHECK_TIMEOUT: u64 = 5;
