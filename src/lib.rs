use camino::{Utf8Path, Utf8PathBuf};
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio_serial::SerialPortBuilderExt;
use tokio_serial::{SerialPort, SerialStream};
use tokio_stream::{StreamExt, StreamMap};
use tokio_util::io::ReaderStream;
use tracing::{error, info};

use std::collections::HashMap;
use std::fs;
use std::os::unix;

#[derive(Error, Debug)]
pub enum Error {
    #[error("could not create link to pty")]
    Link(#[source] std::io::Error),

    #[error("serial error")]
    Serial(#[source] tokio_serial::Error),

    #[error("stream closed")]
    Closed,

    #[error("read error")]
    Read(#[source] std::io::Error),

    #[error("write error")]
    Write(#[source] std::io::Error),
}

pub struct PtyLink {
    // Not used directly but need to keep around to prevent early close of the file descriptor.
    //
    // tokio_serial::SerialStream includes a mio_serial::SerialStream which includes a
    // serialport::TTY which includes a Drop impl that closes the file descriptor.
    _subordinate: SerialStream,
    link: Utf8PathBuf,
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn create_virtual_serial_port<P>(path: P) -> Result<(SerialStream, PtyLink)>
where
    P: AsRef<Utf8Path>,
{
    let (manager, subordinate) = SerialStream::pair().map_err(Error::Serial)?;
    let link = PtyLink::new(subordinate, path)?;

    Ok((manager, link))
}

pub fn open_physical_serial_port<P>(path: P, baud_rate: u32) -> Result<SerialStream>
where
    P: AsRef<Utf8Path>,
{
    tokio_serial::new(path.as_ref().as_str(), baud_rate)
        .open_native_async()
        .map_err(Error::Serial)
}

impl PtyLink {
    fn new<P: AsRef<Utf8Path>>(subordinate: SerialStream, path: P) -> Result<Self> {
        let link = path.as_ref().to_path_buf();
        unix::fs::symlink(subordinate.name().unwrap(), link.as_std_path()).map_err(Error::Link)?;

        Ok(PtyLink {
            _subordinate: subordinate,
            link,
        })
    }

    pub fn link(&self) -> &Utf8Path {
        self.link.as_path()
    }

    pub fn id(&self) -> &str {
        self.link.as_str()
    }
}

impl Drop for PtyLink {
    fn drop(&mut self) {
        if fs::remove_file(&self.link).is_err() {
            eprintln!("error: could not delete {}", self.link);
        }
    }
}

#[tracing::instrument(skip_all)]
pub async fn transfer<R, W>(
    mut sources: StreamMap<String, ReaderStream<R>>,
    mut sinks: HashMap<String, W>,
    routes: HashMap<String, Vec<String>>,
) -> Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    while let Some((src_id, result)) = sources.next().await {
        if let Some(dst_ids) = routes.get(&src_id) {
            let bytes = result.map_err(Error::Read)?;
            info!(?src_id, ?dst_ids, ?bytes, "read");
            for dst_id in dst_ids {
                // This unwrap is OK as long as we validate all route IDs exist first
                // Route IDs are validated in Args::check_route_ids()
                let dst = sinks.get_mut(dst_id).unwrap();
                let mut buf = bytes.clone();
                dst.write_all_buf(&mut buf).await.map_err(Error::Write)?;
                info!(?dst_id, ?bytes, "wrote");
            }
        }
    }

    Ok(())
}
