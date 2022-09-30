mod lambda;
mod wireguard;

use std::{
    future::Future,
    net::{IpAddr, SocketAddr, ToSocketAddrs},
    sync::Arc,
    task::{Context as TaskContext, Poll},
};

use anyhow::{anyhow, Context, Result};
use clap::{builder::ValueParser, ArgAction, Parser};
use lambda::lambda_handler;
use lambda_runtime::Service;
use onetun::config::{X25519PublicKey, X25519SecretKey};
use std::str::FromStr;
use wireguard::start_wireguard;

#[derive(Parser)]
#[command(version, author, disable_help_flag = true)]
struct Args {
    #[command(flatten)]
    wireguard: WireguardArgs,

    /// IP address and port of the home assistant instance on the local network
    #[arg(env, short, long)]
    ha_host: SocketAddr,

    /// The desired log level. Options are 'error', 'info', 'debug'
    #[arg(env, short, long, default_value = "info")]
    log_level: String,

    /// an optional home assistant log lived access token for testing.
    #[arg(env, short = 't', long)]
    long_lived_access_token: Option<String>,

    /// Print help information
    #[arg(long, action = ArgAction::Help, global = true)]
    help: bool,
}

/// Wireguard specific arguments
#[derive(clap::Args)]
struct WireguardArgs {
    /// The wireguard endpoint that consists of either domain name or ip + port
    #[arg(env, short, long, value_parser(ValueParser::new(to_socket_addrs)))]
    endpoint: SocketAddr,
    // Wireguard priavate key for this peer
    #[arg(env, short, long, value_parser(ValueParser::new(parse_secret_key)))]
    private_key: Arc<X25519SecretKey>,
    /// Wireguard public key
    #[arg(
        env,
        short = 'k',
        long,
        value_parser(ValueParser::new(parse_public_key))
    )]
    public_key: Arc<X25519PublicKey>,
    /// The peer ip that has been choosen for this peer
    #[arg(env, short, long)]
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

fn to_socket_addrs(value: &str) -> Result<SocketAddr> {
    value
        .to_socket_addrs()
        .context("Error during address resolution")?
        .next()
        .ok_or_else(|| anyhow!("Failed to resolve socket address!"))
}

fn parse_secret_key(value: &str) -> Result<Arc<X25519SecretKey>> {
    X25519SecretKey::from_str(value)
        .map(Arc::new)
        .map_err(|text| anyhow!(text))
        .context("Failed to parse private key")
}

fn parse_public_key(value: &str) -> Result<Arc<X25519PublicKey>> {
    X25519PublicKey::from_str(value)
        .map(Arc::new)
        .map_err(|text| anyhow!(text))
        .context("Failed to parse private key")
}
