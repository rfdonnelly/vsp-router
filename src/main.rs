use vsp_router::Pty;

use anyhow::anyhow;
use camino::Utf8PathBuf;
use clap::Parser;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio_stream::StreamExt;
use tokio_stream::StreamMap;
use tokio_util::io::ReaderStream;
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
    virtuals: Vec<IdPath>,

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
struct IdPath {
    id: String,
    path: Utf8PathBuf,
}

impl FromStr for IdPath {
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
    for id_path in args.virtuals {
        let pty = Pty::new()?;
        let link = pty.link(&id_path.path)?;
        let (reader, writer) = pty.split();
        sources.insert(id_path.id.clone(), ReaderStream::new(reader));
        sinks.insert(id_path.id.clone(), writer);
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

    transfer(sources, sinks, routes).await?;

    Ok(())
}

async fn transfer<R, W>(
    mut sources: StreamMap<String, ReaderStream<R>>,
    mut sinks: HashMap<String, W>,
    routes: HashMap<String, Vec<String>>,
) -> AppResult<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    while let Some((src_id, result)) = sources.next().await {
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

    Ok(())
}
