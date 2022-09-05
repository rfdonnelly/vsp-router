use camino::{Utf8Path, Utf8PathBuf};
use nix::errno::Errno;
use nix::fcntl::OFlag;
use nix::pty::{grantpt, posix_openpt, ptsname_r, unlockpt, PtyMaster};
use nix::sys::termios::{tcgetattr, cfmakeraw, tcsetattr, SetArg, LocalFlags};
use thiserror::Error;
use tokio_serial::{SerialPort, SerialStream};
use tokio::fs::File;
use tokio::io::{ReadHalf, WriteHalf};
use tracing::info;

use std::ffi::CStr;
use std::fs;
use std::os::unix::{self, io::FromRawFd, io::IntoRawFd};
use std::os::unix::io::AsRawFd;
use std::os::unix::prelude::RawFd;
use std::os::unix::fs::FileTypeExt;

#[derive(Error, Debug)]
pub enum Error {
    #[error("could not create pty")]
    Pty(#[source] Errno),

    #[error("could not create link to pty")]
    Link(#[source] std::io::Error),

    #[error("serial error")]
    Serial(#[source] tokio_serial::Error),
}

pub struct Pty {
    manager: PtyMaster,
    name: String,
}

pub struct Tty {
    master: SerialStream,
    slave: SerialStream,
}

pub struct PtyLink {
    link: Utf8PathBuf,
}

pub type Result<T> = std::result::Result<T, Error>;

impl Pty {
    pub fn new() -> Result<Self> {
        let manager =
            posix_openpt(OFlag::O_RDWR | OFlag::O_NOCTTY).map_err(|src| Error::Pty(src))?;

        grantpt(&manager).map_err(|src| Error::Pty(src))?;
        unlockpt(&manager).map_err(|src| Error::Pty(src))?;

        let name = ptsname_r(&manager)
            .map_err(|src| Error::Pty(src))?;

        let mut termios = tcgetattr(manager.as_raw_fd())
            .map_err(|src| Error::Pty(src))?;
        cfmakeraw(&mut termios);
        tcsetattr(manager.as_raw_fd(), SetArg::TCSANOW, &termios)
            .map_err(|src| Error::Pty(src))?;

        Ok(Self {
            manager,
            name,
        })
    }

    pub fn link<P: AsRef<Utf8Path>>(&self, path: P) -> Result<PtyLink> {
        let link = path.as_ref().to_path_buf();
        unix::fs::symlink(&self.name, link.as_std_path())
            .map_err(|src| Error::Link(src))?;

        Ok(PtyLink {
            link,
        })
    }

    pub async fn split(self) -> (ReadHalf<File>, WriteHalf<File>) {
        let file = unsafe { File::from_raw_fd(self.manager.into_raw_fd()) };
        info!(metadata = ?file.metadata().await.unwrap());
        tokio::io::split(file)
    }
}

impl Tty {
    pub fn new() -> Result<Self> {
        let (master, slave) = SerialStream::pair()
            .map_err(|src| Error::Serial(src))?;

        Ok(Self {
            master,
            slave,
        })
    }

    pub fn link<P: AsRef<Utf8Path>>(&self, path: P) -> Result<PtyLink> {
        let link = path.as_ref().to_path_buf();
        info!("symlinking {} -> {}", self.slave.name().unwrap(), link);
        unix::fs::symlink(&self.slave.name().unwrap(), link.as_std_path())
            .map_err(|src| Error::Link(src))?;

        Ok(PtyLink {
            link,
        })
    }

    pub async fn split(self) -> (ReadHalf<SerialStream>, WriteHalf<SerialStream>) {
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
