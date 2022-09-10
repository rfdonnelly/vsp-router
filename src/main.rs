use vsp_router::create_virtual_serial_port;

use anyhow::anyhow;
use camino::Utf8PathBuf;
use clap::Parser;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio_serial::SerialPortBuilderExt;
use tokio_stream::{StreamExt, StreamMap};
use tokio_util::io::ReaderStream;
use tokio_util::sync::CancellationToken;
use tracing::info;

use std::collections::HashMap;
use std::str::FromStr;

type AppError = anyhow::Error;
type AppResult<T> = anyhow::Result<T>;

const CLAP_AFTER_HELP: &str = "
EXAMPLES:

    Share a physical serial port with two virtual serial ports.

    Data sent from virtual serial port 0 is sent to the physical serial port but not to virtual
    serial port 1.  Similarly, data sent from virtual serial port 1 is sent to the physical serial
    port but not to virtual serial port 0.  Data received fromt the physical serial port is sent to
    both virtual serial ports.

    vsp-router \\
        --virtual 0 \\
        --virtual 1 \\
        --physical 2:/dev/ttyUSB0,115200 \\
        --route 0:2 \\
        --route 1:2 \\
        --route 2:0 \\
        --route 2:1
";

#[derive(Parser)]
#[clap(author, version, about, after_help = CLAP_AFTER_HELP)]
struct Args {
    /// Create a virtual serial port.
    ///
    /// The argument takes the following form: '[<id>:]<path>'
    ///
    /// If no ID is specified, the ID is set to the basename of the path.
    ///
    /// Can use multiple times to create multiple virtual serial ports.
    ///
    /// Examples:
    ///
    /// --virtual path/to/file
    ///
    ///     The path is 'path/to/file' and the ID is 'file'.
    ///
    /// --virtual 0:dev/ttyUSB0
    ///
    ///     The path is '/dev/ttyUSB0' and the ID is '0'.
    #[clap(long = "virtual", id = "VIRTUAL", verbatim_doc_comment)]
    virtuals: Vec<Virtual>,

    /// Create a route between a source port and a destination port.
    ///
    /// The argument takes the following form: '<src-id>:<dst-id>'
    ///
    /// Can use multiple times to create multiple routes.
    ///
    /// Examples:
    ///
    /// --virtual 0:1
    ///
    ///     The source ID is '0' and the destination ID is '1'.
    #[clap(long = "route", id = "ROUTE", verbatim_doc_comment)]
    routes: Vec<Route>,

    /// Open a physical serial port.
    ///
    /// The argument takes the following form: '[<id>:]<path>[,<baud-rate>]'
    ///
    /// If ID is not specified, the ID is set to the basename of the path. If baud rate is not
    /// specificed, the baud rate defaults to 9600.
    ///
    /// Can use multiple times to attached multiple physical serial ports.
    ///
    /// Examples:
    ///
    /// --physical /dev/ttyUSB0
    ///
    ///     The path is '/dev/ttyUSB0', the ID is 'ttyUSB0', and the baud rate is 9600.
    ///
    /// --physical 1:/dev/ttyUSB0
    ///
    ///     The path is '/dev/ttyUSB0', the ID is '1', and the baud rate is 9600.
    ///
    /// --physical 1:/dev/ttyUSB0,115200
    ///
    ///     The path is '/dev/ttyUSB0', the ID is '1', and the baud rate is 115200.
    #[clap(long = "physical", id = "PHYSICAL", verbatim_doc_comment)]
    physicals: Vec<Physical>,
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

#[derive(Clone, Debug)]
struct Physical {
    id: String,
    path: Utf8PathBuf,
    baud_rate: u32,
}

impl FromStr for Physical {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (remainder, baud_rate) = match s.split_once(',') {
            None => (s, 9600),
            Some((remainder, baud_rate)) => {
                let baud_rate = baud_rate.parse()?;
                (remainder, baud_rate)
            }
        };

        let id_path = Virtual::from_str(remainder)?;

        Ok(Physical {
            id: id_path.id,
            path: id_path.path,
            baud_rate,
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

    for physical in args.physicals {
        let port = tokio_serial::new(physical.path.as_str(), physical.baud_rate).open_native_async()?;
        let (reader, writer) = tokio::io::split(port);
        sources.insert(physical.id.clone(), ReaderStream::new(reader));
        sinks.insert(physical.id.clone(), writer);
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
