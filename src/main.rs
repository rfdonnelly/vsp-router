mod cli;

use crate::cli::Args;

use vsp_router::create_virtual_serial_port;

use anyhow::anyhow;
use clap::Parser;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio_serial::SerialPortBuilderExt;
use tokio_stream::{StreamExt, StreamMap};
use tokio_util::io::ReaderStream;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use std::collections::HashMap;

type AppError = anyhow::Error;
type AppResult<T> = anyhow::Result<T>;

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
        let port =
            tokio_serial::new(physical.path.as_str(), physical.baud_rate).open_native_async()?;
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
    let shutdown_token_clone = shutdown_token.clone();

    tokio::spawn(async move {
        match tokio::signal::ctrl_c().await {
            Ok(()) => info!("received ctrl-c, shutting down"),
            Err(e) => error!(?e, "unable to listen for shutdown signal"),
        }

        shutdown_token.cancel();
        info!("waiting for graceful shutdown");
    });

    transfer(sources, sinks, routes, shutdown_token_clone).await?;

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
