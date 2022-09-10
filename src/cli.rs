use crate::AppError;

use anyhow::anyhow;
use camino::Utf8PathBuf;
use clap::Parser;

use std::str::FromStr;

#[derive(Parser)]
#[clap(author, version, about, after_help = CLAP_AFTER_HELP)]
pub(crate) struct Args {
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
    pub(crate) virtuals: Vec<Virtual>,

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
    pub(crate) physicals: Vec<Physical>,

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
    pub(crate) routes: Vec<Route>,
}

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

#[derive(Clone, Debug)]
pub(crate) struct Virtual {
    pub(crate) id: String,
    pub(crate) path: Utf8PathBuf,
}

#[derive(Clone, Debug)]
pub(crate) struct Physical {
    pub(crate) id: String,
    pub(crate) path: Utf8PathBuf,
    pub(crate) baud_rate: u32,
}

#[derive(Clone, Debug)]
pub(crate) struct Route {
    pub(crate) src: String,
    pub(crate) dst: String,
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
