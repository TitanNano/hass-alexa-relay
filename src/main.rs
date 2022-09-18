mod lambda;
mod wireguard;

use std::{
    future::Future,
    net::{IpAddr, SocketAddr, ToSocketAddrs},
    task::{Context as TaskContext, Poll},
};

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use lambda::lambda_handler;
use lambda_runtime::Service;
use onetun::config::{X25519PublicKey, X25519SecretKey};
use wireguard::start_wireguard;

#[derive(Parser)]
#[clap(version, author)]
struct Args {
    #[clap(flatten)]
    wireguard: WireguardArgs,

    /// IP address and port of the home assistant instance on the local network
    #[clap(env, short, long, parse(try_from_str = to_socket_addrs))]
    ha_host: SocketAddr,

    /// The desired log level. Options are 'error', 'info', 'debug'
    #[clap(env, short, long, default_value = "info")]
    log_level: String,

    /// an optional home assistant log lived access token for testing.
    #[clap(env, short = 't', long)]
    long_lived_access_token: Option<String>,
}

/// Wireguard specific arguments
#[derive(clap::Args)]
struct WireguardArgs {
    /// The wireguard endpoint that consists of either domain name or ip + port
    #[clap(env, short, long, parse(try_from_str = to_socket_addrs))]
    endpoint: SocketAddr,
    // Wireguard priavate key for this peer
    #[clap(env, short, long)]
    private_key: X25519SecretKey,
    /// Wireguard public key
    #[clap(env, short = 'k', long)]
    public_key: X25519PublicKey,
    /// The peer ip that has been choosen for this peer
    #[clap(env, short, long)]
    source_peer_ip: IpAddr,
}

fn init_logger(log_level: &str) -> anyhow::Result<()> {
    let mut builder = pretty_env_logger::formatted_timed_builder();
    builder.parse_filters(log_level);
    builder
        .try_init()
        .with_context(|| "Failed to initialize logger")
}

struct LambdaService<T> {
    f: T,
    access_token: Option<String>,
}

impl<T> LambdaService<T> {
    fn new(f: T) -> Self {
        Self {
            f,
            access_token: None,
        }
    }

    fn set_access_token(&mut self, token: String) {
        self.access_token = Some(token);
    }
}

impl<T, F, Request, R, E> Service<Request> for LambdaService<T>
where
    T: FnMut(Request, Option<String>) -> F,
    F: Future<Output = Result<R, E>>,
{
    type Response = R;
    type Error = E;
    type Future = F;

    fn poll_ready(&mut self, _: &mut TaskContext<'_>) -> Poll<Result<(), E>> {
        Ok(()).into()
    }

    fn call(&mut self, req: Request) -> Self::Future {
        (self.f)(req, self.access_token.as_ref().map(|t| t.to_owned()))
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = Args::parse();

    init_logger(&args.log_level)?;

    start_wireguard(
        args.wireguard.endpoint,
        args.wireguard.private_key,
        args.wireguard.public_key,
        args.wireguard.source_peer_ip,
        args.ha_host,
        args.log_level,
    )
    .await?;

    let mut handler = LambdaService::new(lambda_handler);

    if let Some(token) = args.long_lived_access_token {
        handler.set_access_token(token);
    }

    lambda_runtime::run(handler).await
}

fn to_socket_addrs<T>(value: T) -> Result<SocketAddr>
where
    T: ToSocketAddrs,
{
    value
        .to_socket_addrs()
        .context("Error during address resolution")?
        .next()
        .ok_or_else(|| anyhow!("Failed to resolve socket address!"))
}
