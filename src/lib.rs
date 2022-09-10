use camino::{Utf8Path, Utf8PathBuf};
use thiserror::Error;
use tokio_serial::{SerialPort, SerialStream};

use std::fs;
use std::os::unix;

#[derive(Error, Debug)]
pub enum Error {
    #[error("could not create link to pty")]
    Link(#[source] std::io::Error),

    #[error("serial error")]
    Serial(#[source] tokio_serial::Error),
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
    let (manager, subordinate) = SerialStream::pair().map_err(|src| Error::Serial(src))?;
    let link = PtyLink::new(subordinate, path)?;

    Ok((manager, link))
}

impl PtyLink {
    fn new<P: AsRef<Utf8Path>>(subordinate: SerialStream, path: P) -> Result<Self> {
        let link = path.as_ref().to_path_buf();
        unix::fs::symlink(&subordinate.name().unwrap(), link.as_std_path())
            .map_err(|src| Error::Link(src))?;

        Ok(PtyLink {
            _subordinate: subordinate,
            link,
        })
    }

    pub fn link(&self) -> &Utf8Path {
        &self.link.as_path()
    }

    pub fn id(&self) -> &str {
        &self.link.as_str()
    }
}

impl Drop for PtyLink {
    fn drop(&mut self) {
        if let Err(_) = fs::remove_file(&self.link) {
            eprintln!("error: could not delete {}", self.link);
        }
    }
}
