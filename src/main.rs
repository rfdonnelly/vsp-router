use vsp_router::Pty;

use anyhow::{anyhow, Context};
use bytes::Buf;
use bytes::BufMut;
use bytes::BytesMut;
use camino::{Utf8Path, Utf8PathBuf};
use clap::Parser;
use futures::stream::FuturesUnordered;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tracing::info;

use std::collections::HashMap;
use std::str::FromStr;

type AppError = anyhow::Error;
type AppResult<T> = anyhow::Result<T>;

#[derive(Parser)]
#[clap(author, version, about)]
struct Args {
    /// Create a virtual serial port node.  Can use multiple times to create multiple virtual
    /// serial ports.
    #[clap(long = "virtual", id = "VIRTUAL")]
    virtuals: Vec<IdPath>,

    /// Create a route between a source node and a destination node in the form of
    /// '<src-id>:<dst-id>'.  Can use multiple times to create multiple routes.
    #[clap(long = "route", id = "ROUTE")]
    routes: Vec<Route>,

    /// Create a node attached to a physical serial port.  Can use multiple times to attached
    /// multiple physical serial ports.
    #[clap(long = "physical", id = "PHYSICAL")]
    physicals: Vec<String>,
}

#[derive(Clone)]
struct IdPath {
    id: String,
    path: Utf8PathBuf,
}

impl FromStr for IdPath {
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

#[derive(Clone)]
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

#[tokio::main]
async fn main() -> AppResult<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    let mut sources = HashMap::new();
    let mut sinks = HashMap::new();
    let mut links = Vec::new();
    for id_path in args.virtuals {
        let pty = Pty::new()?;
        let link = pty.link(&id_path.path)?;
        let (reader, writer) = pty.split();
        sources.insert(id_path.id.clone(), reader);
        sinks.insert(id_path.id.clone(), writer);
        links.push(link);
    }

    let mut routes: HashMap<String, Vec<String>> = HashMap::new();
    for route in args.routes {
        routes.entry(route.src)
            .and_modify(|dsts| dsts.push(route.dst))
            .or_insert(Vec::new());
    }

    let pty0 = Pty::new()?;
    let pty0_link = pty0.link("0")?;
    let pty1 = Pty::new()?;
    let pty1_link = pty1.link("1")?;
    let pty2 = Pty::new()?;
    let pty2_link = pty2.link("2")?;
    let (mut pty0_rd, mut pty0_wr) = pty0.split();
    let (mut pty1_rd, mut pty1_wr) = pty1.split();
    let (mut pty2_rd, mut pty2_wr) = pty2.split();

    let mut buf0 = BytesMut::with_capacity(1024);
    let mut buf1 = BytesMut::with_capacity(1024);
    let mut buf2 = BytesMut::with_capacity(1024);
    loop {
        let (dsts, buf) = tokio::select! {
            result = pty0_rd.read_buf(&mut buf0) => result.map(|_| (vec![&mut pty2_wr], &mut buf0)),
            result = pty1_rd.read_buf(&mut buf1) => result.map(|_| (vec![&mut pty2_wr], &mut buf1)),
            result = pty2_rd.read_buf(&mut buf2) => result.map(|_| (vec![&mut pty0_wr, &mut pty1_wr], &mut buf2)),
        }?;
        info!(
            ?buf,
            remaining = buf.remaining(),
            remaining_mut = buf.remaining_mut(),
            "read data"
        );

        for dst in dsts {
            let mut buf_clone = buf.clone();
            dst.write_all_buf(&mut buf_clone).await?;
            info!(
                ?buf,
                remaining = buf.remaining(),
                remaining_mut = buf.remaining_mut(),
                "wrote data"
            );
        }
        buf.clear();
    }

    // let mut buf0 = [0; 1024];
    // let mut buf1 = [0; 1024];
    // let mut buf2 = [0; 1024];
    // loop{
    //     let (dsts, mut buf, nbytes) = tokio::select!{
    //         result = pty0_rd.read(&mut buf0) => result.map(|nbytes| (vec![&mut pty2_wr], &mut buf0, nbytes)),
    //         result = pty1_rd.read(&mut buf1) => result.map(|nbytes| (vec![&mut pty2_wr], &mut buf1, nbytes)),
    //         result = pty2_rd.read(&mut buf2) => result.map(|nbytes| (vec![&mut pty0_wr, &mut pty1_wr], &mut buf2, nbytes)),
    //     }?;
    //     info!(buf = ?buf[0..nbytes], "read data");

    //     for dst in dsts {
    //         dst.write_all(&mut buf[0..nbytes]).await?;
    //         info!(buf = ?buf[0..nbytes], "wrote data");
    //     }
    // }
}

#[tracing::instrument(skip(src, dst))]
async fn transfer_buf(
    src_id: &str,
    dst_id: &str,
    src: &mut (impl AsyncRead + Unpin),
    dst: &mut (impl AsyncWrite + Unpin),
) -> AppResult<()> {
    let mut buf = BytesMut::with_capacity(1024);
    while let Ok(_nbytes) = src.read_buf(&mut buf).await {
        info!(
            ?buf,
            remaining = buf.remaining(),
            remaining_mut = buf.remaining_mut(),
            "read data"
        );
        dst.write_buf(&mut buf).await?;
        info!(
            ?buf,
            remaining = buf.remaining(),
            remaining_mut = buf.remaining_mut(),
            "wrote data"
        );
    }

    Ok(())
}
