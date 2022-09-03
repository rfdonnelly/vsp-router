use camino::{Utf8Path, Utf8PathBuf};
use nix::errno::Errno;
use nix::fcntl::{OFlag};
use nix::pty::{grantpt, posix_openpt, ptsname_r, unlockpt, PtyMaster};
use thiserror::Error;

use std::os::unix;
use std::fs;

#[derive(Error, Debug)]
pub enum VspRouterError {
    #[error("could not create pty")]
    PtyCreate { source: Errno },

    #[error("could not create link to pty")]
    Link { source: std::io::Error },
}

pub struct Pty {
    manager: PtyMaster,
    name: String,
    link: Utf8PathBuf,
}

type LibError = VspRouterError;
type LibResult<T> = Result<T, LibError>;

impl Pty {
    pub fn new<P: AsRef<Utf8Path>>(link: P) -> LibResult<Self> {
        let manager = posix_openpt(OFlag::O_RDWR)
            .map_err(|source| VspRouterError::PtyCreate { source })?;

        grantpt(&manager)
            .map_err(|source| VspRouterError::PtyCreate { source })?;
        unlockpt(&manager)
            .map_err(|source| VspRouterError::PtyCreate { source })?;

        let name = ptsname_r(&manager)
            .map_err(|source| VspRouterError::PtyCreate { source })?;
        unix::fs::symlink(&name, link.as_ref().as_std_path())
            .map_err(|source| VspRouterError::Link { source })?;

        Ok(Self {
            manager,
            name,
            link: link.as_ref().to_path_buf(),
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn link(&self) -> &Utf8Path {
        &self.link.as_path()
    }
}

impl Drop for Pty {
    fn drop(&mut self) {
        if let Err(_) = fs::remove_file(&self.link) {
            eprintln!("error: could not delete {}", self.link);
        }
    }
}
