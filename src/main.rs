use vsp_router::Pty;

use anyhow::{anyhow, Context};
use clap::Parser;

use std::str::FromStr;

type AppError = anyhow::Error;
type AppResult<T> = anyhow::Result<T>;

#[derive(Parser)]
#[clap(author, version, about)]
struct Args {
    /// Create a virtual serial port
    #[clap(long = "virtual")]
    virtuals: Vec<String>,

    #[clap(long = "route", value_parser)]
    routes: Vec<Route>,
}

#[derive(Clone)]
struct Route {
    src: String,
    dst: String,
}

impl FromStr for Route {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (src, dst) = s.split_once(':').ok_or(anyhow!("invalid route '{}'", s))?;
        Ok(Self { src: src.to_string(), dst: dst.to_string(), })
    }
}

fn main() -> AppResult<()> {
    let args = Args::parse();

    let ptys = args.virtuals
        .iter()
        .map(|path| Pty::new(path))
        .collect::<Result<Vec<_>, _>>()?;

    for pty in ptys {
        println!("pty: {} -> {}", pty.link(), pty.name());
    }

    for route in args.routes {
        println!("route: {} -> {}", route.src, route.dst);
    }

    Ok(())
}
