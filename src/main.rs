use vsp_router::create_virtual_serial_port;

use anyhow::anyhow;
use camino::Utf8PathBuf;
use clap::Parser;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio_stream::{StreamExt, StreamMap};
use tokio_util::io::ReaderStream;
use tokio_util::sync::CancellationToken;
use tracing::info;

use std::collections::HashMap;
use std::str::FromStr;

type AppError = anyhow::Error;
type AppResult<T> = anyhow::Result<T>;

#[derive(Parser)]
#[clap(author, version, about)]
struct Args {
    /// Create a virtual serial port node.  Can use multiple times to create multiple virtual
    /// serial ports.
    #[clap(long = "virtual", id = "VIRTUAL")]
    virtuals: Vec<Virtual>,

    /// Create a route between a source node and a destination node in the form of
    /// '<src-id>:<dst-id>'.  Can use multiple times to create multiple routes.
    #[clap(long = "route", id = "ROUTE")]
    routes: Vec<Route>,

    /// Create a node attached to a physical serial port.  Can use multiple times to attached
    /// multiple physical serial ports.
    #[clap(long = "physical", id = "PHYSICAL")]
    physicals: Vec<String>,
}

#[derive(Clone, Debug)]
struct Virtual {
    id: String,
    path: Utf8PathBuf,
}

impl FromStr for Virtual {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.split_once(':') {
            None => {
                let path = Utf8PathBuf::from(s);
                let id = path
                    .file_name()
                    .ok_or(anyhow!("invalid path '{s}'"))?
                    .to_owned();
                Ok(Self { id, path })
            }
            Some((id, path)) => {
                let id = id.to_owned();
                let path = Utf8PathBuf::from(path);
                Ok(Self { id, path })
            }
        }
    }
}

#[derive(Clone, Debug)]
struct Route {
    src: String,
    dst: String,
}

impl FromStr for Route {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (src, dst) = s.split_once(':').ok_or(anyhow!("invalid route '{s}'"))?;
        Ok(Self {
            src: src.to_string(),
            dst: dst.to_string(),
        })
    }
}

#[tokio::main]
async fn main() -> AppResult<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    let mut sources = StreamMap::new();
    let mut sinks = HashMap::new();
    let mut links = Vec::new();

    for virtual_ in args.virtuals {
        let (port, link) = create_virtual_serial_port(&virtual_.path)?;
        let (reader, writer) = tokio::io::split(port);
        sources.insert(virtual_.id.clone(), ReaderStream::new(reader));
        sinks.insert(virtual_.id.clone(), writer);
        links.push(link);
    }

    // TODO: Don't include non-routed sources
    // TODO: Warn on non-routed sources
    // TODO: Validate IDs in routes

    let mut routes: HashMap<String, Vec<String>> = HashMap::new();
    for route in args.routes {
        routes
            .entry(route.src)
            .or_insert(Vec::new())
            .push(route.dst);
    }
    info!(?routes);

    let shutdown_token = CancellationToken::new();
    let join_handle = tokio::spawn(transfer(sources, sinks, routes, shutdown_token.clone()));

    // TODO: Fix case where transfer() returns an error but we still wait on ctrl-c
    tokio::signal::ctrl_c().await?;
    info!("received ctrl-c");
    shutdown_token.cancel();
    info!("waiting for graceful shutdown");
    join_handle.await??;

    Ok(())
}

#[tracing::instrument(skip_all)]
async fn transfer<R, W>(
    mut sources: StreamMap<String, ReaderStream<R>>,
    mut sinks: HashMap<String, W>,
    routes: HashMap<String, Vec<String>>,
    shutdown_token: CancellationToken,
) -> AppResult<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    loop {
        tokio::select! {
            next = sources.next() => {
                match next {
                    None => return Err(anyhow!("channel closed")),
                    Some((src_id, result)) => {
                        // TODO: Unwrap will be OK when non-routed sources are filtered
                        let dst_ids = routes.get(&src_id).unwrap();
                        let bytes = result?;
                        info!(?src_id, ?dst_ids, ?bytes, "read");
                        for dst_id in dst_ids {
                            // TODO: Unwrap will be OK when IDs in routes are validated
                            let dst = sinks.get_mut(dst_id).unwrap();
                            let mut buf = bytes.clone();
                            dst.write_all_buf(&mut buf).await?;
                            info!(?dst_id, ?bytes, "wrote");
                        }
                    }
                }
            }
            _ = shutdown_token.cancelled() => {
                return Ok(());
            }
        }
    }
}
