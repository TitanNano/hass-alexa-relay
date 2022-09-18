use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

use anyhow::{Context, Result};
use onetun::{
    config::{PortForwardConfig, PortProtocol, X25519PublicKey, X25519SecretKey},
    events::Bus,
};

pub async fn start_wireguard(
    endpoint: SocketAddr,
    private_key: X25519SecretKey,
    public_key: X25519PublicKey,
    source_peer_ip: IpAddr,
    ha_host: SocketAddr,
    log_level: String,
) -> Result<()> {
    let config = onetun::config::Config {
        private_key: Arc::new(private_key),
        endpoint_public_key: Arc::new(public_key),
        endpoint_addr: endpoint,
        source_peer_ip,
        keepalive_seconds: Some(5),
        port_forwards: vec![PortForwardConfig {
            source: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080),
            destination: ha_host,
            protocol: PortProtocol::Tcp,
            remote: false,
        }],
        remote_port_forwards: Vec::with_capacity(0),
        endpoint_bind_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0),
        max_transmission_unit: 1420,
        log: log_level,
        warnings: Vec::with_capacity(0),
        pcap_file: None,
    };

    let bus = Bus::new();

    onetun::start_tunnels(config, bus)
        .await
        .context("Failed to create wireguard tunnel")
}
