mod cli;

use crate::cli::Cli;

use vsp_router::{create_virtual_serial_port, open_physical_serial_port, transfer};

use clap::Parser;
use futures_util::future::{AbortHandle, Abortable, Aborted};
use tokio_stream::StreamMap;
use tokio_util::io::ReaderStream;
use tracing::{error, info};

use std::collections::HashMap;

type AppError = anyhow::Error;
type AppResult<T> = anyhow::Result<T>;

#[tokio::main]
async fn main() -> AppResult<()> {
    tracing_subscriber::fmt::init();

    let args = Cli::parse();
    args.validate()?;
    // TODO: Warn on non-routed sources

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
        let port = open_physical_serial_port(&physical.path, physical.baud_rate)?;
        let (reader, writer) = tokio::io::split(port);
        sources.insert(physical.id.clone(), ReaderStream::new(reader));
        sinks.insert(physical.id.clone(), writer);
    }

    let mut routes: HashMap<String, Vec<String>> = HashMap::new();
    for route in args.routes {
        routes
            .entry(route.src)
            .or_insert(Vec::new())
            .push(route.dst);
    }
    info!(?routes);

    let (abort_handle, abort_registration) = AbortHandle::new_pair();

    tokio::spawn(async move {
        match tokio::signal::ctrl_c().await {
            Ok(()) => info!("received ctrl-c, shutting down"),
            Err(e) => error!(?e, "unable to listen for shutdown signal"),
        }

        abort_handle.abort();
        info!("waiting for graceful shutdown");
    });

    let abort_result = Abortable::new(transfer(sources, sinks, routes), abort_registration).await;
    match abort_result {
        Ok(transfer_result) => transfer_result?,
        Err(Aborted) => {}
    }

    Ok(())
}
