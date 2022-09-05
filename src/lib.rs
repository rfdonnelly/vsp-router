use camino::{Utf8Path, Utf8PathBuf};
use thiserror::Error;
use tokio::io::{ReadHalf, WriteHalf};
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

pub struct Pty {
    master: SerialStream,
    slave: SerialStream,
}

pub struct PtyLink {
    link: Utf8PathBuf,
}

pub type Result<T> = std::result::Result<T, Error>;

impl Pty {
    pub fn new() -> Result<Self> {
        let (master, slave) = SerialStream::pair().map_err(|src| Error::Serial(src))?;

        Ok(Self { master, slave })
    }

    pub fn link<P: AsRef<Utf8Path>>(&self, path: P) -> Result<PtyLink> {
        let link = path.as_ref().to_path_buf();
        unix::fs::symlink(&self.slave.name().unwrap(), link.as_std_path())
            .map_err(|src| Error::Link(src))?;

        Ok(PtyLink { link })
    }

    pub fn split(self) -> (ReadHalf<SerialStream>, WriteHalf<SerialStream>) {
        // WORKAROUND: Prevent dropping the slave.  If we drop the slave, it closes the master and
        // reads on the master will fail.
        // TODO: Save the slave until app terminate so we can properly drop and clean it up.
        std::mem::forget(self.slave);
        tokio::io::split(self.master)
    }
}

impl PtyLink {
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
