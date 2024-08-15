use bytes::{Buf, Bytes};
use camino::{Utf8Path, Utf8PathBuf};
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncWrite};
#[cfg(unix)]
use tokio_serial::SerialPort;
use tokio_serial::SerialPortBuilderExt;
use tokio_serial::SerialStream;
use tokio_stream::{StreamExt, StreamMap};
use tokio_util::io::ReaderStream;
use tracing::{error, info, warn};

use std::collections::HashMap;
use std::fs;
use std::io::ErrorKind;
use std::pin::Pin;
use std::task::Poll::Ready;

#[cfg(unix)]
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

#[cfg(unix)]
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

#[cfg(unix)]
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
                if let Some(dst) = sinks.get_mut(dst_id) {
                    let mut buf = bytes.clone();
                    if let Err(e) = write_non_blocking(dst, &mut buf).await {
                        if let Error::Write(io_err) = &e {
                            if io_err.kind() == ErrorKind::WouldBlock {
                                warn!(?dst_id, ?bytes, "discarded");
                            } else {
                                error!(?dst_id, ?e, "write error");
                            }
                        }
                    } else {
                        info!(?dst_id, ?bytes, "wrote");
                    }
                }
            }
        }
    }

    Ok(())
}

async fn write_non_blocking<W: AsyncWrite + Unpin>(dst: &mut W, buf: &mut Bytes) -> Result<()> {
    let waker = futures::task::noop_waker();
    let mut cx = futures::task::Context::from_waker(&waker);

    let pinned_dst = Pin::new(dst);
    match pinned_dst.poll_write(&mut cx, buf) {
        Ready(Ok(bytes_written)) => {
            buf.advance(bytes_written);
            Ok(())
        }
        Ready(Err(e)) => Err(Error::Write(e)),
        _ => Err(Error::Write(std::io::Error::new(
            ErrorKind::WouldBlock,
            "Would block",
        ))),
    }
}
